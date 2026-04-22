use std::sync::Mutex;

use octopus_sdk_contracts::{AssistantEvent, ContentBlock, Message, SessionEvent, Usage};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageLedgerSnapshot {
    pub model_usage: Usage,
    pub sessions_started: u32,
    pub assistant_messages: u32,
    pub tool_calls: u32,
    pub asks: u32,
    pub renders: u32,
}

#[derive(Debug, Default)]
pub struct UsageLedger {
    snapshot: Mutex<UsageLedgerSnapshot>,
}

impl UsageLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_session_event(&self, event: &SessionEvent) -> Result<(), UsageLedgerError> {
        let mut snapshot = self.snapshot.lock().expect("usage ledger lock poisoned");
        match event {
            SessionEvent::SessionStarted { .. } => {
                snapshot.sessions_started = snapshot.sessions_started.saturating_add(1);
            }
            SessionEvent::AssistantMessage(message) => {
                snapshot.assistant_messages = snapshot.assistant_messages.saturating_add(1);
                snapshot.model_usage = &snapshot.model_usage + &usage_from_message(message)?;
            }
            SessionEvent::ToolExecuted { .. } => {
                snapshot.tool_calls = snapshot.tool_calls.saturating_add(1);
            }
            SessionEvent::Ask { .. } => {
                snapshot.asks = snapshot.asks.saturating_add(1);
            }
            SessionEvent::Render { .. } => {
                snapshot.renders = snapshot.renders.saturating_add(1);
            }
            SessionEvent::SessionPluginsSnapshot { .. }
            | SessionEvent::UserMessage(_)
            | SessionEvent::Checkpoint { .. }
            | SessionEvent::SessionEnded { .. } => {}
        }

        Ok(())
    }

    pub fn record_assistant_event(&self, event: &AssistantEvent) {
        if let AssistantEvent::Usage(usage) = event {
            let mut snapshot = self.snapshot.lock().expect("usage ledger lock poisoned");
            snapshot.model_usage = &snapshot.model_usage + usage;
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> UsageLedgerSnapshot {
        self.snapshot
            .lock()
            .expect("usage ledger lock poisoned")
            .clone()
    }
}

#[derive(Debug, Error)]
pub enum UsageLedgerError {
    #[error("assistant usage marker deserialization failed: {0}")]
    Deserialize(#[from] serde_json::Error),
}

pub(crate) fn usage_from_message(message: &Message) -> Result<Usage, UsageLedgerError> {
    usage_from_blocks(&message.content)
}

fn usage_from_blocks(blocks: &[ContentBlock]) -> Result<Usage, UsageLedgerError> {
    let mut total = Usage::default();

    for block in blocks {
        total = &total + &usage_from_block(block)?;
    }

    Ok(total)
}

fn usage_from_block(block: &ContentBlock) -> Result<Usage, UsageLedgerError> {
    match block {
        ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
            match serde_json::from_str::<AssistantEvent>(text) {
                Ok(AssistantEvent::Usage(usage)) => Ok(usage),
                Err(error) if looks_like_json(text) => Err(UsageLedgerError::Deserialize(error)),
                Ok(_) | Err(_) => Ok(Usage::default()),
            }
        }
        ContentBlock::ToolUse { .. } => Ok(Usage::default()),
        ContentBlock::ToolResult { content, .. } => usage_from_blocks(content),
    }
}

fn looks_like_json(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('{') && trimmed.ends_with('}')
}
