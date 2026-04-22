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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityProviderState {
    pub provider_key: String,
    pub state: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub degraded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityPlanSummary {
    pub visible_tools: Vec<String>,
    #[serde(default)]
    pub deferred_tools: Vec<String>,
    pub discoverable_skills: Vec<String>,
    #[serde(default)]
    pub available_resources: Vec<String>,
    #[serde(default)]
    pub hidden_capabilities: Vec<String>,
    #[serde(default)]
    pub discovered_tools: Vec<String>,
    #[serde(default)]
    pub activated_tools: Vec<String>,
    #[serde(default)]
    pub exposed_tools: Vec<String>,
    #[serde(default)]
    pub granted_tools: Vec<String>,
    #[serde(default)]
    pub pending_tools: Vec<String>,
    #[serde(default)]
    pub approved_tools: Vec<String>,
    #[serde(default)]
    pub auth_resolved_tools: Vec<String>,
    #[serde(default)]
    pub provider_fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityExecutionOutcome {
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub dispatch_kind: Option<String>,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default)]
    pub concurrency_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePendingMediation {
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub approval_layer: Option<String>,
    #[serde(default)]
    pub auth_challenge_id: Option<String>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub checkpoint_ref: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub escalation_reason: Option<String>,
    #[serde(default)]
    pub mediation_id: Option<String>,
    #[serde(default)]
    pub mediation_kind: String,
    #[serde(default)]
    pub dispatch_kind: Option<String>,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub concurrency_policy: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub required_permission: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub target_kind: String,
    #[serde(default)]
    pub target_ref: String,
    #[serde(default)]
    pub tool_name: Option<String>,
}

pub type RuntimePendingMediationSummary = RuntimePendingMediation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMediationOutcome {
    #[serde(default)]
    pub approval_layer: Option<String>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub checkpoint_ref: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub mediation_id: Option<String>,
    #[serde(default)]
    pub mediation_kind: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default)]
    pub resolved_at: Option<u64>,
    #[serde(default)]
    pub target_kind: String,
    #[serde(default)]
    pub target_ref: String,
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAuthChallengeSummary {
    #[serde(default)]
    pub approval_layer: String,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub checkpoint_ref: Option<String>,
    #[serde(default)]
    pub conversation_id: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub escalation_reason: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub dispatch_kind: Option<String>,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub concurrency_policy: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub required_permission: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub target_kind: String,
    #[serde(default)]
    pub target_ref: String,
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAuthStateSummary {
    #[serde(default)]
    pub challenged_provider_keys: Vec<String>,
    #[serde(default)]
    pub failed_provider_keys: Vec<String>,
    #[serde(default)]
    pub last_challenge_at: Option<u64>,
    #[serde(default)]
    pub pending_challenge_count: u64,
    #[serde(default)]
    pub resolved_provider_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePolicyDecisionSummary {
    #[serde(default)]
    pub allow_count: u64,
    #[serde(default)]
    pub approval_required_count: u64,
    #[serde(default)]
    pub auth_required_count: u64,
    #[serde(default)]
    pub compiled_at: Option<u64>,
    #[serde(default)]
    pub deferred_capability_count: u64,
    #[serde(default)]
    pub denied_exposure_count: u64,
    #[serde(default)]
    pub hidden_capability_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePendingMediationSummaryLegacy {
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub mediation_kind: String,
    #[serde(default)]
    pub reason: Option<String>,
}

pub type RuntimeCapabilitySummary = RuntimeCapabilityPlanSummary;
