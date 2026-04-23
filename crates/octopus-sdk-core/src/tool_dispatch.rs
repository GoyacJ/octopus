use std::sync::{Arc, Mutex};

use futures::future::join_all;
use octopus_sdk_context::{Compactor, SessionView};
use octopus_sdk_contracts::{
    CompactionStrategyTag, ContentBlock, EventId, EventSink, HookEvent, HookToolResult, Message,
    PermissionOutcome, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, RenderPhase, Role,
    SessionEvent, SessionId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::ModelProvider;
use octopus_sdk_observability::{
    session_span_id, session_trace_id, stable_input_hash, tool_span_id, TraceSpan, TraceValue,
    Tracer,
};
use octopus_sdk_permissions::ApprovalBroker;
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_sandbox::{SandboxBackend, SandboxSpec};
use octopus_sdk_session::SessionStore;
use octopus_sdk_tools::{partition_tool_calls, ExecBatch, ToolContext, ToolError, ToolRegistry};
use tokio_util::sync::CancellationToken;

use crate::{
    session_boot::{
        estimate_tokens, message_event, persist_compaction_checkpoint, TranscriptState,
    },
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
    pub permission_mode: octopus_sdk_contracts::PermissionMode,
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
    pub model_id: String,
    pub model_version: String,
    pub config_snapshot_id: String,
    pub token_budget: u32,
}

pub(crate) async fn execute_tool_round(
    ctx: DispatchContext<'_>,
    calls: &[ToolCallRequest],
    transcript: &mut TranscriptState,
) -> Result<bool, RuntimeError> {
    let batches = partition_tool_calls(calls, ctx.tool_registry);
    let mut compacted = false;

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

    compacted |= maybe_compact_transcript(
        &ctx.model_provider,
        &ctx.plugin_registry,
        ctx.session_id,
        ctx.session_store.as_ref(),
        transcript,
        ctx.token_budget,
    )
    .await?;

    Ok(compacted)
}

async fn execute_single_tool_call(
    ctx: &DispatchContext<'_>,
    original_call: ToolCallRequest,
) -> Result<Message, RuntimeError> {
    if ctx.cancellation.is_cancelled() {
        return Err(RuntimeError::Cancelled);
    }
    let execution_started = std::time::Instant::now();

    let Some(tool) = ctx.tool_registry.get(&original_call.name) else {
        return Err(RuntimeError::ToolNotFound {
            name: original_call.name.clone(),
        });
    };
    let category = tool.spec().category;
    let pre_hook_started = std::time::Instant::now();
    let call = run_pre_tool_hook(ctx, original_call, category).await?;
    let pre_hook_ms = pre_hook_started.elapsed().as_millis() as u64;
    ctx.session_store
        .append(
            ctx.session_id,
            SessionEvent::Render {
                blocks: vec![tool_status_render_block(
                    "Tool started",
                    &call.name,
                    serde_json::json!({ "status": "started", "toolName": call.name }),
                    None,
                )],
                lifecycle: RenderLifecycle::tool_phase(
                    RenderPhase::OnToolUse,
                    call.id.clone(),
                    call.name.clone(),
                ),
            },
        )
        .await?;

    let permission_context = ctx
        .permissions
        .tool_permission_context(ctx.permission_mode, &call.name);
    let event_sink = BufferedEventSink::new();
    let permission_started = std::time::Instant::now();
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
    let permission_ms = permission_started.elapsed().as_millis() as u64;

    ctx.session_store
        .append(
            ctx.session_id,
            SessionEvent::PermissionDecision {
                call: call.id.clone(),
                name: call.name.clone(),
                mode: ctx.permission_mode,
                outcome: permission_outcome.clone(),
            },
        )
        .await?;

    let (result, sandbox_backend, sandbox_ms, execute_ms, error_hook_ms) =
        match permission_outcome.clone() {
            PermissionOutcome::Allow => {
                ctx.session_store
                    .append(
                        ctx.session_id,
                        SessionEvent::Render {
                            blocks: vec![tool_status_render_block(
                                "Tool progress",
                                &call.name,
                                serde_json::json!({
                                    "status": "permission_granted",
                                    "toolName": call.name.clone(),
                                    "permissionMode": format!("{:?}", ctx.permission_mode),
                                    "permission": permission_context_payload(&permission_context),
                                }),
                                None,
                            )],
                            lifecycle: RenderLifecycle::tool_phase(
                                RenderPhase::OnToolProgress,
                                call.id.clone(),
                                call.name.clone(),
                            ),
                        },
                    )
                    .await?;
                let sandbox_started = std::time::Instant::now();
                let sandbox = ctx
                    .sandbox_backend
                    .provision(SandboxSpec {
                        fs_whitelist: vec![ctx.working_dir.to_path_buf()],
                        env_allowlist: vec!["PATH".into(), "HOME".into()],
                        ..SandboxSpec::default()
                    })
                    .await?;
                let sandbox_ms = sandbox_started.elapsed().as_millis() as u64;
                let sandbox_backend = sandbox.backend_name().to_string();
                ctx.session_store
                    .append(
                        ctx.session_id,
                        SessionEvent::Render {
                            blocks: vec![tool_status_render_block(
                                "Tool progress",
                                &call.name,
                                serde_json::json!({
                                    "status": "sandbox_ready",
                                    "toolName": call.name.clone(),
                                    "sandbox": sandbox_payload(&sandbox),
                                }),
                                None,
                            )],
                            lifecycle: RenderLifecycle::tool_phase(
                                RenderPhase::OnToolProgress,
                                call.id.clone(),
                                call.name.clone(),
                            ),
                        },
                    )
                    .await?;
                let tool_ctx = ToolContext {
                    session_id: ctx.session_id.clone(),
                    tool_call_id: Some(call.id.clone()),
                    permissions: Arc::clone(&ctx.permissions),
                    sandbox: sandbox.clone(),
                    session_store: Arc::clone(&ctx.session_store),
                    secret_vault: Arc::clone(&ctx.secret_vault),
                    ask_resolver: Arc::clone(&ctx.ask_resolver),
                    event_sink: Arc::new(event_sink.clone()),
                    working_dir: ctx.working_dir.to_path_buf(),
                    hooks: ctx.plugin_registry.hooks_arc(),
                    permission_context: permission_context.clone(),
                    cancellation: ctx.cancellation.clone(),
                };
                tool.validate(&call.input).map_err(RuntimeError::from)?;
                let execute_started = std::time::Instant::now();
                let execute_result = match tool.execute(tool_ctx, call.input.clone()).await {
                    Ok(result) => result,
                    Err(error) => error.as_tool_result(),
                };
                let execute_ms = execute_started.elapsed().as_millis() as u64;
                let (execute_result, error_hook_ms) = if execute_result.is_error {
                    let error_hook_started = std::time::Instant::now();
                    let next = run_tool_error_hook(ctx, &call, execute_result).await?;
                    (next, error_hook_started.elapsed().as_millis() as u64)
                } else {
                    (execute_result, 0)
                };
                let _ = ctx.sandbox_backend.terminate(sandbox).await;
                (
                    execute_result,
                    Some(sandbox_backend),
                    sandbox_ms,
                    execute_ms,
                    error_hook_ms,
                )
            }
            PermissionOutcome::Deny { reason } => {
                return finalize_tool_denial(
                    ctx,
                    &call,
                    &permission_context,
                    permission_outcome,
                    reason,
                    pre_hook_ms,
                    permission_ms,
                    execution_started.elapsed().as_millis() as u64,
                )
                .await;
            }
            PermissionOutcome::AskApproval { .. } | PermissionOutcome::RequireAuth { .. } => {
                return Err(RuntimeError::Hook(
                    "approval broker returned unresolved prompt".into(),
                ));
            }
        };

    flush_sink(ctx.session_store.as_ref(), ctx.session_id, &event_sink).await?;
    let post_hook_started = std::time::Instant::now();
    let result = run_post_tool_hook(ctx, &call, result).await?;
    let post_hook_ms = post_hook_started.elapsed().as_millis() as u64;

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
                    blocks: vec![render],
                    lifecycle: if result.is_error {
                        RenderLifecycle::tool_phase(
                            RenderPhase::OnToolError,
                            call.id.clone(),
                            call.name.clone(),
                        )
                    } else {
                        RenderLifecycle::tool_phase(
                            RenderPhase::OnToolResult,
                            call.id.clone(),
                            call.name.clone(),
                        )
                    },
                },
            )
            .await?;
    } else if result.is_error {
        ctx.session_store
            .append(
                ctx.session_id,
                SessionEvent::Render {
                    blocks: vec![tool_status_render_block(
                        "Tool failed",
                        &call.name,
                        serde_json::json!({
                            "status": "error",
                            "toolName": call.name.clone(),
                            "content": result.content.clone(),
                        }),
                        None,
                    )],
                    lifecycle: RenderLifecycle::tool_phase(
                        RenderPhase::OnToolError,
                        call.id.clone(),
                        call.name.clone(),
                    ),
                },
            )
            .await?;
    }

    emit_execution_trace(
        ctx,
        &call,
        &permission_outcome,
        sandbox_backend.as_deref(),
        pre_hook_ms,
        permission_ms,
        sandbox_ms,
        execute_ms,
        post_hook_ms,
        error_hook_ms,
        execution_started.elapsed().as_millis() as u64,
        result.is_error,
    );

    Ok(tool_result_message(&call, &result))
}

async fn finalize_tool_denial(
    ctx: &DispatchContext<'_>,
    call: &ToolCallRequest,
    permission_context: &octopus_sdk_contracts::ToolPermissionContext,
    permission_outcome: PermissionOutcome,
    reason: String,
    pre_hook_ms: u64,
    permission_ms: u64,
    total_ms: u64,
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

    ctx.session_store
        .append(
            ctx.session_id,
            SessionEvent::Render {
                blocks: vec![tool_status_render_block(
                    "Tool rejected",
                    &call.name,
                    serde_json::json!({
                        "status": "rejected",
                        "toolName": call.name.clone(),
                        "reason": reason.clone(),
                        "permissionMode": format!("{:?}", ctx.permission_mode),
                        "permission": permission_context_payload(permission_context),
                        "retryHint": permission_retry_hint(permission_context),
                    }),
                    None,
                )],
                lifecycle: RenderLifecycle::tool_phase(
                    RenderPhase::OnToolRejected,
                    call.id.clone(),
                    call.name.clone(),
                ),
            },
        )
        .await?;

    emit_execution_trace(
        ctx,
        call,
        &permission_outcome,
        None,
        pre_hook_ms,
        permission_ms,
        0,
        0,
        0,
        0,
        total_ms,
        true,
    );

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

async fn run_tool_error_hook(
    ctx: &DispatchContext<'_>,
    call: &ToolCallRequest,
    result: octopus_sdk_tools::ToolResult,
) -> Result<octopus_sdk_tools::ToolResult, RuntimeError> {
    let outcome = ctx
        .plugin_registry
        .hooks()
        .run(HookEvent::OnToolError {
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

    match outcome.final_payload {
        Some(octopus_sdk_contracts::RewritePayload::ToolResult { result }) => {
            Ok(octopus_sdk_tools::ToolResult {
                content: result.content,
                is_error: result.is_error,
                duration_ms: result.duration_ms,
                render: result.render,
            })
        }
        _ => Ok(result),
    }
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

fn tool_result_message(call: &ToolCallRequest, result: &octopus_sdk_tools::ToolResult) -> Message {
    Message {
        role: Role::Tool,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: call.id.clone(),
            content: result.content.clone(),
            is_error: result.is_error,
        }],
    }
}

fn tool_status_render_block(
    title: &str,
    tool_name: &str,
    payload: serde_json::Value,
    parent: Option<EventId>,
) -> RenderBlock {
    RenderBlock {
        kind: RenderKind::Record,
        payload: serde_json::json!({
            "title": title,
            "rows": [
                { "label": "tool", "value": tool_name },
                { "label": "detail", "value": payload }
            ]
        }),
        meta: RenderMeta {
            id: EventId::new_v4(),
            parent,
            ts_ms: now_ms(),
        },
    }
}

fn permission_context_payload(
    context: &octopus_sdk_contracts::ToolPermissionContext,
) -> serde_json::Value {
    serde_json::json!({
        "mode": format!("{:?}", context.mode),
        "alwaysAllowRuleSources": context.always_allow_rules.len(),
        "alwaysAskRuleSources": context.always_ask_rules.len(),
        "alwaysDenyRuleSources": context.always_deny_rules.len(),
        "shouldAvoidPermissionPrompts": context.should_avoid_permission_prompts,
        "awaitAutomatedChecksBeforeDialog": context.await_automated_checks_before_dialog,
    })
}

fn permission_retry_hint(context: &octopus_sdk_contracts::ToolPermissionContext) -> String {
    if context.should_avoid_permission_prompts == Some(true) {
        return "retry in an interactive permission mode or relax the matching rule".into();
    }

    "retry after approving the request or relax the matching rule".into()
}

fn sandbox_payload(sandbox: &octopus_sdk_tools::SandboxHandle) -> serde_json::Value {
    serde_json::json!({
        "backend": sandbox.backend_name(),
        "cwd": sandbox.cwd().display().to_string(),
        "envAllowlist": sandbox.env_allowlist(),
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_execution_trace(
    ctx: &DispatchContext<'_>,
    call: &ToolCallRequest,
    permission_outcome: &PermissionOutcome,
    sandbox_backend: Option<&str>,
    pre_hook_ms: u64,
    permission_ms: u64,
    sandbox_ms: u64,
    execute_ms: u64,
    post_hook_ms: u64,
    error_hook_ms: u64,
    total_ms: u64,
    is_error: bool,
) {
    let mut span = TraceSpan::new("tool_execution")
        .with_trace_id(session_trace_id(&ctx.session_id.0))
        .with_span_id(tool_span_id(&call.id.0))
        .with_parent_span_id(session_span_id(&ctx.session_id.0))
        .with_agent_role("main")
        .with_field("session_id", TraceValue::String(ctx.session_id.0.clone()))
        .with_field("tool_call_id", TraceValue::String(call.id.0.clone()))
        .with_field("tool_name", TraceValue::String(call.name.clone()))
        .with_field(
            "input_hash",
            TraceValue::String(stable_input_hash(&call.input)),
        )
        .with_field(
            "permission_mode",
            TraceValue::String(format!("{:?}", ctx.permission_mode)),
        )
        .with_field(
            "permission_decision",
            TraceValue::String(format!("{permission_outcome:?}")),
        )
        .with_field("model_id", TraceValue::String(ctx.model_id.clone()))
        .with_field(
            "model_version",
            TraceValue::String(ctx.model_version.clone()),
        )
        .with_field(
            "config_snapshot_id",
            TraceValue::String(ctx.config_snapshot_id.clone()),
        )
        .with_field("pre_hook_ms", TraceValue::U64(pre_hook_ms))
        .with_field("permission_ms", TraceValue::U64(permission_ms))
        .with_field("sandbox_ms", TraceValue::U64(sandbox_ms))
        .with_field("execute_ms", TraceValue::U64(execute_ms))
        .with_field("post_hook_ms", TraceValue::U64(post_hook_ms))
        .with_field("error_hook_ms", TraceValue::U64(error_hook_ms))
        .with_field("total_ms", TraceValue::U64(total_ms))
        .with_field("is_error", TraceValue::Bool(is_error));

    if let Some(sandbox_backend) = sandbox_backend {
        span = span.with_field(
            "sandbox_backend",
            TraceValue::String(sandbox_backend.into()),
        );
    }

    ctx.tracer.record(span);
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should stay after unix epoch")
        .as_millis() as i64
}

async fn maybe_compact_transcript(
    model_provider: &Arc<dyn ModelProvider>,
    plugin_registry: &Arc<PluginRegistry>,
    session_id: &SessionId,
    session_store: &dyn SessionStore,
    transcript: &mut TranscriptState,
    token_budget: u32,
) -> Result<bool, RuntimeError> {
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
            persist_compaction_checkpoint(
                session_store,
                session_id,
                view.event_ids.as_mut_slice(),
                &result,
            )
            .await?;
        }
        let _ = plugin_registry
            .hooks()
            .run(HookEvent::PostCompact {
                session: session_id.clone(),
                result,
            })
            .await;
        transcript.event_ids = view.event_ids;
        return Ok(true);
    }
    transcript.event_ids = view.event_ids;
    Ok(false)
}
