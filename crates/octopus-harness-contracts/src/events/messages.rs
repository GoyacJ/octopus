use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UserMessageAppendedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub message_id: MessageId,
    pub input: TurnInput,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AssistantDeltaProducedEvent {
    pub run_id: RunId,
    pub message_id: MessageId,
    pub delta_id: DeltaId,
    pub content: Value,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AssistantMessageCompletedEvent {
    pub run_id: RunId,
    pub message: Message,
    pub stop_reason: StopReason,
    pub usage: UsageSnapshot,
    pub at: DateTime<Utc>,
}
