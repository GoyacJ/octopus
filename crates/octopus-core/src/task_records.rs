use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::ArtifactVersionReference;

pub fn default_project_permission_allow() -> String {
    "allow".into()
}

pub fn default_project_permission_inherit() -> String {
    "inherit".into()
}

pub fn default_task_context_pin_mode() -> String {
    "snapshot".into()
}

pub fn default_task_context_resolution_mode() -> String {
    "explicit_only".into()
}

pub fn default_task_view_status() -> String {
    "configured".into()
}

pub fn default_task_lifecycle_status() -> String {
    "draft".into()
}

pub fn default_task_trigger_type() -> String {
    "manual".into()
}

pub fn default_task_run_status() -> String {
    "queued".into()
}

pub fn default_task_intervention_status() -> String {
    "accepted".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskContextRef {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub ref_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub version_ref: Option<String>,
    #[serde(default = "default_task_context_pin_mode")]
    pub pin_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskContextBundle {
    #[serde(default)]
    pub refs: Vec<TaskContextRef>,
    #[serde(default)]
    pub pinned_instructions: String,
    #[serde(default = "default_task_context_resolution_mode")]
    pub resolution_mode: String,
    #[serde(default)]
    pub last_resolved_at: Option<u64>,
}

impl Default for TaskContextBundle {
    fn default() -> Self {
        Self {
            refs: Vec::new(),
            pinned_instructions: String::new(),
            resolution_mode: default_task_context_resolution_mode(),
            last_resolved_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskStateTransitionSummary {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub at: u64,
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskAnalyticsSummary {
    pub run_count: u64,
    pub manual_run_count: u64,
    pub scheduled_run_count: u64,
    pub completion_count: u64,
    pub failure_count: u64,
    pub takeover_count: u64,
    pub approval_required_count: u64,
    pub average_run_duration_ms: u64,
    #[serde(default)]
    pub last_successful_run_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTaskRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub goal: String,
    pub brief: String,
    pub default_actor_ref: String,
    #[serde(default = "default_task_lifecycle_status")]
    pub status: String,
    #[serde(default)]
    pub schedule_spec: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<u64>,
    #[serde(default)]
    pub last_run_at: Option<u64>,
    #[serde(default)]
    pub active_task_run_id: Option<String>,
    #[serde(default)]
    pub latest_result_summary: Option<String>,
    #[serde(default)]
    pub latest_failure_category: Option<String>,
    #[serde(default)]
    pub latest_transition: Option<TaskStateTransitionSummary>,
    #[serde(default = "default_task_view_status")]
    pub view_status: String,
    #[serde(default)]
    pub attention_reasons: Vec<String>,
    #[serde(default)]
    pub attention_updated_at: Option<u64>,
    #[serde(default)]
    pub analytics_summary: TaskAnalyticsSummary,
    #[serde(default)]
    pub context_bundle: TaskContextBundle,
    #[serde(default)]
    pub latest_deliverable_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub latest_artifact_refs: Vec<ArtifactVersionReference>,
    pub created_by: String,
    #[serde(default)]
    pub updated_by: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTaskRunRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub task_id: String,
    #[serde(default = "default_task_trigger_type")]
    pub trigger_type: String,
    #[serde(default = "default_task_run_status")]
    pub status: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub runtime_run_id: Option<String>,
    pub actor_ref: String,
    pub started_at: u64,
    #[serde(default)]
    pub completed_at: Option<u64>,
    #[serde(default)]
    pub result_summary: Option<String>,
    #[serde(default)]
    pub pending_approval_id: Option<String>,
    #[serde(default)]
    pub failure_category: Option<String>,
    #[serde(default)]
    pub failure_summary: Option<String>,
    #[serde(default = "default_task_view_status")]
    pub view_status: String,
    #[serde(default)]
    pub attention_reasons: Vec<String>,
    #[serde(default)]
    pub attention_updated_at: Option<u64>,
    #[serde(default)]
    pub deliverable_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub artifact_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub latest_transition: Option<TaskStateTransitionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTaskInterventionRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub task_id: String,
    #[serde(default)]
    pub task_run_id: Option<String>,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub payload: JsonValue,
    pub created_by: String,
    pub created_at: u64,
    #[serde(default)]
    pub applied_to_session_id: Option<String>,
    #[serde(default = "default_task_intervention_status")]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTaskSchedulerClaimRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub task_id: String,
    #[serde(default)]
    pub claim_token: Option<String>,
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub claim_until: Option<u64>,
    #[serde(default)]
    pub last_dispatched_at: Option<u64>,
    #[serde(default)]
    pub last_evaluated_at: Option<u64>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunSummary {
    pub id: String,
    pub task_id: String,
    pub trigger_type: String,
    pub status: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub runtime_run_id: Option<String>,
    pub actor_ref: String,
    pub started_at: u64,
    #[serde(default)]
    pub completed_at: Option<u64>,
    #[serde(default)]
    pub result_summary: Option<String>,
    #[serde(default)]
    pub pending_approval_id: Option<String>,
    #[serde(default)]
    pub failure_category: Option<String>,
    #[serde(default)]
    pub failure_summary: Option<String>,
    pub view_status: String,
    #[serde(default)]
    pub attention_reasons: Vec<String>,
    #[serde(default)]
    pub attention_updated_at: Option<u64>,
    #[serde(default)]
    pub deliverable_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub artifact_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub latest_transition: Option<TaskStateTransitionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskInterventionRecord {
    pub id: String,
    pub task_id: String,
    #[serde(default)]
    pub task_run_id: Option<String>,
    pub r#type: String,
    #[serde(default)]
    pub payload: JsonValue,
    pub created_by: String,
    pub created_at: u64,
    #[serde(default)]
    pub applied_to_session_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub goal: String,
    pub default_actor_ref: String,
    pub status: String,
    #[serde(default)]
    pub schedule_spec: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<u64>,
    #[serde(default)]
    pub last_run_at: Option<u64>,
    #[serde(default)]
    pub latest_result_summary: Option<String>,
    #[serde(default)]
    pub latest_failure_category: Option<String>,
    #[serde(default)]
    pub latest_transition: Option<TaskStateTransitionSummary>,
    pub view_status: String,
    #[serde(default)]
    pub attention_reasons: Vec<String>,
    #[serde(default)]
    pub attention_updated_at: Option<u64>,
    #[serde(default)]
    pub active_task_run_id: Option<String>,
    #[serde(default)]
    pub analytics_summary: TaskAnalyticsSummary,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub goal: String,
    pub brief: String,
    pub default_actor_ref: String,
    pub status: String,
    #[serde(default)]
    pub schedule_spec: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<u64>,
    #[serde(default)]
    pub last_run_at: Option<u64>,
    #[serde(default)]
    pub latest_result_summary: Option<String>,
    #[serde(default)]
    pub latest_failure_category: Option<String>,
    #[serde(default)]
    pub latest_transition: Option<TaskStateTransitionSummary>,
    pub view_status: String,
    #[serde(default)]
    pub attention_reasons: Vec<String>,
    #[serde(default)]
    pub attention_updated_at: Option<u64>,
    #[serde(default)]
    pub active_task_run_id: Option<String>,
    #[serde(default)]
    pub analytics_summary: TaskAnalyticsSummary,
    #[serde(default)]
    pub context_bundle: TaskContextBundle,
    #[serde(default)]
    pub latest_deliverable_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub latest_artifact_refs: Vec<ArtifactVersionReference>,
    #[serde(default)]
    pub run_history: Vec<TaskRunSummary>,
    #[serde(default)]
    pub intervention_history: Vec<TaskInterventionRecord>,
    #[serde(default)]
    pub active_run: Option<TaskRunSummary>,
    pub created_by: String,
    #[serde(default)]
    pub updated_by: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
    pub goal: String,
    pub brief: String,
    pub default_actor_ref: String,
    #[serde(default)]
    pub schedule_spec: Option<String>,
    #[serde(default)]
    pub context_bundle: TaskContextBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub brief: Option<String>,
    #[serde(default)]
    pub default_actor_ref: Option<String>,
    #[serde(default)]
    pub schedule_spec: Option<String>,
    #[serde(default)]
    pub context_bundle: Option<TaskContextBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LaunchTaskRequest {
    #[serde(default)]
    pub actor_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RerunTaskRequest {
    #[serde(default)]
    pub actor_ref: Option<String>,
    #[serde(default)]
    pub source_task_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInterventionRequest {
    #[serde(default)]
    pub task_run_id: Option<String>,
    #[serde(default)]
    pub approval_id: Option<String>,
    pub r#type: String,
    #[serde(default)]
    pub payload: JsonValue,
}
