use serde::{Deserialize, Serialize};

use crate::EventId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategyTag {
    Summarize,
    ClearToolResults,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionResult {
    pub summary: String,
    pub folded_turn_ids: Vec<EventId>,
    pub tool_results_cleared: u32,
    pub tokens_before: u32,
    pub tokens_after: u32,
    pub strategy: CompactionStrategyTag,
}
