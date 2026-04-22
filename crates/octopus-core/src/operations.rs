use serde::{Deserialize, Serialize};

use crate::{ClientAppRecord, WorkspaceSummary};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersionReference {
    pub artifact_id: String,
    pub version: u32,
    pub title: String,
    pub preview_kind: String,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub conversation_id: String,
    pub title: String,
    pub status: String,
    pub preview_kind: String,
    pub latest_version: u32,
    pub latest_version_ref: ArtifactVersionReference,
    pub promotion_state: String,
    pub updated_at: u64,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableDetail {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub conversation_id: String,
    pub session_id: String,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_artifact_id: Option<String>,
    pub title: String,
    pub status: String,
    pub preview_kind: String,
    pub latest_version: u32,
    pub latest_version_ref: ArtifactVersionReference,
    pub promotion_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promotion_knowledge_id: Option<String>,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableVersionSummary {
    pub artifact_id: String,
    pub version: u32,
    pub title: String,
    pub preview_kind: String,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableVersionContent {
    pub artifact_id: String,
    pub version: u32,
    pub preview_kind: String,
    pub editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeliverableVersionInput {
    #[serde(default)]
    pub title: Option<String>,
    pub preview_kind: String,
    #[serde(default)]
    pub text_content: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub source_message_id: Option<String>,
    #[serde(default)]
    pub parent_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoteDeliverableInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForkDeliverableInput {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntryRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub scope: String,
    pub status: String,
    pub source_type: String,
    pub source_ref: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub target_user_id: String,
    pub item_type: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub actionable: bool,
    pub route_to: Option<String>,
    pub action_label: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeletionRequest {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub requested_by_user_id: String,
    pub status: String,
    pub reason: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub review_comment: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub reviewed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectDeletionRequestInput {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProjectDeletionRequestInput {
    #[serde(default)]
    pub review_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceEventRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub event_kind: String,
    pub title: String,
    pub detail: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub actor_type: String,
    pub actor_id: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CostLedgerEntry {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub configured_model_id: Option<String>,
    pub metric: String,
    pub amount: i64,
    pub unit: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTokenUsageProjection {
    pub project_id: String,
    pub used_tokens: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemBootstrapStatus {
    pub workspace: WorkspaceSummary,
    pub setup_required: bool,
    pub owner_ready: bool,
    pub registered_apps: Vec<ClientAppRecord>,
    pub protocol_version: String,
    pub api_base_path: String,
    pub transport_security: String,
    pub auth_mode: String,
    pub capabilities: WorkspaceCapabilitySet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub matched_role_binding_ids: Vec<String>,
    pub matched_policy_ids: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCapabilitySet {
    pub polling: bool,
    pub sse: bool,
    pub idempotency: bool,
    pub reconnect: bool,
    pub event_replay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub request_id: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorDetail,
}
