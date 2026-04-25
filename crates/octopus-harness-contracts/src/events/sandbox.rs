use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxExecutionStartedEvent {
    pub tool_use_id: ToolUseId,
    pub policy: SandboxPolicySummary,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxExecutionCompletedEvent {
    pub tool_use_id: ToolUseId,
    pub status: SandboxExitStatus,
    pub duration_ms: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxActivityHeartbeatEvent {
    pub tool_use_id: ToolUseId,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxActivityTimeoutFiredEvent {
    pub tool_use_id: ToolUseId,
    pub timeout_ms: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxOutputSpilledEvent {
    pub tool_use_id: ToolUseId,
    pub stream: String,
    pub blob_ref: BlobRef,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxBackpressureAppliedEvent {
    pub tool_use_id: ToolUseId,
    pub reason: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxSnapshotCreatedEvent {
    pub snapshot_id: SnapshotId,
    pub session_id: SessionId,
    pub kind: SessionSnapshotKind,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxContainerLifecycleTransitionEvent {
    pub session_id: SessionId,
    pub from: String,
    pub to: String,
    pub at: DateTime<Utc>,
}
