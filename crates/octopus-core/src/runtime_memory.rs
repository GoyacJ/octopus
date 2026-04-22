use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemorySummary {
    pub summary: String,
    pub durable_memory_count: u64,
    pub selected_memory_ids: Vec<String>,
}

fn default_runtime_memory_recall_mode() -> String {
    "default".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemorySelectionSummary {
    pub total_candidate_count: u64,
    pub selected_count: u64,
    pub ignored_count: u64,
    #[serde(default = "default_runtime_memory_recall_mode")]
    pub recall_mode: String,
    #[serde(default)]
    pub selected_memory_ids: Vec<String>,
}

impl Default for RuntimeMemorySelectionSummary {
    fn default() -> Self {
        Self {
            total_candidate_count: 0,
            selected_count: 0,
            ignored_count: 0,
            recall_mode: default_runtime_memory_recall_mode(),
            selected_memory_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSelectedMemoryItem {
    pub memory_id: String,
    pub title: String,
    pub summary: String,
    pub kind: String,
    pub scope: String,
    #[serde(default)]
    pub owner_ref: Option<String>,
    #[serde(default)]
    pub source_run_id: Option<String>,
    pub freshness_state: String,
    #[serde(default)]
    pub last_validated_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemoryProposalReview {
    pub decision: String,
    pub reviewed_at: u64,
    #[serde(default)]
    pub reviewer_ref: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemoryProposal {
    pub proposal_id: String,
    pub session_id: String,
    pub source_run_id: String,
    pub memory_id: String,
    pub title: String,
    pub summary: String,
    pub kind: String,
    pub scope: String,
    pub proposal_state: String,
    pub proposal_reason: String,
    #[serde(default)]
    pub review: Option<RuntimeMemoryProposalReview>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub normalized_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMemoryFreshnessSummary {
    pub freshness_required: bool,
    pub fresh_count: u64,
    pub stale_count: u64,
}
