use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseRequestedEvent {
    pub tool_use_id: ToolUseId,
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tool_name: String,
    pub input: Value,
    pub origin: ToolOrigin,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseApprovedEvent {
    pub tool_use_id: ToolUseId,
    pub decision_id: DecisionId,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseDeniedEvent {
    pub tool_use_id: ToolUseId,
    pub reason: String,
    pub decided_by: DecidedBy,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseCompletedEvent {
    pub tool_use_id: ToolUseId,
    pub result: ToolResultEnvelope,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseFailedEvent {
    pub tool_use_id: ToolUseId,
    pub error: String,
    pub retriable: bool,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolUseHeartbeatEvent {
    pub tool_use_id: ToolUseId,
    pub stage: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolResultOffloadedEvent {
    pub tool_use_id: ToolUseId,
    pub blob_ref: BlobRef,
    pub original_size: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolRegistrationShadowedEvent {
    pub tool_name: String,
    pub reason: ShadowReason,
    pub at: DateTime<Utc>,
}
