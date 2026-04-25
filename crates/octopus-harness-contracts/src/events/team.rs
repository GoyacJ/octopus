use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamCreatedEvent {
    pub team_id: TeamId,
    pub tenant_id: TenantId,
    pub name: String,
    pub topology_kind: TopologyKind,
    pub member_specs_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamMemberJoinedEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub spec_snapshot_id: BlobRef,
    pub spec_hash: [u8; 32],
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamMemberLeftEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub reason: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamMemberStalledEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub silent_for_ms: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentMessageSentEvent {
    pub team_id: TeamId,
    pub from: AgentId,
    pub to: Option<AgentId>,
    pub message_id: MessageId,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentMessageRoutedEvent {
    pub team_id: TeamId,
    pub message_id: MessageId,
    pub routed_to: Vec<AgentId>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamTurnCompletedEvent {
    pub team_id: TeamId,
    pub usage: UsageSnapshot,
    pub transcript_ref: Option<TranscriptRef>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TeamTerminatedEvent {
    pub team_id: TeamId,
    pub reason: TeamTerminationReason,
    pub at: DateTime<Utc>,
}
