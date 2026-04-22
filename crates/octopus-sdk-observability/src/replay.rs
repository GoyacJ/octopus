use futures::StreamExt;
use octopus_sdk_contracts::{SessionEvent, SessionId};
use octopus_sdk_session::{EventRange, SessionError, SessionStore};
use thiserror::Error;

use crate::{TraceSpan, TraceValue, Tracer, UsageLedger, UsageLedgerError};

pub struct ReplayTracer;

impl ReplayTracer {
    pub async fn replay_session(
        store: &dyn SessionStore,
        session_id: &SessionId,
        tracer: &dyn Tracer,
        usage_ledger: &UsageLedger,
    ) -> Result<(), ReplayError> {
        let mut stream = store.stream(session_id, EventRange::default()).await?;

        while let Some(event) = stream.next().await {
            let event = event?;
            tracer.record(span_for_event(session_id, &event));
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

fn span_for_event(session_id: &SessionId, event: &SessionEvent) -> TraceSpan {
    TraceSpan::new("session_event")
        .with_field("session_id", TraceValue::String(session_id.0.clone()))
        .with_field("kind", TraceValue::String(event_kind(event).into()))
}

fn event_kind(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::SessionStarted { .. } => "session_started",
        SessionEvent::SessionPluginsSnapshot { .. } => "session_plugins_snapshot",
        SessionEvent::UserMessage(_) => "user_message",
        SessionEvent::AssistantMessage(_) => "assistant_message",
        SessionEvent::ToolExecuted { .. } => "tool_executed",
        SessionEvent::Render { .. } => "render",
        SessionEvent::Ask { .. } => "ask",
        SessionEvent::Checkpoint { .. } => "checkpoint",
        SessionEvent::SessionEnded { .. } => "session_ended",
    }
}
