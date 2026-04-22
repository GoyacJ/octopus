use std::sync::{Arc, Mutex};

use futures::future::join_all;
use octopus_sdk_context::{Compactor, SessionView};
use octopus_sdk_contracts::{
    CompactionStrategyTag, ContentBlock, EventId, EventSink, HookEvent, HookToolResult, Message,
    PermissionOutcome,
    RenderLifecycle, Role, SessionEvent, SessionId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::ModelProvider;
use octopus_sdk_observability::{TraceSpan, TraceValue, Tracer};
use octopus_sdk_permissions::ApprovalBroker;
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_sandbox::{SandboxBackend, SandboxSpec};
use octopus_sdk_session::SessionStore;
use octopus_sdk_tools::{partition_tool_calls, ExecBatch, ToolContext, ToolError, ToolRegistry};
use tokio_util::sync::CancellationToken;

use crate::{
    session_boot::{message_event, TranscriptState},
    RuntimeError,
};

#[derive(Clone, Default)]
pub(crate) struct BufferedEventSink {
    events: Arc<Mutex<Vec<SessionEvent>>>,
}

impl BufferedEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn take(&self) -> Vec<SessionEvent> {
        std::mem::take(&mut *self.events.lock().expect("event sink lock poisoned"))
    }
}

impl EventSink for BufferedEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events
            .lock()
            .expect("event sink lock poisoned")
            .push(event);
    }
}

pub(crate) struct DispatchContext<'a> {
    pub session_id: &'a SessionId,
    pub working_dir: &'a std::path::Path,
    pub permissions: Arc<dyn octopus_sdk_contracts::PermissionGate>,
    pub ask_resolver: Arc<dyn octopus_sdk_contracts::AskResolver>,
    pub secret_vault: Arc<dyn octopus_sdk_contracts::SecretVault>,
    pub session_store: Arc<dyn SessionStore>,
    pub tool_registry: &'a ToolRegistry,
    pub plugin_registry: Arc<PluginRegistry>,
    pub sandbox_backend: Arc<dyn SandboxBackend>,
    pub tracer: Arc<dyn Tracer>,
    pub cancellation: CancellationToken,
    pub model_provider: Arc<dyn ModelProvider>,
    pub token_budget: u32,
}

pub(crate) async fn execute_tool_round(
    ctx: DispatchContext<'_>,
    calls: &[ToolCallRequest],
    transcript: &mut TranscriptState,
) -> Result<(), RuntimeError> {
    let batches = partition_tool_calls(calls, ctx.tool_registry);

    for batch in batches {
        match batch {
            ExecBatch::Concurrent(batch_calls) => {
                let futures = batch_calls
                    .into_iter()
                    .map(|call| execute_single_tool_call(&ctx, call.clone()));
                for outcome in join_all(futures).await {
                    let tool_message = outcome?;
                    let event_id = ctx
                        .session_store
                        .append(ctx.session_id, message_event(tool_message.clone()))
                        .await?;
                    transcript.push(event_id, tool_message);
                }
            }
            ExecBatch::Serial(batch_calls) => {
                for call in batch_calls {
                    let tool_message = execute_single_tool_call(&ctx, call.clone()).await?;
                    let event_id = ctx
                        .session_store
                        .append(ctx.session_id, message_event(tool_message.clone()))
                        .await?;
                    transcript.push(event_id, tool_message);
                }
            }
        }
    }

    maybe_compact_transcript(
        &ctx.model_provider,
        &ctx.plugin_registry,
        ctx.session_id,
        ctx.session_store.as_ref(),
        transcript,
        ctx.token_budget,
    )
    .await?;

    Ok(())
}

async fn execute_single_tool_call(
    ctx: &DispatchContext<'_>,
    original_call: ToolCallRequest,
) -> Result<Message, RuntimeError> {
    if ctx.cancellation.is_cancelled() {
        return Err(RuntimeError::Cancelled);
    }

    let Some(tool) = ctx.tool_registry.get(&original_call.name) else {
        return Err(RuntimeError::ToolNotFound {
            name: original_call.name.clone(),
        });
    };
    let category = tool.spec().category;
    let call = run_pre_tool_hook(ctx, original_call, category).await?;

    let event_sink = BufferedEventSink::new();
    let permission_outcome = match ctx.permissions.check(&call).await {
        PermissionOutcome::AskApproval { prompt } | PermissionOutcome::RequireAuth { prompt } => {
            let broker =
                ApprovalBroker::new(Arc::new(event_sink.clone()), Arc::clone(&ctx.ask_resolver));
            let outcome = broker.request_approval(&call, prompt).await;
            flush_sink(ctx.session_store.as_ref(), ctx.session_id, &event_sink).await?;
            outcome
        }
        outcome => outcome,
    };

    let result = match permission_outcome {
        PermissionOutcome::Allow => {
            let sandbox = ctx
                .sandbox_backend
                .provision(SandboxSpec {
                    fs_whitelist: vec![ctx.working_dir.to_path_buf()],
                    env_allowlist: vec!["PATH".into(), "HOME".into()],
                    ..SandboxSpec::default()
                })
                .await?;
            let tool_ctx = ToolContext {
                session_id: ctx.session_id.clone(),
                permissions: Arc::clone(&ctx.permissions),
                sandbox: sandbox.clone(),
                session_store: Arc::clone(&ctx.session_store),
                secret_vault: Arc::clone(&ctx.secret_vault),
                ask_resolver: Arc::clone(&ctx.ask_resolver),
                event_sink: Arc::new(event_sink.clone()),
                working_dir: ctx.working_dir.to_path_buf(),
                cancellation: ctx.cancellation.clone(),
            };
            let execute_result = match tool.execute(tool_ctx, call.input.clone()).await {
                Ok(result) => result,
                Err(error) => error.as_tool_result(),
            };
            let _ = ctx.sandbox_backend.terminate(sandbox).await;
            execute_result
        }
        PermissionOutcome::Deny { reason } => {
            return finalize_tool_denial(ctx, &call, reason).await;
        }
        PermissionOutcome::AskApproval { .. } | PermissionOutcome::RequireAuth { .. } => {
            return Err(RuntimeError::Hook("approval broker returned unresolved prompt".into()));
        }
    };

    flush_sink(ctx.session_store.as_ref(), ctx.session_id, &event_sink).await?;
    let result = run_post_tool_hook(ctx, &call, result).await?;

    ctx.session_store
        .append(
            ctx.session_id,
            SessionEvent::ToolExecuted {
                call: call.id.clone(),
                name: call.name.clone(),
                duration_ms: result.duration_ms,
                is_error: result.is_error,
            },
        )
        .await?;

    if let Some(render) = result.render.clone() {
        ctx.session_store
            .append(
                ctx.session_id,
                SessionEvent::Render {
                    block: render,
                    lifecycle: if result.is_error {
                        RenderLifecycle::OnToolError
                    } else {
                        RenderLifecycle::OnToolResult
                    },
                },
            )
            .await?;
    }

    ctx.tracer.record(
        TraceSpan::new("tool_executed")
            .with_field("tool_name", TraceValue::String(call.name.clone()))
            .with_field("is_error", TraceValue::Bool(result.is_error)),
    );

    Ok(tool_result_message(&call, &result))
}

async fn finalize_tool_denial(
    ctx: &DispatchContext<'_>,
    call: &ToolCallRequest,
    reason: String,
) -> Result<Message, RuntimeError> {
    let message = Message {
        role: Role::Tool,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: call.id.clone(),
            content: vec![ContentBlock::Text {
                text: reason.clone(),
            }],
            is_error: true,
        }],
    };

    ctx.session_store
        .append(
            ctx.session_id,
            SessionEvent::ToolExecuted {
                call: call.id.clone(),
                name: call.name.clone(),
                duration_ms: 0,
                is_error: true,
            },
        )
        .await?;

    Err(RuntimeError::PermissionDenied {
        name: call.name.clone(),
        reason,
    })
    .or(Ok(message))
}

async fn run_pre_tool_hook(
    ctx: &DispatchContext<'_>,
    mut call: ToolCallRequest,
    category: ToolCategory,
) -> Result<ToolCallRequest, RuntimeError> {
    let outcome = ctx
        .plugin_registry
        .hooks()
        .run(HookEvent::PreToolUse {
            call: call.clone(),
            category,
        })
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?;

    if let Some(reason) = outcome.aborted {
        return Err(RuntimeError::PermissionDenied {
            name: call.name,
            reason,
        });
    }

    if let Some(octopus_sdk_contracts::RewritePayload::ToolCall { call: rewritten }) =
        outcome.final_payload
    {
        call = rewritten;
    }

    Ok(call)
}

async fn run_post_tool_hook(
    ctx: &DispatchContext<'_>,
    call: &ToolCallRequest,
    result: octopus_sdk_tools::ToolResult,
) -> Result<octopus_sdk_tools::ToolResult, RuntimeError> {
    let outcome = ctx
        .plugin_registry
        .hooks()
        .run(HookEvent::PostToolUse {
            call: call.clone(),
            result: HookToolResult {
                content: result.content.clone(),
                is_error: result.is_error,
                duration_ms: result.duration_ms,
                render: result.render.clone(),
            },
        })
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?;

    if let Some(reason) = outcome.aborted {
        return Ok(ToolError::Execution { message: reason }.as_tool_result());
    }

    if let Some(octopus_sdk_contracts::RewritePayload::ToolResult { result: rewritten }) =
        outcome.final_payload
    {
        return Ok(octopus_sdk_tools::ToolResult {
            content: rewritten.content,
            is_error: rewritten.is_error,
            duration_ms: rewritten.duration_ms,
            render: rewritten.render,
        });
    }

    Ok(result)
}

async fn flush_sink(
    store: &dyn SessionStore,
    session_id: &SessionId,
    sink: &BufferedEventSink,
) -> Result<(), RuntimeError> {
    for event in sink.take() {
        store.append(session_id, event).await?;
    }
    Ok(())
}

fn tool_result_message(
    call: &ToolCallRequest,
    result: &octopus_sdk_tools::ToolResult,
) -> Message {
    Message {
        role: Role::Tool,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: call.id.clone(),
            content: result.content.clone(),
            is_error: result.is_error,
        }],
    }
}

async fn maybe_compact_transcript(
    model_provider: &Arc<dyn ModelProvider>,
    plugin_registry: &Arc<PluginRegistry>,
    session_id: &SessionId,
    session_store: &dyn SessionStore,
    transcript: &mut TranscriptState,
    token_budget: u32,
) -> Result<(), RuntimeError> {
    let estimated_tokens = estimate_tokens(&transcript.messages);
    let event_ids = std::mem::take(&mut transcript.event_ids);
    let mut view = SessionView {
        messages: &mut transcript.messages,
        tokens: estimated_tokens,
        tokens_budget: token_budget,
        event_ids,
    };
    let compactor = Compactor::new(
        1.1,
        octopus_sdk_contracts::CompactionStrategyTag::Summarize,
        Arc::clone(model_provider),
    );
    if let Some(result) = compactor
        .maybe_compact(&mut view)
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?
    {
        if matches!(result.strategy, CompactionStrategyTag::Summarize) {
            persist_compaction_checkpoint(session_store, session_id, &mut view, &result).await?;
        }
        let _ = plugin_registry
            .hooks()
            .run(HookEvent::PostCompact {
                session: session_id.clone(),
                result,
            })
            .await;
    }
    transcript.event_ids = view.event_ids;
    Ok(())
}

async fn persist_compaction_checkpoint(
    session_store: &dyn SessionStore,
    session_id: &SessionId,
    view: &mut SessionView<'_>,
    result: &octopus_sdk_contracts::CompactionResult,
) -> Result<(), RuntimeError> {
    let Some(anchor_event_id) = result.folded_turn_ids.last().cloned() else {
        return Ok(());
    };
    let checkpoint_event_id = session_store
        .append(
            session_id,
            SessionEvent::Checkpoint {
                id: format!("checkpoint:{}", EventId::new_v4().0),
                anchor_event_id,
                compaction: Some(result.clone()),
            },
        )
        .await?;

    if let Some(summary_event_id) = view.event_ids.first_mut() {
        *summary_event_id = checkpoint_event_id;
    }

    Ok(())
}

fn estimate_tokens(messages: &[Message]) -> u32 {
    messages
        .iter()
        .flat_map(|message| &message.content)
        .map(|block| match block {
            ContentBlock::Text { text } | ContentBlock::Thinking { text } => text.len(),
            ContentBlock::ToolUse { name, input, .. } => name.len() + input.to_string().len(),
            ContentBlock::ToolResult { content, .. } => content
                .iter()
                .map(|child| match child {
                    ContentBlock::Text { text } | ContentBlock::Thinking { text } => text.len(),
                    ContentBlock::ToolUse { name, input, .. } => {
                        name.len() + input.to_string().len()
                    }
                    ContentBlock::ToolResult { .. } => 0,
                })
                .sum::<usize>(),
        })
        .sum::<usize>()
        .div_ceil(4) as u32
}
