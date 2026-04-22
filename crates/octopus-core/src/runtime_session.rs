use serde::{Deserialize, Serialize};

use crate::{
    ArtifactVersionReference, ResolvedExecutionTarget, RuntimeAuthChallengeSummary,
    RuntimeAuthStateSummary, RuntimeCapabilityExecutionOutcome, RuntimeCapabilityPlanSummary,
    RuntimeCapabilityProviderState, RuntimeMediationOutcome, RuntimeMemoryFreshnessSummary,
    RuntimeMemoryProposal, RuntimeMemorySelectionSummary, RuntimeMemorySummary,
    RuntimePendingMediationSummary, RuntimePolicyDecisionSummary, RuntimeSelectedMemoryItem,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionPolicySnapshot {
    #[serde(default)]
    pub selected_actor_ref: String,
    #[serde(default)]
    pub selected_configured_model_id: String,
    #[serde(default)]
    pub execution_permission_mode: String,
    #[serde(default)]
    pub config_snapshot_id: String,
    #[serde(default)]
    pub manifest_revision: String,
    #[serde(default = "default_runtime_policy_envelope")]
    pub capability_policy: serde_json::Value,
    #[serde(default = "default_runtime_policy_envelope")]
    pub memory_policy: serde_json::Value,
    #[serde(default = "default_runtime_policy_envelope")]
    pub delegation_policy: serde_json::Value,
    #[serde(default = "default_runtime_policy_envelope")]
    pub approval_preference: serde_json::Value,
}

impl Default for RuntimeSessionPolicySnapshot {
    fn default() -> Self {
        Self {
            selected_actor_ref: String::new(),
            selected_configured_model_id: String::new(),
            execution_permission_mode: String::new(),
            config_snapshot_id: String::new(),
            manifest_revision: String::new(),
            capability_policy: default_runtime_policy_envelope(),
            memory_policy: default_runtime_policy_envelope(),
            delegation_policy: default_runtime_policy_envelope(),
            approval_preference: default_runtime_policy_envelope(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeUsageSummary {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTraceContext {
    pub session_id: String,
    pub trace_id: String,
    pub turn_id: String,
    #[serde(default)]
    pub parent_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePendingApproval {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub required_permission_mode: String,
    pub reason: String,
    pub tool_use_id: String,
    pub trace_context: RuntimeTraceContext,
}

fn default_runtime_policy_envelope() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRunCheckpoint {
    #[serde(default)]
    pub approval_layer: Option<String>,
    #[serde(default)]
    pub broker_decision: Option<String>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub checkpoint_artifact_ref: Option<String>,
    #[serde(default)]
    pub current_iteration_index: u32,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub dispatch_kind: Option<String>,
    #[serde(default)]
    pub concurrency_policy: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub usage_summary: RuntimeUsageSummary,
    #[serde(default)]
    pub pending_approval: Option<ApprovalRequestRecord>,
    #[serde(default)]
    pub pending_auth_challenge: Option<RuntimeAuthChallengeSummary>,
    #[serde(default)]
    pub pending_mediation: Option<RuntimePendingMediationSummary>,
    #[serde(default)]
    pub provider_key: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub required_permission: Option<String>,
    #[serde(default)]
    pub requires_approval: Option<bool>,
    #[serde(default)]
    pub requires_auth: Option<bool>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub target_ref: Option<String>,
    #[serde(default)]
    pub capability_plan_summary: RuntimeCapabilityPlanSummary,
    #[serde(default)]
    pub last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    #[serde(default)]
    pub last_mediation_outcome: Option<RuntimeMediationOutcome>,
}

fn default_runtime_session_kind() -> String {
    "project".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSubrunSummary {
    pub run_id: String,
    pub parent_run_id: Option<String>,
    pub actor_ref: String,
    pub label: String,
    pub status: String,
    pub run_kind: String,
    pub delegated_by_tool_call_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub mailbox_ref: Option<String>,
    pub handoff_ref: Option<String>,
    pub started_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMailboxSummary {
    pub mailbox_ref: String,
    pub channel: String,
    pub status: String,
    pub pending_count: u64,
    pub total_messages: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHandoffSummary {
    pub handoff_ref: String,
    pub mailbox_ref: String,
    pub sender_actor_ref: String,
    pub receiver_actor_ref: String,
    pub state: String,
    pub artifact_refs: Vec<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkflowSummary {
    pub workflow_run_id: String,
    pub label: String,
    pub status: String,
    pub total_steps: u64,
    pub completed_steps: u64,
    pub current_step_id: Option<String>,
    pub current_step_label: Option<String>,
    pub background_capable: bool,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkflowStepSummary {
    pub step_id: String,
    pub node_kind: String,
    pub label: String,
    pub actor_ref: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub parent_run_id: Option<String>,
    #[serde(default)]
    pub delegated_by_tool_call_id: Option<String>,
    #[serde(default)]
    pub mailbox_ref: Option<String>,
    #[serde(default)]
    pub handoff_ref: Option<String>,
    pub status: String,
    pub started_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkflowBlockingSummary {
    pub run_id: String,
    pub actor_ref: String,
    pub mediation_kind: String,
    pub state: String,
    pub target_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkflowRunDetail {
    pub workflow_run_id: String,
    pub status: String,
    pub current_step_id: Option<String>,
    pub current_step_label: Option<String>,
    pub total_steps: u64,
    pub completed_steps: u64,
    pub background_capable: bool,
    #[serde(default)]
    pub steps: Vec<RuntimeWorkflowStepSummary>,
    #[serde(default)]
    pub blocking: Option<RuntimeWorkflowBlockingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackgroundRunSummary {
    pub run_id: String,
    pub workflow_run_id: Option<String>,
    pub status: String,
    pub background_capable: bool,
    #[serde(default)]
    pub continuation_state: String,
    #[serde(default)]
    pub blocking: Option<RuntimeWorkflowBlockingSummary>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkerDispatchSummary {
    pub total_subruns: u64,
    pub active_subruns: u64,
    pub completed_subruns: u64,
    pub failed_subruns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionSummary {
    pub id: String,
    pub conversation_id: String,
    pub project_id: String,
    pub title: String,
    #[serde(default = "default_runtime_session_kind")]
    pub session_kind: String,
    pub status: String,
    pub updated_at: u64,
    pub last_message_preview: Option<String>,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub started_from_scope_set: Vec<String>,
    #[serde(default)]
    pub selected_actor_ref: String,
    #[serde(default)]
    pub manifest_revision: String,
    #[serde(default)]
    pub session_policy: RuntimeSessionPolicySnapshot,
    #[serde(default)]
    pub active_run_id: String,
    #[serde(default)]
    pub subrun_count: u64,
    #[serde(default)]
    pub workflow: Option<RuntimeWorkflowSummary>,
    #[serde(default)]
    pub pending_mailbox: Option<RuntimeMailboxSummary>,
    #[serde(default)]
    pub background_run: Option<RuntimeBackgroundRunSummary>,
    #[serde(default)]
    pub memory_summary: RuntimeMemorySummary,
    #[serde(default)]
    pub memory_selection_summary: RuntimeMemorySelectionSummary,
    #[serde(default)]
    pub pending_memory_proposal_count: u64,
    #[serde(default)]
    pub memory_state_ref: String,
    #[serde(default, rename = "capabilityPlanSummary", alias = "capabilitySummary")]
    pub capability_summary: RuntimeCapabilityPlanSummary,
    #[serde(default)]
    pub provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    #[serde(default)]
    pub auth_state_summary: RuntimeAuthStateSummary,
    #[serde(default)]
    pub pending_mediation: Option<RuntimePendingMediationSummary>,
    #[serde(default)]
    pub policy_decision_summary: RuntimePolicyDecisionSummary,
    #[serde(default)]
    pub last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRunSnapshot {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub status: String,
    pub current_step: String,
    pub started_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub selected_memory: Vec<RuntimeSelectedMemoryItem>,
    #[serde(default)]
    pub freshness_summary: Option<RuntimeMemoryFreshnessSummary>,
    #[serde(default)]
    pub pending_memory_proposal: Option<RuntimeMemoryProposal>,
    #[serde(default)]
    pub memory_state_ref: String,
    pub configured_model_id: Option<String>,
    pub configured_model_name: Option<String>,
    pub model_id: Option<String>,
    pub consumed_tokens: Option<u32>,
    pub next_action: Option<String>,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub started_from_scope_set: Vec<String>,
    pub run_kind: String,
    pub parent_run_id: Option<String>,
    #[serde(default)]
    pub actor_ref: String,
    pub delegated_by_tool_call_id: Option<String>,
    #[serde(default)]
    pub workflow_run: Option<String>,
    #[serde(default)]
    pub workflow_run_detail: Option<RuntimeWorkflowRunDetail>,
    #[serde(default)]
    pub mailbox_ref: Option<String>,
    #[serde(default)]
    pub handoff_ref: Option<String>,
    #[serde(default)]
    pub background_state: Option<String>,
    #[serde(default)]
    pub worker_dispatch: Option<RuntimeWorkerDispatchSummary>,
    pub approval_state: String,
    #[serde(default)]
    pub approval_target: Option<ApprovalRequestRecord>,
    #[serde(default)]
    pub auth_target: Option<RuntimeAuthChallengeSummary>,
    pub usage_summary: RuntimeUsageSummary,
    pub artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deliverable_refs: Vec<ArtifactVersionReference>,
    pub trace_context: RuntimeTraceContext,
    #[serde(default)]
    pub checkpoint: RuntimeRunCheckpoint,
    #[serde(default)]
    pub capability_plan_summary: RuntimeCapabilityPlanSummary,
    #[serde(default)]
    pub provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    #[serde(default)]
    pub pending_mediation: Option<RuntimePendingMediationSummary>,
    #[serde(default)]
    pub last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    #[serde(default)]
    pub last_mediation_outcome: Option<RuntimeMediationOutcome>,
    pub resolved_target: Option<ResolvedExecutionTarget>,
    pub requested_actor_kind: Option<String>,
    pub requested_actor_id: Option<String>,
    pub resolved_actor_kind: Option<String>,
    pub resolved_actor_id: Option<String>,
    pub resolved_actor_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMessage {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub sender_type: String,
    pub sender_label: String,
    pub content: String,
    pub timestamp: u64,
    pub configured_model_id: Option<String>,
    pub configured_model_name: Option<String>,
    pub model_id: Option<String>,
    pub status: String,
    pub requested_actor_kind: Option<String>,
    pub requested_actor_id: Option<String>,
    pub resolved_actor_kind: Option<String>,
    pub resolved_actor_id: Option<String>,
    pub resolved_actor_label: Option<String>,
    pub used_default_actor: Option<bool>,
    pub resource_ids: Option<Vec<String>>,
    pub attachments: Option<Vec<String>>,
    pub artifacts: Option<Vec<String>>,
    pub deliverable_refs: Option<Vec<ArtifactVersionReference>>,
    pub usage: Option<serde_json::Value>,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub process_entries: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTraceItem {
    pub id: String,
    pub session_id: String,
    pub run_id: String,
    pub conversation_id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub tone: String,
    pub timestamp: u64,
    pub actor: String,
    pub actor_kind: Option<String>,
    pub actor_id: Option<String>,
    pub related_message_id: Option<String>,
    pub related_tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub tool_name: String,
    pub summary: String,
    pub detail: String,
    pub risk_level: String,
    pub created_at: u64,
    pub status: String,
    #[serde(default)]
    pub approval_layer: Option<String>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub checkpoint_ref: Option<String>,
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
    pub target_kind: Option<String>,
    #[serde(default)]
    pub target_ref: Option<String>,
    #[serde(default)]
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventEnvelope {
    pub id: String,
    pub event_type: String,
    pub kind: Option<String>,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub session_id: String,
    pub conversation_id: String,
    pub run_id: Option<String>,
    #[serde(default)]
    pub parent_run_id: Option<String>,
    pub emitted_at: u64,
    pub sequence: u64,
    #[serde(default)]
    pub iteration: Option<u32>,
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    #[serde(default)]
    pub workflow_step_id: Option<String>,
    #[serde(default)]
    pub actor_ref: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub approval_layer: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub target_ref: Option<String>,
    pub run: Option<RuntimeRunSnapshot>,
    pub message: Option<RuntimeMessage>,
    #[serde(default)]
    pub memory_proposal: Option<RuntimeMemoryProposal>,
    #[serde(default)]
    pub memory_selection_summary: Option<RuntimeMemorySelectionSummary>,
    #[serde(default)]
    pub freshness_summary: Option<RuntimeMemoryFreshnessSummary>,
    #[serde(default)]
    pub selected_memory: Option<Vec<RuntimeSelectedMemoryItem>>,
    pub trace: Option<RuntimeTraceItem>,
    pub approval: Option<ApprovalRequestRecord>,
    #[serde(default)]
    pub auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pub decision: Option<String>,
    pub summary: Option<RuntimeSessionSummary>,
    pub error: Option<String>,
    #[serde(default)]
    pub capability_plan_summary: Option<RuntimeCapabilityPlanSummary>,
    #[serde(default)]
    pub provider_state_summary: Option<Vec<RuntimeCapabilityProviderState>>,
    #[serde(default)]
    pub pending_mediation: Option<RuntimePendingMediationSummary>,
    #[serde(default)]
    pub last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    #[serde(default)]
    pub last_mediation_outcome: Option<RuntimeMediationOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionDetail {
    pub summary: RuntimeSessionSummary,
    #[serde(default)]
    pub selected_actor_ref: String,
    #[serde(default)]
    pub manifest_revision: String,
    #[serde(default)]
    pub session_policy: RuntimeSessionPolicySnapshot,
    #[serde(default)]
    pub active_run_id: String,
    #[serde(default)]
    pub subrun_count: u64,
    #[serde(default)]
    pub workflow: Option<RuntimeWorkflowSummary>,
    #[serde(default)]
    pub pending_mailbox: Option<RuntimeMailboxSummary>,
    #[serde(default)]
    pub background_run: Option<RuntimeBackgroundRunSummary>,
    #[serde(default)]
    pub memory_summary: RuntimeMemorySummary,
    #[serde(default)]
    pub memory_selection_summary: RuntimeMemorySelectionSummary,
    #[serde(default)]
    pub pending_memory_proposal_count: u64,
    #[serde(default)]
    pub memory_state_ref: String,
    #[serde(default, rename = "capabilityPlanSummary", alias = "capabilitySummary")]
    pub capability_summary: RuntimeCapabilityPlanSummary,
    #[serde(default)]
    pub provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    #[serde(default)]
    pub auth_state_summary: RuntimeAuthStateSummary,
    #[serde(default)]
    pub pending_mediation: Option<RuntimePendingMediationSummary>,
    #[serde(default)]
    pub policy_decision_summary: RuntimePolicyDecisionSummary,
    #[serde(default)]
    pub last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    pub run: RuntimeRunSnapshot,
    #[serde(default)]
    pub subruns: Vec<RuntimeSubrunSummary>,
    #[serde(default)]
    pub handoffs: Vec<RuntimeHandoffSummary>,
    pub messages: Vec<RuntimeMessage>,
    pub trace: Vec<RuntimeTraceItem>,
    pub pending_approval: Option<ApprovalRequestRecord>,
}
