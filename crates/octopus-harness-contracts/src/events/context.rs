use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompactionAppliedEvent {
    pub session_id: SessionId,
    pub strategy: CompactStrategyId,
    pub trigger: CompactTrigger,
    pub outcome: CompactOutcome,
    pub before_tokens: u64,
    pub after_tokens: u64,
    pub summary_ref: BlobRef,
    pub child_session_id: Option<SessionId>,
    pub handoff: Option<CompactionHandoff>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContextStageTransitionedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub from: Option<ContextStage>,
    pub to: ContextStage,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContextBudgetExceededEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub budget: BudgetKind,
    pub current_tokens: u64,
    pub limit_tokens: u64,
    pub at: DateTime<Utc>,
}
