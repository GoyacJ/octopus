use futures::StreamExt;
use octopus_sdk_contracts::{SessionEvent, SessionId, SubagentSummary};
use octopus_sdk_session::{EventRange, SessionError, SessionStore};
use thiserror::Error;

use crate::{
    session_span_id, session_trace_id, subagent_permission_span_id, tool_span_id, TraceSpan,
    TraceValue, Tracer, UsageLedger, UsageLedgerError,
};

pub struct ReplayTracer;

impl ReplayTracer {
    pub async fn replay_session(
        store: &dyn SessionStore,
        session_id: &SessionId,
        tracer: &dyn Tracer,
        usage_ledger: &UsageLedger,
    ) -> Result<(), ReplayError> {
        let mut stream = store.stream(session_id, EventRange::default()).await?;
        let mut trace_context = ReplayTraceContext::for_session(session_id);

        while let Some(event) = stream.next().await {
            let event = event?;
            tracer.record(span_for_event(session_id, &event, &trace_context));
            trace_context.observe(session_id, &event);
            usage_ledger.record_session_event(&event)?;
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Usage(#[from] UsageLedgerError),
}

fn span_for_event(
    session_id: &SessionId,
    event: &SessionEvent,
    trace_context: &ReplayTraceContext,
) -> TraceSpan {
    if let Some((name, summary)) = subagent_summary_event(event) {
        return subagent_summary_span(name, &summary);
    }

    let mut span = TraceSpan::new("session_event")
        .with_trace_id(trace_context.trace_id.clone())
        .with_agent_role(trace_context.agent_role.clone())
        .with_field("session_id", TraceValue::String(session_id.0.clone()))
        .with_field("kind", TraceValue::String(event_kind(event).into()));

    match event {
        SessionEvent::SessionStarted {
            model,
            config_snapshot_id,
            ..
        } => {
            span = span
                .with_span_id(trace_context.session_span_id.clone())
                .with_field("model_id", TraceValue::String(model.clone()))
                .with_field(
                    "config_snapshot_id",
                    TraceValue::String(config_snapshot_id.clone()),
                );
        }
        SessionEvent::ToolExecuted { call, .. } => {
            span = span
                .with_span_id(format!("tool_executed:{}", call.0))
                .with_parent_span_id(trace_context.session_span_id.clone())
                .with_field("tool_call_id", TraceValue::String(call.0.clone()));
        }
        SessionEvent::PermissionDecision { call, outcome, .. } => {
            span = span
                .with_span_id(if trace_context.agent_role == "main" {
                    format!("permission:{}", call.0)
                } else {
                    subagent_permission_span_id(&call.0)
                })
                .with_parent_span_id(tool_span_id(&call.0))
                .with_field("tool_call_id", TraceValue::String(call.0.clone()))
                .with_field(
                    "permission_decision",
                    TraceValue::String(format!("{outcome:?}")),
                );
        }
        SessionEvent::Render { lifecycle, .. } => {
            let span_id = lifecycle
                .tool_call_id
                .as_ref()
                .map(|call| format!("render:{}", call.0))
                .unwrap_or_else(|| format!("render:{}", session_id.0));
            span = span
                .with_span_id(span_id)
                .with_parent_span_id(trace_context.session_span_id.clone());
            if let Some(call) = &lifecycle.tool_call_id {
                span = span.with_field("tool_call_id", TraceValue::String(call.0.clone()));
            }
        }
        SessionEvent::AssistantMessage(_)
        | SessionEvent::UserMessage(_)
        | SessionEvent::Ask { .. }
        | SessionEvent::Checkpoint { .. }
        | SessionEvent::SessionEnded { .. }
        | SessionEvent::SessionPluginsSnapshot { .. } => {
            span = span
                .with_span_id(format!("{}:{}", event_kind(event), session_id.0))
                .with_parent_span_id(trace_context.session_span_id.clone());
        }
    }

    span
}

#[derive(Clone)]
struct ReplayTraceContext {
    trace_id: String,
    session_span_id: String,
    agent_role: String,
}

impl ReplayTraceContext {
    fn for_session(session_id: &SessionId) -> Self {
        Self {
            trace_id: session_trace_id(&session_id.0),
            session_span_id: session_span_id(&session_id.0),
            agent_role: "main".into(),
        }
    }

    fn observe(&mut self, session_id: &SessionId, event: &SessionEvent) {
        let Some(("subagent_spawn", summary)) = subagent_summary_event(event) else {
            return;
        };
        if summary.session_id != *session_id {
            return;
        }
        self.trace_id = summary.trace_id;
        self.session_span_id = summary.span_id;
        self.agent_role = summary.agent_role;
    }
}

fn subagent_summary_event(event: &SessionEvent) -> Option<(&'static str, SubagentSummary)> {
    let SessionEvent::Render { blocks, .. } = event else {
        return None;
    };

    for block in blocks {
        let title = block
            .payload
            .get("title")
            .and_then(serde_json::Value::as_str)?;
        let summary = block.payload.get("summary")?.clone();
        let summary = serde_json::from_value::<SubagentSummary>(summary).ok()?;
        match title {
            "subagent.spawn" => return Some(("subagent_spawn", summary)),
            "subagent.summary" => return Some(("subagent_summary", summary)),
            _ => {}
        }
    }

    None
}

fn subagent_summary_span(name: &str, summary: &SubagentSummary) -> TraceSpan {
    let mut span = TraceSpan::new(name)
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
        );

    if let Some(resume_session_id) = &summary.resume_session_id {
        span = span.with_field(
            "resume_session_id",
            TraceValue::String(resume_session_id.0.clone()),
        );
    }

    span
}

fn event_kind(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::SessionStarted { .. } => "session_started",
        SessionEvent::SessionPluginsSnapshot { .. } => "session_plugins_snapshot",
        SessionEvent::UserMessage(_) => "user_message",
        SessionEvent::AssistantMessage(_) => "assistant_message",
        SessionEvent::ToolExecuted { .. } => "tool_executed",
        SessionEvent::PermissionDecision { .. } => "permission_decision",
        SessionEvent::Render { .. } => "render",
        SessionEvent::Ask { .. } => "ask",
        SessionEvent::Checkpoint { .. } => "checkpoint",
        SessionEvent::SessionEnded { .. } => "session_ended",
    }
}
