use std::path::PathBuf;

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ContentBlock, EventId, EventSink,
    Message, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, SecretValue, SecretVault,
    SessionEvent, SessionId, StopReason, SubagentError, SubagentOutput, SubagentSummary,
    ToolCallRequest, Usage, VaultError,
};
use octopus_sdk_model::{
    CacheControlStrategy, ModelId, ModelRequest, ModelRole, ResponseFormat, ThinkingConfig,
};
use octopus_sdk_observability::{
    subagent_permission_span_id, subagent_span_id, tool_span_id, TraceSpan, TraceValue, Tracer,
};
use serde_json::json;

use crate::{ParentSessionContext, SubagentContext};

pub(crate) fn build_request(context: &SubagentContext, messages: &[Message]) -> ModelRequest {
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
            .assemble_surface(&octopus_sdk_tools::ToolSurfaceState::default())
            .request_tools(),
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

pub(crate) fn usage_message(usage: &Usage) -> Result<Message, SubagentError> {
    Ok(Message {
        role: octopus_sdk_contracts::Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::Usage(*usage)).map_err(|error| {
                SubagentError::Storage {
                    reason: error.to_string(),
                }
            })?,
        }],
    })
}

pub(crate) fn stop_message(stop_reason: StopReason) -> Result<Message, SubagentError> {
    Ok(Message {
        role: octopus_sdk_contracts::Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::MessageStop { stop_reason }).map_err(
                |error| SubagentError::Storage {
                    reason: error.to_string(),
                },
            )?,
        }],
    })
}

pub(crate) fn relative_scratchpad_path(session_id: &SessionId) -> PathBuf {
    PathBuf::from("runtime")
        .join("notes")
        .join(format!("{}.md", session_id.0))
}

pub(crate) fn subagent_render_event(
    title: &str,
    text: impl Into<String>,
    meta: &SubagentSummary,
) -> SessionEvent {
    SessionEvent::Render {
        blocks: vec![RenderBlock {
            kind: RenderKind::Markdown,
            payload: json!({
                "title": title,
                "text": text.into(),
                "summary": meta,
            }),
            meta: RenderMeta {
                id: EventId::new_v4(),
                parent: None,
                ts_ms: now_millis(),
            },
        }],
        lifecycle: RenderLifecycle::assistant_message(),
    }
}

pub(crate) fn subagent_summary(
    parent: &ParentSessionContext,
    context: &SubagentContext,
    child_session_id: &SessionId,
    turns: u16,
    tokens_used: u32,
    duration_ms: u64,
    resume_session_id: Option<SessionId>,
) -> SubagentSummary {
    SubagentSummary {
        session_id: child_session_id.clone(),
        parent_session_id: context.parent_session.clone(),
        resume_session_id,
        spec_id: context.spec.id.clone(),
        agent_role: context.spec.agent_role.clone(),
        parent_agent_role: parent.trace.agent_role.clone(),
        turns,
        tokens_used,
        duration_ms,
        trace_id: parent.trace.trace_id.clone(),
        span_id: subagent_span_id(&child_session_id.0),
        parent_span_id: parent.trace.span_id.clone(),
        model_id: parent.trace.model_id.clone(),
        model_version: parent.trace.model_version.clone(),
        config_snapshot_id: parent.trace.config_snapshot_id.clone(),
        permission_mode: context.spec.permission_mode,
        allowed_tools: context.allowed_tools(),
    }
}

pub(crate) fn emit_subagent_summary_trace(
    tracer: &dyn Tracer,
    name: &str,
    summary: &SubagentSummary,
) {
    tracer.record(
        TraceSpan::new(name)
            .with_trace_id(summary.trace_id.clone())
            .with_span_id(summary.span_id.clone())
            .with_parent_span_id(summary.parent_span_id.clone())
            .with_agent_role(summary.agent_role.clone())
            .with_field(
                "session_id",
                TraceValue::String(summary.session_id.0.clone()),
            )
            .with_field(
                "parent_session_id",
                TraceValue::String(summary.parent_session_id.0.clone()),
            )
            .with_field("spec_id", TraceValue::String(summary.spec_id.clone()))
            .with_field(
                "parent_agent_role",
                TraceValue::String(summary.parent_agent_role.clone()),
            )
            .with_field("model_id", TraceValue::String(summary.model_id.clone()))
            .with_field(
                "model_version",
                TraceValue::String(summary.model_version.clone()),
            )
            .with_field(
                "config_snapshot_id",
                TraceValue::String(summary.config_snapshot_id.clone()),
            )
            .with_field(
                "permission_mode",
                TraceValue::String(format!("{:?}", summary.permission_mode)),
            )
            .with_field("turns", TraceValue::U64(u64::from(summary.turns)))
            .with_field(
                "tokens_used",
                TraceValue::U64(u64::from(summary.tokens_used)),
            )
            .with_field("duration_ms", TraceValue::U64(summary.duration_ms))
            .with_field(
                "allowed_tools",
                TraceValue::Json(serde_json::json!(summary.allowed_tools)),
            ),
    );
}

pub(crate) fn emit_subagent_permission_trace(
    tracer: &dyn Tracer,
    parent: &ParentSessionContext,
    context: &SubagentContext,
    child_session_id: &SessionId,
    call: &ToolCallRequest,
    permission_decision: &str,
) {
    tracer.record(
        TraceSpan::new("subagent_permission_decision")
            .with_trace_id(parent.trace.trace_id.clone())
            .with_span_id(subagent_permission_span_id(&call.id.0))
            .with_parent_span_id(tool_span_id(&call.id.0))
            .with_agent_role(context.spec.agent_role.clone())
            .with_field("session_id", TraceValue::String(child_session_id.0.clone()))
            .with_field(
                "parent_session_id",
                TraceValue::String(context.parent_session.0.clone()),
            )
            .with_field("tool_call_id", TraceValue::String(call.id.0.clone()))
            .with_field("tool_name", TraceValue::String(call.name.clone()))
            .with_field(
                "permission_mode",
                TraceValue::String(format!("{:?}", context.spec.permission_mode)),
            )
            .with_field(
                "permission_decision",
                TraceValue::String(permission_decision.to_string()),
            )
            .with_field(
                "model_id",
                TraceValue::String(parent.trace.model_id.clone()),
            )
            .with_field(
                "model_version",
                TraceValue::String(parent.trace.model_version.clone()),
            )
            .with_field(
                "config_snapshot_id",
                TraceValue::String(parent.trace.config_snapshot_id.clone()),
            ),
    );
}

pub(crate) fn emit_subagent_tool_trace(
    tracer: &dyn Tracer,
    parent: &ParentSessionContext,
    context: &SubagentContext,
    child_session_id: &SessionId,
    call: &ToolCallRequest,
    input_hash: &str,
    permission_decision: &str,
    duration_ms: u64,
    is_error: bool,
) {
    tracer.record(
        TraceSpan::new("subagent_tool_execution")
            .with_trace_id(parent.trace.trace_id.clone())
            .with_span_id(tool_span_id(&call.id.0))
            .with_parent_span_id(subagent_span_id(&child_session_id.0))
            .with_agent_role(context.spec.agent_role.clone())
            .with_field("session_id", TraceValue::String(child_session_id.0.clone()))
            .with_field(
                "parent_session_id",
                TraceValue::String(context.parent_session.0.clone()),
            )
            .with_field("tool_call_id", TraceValue::String(call.id.0.clone()))
            .with_field("tool_name", TraceValue::String(call.name.clone()))
            .with_field("input_hash", TraceValue::String(input_hash.to_string()))
            .with_field(
                "permission_mode",
                TraceValue::String(format!("{:?}", context.spec.permission_mode)),
            )
            .with_field(
                "permission_decision",
                TraceValue::String(permission_decision.to_string()),
            )
            .with_field(
                "model_id",
                TraceValue::String(parent.trace.model_id.clone()),
            )
            .with_field(
                "model_version",
                TraceValue::String(parent.trace.model_version.clone()),
            )
            .with_field(
                "config_snapshot_id",
                TraceValue::String(parent.trace.config_snapshot_id.clone()),
            )
            .with_field("duration_ms", TraceValue::U64(duration_ms))
            .with_field("is_error", TraceValue::Bool(is_error)),
    );
}

pub(crate) fn output_meta(output: &SubagentOutput) -> &SubagentSummary {
    match output {
        SubagentOutput::Summary { meta, .. }
        | SubagentOutput::FileRef { meta, .. }
        | SubagentOutput::Json { meta, .. } => meta,
    }
}

pub(crate) fn provider_error(error: octopus_sdk_model::ModelError) -> SubagentError {
    SubagentError::Provider {
        reason: error.to_string(),
    }
}

pub(crate) fn storage_error(error: octopus_sdk_session::SessionError) -> SubagentError {
    SubagentError::Storage {
        reason: error.to_string(),
    }
}

pub(crate) fn memory_error(error: octopus_sdk_contracts::MemoryError) -> SubagentError {
    SubagentError::Storage {
        reason: error.to_string(),
    }
}

pub(crate) fn max_turns_reached(context: &SubagentContext) -> bool {
    context.spec.max_turns > 0 && context.turns() >= context.spec.max_turns
}

pub(crate) struct NoopAskResolver;
pub(crate) struct NoopEventSink;
pub(crate) struct NoopSecretVault;

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
