use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentSpawnedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub tenant_id: TenantId,
    pub spec_snapshot_id: BlobRef,
    pub spec_hash: [u8; 32],
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentAnnouncedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub status: SubagentStatus,
    pub summary: String,
    pub result: Option<Value>,
    pub usage: UsageSnapshot,
    pub transcript_ref: Option<TranscriptRef>,
    pub renderer_id: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentTerminatedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub reason: SubagentTerminationReason,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentSpawnPausedEvent {
    pub tenant_id: TenantId,
    pub paused: bool,
    pub reason: Option<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentPermissionForwardedEvent {
    pub parent_session_id: SessionId,
    pub subagent_id: SubagentId,
    pub request_id: RequestId,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubagentPermissionResolvedEvent {
    pub parent_session_id: SessionId,
    pub subagent_id: SubagentId,
    pub request_id: RequestId,
    pub decision: Decision,
    pub at: DateTime<Utc>,
}
