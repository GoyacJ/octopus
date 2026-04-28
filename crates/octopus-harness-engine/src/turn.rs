use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use futures::{stream, StreamExt};
use harness_context::ContextSessionView;
use harness_contracts::{
    AssistantDeltaProducedEvent, AssistantMessageCompletedEvent, CausationId, ConfigHash,
    CorrelationId, DecidedBy, Decision, DecisionId, DecisionScope, DeltaChunk, DenyReason,
    EndReason, Event, EventId, FallbackPolicy, InteractivityLevel, Message, MessageContent,
    MessageId, MessageMetadata, MessagePart, MessageRole, NoopRedactor, PermissionMode,
    PermissionRequestedEvent, PermissionResolvedEvent, Redactor, RunEndedEvent, RunStartedEvent,
    StopReason, TenantId, ToolDescriptor, ToolError, ToolErrorPayload, ToolResult,
    ToolUseApprovedEvent, ToolUseCompletedEvent, ToolUseDeniedEvent, ToolUseFailedEvent, ToolUseId,
    ToolUseRequestedEvent, TrustLevel, TurnInput, UsageSnapshot,
};
use harness_hook::{
    HookContext, HookEvent, HookMessageView, HookOutcome, HookSessionView, ReplayMode,
    ToolDescriptorView,
};
use harness_model::{ContentDelta, InferContext, ModelRequest, ModelStreamEvent};
use harness_permission::{PermissionBroker, PermissionContext, PermissionRequest, RuleSnapshot};
use harness_tool::{
    InterruptToken, OrchestratorContext, ToolCall, ToolEventEmitter, ToolOrchestrator,
    ToolResultEnvelope as RuntimeToolResultEnvelope,
};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};

use crate::{
    end_reason_for_interrupt, result_inject, Engine, EngineError, EventStream, RunContext,
    SessionHandle,
};

pub(crate) async fn run_turn(
    engine: &Engine,
    session: SessionHandle,
    input: TurnInput,
    ctx: RunContext,
) -> Result<EventStream, EngineError> {
    if session.tenant_id != ctx.tenant_id || session.session_id != ctx.session_id {
        return Err(engine_error(
            "context mismatch between session handle and run context",
        ));
    }

    let mut emitted = Vec::new();
    let correlation_id = CorrelationId::new();
    let run_started = Event::RunStarted(RunStartedEvent {
        run_id: ctx.run_id,
        session_id: session.session_id,
        tenant_id: session.tenant_id,
        parent_run_id: None,
        input: input.clone(),
        snapshot_id: harness_contracts::SnapshotId::from_u128(0),
        effective_config_hash: ConfigHash([0; 32]),
        started_at: harness_contracts::now(),
        correlation_id,
    });
    append(
        engine,
        session.tenant_id,
        session.session_id,
        &mut emitted,
        vec![run_started],
    )
    .await?;
    let mut usage = UsageSnapshot::default();

    if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone()).await? {
        return Ok(Box::pin(stream::iter(emitted)));
    }

    dispatch_user_prompt_hook(engine, &session, &ctx, &input, &[]).await?;

    if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone()).await? {
        return Ok(Box::pin(stream::iter(emitted)));
    }

    let mut working_messages = collected_messages(&emitted);
    let mut next_input = input;
    let mut grace_active = false;
    let mut iterations = 0;
    let mut appended_user_messages = HashSet::new();

    loop {
        if iterations >= engine.max_iterations {
            append_run_end(
                engine,
                &session,
                &mut emitted,
                ctx.run_id,
                EndReason::MaxIterationsReached,
                usage.clone(),
            )
            .await?;
            return Ok(Box::pin(stream::iter(emitted)));
        }

        if !grace_active && iterations + 1 >= engine.max_iterations {
            let grace = Event::GraceCallTriggered(harness_contracts::GraceCallTriggeredEvent {
                run_id: ctx.run_id,
                session_id: session.session_id,
                tenant_id: session.tenant_id,
                current_iteration: iterations,
                max_iterations: engine.max_iterations,
                usage_snapshot: usage.clone(),
                at: harness_contracts::now(),
                correlation_id,
            });
            append(
                engine,
                session.tenant_id,
                session.session_id,
                &mut emitted,
                vec![grace],
            )
            .await?;
            grace_active = true;
        }

        if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone())
            .await?
        {
            return Ok(Box::pin(stream::iter(emitted)));
        }

        apply_steering(
            engine,
            &session,
            &mut emitted,
            &ctx,
            &mut working_messages,
            &mut next_input,
        )
        .await?;

        let prompt_view = TurnContextView {
            tenant_id: session.tenant_id,
            session_id: session.session_id,
            system: engine.system_prompt.clone(),
            messages: working_messages.clone(),
            tools: engine
                .tools
                .iter()
                .map(|tool| tool.descriptor().clone())
                .collect(),
        };
        let assembled = engine
            .context
            .assemble(&prompt_view, &next_input)
            .await
            .map_err(engine_error)?;

        let request = ModelRequest {
            model_id: engine.model_id.clone(),
            messages: assembled.messages,
            tools: (!assembled.tools_snapshot.is_empty()).then_some(assembled.tools_snapshot),
            system: assembled.system,
            temperature: None,
            max_tokens: None,
            stream: true,
            cache_breakpoints: assembled.cache_breakpoints,
            api_mode: engine.api_mode,
            extra: Value::Null,
        };
        let mut infer_ctx = InferContext::for_test();
        infer_ctx.tenant_id = session.tenant_id;
        infer_ctx.session_id = Some(session.session_id);
        infer_ctx.run_id = Some(ctx.run_id);

        if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone())
            .await?
        {
            return Ok(Box::pin(stream::iter(emitted)));
        }

        let mut stream = match engine.model.infer(request, infer_ctx).await {
            Ok(stream) => stream,
            Err(error) => {
                finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &error).await?;
                return Err(engine_error(error));
            }
        };

        let assistant_message_id = MessageId::new();
        let mut assistant_text = String::new();
        let mut tool_calls = Vec::new();
        let mut tool_collector = StreamingToolCallCollector::default();
        let mut stop_reason = StopReason::EndTurn;

        while let Some(event) = stream.next().await {
            match event {
                ModelStreamEvent::MessageStart {
                    usage: start_usage, ..
                } => add_usage(&mut usage, &start_usage),
                ModelStreamEvent::ContentBlockDelta { index, delta } => match delta {
                    ContentDelta::Text(text) => {
                        assistant_text.push_str(&text);
                        append(
                            engine,
                            session.tenant_id,
                            session.session_id,
                            &mut emitted,
                            vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                run_id: ctx.run_id,
                                message_id: assistant_message_id,
                                delta: DeltaChunk::Text(text),
                                at: harness_contracts::now(),
                            })],
                        )
                        .await?;
                    }
                    ContentDelta::Thinking(thinking) => {
                        if let Some(text) = thinking.text.clone() {
                            assistant_text.push_str(&text);
                        }
                        append(
                            engine,
                            session.tenant_id,
                            session.session_id,
                            &mut emitted,
                            vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                run_id: ctx.run_id,
                                message_id: assistant_message_id,
                                delta: DeltaChunk::Thought(harness_contracts::ThoughtChunk {
                                    text: thinking.text,
                                    provider_id: "model".to_owned(),
                                    provider_native: thinking.provider_native,
                                    signature: thinking.signature,
                                }),
                                at: harness_contracts::now(),
                            })],
                        )
                        .await?;
                    }
                    ContentDelta::ToolUseComplete { id, name, input } => {
                        tool_collector.discard(index);
                        tool_calls.push(ToolCall {
                            tool_use_id: id,
                            tool_name: name.clone(),
                            input,
                        });
                        append(
                            engine,
                            session.tenant_id,
                            session.session_id,
                            &mut emitted,
                            vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                run_id: ctx.run_id,
                                message_id: assistant_message_id,
                                delta: DeltaChunk::ToolUseEnd { tool_use_id: id },
                                at: harness_contracts::now(),
                            })],
                        )
                        .await?;
                    }
                    ContentDelta::ToolUseStart { id, name } => {
                        let tool_use_id = tool_collector.start(index, id, name.clone());
                        append(
                            engine,
                            session.tenant_id,
                            session.session_id,
                            &mut emitted,
                            vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                run_id: ctx.run_id,
                                message_id: assistant_message_id,
                                delta: DeltaChunk::ToolUseStart {
                                    tool_use_id,
                                    tool_name: name,
                                },
                                at: harness_contracts::now(),
                            })],
                        )
                        .await?;
                    }
                    ContentDelta::ToolUseInputJson(delta) => {
                        let tool_use_id = match tool_collector.push_input(index, &delta) {
                            Ok(tool_use_id) => tool_use_id,
                            Err(error) => {
                                finalize_run_error(
                                    engine,
                                    &session,
                                    &mut emitted,
                                    ctx.run_id,
                                    &error,
                                )
                                .await?;
                                return Err(engine_error(error));
                            }
                        };
                        append(
                            engine,
                            session.tenant_id,
                            session.session_id,
                            &mut emitted,
                            vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                run_id: ctx.run_id,
                                message_id: assistant_message_id,
                                delta: DeltaChunk::ToolUseInputDelta { tool_use_id, delta },
                                at: harness_contracts::now(),
                            })],
                        )
                        .await?;
                    }
                },
                ModelStreamEvent::MessageDelta {
                    stop_reason: next_stop_reason,
                    usage_delta,
                } => {
                    add_usage(&mut usage, &usage_delta);
                    if let Some(next_stop_reason) = next_stop_reason {
                        stop_reason = next_stop_reason;
                    }
                }
                ModelStreamEvent::StreamError { error, class, .. } => {
                    let message = format!("model stream error ({class:?}): {error}");
                    finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &message)
                        .await?;
                    return Err(engine_error(message));
                }
                ModelStreamEvent::ContentBlockStop { index } => {
                    match tool_collector.finish(index) {
                        Ok(Some(call)) => {
                            let tool_use_id = call.tool_use_id;
                            tool_calls.push(call);
                            append(
                                engine,
                                session.tenant_id,
                                session.session_id,
                                &mut emitted,
                                vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                    run_id: ctx.run_id,
                                    message_id: assistant_message_id,
                                    delta: DeltaChunk::ToolUseEnd { tool_use_id },
                                    at: harness_contracts::now(),
                                })],
                            )
                            .await?;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &error)
                                .await?;
                            return Err(engine_error(error));
                        }
                    }
                }
                ModelStreamEvent::MessageStop => match tool_collector.finish_all() {
                    Ok(calls) => {
                        for call in calls {
                            let tool_use_id = call.tool_use_id;
                            tool_calls.push(call);
                            append(
                                engine,
                                session.tenant_id,
                                session.session_id,
                                &mut emitted,
                                vec![Event::AssistantDeltaProduced(AssistantDeltaProducedEvent {
                                    run_id: ctx.run_id,
                                    message_id: assistant_message_id,
                                    delta: DeltaChunk::ToolUseEnd { tool_use_id },
                                    at: harness_contracts::now(),
                                })],
                            )
                            .await?;
                        }
                    }
                    Err(error) => {
                        finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &error)
                            .await?;
                        return Err(engine_error(error));
                    }
                },
                ModelStreamEvent::ContentBlockStart { .. } => {}
            }
            if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone())
                .await?
            {
                return Ok(Box::pin(stream::iter(emitted)));
            }
        }

        if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone())
            .await?
        {
            return Ok(Box::pin(stream::iter(emitted)));
        }

        if next_input.message.role == MessageRole::User
            && appended_user_messages.insert(next_input.message.id)
        {
            append(
                engine,
                session.tenant_id,
                session.session_id,
                &mut emitted,
                vec![Event::UserMessageAppended(
                    harness_contracts::UserMessageAppendedEvent {
                        run_id: ctx.run_id,
                        message_id: next_input.message.id,
                        content: message_content(&next_input.message),
                        metadata: MessageMetadata::default(),
                        at: harness_contracts::now(),
                    },
                )],
            )
            .await?;
        }
        working_messages.push(next_input.message.clone());

        if tool_calls.is_empty() {
            append(
                engine,
                session.tenant_id,
                session.session_id,
                &mut emitted,
                vec![Event::AssistantMessageCompleted(
                    AssistantMessageCompletedEvent {
                        run_id: ctx.run_id,
                        message_id: assistant_message_id,
                        content: MessageContent::Text(assistant_text),
                        tool_uses: Vec::new(),
                        usage: usage.clone(),
                        pricing_snapshot_id: None,
                        stop_reason,
                        at: harness_contracts::now(),
                    },
                )],
            )
            .await?;
            append_run_end(
                engine,
                &session,
                &mut emitted,
                ctx.run_id,
                EndReason::Completed,
                usage,
            )
            .await?;
            return Ok(Box::pin(stream::iter(emitted)));
        }

        let assistant_tool_message = result_inject::assistant_tool_message(
            assistant_message_id,
            assistant_text.clone(),
            &tool_calls,
        );
        append(
            engine,
            session.tenant_id,
            session.session_id,
            &mut emitted,
            vec![Event::AssistantMessageCompleted(
                AssistantMessageCompletedEvent {
                    run_id: ctx.run_id,
                    message_id: assistant_message_id,
                    content: result_inject::assistant_tool_content(assistant_text, &tool_calls),
                    tool_uses: tool_calls
                        .iter()
                        .map(|call| harness_contracts::ToolUseSummary {
                            tool_use_id: call.tool_use_id,
                            tool_name: call.tool_name.clone(),
                        })
                        .collect(),
                    usage: usage.clone(),
                    pricing_snapshot_id: None,
                    stop_reason,
                    at: harness_contracts::now(),
                },
            )],
        )
        .await?;
        working_messages.push(assistant_tool_message);

        if append_interrupt_if_cancelled(engine, &session, &mut emitted, &ctx, usage.clone())
            .await?
        {
            return Ok(Box::pin(stream::iter(emitted)));
        }

        if grace_active {
            append_run_end(
                engine,
                &session,
                &mut emitted,
                ctx.run_id,
                EndReason::MaxIterationsReached,
                usage,
            )
            .await?;
            return Ok(Box::pin(stream::iter(emitted)));
        }

        for call in &tool_calls {
            let Some(descriptor) = engine.tools.descriptor(&call.tool_name) else {
                let message = format!("tool descriptor missing: {}", call.tool_name);
                finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &message).await?;
                return Err(engine_error(message));
            };
            append(
                engine,
                session.tenant_id,
                session.session_id,
                &mut emitted,
                vec![Event::ToolUseRequested(ToolUseRequestedEvent {
                    run_id: ctx.run_id,
                    tool_use_id: call.tool_use_id,
                    tool_name: call.tool_name.clone(),
                    input: call.input.clone(),
                    properties: descriptor.properties.clone(),
                    causation_id: EventId::new(),
                    at: harness_contracts::now(),
                })],
            )
            .await?;
        }

        let permission_recorder = Arc::new(RecordingPermissionBroker::new(
            engine.permission_broker.clone(),
        ));
        let tool_event_emitter = Arc::new(RecordingToolEventEmitter::new());
        let tool_interrupt = InterruptToken::new();
        let orchestrator = ToolOrchestrator::default();
        let mut dispatch = Box::pin(orchestrator.dispatch(
            tool_calls.clone(),
            OrchestratorContext {
                pool: engine.tools.clone(),
                tool_context: harness_tool::ToolContext {
                    tool_use_id: ToolUseId::new(),
                    run_id: ctx.run_id,
                    session_id: session.session_id,
                    tenant_id: session.tenant_id,
                    agent_id: harness_contracts::AgentId::from_u128(1),
                    workspace_root: engine.workspace_root.clone(),
                    sandbox: engine.sandbox.clone(),
                    permission_broker: permission_recorder.clone(),
                    cap_registry: engine.cap_registry.clone(),
                    interrupt: tool_interrupt.clone(),
                    parent_run: None,
                },
                permission_context: permission_context(&session),
                blob_store: engine.blob_store.clone(),
                event_emitter: tool_event_emitter.clone(),
            },
        ));
        let tool_results = tokio::select! {
            results = &mut dispatch => results,
            cause = ctx.cancellation.cancelled() => {
                tool_interrupt.interrupt();
                let _ = dispatch.await;
                append_run_end(
                    engine,
                    &session,
                    &mut emitted,
                    ctx.run_id,
                    end_reason_for_interrupt(cause),
                    usage,
                )
                .await?;
                return Ok(Box::pin(stream::iter(emitted)));
            }
        };

        let mut post_tool_events =
            permission_events(ctx.run_id, permission_recorder.records().await);
        for event in tool_event_emitter.take().await {
            post_tool_events.push(event);
        }
        for result in &tool_results {
            post_tool_events.extend(tool_result_events(result));
        }
        append(
            engine,
            session.tenant_id,
            session.session_id,
            &mut emitted,
            post_tool_events,
        )
        .await?;

        if let Err(error) = engine
            .context
            .after_turn(&prompt_view, &context_tool_results(&tool_results))
            .await
        {
            finalize_run_error(engine, &session, &mut emitted, ctx.run_id, &error).await?;
            return Err(engine_error(error));
        }

        let mut reinjected = result_inject::tool_result_messages(&tool_results);
        let next_message = reinjected
            .pop()
            .ok_or_else(|| engine_error("tool dispatch produced no results"))?;
        working_messages.extend(reinjected);
        next_input = result_inject::turn_input_from_message(next_message);
        iterations = iterations.saturating_add(1);
    }
}

async fn dispatch_user_prompt_hook(
    engine: &Engine,
    session: &SessionHandle,
    ctx: &RunContext,
    input: &TurnInput,
    messages: &[Message],
) -> Result<(), EngineError> {
    let result = engine
        .hooks
        .dispatch(
            HookEvent::UserPromptSubmit {
                run_id: ctx.run_id,
                input: json!({ "prompt": message_text(&input.message) }),
            },
            hook_context(engine, session, ctx.run_id, messages),
        )
        .await
        .map_err(engine_error)?;
    if let HookOutcome::Block { reason } = result.final_outcome {
        return Err(engine_error(format!("run blocked by hook: {reason}")));
    }
    Ok(())
}

async fn apply_steering(
    engine: &Engine,
    session: &SessionHandle,
    emitted: &mut Vec<Event>,
    ctx: &RunContext,
    working_messages: &mut Vec<Message>,
    next_input: &mut TurnInput,
) -> Result<(), EngineError> {
    let Some(steering_drain) = &engine.steering_drain else {
        return Ok(());
    };
    let target_message_id = if next_input.message.role == MessageRole::User {
        next_input.message.id
    } else {
        MessageId::new()
    };
    let Some(merge) = steering_drain
        .drain_and_merge(session, ctx.run_id, target_message_id)
        .await?
    else {
        return Ok(());
    };

    if merge.already_persisted {
        emitted.push(merge.applied_event);
    } else {
        append(
            engine,
            session.tenant_id,
            session.session_id,
            emitted,
            vec![merge.applied_event],
        )
        .await?;
    }

    if merge.body.is_empty() {
        return Ok(());
    }
    if next_input.message.role == MessageRole::User {
        append_text_to_message(&mut next_input.message, &merge.body);
    } else {
        working_messages.push(next_input.message.clone());
        *next_input = TurnInput {
            message: Message {
                id: target_message_id,
                role: MessageRole::User,
                parts: vec![MessagePart::Text(merge.body)],
                created_at: harness_contracts::now(),
            },
            metadata: json!({ "source": "steering" }),
        };
        dispatch_user_prompt_hook(engine, session, ctx, next_input, working_messages).await?;
    }
    Ok(())
}

fn append_text_to_message(message: &mut Message, text: &str) {
    if let Some(MessagePart::Text(existing)) = message
        .parts
        .iter_mut()
        .find(|part| matches!(part, MessagePart::Text(_)))
    {
        if !existing.is_empty() {
            existing.push('\n');
        }
        existing.push_str(text);
        return;
    }
    message.parts.push(MessagePart::Text(text.to_owned()));
}

#[derive(Default)]
struct StreamingToolCallCollector {
    pending: BTreeMap<u32, PendingToolUse>,
}

struct PendingToolUse {
    tool_use_id: ToolUseId,
    provider_id: String,
    name: String,
    input_json: String,
}

impl StreamingToolCallCollector {
    fn start(&mut self, index: u32, provider_id: String, name: String) -> ToolUseId {
        let tool_use_id = ToolUseId::new();
        self.pending.insert(
            index,
            PendingToolUse {
                tool_use_id,
                provider_id,
                name,
                input_json: String::new(),
            },
        );
        tool_use_id
    }

    fn push_input(&mut self, index: u32, delta: &str) -> Result<ToolUseId, String> {
        let pending = self.pending.get_mut(&index).ok_or_else(|| {
            format!("tool input delta received before tool start for content block {index}")
        })?;
        pending.input_json.push_str(delta);
        Ok(pending.tool_use_id)
    }

    fn finish(&mut self, index: u32) -> Result<Option<ToolCall>, String> {
        self.pending
            .remove(&index)
            .map(Self::finish_pending)
            .transpose()
    }

    fn finish_all(&mut self) -> Result<Vec<ToolCall>, String> {
        std::mem::take(&mut self.pending)
            .into_values()
            .map(Self::finish_pending)
            .collect()
    }

    fn discard(&mut self, index: u32) {
        self.pending.remove(&index);
    }

    fn finish_pending(pending: PendingToolUse) -> Result<ToolCall, String> {
        let input = if pending.input_json.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&pending.input_json).map_err(|error| {
                format!(
                    "invalid tool input json for {} (provider id {}): {error}",
                    pending.name, pending.provider_id
                )
            })?
        };
        Ok(ToolCall {
            tool_use_id: pending.tool_use_id,
            tool_name: pending.name,
            input,
        })
    }
}

async fn append(
    engine: &Engine,
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    emitted: &mut Vec<Event>,
    events: Vec<Event>,
) -> Result<(), EngineError> {
    engine
        .event_store
        .append(tenant_id, session_id, &events)
        .await
        .map_err(engine_error)?;
    emitted.extend(events);
    Ok(())
}

async fn append_run_end(
    engine: &Engine,
    session: &SessionHandle,
    emitted: &mut Vec<Event>,
    run_id: harness_contracts::RunId,
    reason: EndReason,
    usage: UsageSnapshot,
) -> Result<(), EngineError> {
    append(
        engine,
        session.tenant_id,
        session.session_id,
        emitted,
        vec![Event::RunEnded(RunEndedEvent {
            run_id,
            reason,
            usage: Some(usage),
            ended_at: harness_contracts::now(),
        })],
    )
    .await
}

async fn append_interrupt_if_cancelled(
    engine: &Engine,
    session: &SessionHandle,
    emitted: &mut Vec<Event>,
    ctx: &RunContext,
    usage: UsageSnapshot,
) -> Result<bool, EngineError> {
    let Some(cause) = ctx.cancellation.cause().await else {
        return Ok(false);
    };
    append_run_end(
        engine,
        session,
        emitted,
        ctx.run_id,
        end_reason_for_interrupt(cause),
        usage,
    )
    .await?;
    Ok(true)
}

async fn finalize_run_error(
    engine: &Engine,
    session: &SessionHandle,
    emitted: &mut Vec<Event>,
    run_id: harness_contracts::RunId,
    error: impl std::fmt::Display,
) -> Result<(), EngineError> {
    append_run_end(
        engine,
        session,
        emitted,
        run_id,
        EndReason::Error(error.to_string()),
        UsageSnapshot::default(),
    )
    .await
}

#[derive(Clone)]
struct TurnContextView {
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    system: Option<String>,
    messages: Vec<Message>,
    tools: Vec<ToolDescriptor>,
}

impl ContextSessionView for TurnContextView {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    fn session_id(&self) -> Option<harness_contracts::SessionId> {
        Some(self.session_id)
    }

    fn system(&self) -> Option<String> {
        self.system.clone()
    }

    fn messages(&self) -> Vec<Message> {
        self.messages.clone()
    }

    fn tools_snapshot(&self) -> Vec<ToolDescriptor> {
        self.tools.clone()
    }
}

struct TurnHookView {
    workspace_root: PathBuf,
    messages: Vec<Message>,
    redactor: NoopRedactor,
}

impl HookSessionView for TurnHookView {
    fn workspace_root(&self) -> Option<&Path> {
        Some(&self.workspace_root)
    }

    fn recent_messages(&self, limit: usize) -> Vec<HookMessageView> {
        self.messages
            .iter()
            .rev()
            .take(limit)
            .map(|message| HookMessageView {
                role: message.role,
                text_snippet: message_text(message),
                tool_use_id: None,
            })
            .collect()
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn redacted(&self) -> &dyn Redactor {
        &self.redactor
    }

    fn current_tool_descriptor(&self) -> Option<ToolDescriptorView> {
        None
    }
}

fn hook_context(
    engine: &Engine,
    session: &SessionHandle,
    run_id: harness_contracts::RunId,
    messages: &[Message],
) -> HookContext {
    HookContext {
        tenant_id: session.tenant_id,
        session_id: session.session_id,
        run_id: Some(run_id),
        turn_index: Some(next_turn_index(messages)),
        correlation_id: CorrelationId::new(),
        causation_id: CausationId::new(),
        trust_level: TrustLevel::UserControlled,
        permission_mode: PermissionMode::Default,
        interactivity: InteractivityLevel::NoInteractive,
        at: harness_contracts::now(),
        view: Arc::new(TurnHookView {
            workspace_root: engine.workspace_root.clone(),
            messages: messages.to_vec(),
            redactor: NoopRedactor,
        }),
        upstream_outcome: None,
        replay_mode: ReplayMode::Live,
    }
}

#[derive(Clone)]
struct PermissionDecisionRecord {
    request: PermissionRequest,
    decision: Decision,
}

struct RecordingPermissionBroker {
    inner: Arc<dyn PermissionBroker>,
    records: Mutex<Vec<PermissionDecisionRecord>>,
}

impl RecordingPermissionBroker {
    fn new(inner: Arc<dyn PermissionBroker>) -> Self {
        Self {
            inner,
            records: Mutex::new(Vec::new()),
        }
    }

    async fn records(&self) -> Vec<PermissionDecisionRecord> {
        self.records.lock().await.clone()
    }
}

#[async_trait]
impl PermissionBroker for RecordingPermissionBroker {
    async fn decide(&self, request: PermissionRequest, ctx: PermissionContext) -> Decision {
        let decision = self.inner.decide(request.clone(), ctx).await;
        self.records.lock().await.push(PermissionDecisionRecord {
            request,
            decision: decision.clone(),
        });
        decision
    }

    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), harness_contracts::PermissionError> {
        self.inner.persist(decision_id, scope).await
    }
}

struct RecordingToolEventEmitter {
    sender: mpsc::UnboundedSender<Event>,
    receiver: Mutex<mpsc::UnboundedReceiver<Event>>,
}

impl RecordingToolEventEmitter {
    fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    async fn take(&self) -> Vec<Event> {
        let mut receiver = self.receiver.lock().await;
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl ToolEventEmitter for RecordingToolEventEmitter {
    fn emit(&self, event: Event) {
        let _ignored = self.sender.send(event);
    }
}

fn permission_context(session: &SessionHandle) -> PermissionContext {
    PermissionContext {
        permission_mode: PermissionMode::Default,
        previous_mode: None,
        session_id: session.session_id,
        tenant_id: session.tenant_id,
        interactivity: InteractivityLevel::NoInteractive,
        timeout_policy: None,
        fallback_policy: FallbackPolicy::DenyAll,
        rule_snapshot: Arc::new(RuleSnapshot {
            rules: Vec::new(),
            generation: 0,
            built_at: harness_contracts::now(),
        }),
        hook_overrides: Vec::new(),
    }
}

fn permission_events(
    run_id: harness_contracts::RunId,
    records: Vec<PermissionDecisionRecord>,
) -> Vec<Event> {
    let mut events = Vec::with_capacity(records.len() * 3);
    for record in records {
        events.push(Event::PermissionRequested(PermissionRequestedEvent {
            request_id: record.request.request_id,
            run_id,
            session_id: record.request.session_id,
            tenant_id: record.request.tenant_id,
            tool_use_id: record.request.tool_use_id,
            tool_name: record.request.tool_name.clone(),
            subject: record.request.subject.clone(),
            severity: record.request.severity,
            scope_hint: record.request.scope_hint.clone(),
            fingerprint: None,
            presented_options: vec![Decision::AllowOnce, Decision::DenyOnce],
            interactivity: InteractivityLevel::NoInteractive,
            causation_id: EventId::new(),
            at: harness_contracts::now(),
        }));
        events.push(Event::PermissionResolved(PermissionResolvedEvent {
            request_id: record.request.request_id,
            decision: record.decision.clone(),
            decided_by: DecidedBy::Broker {
                broker_id: "engine-turn-runtime".to_owned(),
            },
            scope: record.request.scope_hint.clone(),
            fingerprint: None,
            rationale: None,
            at: harness_contracts::now(),
        }));
        if decision_allows(&record.decision) {
            events.push(Event::ToolUseApproved(ToolUseApprovedEvent {
                tool_use_id: record.request.tool_use_id,
                decision_id: DecisionId::new(),
                scope: record.request.scope_hint,
                at: harness_contracts::now(),
            }));
        } else {
            events.push(Event::ToolUseDenied(ToolUseDeniedEvent {
                tool_use_id: record.request.tool_use_id,
                reason: DenyReason::UserDenied,
                at: harness_contracts::now(),
            }));
        }
    }
    events
}

fn tool_result_events(result: &RuntimeToolResultEnvelope) -> Vec<Event> {
    match &result.result {
        Ok(tool_result) => vec![Event::ToolUseCompleted(ToolUseCompletedEvent {
            tool_use_id: result.tool_use_id,
            result: tool_result.clone(),
            usage: None,
            duration_ms: result.duration.as_millis().min(u128::from(u64::MAX)) as u64,
            at: harness_contracts::now(),
        })],
        Err(error) => vec![Event::ToolUseFailed(ToolUseFailedEvent {
            tool_use_id: result.tool_use_id,
            error: tool_error_payload(error),
            at: harness_contracts::now(),
        })],
    }
}

fn context_tool_results(
    results: &[RuntimeToolResultEnvelope],
) -> Vec<harness_contracts::ToolResultEnvelope> {
    results
        .iter()
        .map(|result| harness_contracts::ToolResultEnvelope {
            result: result
                .result
                .clone()
                .unwrap_or_else(|error| ToolResult::Text(error.to_string())),
            usage: None,
            is_error: result.result.is_err(),
            overflow: result.overflow.clone(),
        })
        .collect()
}

fn message_content(message: &Message) -> MessageContent {
    if let [MessagePart::Text(text)] = message.parts.as_slice() {
        return MessageContent::Text(text.clone());
    }
    MessageContent::Multimodal(message.parts.clone())
}

fn collected_messages(events: &[Event]) -> Vec<Message> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::UserMessageAppended(event) => Some(Message {
                id: event.message_id,
                role: MessageRole::User,
                parts: message_parts(event.content.clone()),
                created_at: event.at,
            }),
            Event::AssistantMessageCompleted(event) => Some(Message {
                id: event.message_id,
                role: MessageRole::Assistant,
                parts: message_parts(event.content.clone()),
                created_at: event.at,
            }),
            _ => None,
        })
        .collect()
}

fn message_parts(content: MessageContent) -> Vec<MessagePart> {
    match content {
        MessageContent::Text(text) => vec![MessagePart::Text(text)],
        MessageContent::Structured(value) => vec![MessagePart::Text(value.to_string())],
        MessageContent::Multimodal(parts) => parts,
    }
}

fn message_text(message: &Message) -> String {
    message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn next_turn_index(messages: &[Message]) -> u32 {
    messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .count()
        .saturating_add(1)
        .min(u32::MAX as usize) as u32
}

fn add_usage(total: &mut UsageSnapshot, delta: &UsageSnapshot) {
    total.input_tokens = total.input_tokens.saturating_add(delta.input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(delta.output_tokens);
    total.cache_read_tokens = total
        .cache_read_tokens
        .saturating_add(delta.cache_read_tokens);
    total.cache_write_tokens = total
        .cache_write_tokens
        .saturating_add(delta.cache_write_tokens);
    total.cost_micros = total.cost_micros.saturating_add(delta.cost_micros);
}

fn decision_allows(decision: &Decision) -> bool {
    matches!(
        decision,
        Decision::AllowOnce | Decision::AllowSession | Decision::AllowPermanent
    )
}

fn tool_error_payload(error: &ToolError) -> ToolErrorPayload {
    ToolErrorPayload {
        code: match error {
            ToolError::Validation(_) => "validation",
            ToolError::PermissionDenied(_) => "permission_denied",
            ToolError::Sandbox(_) => "sandbox",
            ToolError::Timeout => "timeout",
            ToolError::Interrupted => "interrupted",
            ToolError::ResultTooLarge { .. } => "result_too_large",
            ToolError::OffloadFailed(_) => "offload_failed",
            ToolError::CapabilityMissing(_) => "capability_missing",
            ToolError::SchemaResolution(_) => "schema_resolution",
            ToolError::Internal(_) => "internal",
            ToolError::Message(_) => "message",
            _ => "unknown",
        }
        .to_owned(),
        message: error.to_string(),
        retriable: matches!(error, ToolError::Timeout | ToolError::Interrupted),
    }
}

fn engine_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::Message(error.to_string())
}
