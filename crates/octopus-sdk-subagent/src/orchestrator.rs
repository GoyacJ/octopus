use std::{path::PathBuf, sync::Arc, time::Instant};

use futures::{future::join_all, StreamExt};
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ContentBlock, EventId, EventSink,
    Message, PermissionOutcome, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, Role,
    SecretValue, SecretVault, SessionEvent, SessionId, StopReason, SubagentError, SubagentOutput,
    SubagentSpec, SubagentSummary, ToolCallRequest, Usage, VaultError,
};
use octopus_sdk_model::{
    CacheControlStrategy, ModelId, ModelRequest, ModelRole, ResponseFormat, ThinkingConfig,
};
use octopus_sdk_sandbox::SandboxHandle;
use serde_json::json;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

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
        let merged = outputs
            .iter()
            .map(output_meta)
            .fold(None, |state, meta| match state {
                None => Some(SubagentSummary {
                    session_id: meta.session_id.clone(),
                    turns: meta.turns,
                    tokens_used: meta.tokens_used,
                    duration_ms: meta.duration_ms,
                    trace_id: format!("fan-in:{}", meta.trace_id),
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
                turns: 0,
                tokens_used: 0,
                duration_ms: 0,
                trace_id: "fan-in:empty".into(),
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

        let meta = SubagentSummary {
            session_id: child_session_id.clone(),
            turns: context.turns(),
            tokens_used: context.tokens_used(),
            duration_ms: started_at.elapsed().as_millis() as u64,
            trace_id: format!("subagent:{}", child_session_id.0),
        };

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
        let SubagentOutput::Summary { text, .. } = output else {
            return self
                .parent
                .session_store
                .append(
                    &self.parent.session_id,
                    parent_summary_event("subagent output".into()),
                )
                .await;
        };

        self.parent
            .session_store
            .append(&self.parent.session_id, parent_summary_event(text.clone()))
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
            let result = self
                .execute_tool(context, child_session_id, call.clone())
                .await?;
            let tool_message = Message {
                role: Role::Tool,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: result.content,
                    is_error: result.is_error,
                }],
            };
            context
                .session_store
                .append(
                    child_session_id,
                    SessionEvent::ToolExecuted {
                        call: call.id.clone(),
                        name: call.name.clone(),
                        duration_ms: result.duration_ms,
                        is_error: result.is_error,
                    },
                )
                .await
                .map_err(storage_error)?;
            if let Some(render) = result.render {
                context
                    .session_store
                    .append(
                        child_session_id,
                        SessionEvent::Render {
                            block: render,
                            lifecycle: RenderLifecycle::OnToolResult,
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
    ) -> Result<ToolResult, SubagentError> {
        let Some(tool) = context.tools.get(&call.name) else {
            return Ok(ToolError::Execution {
                message: format!("tool `{}` is not registered", call.name),
            }
            .as_tool_result());
        };

        let effective_call = match context.permissions.check(&call).await {
            PermissionOutcome::Allow => call,
            PermissionOutcome::Deny { reason } => {
                return Ok(ToolError::Permission { message: reason }.as_tool_result());
            }
            PermissionOutcome::AskApproval { prompt } => {
                return Ok(ToolError::Permission {
                    message: format!("approval required for {}", prompt.kind),
                }
                .as_tool_result());
            }
            PermissionOutcome::RequireAuth { prompt } => {
                return Ok(ToolError::Permission {
                    message: format!("authentication required for {}", prompt.kind),
                }
                .as_tool_result());
            }
        };

        let working_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let tool_context = ToolContext {
            session_id: child_session_id.clone(),
            permissions: Arc::clone(&context.permissions),
            sandbox: SandboxHandle::new(working_dir.clone(), vec!["PATH".into()], "noop"),
            session_store: Arc::clone(&context.session_store),
            secret_vault: Arc::new(NoopSecretVault),
            ask_resolver: Arc::new(NoopAskResolver),
            event_sink: Arc::new(NoopEventSink),
            working_dir,
            cancellation: CancellationToken::new(),
        };

        Ok(
            match tool.execute(tool_context, effective_call.input).await {
                Ok(result) => result,
                Err(error) => error.as_tool_result(),
            },
        )
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

fn build_request(context: &SubagentContext, messages: &[Message]) -> ModelRequest {
    let max_tokens = (context.spec.task_budget.total > 0).then_some(context.spec.task_budget.total);

    ModelRequest {
        model: ModelId(context.spec.model_role.clone()),
        system_prompt: (!context.spec.system_prompt.is_empty())
            .then_some(context.spec.system_prompt.clone())
            .into_iter()
            .collect::<Vec<_>>(),
        messages: messages.to_vec(),
        tools: context
            .tools
            .schemas_sorted()
            .into_iter()
            .map(|spec| spec.to_mcp())
            .collect::<Vec<_>>(),
        role: ModelRole::SubagentDefault,
        cache_breakpoints: Vec::new(),
        response_format: None::<ResponseFormat>,
        thinking: None::<ThinkingConfig>,
        cache_control: CacheControlStrategy::None,
        max_tokens,
        temperature: None,
        stream: true,
    }
}

fn usage_message(usage: &Usage) -> Result<Message, SubagentError> {
    Ok(Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::Usage(*usage)).map_err(|error| {
                SubagentError::Storage {
                    reason: error.to_string(),
                }
            })?,
        }],
    })
}

fn stop_message(stop_reason: StopReason) -> Result<Message, SubagentError> {
    Ok(Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::MessageStop { stop_reason }).map_err(
                |error| SubagentError::Storage {
                    reason: error.to_string(),
                },
            )?,
        }],
    })
}

fn relative_scratchpad_path(session_id: &SessionId) -> PathBuf {
    PathBuf::from("runtime")
        .join("notes")
        .join(format!("{}.md", session_id.0))
}

fn parent_summary_event(text: String) -> SessionEvent {
    SessionEvent::Render {
        block: RenderBlock {
            kind: RenderKind::Markdown,
            payload: json!({
                "title": "subagent.summary",
                "text": text,
            }),
            meta: RenderMeta {
                id: EventId::new_v4(),
                parent: None,
                ts_ms: now_millis(),
            },
        },
        lifecycle: RenderLifecycle::OnToolResult,
    }
}

fn output_meta(output: &SubagentOutput) -> &SubagentSummary {
    match output {
        SubagentOutput::Summary { meta, .. }
        | SubagentOutput::FileRef { meta, .. }
        | SubagentOutput::Json { meta, .. } => meta,
    }
}

fn provider_error(error: octopus_sdk_model::ModelError) -> SubagentError {
    SubagentError::Provider {
        reason: error.to_string(),
    }
}

fn storage_error(error: octopus_sdk_session::SessionError) -> SubagentError {
    SubagentError::Storage {
        reason: error.to_string(),
    }
}

fn memory_error(error: octopus_sdk_contracts::MemoryError) -> SubagentError {
    SubagentError::Storage {
        reason: error.to_string(),
    }
}

fn max_turns_reached(context: &SubagentContext) -> bool {
    context.spec.max_turns > 0 && context.turns() >= context.spec.max_turns
}

struct NoopAskResolver;
struct NoopEventSink;
struct NoopSecretVault;

#[async_trait]
impl AskResolver for NoopAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Err(AskError::NotResolvable)
    }
}

impl EventSink for NoopEventSink {
    fn emit(&self, _event: SessionEvent) {}
}

#[async_trait]
impl SecretVault for NoopSecretVault {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Err(VaultError::NotFound)
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64
}
