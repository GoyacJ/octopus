use std::{collections::BTreeSet, sync::Arc, time::Instant};

use futures::{future::join_all, StreamExt};
use octopus_sdk_contracts::{
    AssistantEvent, ContentBlock, EventId, Message, PermissionOutcome, RenderLifecycle, Role,
    SessionEvent, SessionId, StopReason, SubagentError, SubagentOutput, SubagentSpec,
    SubagentSummary, ToolCallRequest, Usage,
};
use octopus_sdk_model::ModelRequest;
use octopus_sdk_observability::stable_input_hash;
use octopus_sdk_sandbox::SandboxHandle;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::orchestrator_support::{
    build_request, emit_subagent_permission_trace, emit_subagent_summary_trace,
    emit_subagent_tool_trace, max_turns_reached, memory_error, output_meta, provider_error,
    relative_scratchpad_path, stop_message, storage_error, subagent_render_event, subagent_summary,
    usage_message, NoopAskResolver, NoopEventSink, NoopSecretVault,
};
use crate::{ParentSessionContext, SubagentContext, FILE_REF_THRESHOLD};
use async_trait::async_trait;
use octopus_sdk_tools::{TaskFn, ToolContext, ToolError, ToolResult};

#[derive(Clone)]
pub struct OrchestratorWorkers {
    parent: ParentSessionContext,
    semaphore: Arc<Semaphore>,
}

impl OrchestratorWorkers {
    #[must_use]
    pub fn new(parent: ParentSessionContext, max_concurrency: usize) -> Self {
        Self {
            parent,
            semaphore: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }

    pub async fn run(
        &self,
        specs: Vec<SubagentSpec>,
        inputs: Vec<String>,
    ) -> Vec<Result<SubagentOutput, SubagentError>> {
        let futures = specs
            .into_iter()
            .zip(inputs)
            .map(|(spec, input)| self.run_worker(spec, input))
            .collect::<Vec<_>>();
        let results = join_all(futures).await;
        let successful = results
            .iter()
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect::<Vec<_>>();

        if !successful.is_empty() {
            let summary = Self::fan_in(successful);
            let _ = self.append_parent_summary(&summary).await;
        }

        results
    }

    #[must_use]
    pub fn fan_in(outputs: Vec<SubagentOutput>) -> SubagentOutput {
        let bullets = outputs
            .iter()
            .map(|output| match output {
                SubagentOutput::Summary { text, .. } => format!("- {text}"),
                SubagentOutput::FileRef { path, bytes, .. } => {
                    format!("- file: {} ({} bytes)", path.display(), bytes)
                }
                SubagentOutput::Json { value, .. } => format!("- json: {value}"),
            })
            .collect::<Vec<_>>();
        let allowed_tools = outputs
            .iter()
            .flat_map(|output| output_meta(output).allowed_tools.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let merged = outputs
            .iter()
            .map(output_meta)
            .fold(None, |state, meta| match state {
                None => Some(SubagentSummary {
                    session_id: meta.session_id.clone(),
                    parent_session_id: meta.parent_session_id.clone(),
                    resume_session_id: None,
                    spec_id: "fan-in".into(),
                    agent_role: "coordinator".into(),
                    parent_agent_role: meta.parent_agent_role.clone(),
                    turns: meta.turns,
                    tokens_used: meta.tokens_used,
                    duration_ms: meta.duration_ms,
                    trace_id: meta.trace_id.clone(),
                    span_id: format!("subagent-fan-in:{}", meta.parent_session_id.0),
                    parent_span_id: meta.parent_span_id.clone(),
                    model_id: meta.model_id.clone(),
                    model_version: meta.model_version.clone(),
                    config_snapshot_id: meta.config_snapshot_id.clone(),
                    permission_mode: meta.permission_mode,
                    allowed_tools: allowed_tools.clone(),
                }),
                Some(mut aggregate) => {
                    aggregate.turns = aggregate.turns.saturating_add(meta.turns);
                    aggregate.tokens_used = aggregate.tokens_used.saturating_add(meta.tokens_used);
                    aggregate.duration_ms = aggregate.duration_ms.saturating_add(meta.duration_ms);
                    Some(aggregate)
                }
            })
            .unwrap_or(SubagentSummary {
                session_id: SessionId("fan-in-empty".into()),
                parent_session_id: SessionId("fan-in-empty".into()),
                resume_session_id: None,
                spec_id: "fan-in".into(),
                agent_role: "coordinator".into(),
                parent_agent_role: "main".into(),
                turns: 0,
                tokens_used: 0,
                duration_ms: 0,
                trace_id: "trace:fan-in-empty".into(),
                span_id: "subagent-fan-in:empty".into(),
                parent_span_id: "session:fan-in-empty".into(),
                model_id: "main".into(),
                model_version: "unknown".into(),
                config_snapshot_id: "fan-in-empty".into(),
                permission_mode: octopus_sdk_contracts::PermissionMode::Default,
                allowed_tools,
            });

        SubagentOutput::Summary {
            text: bullets.join("\n"),
            meta: merged,
        }
    }

    #[must_use]
    pub fn into_task_fn(self) -> Arc<dyn TaskFn> {
        Arc::new(OrchestratorTaskFn { workers: self })
    }

    pub async fn run_worker(
        &self,
        spec: SubagentSpec,
        input: impl Into<String>,
    ) -> Result<SubagentOutput, SubagentError> {
        if spec.depth > 2 {
            return Err(SubagentError::DepthExceeded { depth: spec.depth });
        }

        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("subagent semaphore should stay open");
        let input = input.into();
        let mut context = SubagentContext::from_parent(self.parent.clone(), spec.clone());
        let child_session_id = context
            .session_store
            .new_child_session(&context.parent_session, &spec)
            .await
            .map_err(storage_error)?;
        let spawn_meta = subagent_summary(
            &self.parent,
            &context,
            &child_session_id,
            0,
            0,
            0,
            Some(child_session_id.clone()),
        );
        context
            .session_store
            .append(
                &child_session_id,
                subagent_render_event("subagent.spawn", "", &spawn_meta),
            )
            .await
            .map_err(storage_error)?;
        emit_subagent_summary_trace(
            self.parent.trace.tracer.as_ref(),
            "subagent_spawn",
            &spawn_meta,
        );
        context
            .session_store
            .append(
                &child_session_id,
                SessionEvent::UserMessage(Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: input.clone(),
                    }],
                }),
            )
            .await
            .map_err(storage_error)?;
        let started_at = Instant::now();
        let mut transcript = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: input.clone(),
            }],
        }];
        let mut rendered_text = String::new();

        loop {
            let turn = self
                .run_turn(
                    &context,
                    &child_session_id,
                    build_request(&context, &transcript),
                )
                .await?;

            context.on_turn_end(&turn.usage);
            transcript.push(turn.assistant_message.clone());
            if !turn.text.is_empty() {
                if !rendered_text.is_empty() {
                    rendered_text.push_str("\n\n");
                }
                rendered_text.push_str(&turn.text);
            }

            if turn.usage != Usage::default() {
                context
                    .session_store
                    .append(
                        &child_session_id,
                        SessionEvent::AssistantMessage(usage_message(&turn.usage)?),
                    )
                    .await
                    .map_err(storage_error)?;
            }

            let checkpoint_anchor_event_id = context
                .session_store
                .append(
                    &child_session_id,
                    SessionEvent::AssistantMessage(stop_message(turn.stop_reason.clone())?),
                )
                .await
                .map_err(storage_error)?;

            if context.completion_threshold_reached() {
                context
                    .session_store
                    .append(
                        &child_session_id,
                        SessionEvent::Checkpoint {
                            id: "subagent_budget_exceeded".into(),
                            anchor_event_id: checkpoint_anchor_event_id,
                            compaction: None,
                        },
                    )
                    .await
                    .map_err(storage_error)?;
                break;
            }

            if max_turns_reached(&context) || turn.stop_reason != StopReason::ToolUse {
                break;
            }

            let tool_messages = self
                .execute_tool_uses(&context, &child_session_id, &turn.tool_uses)
                .await?;
            if tool_messages.is_empty() {
                break;
            }
            transcript.extend(tool_messages);
        }

        let meta = subagent_summary(
            &self.parent,
            &context,
            &child_session_id,
            context.turns(),
            context.tokens_used(),
            started_at.elapsed().as_millis() as u64,
            Some(child_session_id.clone()),
        );
        emit_subagent_summary_trace(
            self.parent.trace.tracer.as_ref(),
            "subagent_complete",
            &meta,
        );

        if rendered_text.len() > FILE_REF_THRESHOLD {
            context
                .scratchpad
                .write(&child_session_id, &rendered_text)
                .await
                .map_err(memory_error)?;
            return Ok(SubagentOutput::FileRef {
                path: relative_scratchpad_path(&child_session_id),
                bytes: rendered_text.len() as u64,
                meta,
            });
        }

        Ok(SubagentOutput::Summary {
            text: rendered_text,
            meta,
        })
    }

    async fn append_parent_summary(
        &self,
        output: &SubagentOutput,
    ) -> Result<EventId, octopus_sdk_session::SessionError> {
        let meta = output_meta(output);
        let SubagentOutput::Summary { text, .. } = output else {
            return self
                .parent
                .session_store
                .append(
                    &self.parent.session_id,
                    subagent_render_event("subagent.summary", "subagent output", meta),
                )
                .await;
        };

        self.parent
            .session_store
            .append(
                &self.parent.session_id,
                subagent_render_event("subagent.summary", text.clone(), meta),
            )
            .await
    }
}

struct TurnOutput {
    assistant_message: Message,
    tool_uses: Vec<ToolCallRequest>,
    text: String,
    usage: Usage,
    stop_reason: StopReason,
}

struct ToolExecutionRecord {
    result: ToolResult,
    permission_decision: String,
    input_hash: String,
}

impl OrchestratorWorkers {
    async fn run_turn(
        &self,
        context: &SubagentContext,
        child_session_id: &SessionId,
        request: ModelRequest,
    ) -> Result<TurnOutput, SubagentError> {
        let mut stream = context
            .model
            .complete(request)
            .await
            .map_err(provider_error)?;
        let mut text = String::new();
        let mut assistant_blocks = Vec::new();
        let mut tool_uses = Vec::new();
        let mut usage = Usage::default();
        let mut stop_reason = StopReason::EndTurn;

        while let Some(event) = stream.next().await {
            match event.map_err(provider_error)? {
                AssistantEvent::TextDelta(delta) => text.push_str(&delta),
                AssistantEvent::ToolUse { id, name, input } => {
                    tool_uses.push(ToolCallRequest {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    });
                    assistant_blocks.push(ContentBlock::ToolUse { id, name, input });
                }
                AssistantEvent::Usage(next_usage) => usage = next_usage,
                AssistantEvent::PromptCache(_) => {}
                AssistantEvent::MessageStop {
                    stop_reason: next_reason,
                } => {
                    stop_reason = next_reason;
                    break;
                }
            }
        }

        if !text.is_empty() {
            assistant_blocks.insert(0, ContentBlock::Text { text: text.clone() });
        }

        let assistant_message = Message {
            role: Role::Assistant,
            content: assistant_blocks,
        };
        if !assistant_message.content.is_empty() {
            context
                .session_store
                .append(
                    child_session_id,
                    SessionEvent::AssistantMessage(assistant_message.clone()),
                )
                .await
                .map_err(storage_error)?;
        }

        Ok(TurnOutput {
            assistant_message,
            tool_uses,
            text,
            usage,
            stop_reason,
        })
    }

    async fn execute_tool_uses(
        &self,
        context: &SubagentContext,
        child_session_id: &SessionId,
        tool_uses: &[ToolCallRequest],
    ) -> Result<Vec<Message>, SubagentError> {
        let mut tool_messages = Vec::new();

        for call in tool_uses {
            let execution = self
                .execute_tool(context, child_session_id, call.clone())
                .await?;
            emit_subagent_permission_trace(
                self.parent.trace.tracer.as_ref(),
                &self.parent,
                context,
                child_session_id,
                call,
                &execution.permission_decision,
            );
            emit_subagent_tool_trace(
                self.parent.trace.tracer.as_ref(),
                &self.parent,
                context,
                child_session_id,
                call,
                &execution.input_hash,
                &execution.permission_decision,
                execution.result.duration_ms,
                execution.result.is_error,
            );
            let tool_message = Message {
                role: Role::Tool,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: execution.result.content.clone(),
                    is_error: execution.result.is_error,
                }],
            };
            context
                .session_store
                .append(
                    child_session_id,
                    SessionEvent::ToolExecuted {
                        call: call.id.clone(),
                        name: call.name.clone(),
                        duration_ms: execution.result.duration_ms,
                        is_error: execution.result.is_error,
                    },
                )
                .await
                .map_err(storage_error)?;
            if let Some(render) = execution.result.render.clone() {
                context
                    .session_store
                    .append(
                        child_session_id,
                        SessionEvent::Render {
                            blocks: vec![render],
                            lifecycle: RenderLifecycle::tool_phase(
                                octopus_sdk_contracts::RenderPhase::OnToolResult,
                                call.id.clone(),
                                call.name.clone(),
                            ),
                        },
                    )
                    .await
                    .map_err(storage_error)?;
            }
            context
                .session_store
                .append(
                    child_session_id,
                    SessionEvent::AssistantMessage(tool_message.clone()),
                )
                .await
                .map_err(storage_error)?;
            tool_messages.push(tool_message);
        }

        Ok(tool_messages)
    }

    async fn execute_tool(
        &self,
        context: &SubagentContext,
        child_session_id: &SessionId,
        call: ToolCallRequest,
    ) -> Result<ToolExecutionRecord, SubagentError> {
        let input_hash = stable_input_hash(&call.input);
        let Some(tool) = context.tools.get(&call.name) else {
            return Ok(ToolExecutionRecord {
                result: ToolError::Execution {
                    message: format!("tool `{}` is not registered", call.name),
                }
                .as_tool_result(),
                permission_decision: "tool_missing".into(),
                input_hash,
            });
        };

        let (effective_call, permission_decision) = match context.permissions.check(&call).await {
            PermissionOutcome::Allow => (call, "allow".into()),
            PermissionOutcome::Deny { reason } => {
                return Ok(ToolExecutionRecord {
                    result: ToolError::Permission { message: reason }.as_tool_result(),
                    permission_decision: "deny".into(),
                    input_hash,
                });
            }
            PermissionOutcome::AskApproval { prompt } => {
                return Ok(ToolExecutionRecord {
                    result: ToolError::Permission {
                        message: format!("approval required for {}", prompt.kind),
                    }
                    .as_tool_result(),
                    permission_decision: "ask_approval".into(),
                    input_hash,
                });
            }
            PermissionOutcome::RequireAuth { prompt } => {
                return Ok(ToolExecutionRecord {
                    result: ToolError::Permission {
                        message: format!("authentication required for {}", prompt.kind),
                    }
                    .as_tool_result(),
                    permission_decision: "require_auth".into(),
                    input_hash,
                });
            }
        };

        let working_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let tool_context = ToolContext {
            session_id: child_session_id.clone(),
            tool_call_id: Some(effective_call.id.clone()),
            permissions: Arc::clone(&context.permissions),
            sandbox: SandboxHandle::new(working_dir.clone(), vec!["PATH".into()], "noop"),
            session_store: Arc::clone(&context.session_store),
            secret_vault: Arc::new(NoopSecretVault),
            ask_resolver: Arc::new(NoopAskResolver),
            event_sink: Arc::new(NoopEventSink),
            working_dir,
            hooks: Arc::clone(&context.hooks),
            permission_context: context
                .permissions
                .tool_permission_context(context.spec.permission_mode, &effective_call.name),
            cancellation: CancellationToken::new(),
        };

        tool.validate(&effective_call.input)
            .map_err(|error| SubagentError::Provider {
                reason: error.to_string(),
            })?;

        Ok(ToolExecutionRecord {
            result: match tool.execute(tool_context, effective_call.input).await {
                Ok(result) => result,
                Err(error) => error.as_tool_result(),
            },
            permission_decision,
            input_hash,
        })
    }
}

struct OrchestratorTaskFn {
    workers: OrchestratorWorkers,
}

#[async_trait]
impl TaskFn for OrchestratorTaskFn {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError> {
        self.workers
            .run_worker(spec.clone(), input.to_string())
            .await
    }
}
