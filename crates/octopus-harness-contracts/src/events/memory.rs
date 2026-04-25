use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryUpsertedEvent {
    pub memory_id: MemoryId,
    pub tenant_id: TenantId,
    pub actor: MemoryActor,
    pub target: MemoryWriteTarget,
    pub action: MemoryWriteAction,
    pub source: MemorySource,
    pub content_hash: ContentHash,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryRecalledEvent {
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub actor: MemoryActor,
    pub recalled: Vec<MemoryId>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryRecallDegradedEvent {
    pub run_id: RunId,
    pub reason: MemoryRecallDegradedReason,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryRecallSkippedEvent {
    pub run_id: RunId,
    pub reason: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryThreatDetectedEvent {
    pub memory_id: Option<MemoryId>,
    pub tenant_id: TenantId,
    pub category: ThreatCategory,
    pub action: ThreatAction,
    pub direction: ThreatDirection,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemdirOverflowEvent {
    pub tenant_id: TenantId,
    pub file: MemdirFileTag,
    pub limit_bytes: u64,
    pub actual_bytes: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryConsolidationRanEvent {
    pub tenant_id: TenantId,
    pub input_count: u32,
    pub output_count: u32,
    pub at: DateTime<Utc>,
}
