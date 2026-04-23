use serde::{Deserialize, Serialize};

use crate::task_records::{
    default_project_permission_allow, default_project_permission_inherit, TaskSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub client_app_id: String,
    pub username: String,
    pub password: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarUploadPayload {
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBootstrapAdminRequest {
    pub client_app_id: String,
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub confirm_password: String,
    pub avatar: AvatarUploadPayload,
    pub workspace_id: Option<String>,
    pub mapped_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session: SessionRecord,
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBootstrapAdminResponse {
    pub session: SessionRecord,
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppRecord {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub status: String,
    pub first_party: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub session_policy: String,
    pub default_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_path: Option<String>,
    pub avatar_content_type: Option<String>,
    pub avatar_byte_size: Option<u64>,
    pub avatar_content_hash: Option<String>,
    pub status: String,
    pub password_state: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub client_app_id: String,
    pub token: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub slug: String,
    pub deployment: String,
    pub bootstrap_status: String,
    pub owner_user_id: Option<String>,
    pub host: String,
    pub listen_address: String,
    pub default_project_id: String,
    pub mapped_directory: Option<String>,
    pub mapped_directory_default: Option<String>,
    pub project_default_permissions: ProjectDefaultPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectModelAssignments {
    pub configured_model_ids: Vec<String>,
    pub default_configured_model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectToolAssignments {
    #[serde(default)]
    pub source_keys: Vec<String>,
    #[serde(default)]
    pub excluded_source_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentAssignments {
    #[serde(default)]
    pub agent_ids: Vec<String>,
    #[serde(default)]
    pub team_ids: Vec<String>,
    #[serde(default)]
    pub excluded_agent_ids: Vec<String>,
    #[serde(default)]
    pub excluded_team_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWorkspaceAssignments {
    pub models: Option<ProjectModelAssignments>,
    pub tools: Option<ProjectToolAssignments>,
    pub agents: Option<ProjectAgentAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDefaultPermissions {
    #[serde(default = "default_project_permission_allow")]
    pub agents: String,
    #[serde(default = "default_project_permission_allow")]
    pub resources: String,
    #[serde(default = "default_project_permission_allow")]
    pub tools: String,
    #[serde(default = "default_project_permission_allow")]
    pub knowledge: String,
    #[serde(default = "default_project_permission_allow")]
    pub tasks: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPermissionOverrides {
    #[serde(default = "default_project_permission_inherit")]
    pub agents: String,
    #[serde(default = "default_project_permission_inherit")]
    pub resources: String,
    #[serde(default = "default_project_permission_inherit")]
    pub tools: String,
    #[serde(default = "default_project_permission_inherit")]
    pub knowledge: String,
    #[serde(default = "default_project_permission_inherit")]
    pub tasks: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLinkedWorkspaceAssets {
    pub agent_ids: Vec<String>,
    pub resource_ids: Vec<String>,
    pub tool_source_keys: Vec<String>,
    pub knowledge_ids: Vec<String>,
}

#[must_use]
pub fn empty_project_linked_workspace_assets() -> ProjectLinkedWorkspaceAssets {
    ProjectLinkedWorkspaceAssets {
        agent_ids: Vec::new(),
        resource_ids: Vec::new(),
        tool_source_keys: Vec::new(),
        knowledge_ids: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub status: String,
    pub description: String,
    pub resource_directory: String,
    pub leader_agent_id: Option<String>,
    pub manager_user_id: Option<String>,
    pub preset_code: Option<String>,
    pub owner_user_id: String,
    pub member_user_ids: Vec<String>,
    pub permission_overrides: ProjectPermissionOverrides,
    #[serde(
        default = "empty_project_linked_workspace_assets",
        skip_serializing,
        skip_deserializing
    )]
    pub linked_workspace_assets: ProjectLinkedWorkspaceAssets,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub assignments: Option<ProjectWorkspaceAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: String,
    pub resource_directory: String,
    pub owner_user_id: Option<String>,
    pub member_user_ids: Option<Vec<String>>,
    pub permission_overrides: Option<ProjectPermissionOverrides>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub linked_workspace_assets: Option<ProjectLinkedWorkspaceAssets>,
    pub leader_agent_id: Option<String>,
    pub manager_user_id: Option<String>,
    pub preset_code: Option<String>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub assignments: Option<ProjectWorkspaceAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: String,
    pub status: String,
    pub resource_directory: String,
    pub owner_user_id: Option<String>,
    pub member_user_ids: Option<Vec<String>>,
    pub permission_overrides: Option<ProjectPermissionOverrides>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub linked_workspace_assets: Option<ProjectLinkedWorkspaceAssets>,
    pub leader_agent_id: Option<String>,
    pub manager_user_id: Option<String>,
    pub preset_code: Option<String>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub assignments: Option<ProjectWorkspaceAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar: Option<AvatarUploadPayload>,
    #[serde(default)]
    pub remove_avatar: Option<bool>,
    #[serde(default)]
    pub mapped_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPromotionRequest {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub requested_by_user_id: String,
    pub submitted_by_owner_user_id: String,
    pub required_workspace_capability: String,
    pub status: String,
    pub reviewed_by_user_id: Option<String>,
    pub review_comment: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub reviewed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectPromotionRequestInput {
    pub asset_type: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProjectPromotionRequestInput {
    pub approved: bool,
    pub review_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMetricRecord {
    pub id: String,
    pub label: String,
    pub value: String,
    pub helper: Option<String>,
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivityRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub timestamp: u64,
    pub actor_id: Option<String>,
    pub actor_type: Option<String>,
    pub resource: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub updated_at: u64,
    pub last_message_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTokenUsageRecord {
    pub project_id: String,
    pub project_name: String,
    pub used_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOverviewSnapshot {
    pub workspace: WorkspaceSummary,
    pub metrics: Vec<WorkspaceMetricRecord>,
    pub projects: Vec<ProjectRecord>,
    pub project_token_usage: Vec<ProjectTokenUsageRecord>,
    pub recent_conversations: Vec<ConversationRecord>,
    pub recent_activity: Vec<WorkspaceActivityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardSnapshot {
    pub project: ProjectRecord,
    pub metrics: Vec<WorkspaceMetricRecord>,
    pub overview: ProjectDashboardSummary,
    pub trend: Vec<ProjectDashboardTrendPoint>,
    pub user_stats: Vec<ProjectDashboardUserStat>,
    pub conversation_insights: Vec<ProjectDashboardConversationInsight>,
    pub tool_ranking: Vec<ProjectDashboardRankingItem>,
    pub resource_breakdown: Vec<ProjectDashboardBreakdownItem>,
    pub model_breakdown: Vec<ProjectDashboardBreakdownItem>,
    pub recent_conversations: Vec<ConversationRecord>,
    pub recent_activity: Vec<WorkspaceActivityRecord>,
    #[serde(default)]
    pub recent_tasks: Vec<TaskSummary>,
    pub used_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardSummary {
    pub member_count: u64,
    pub active_user_count: u64,
    pub agent_count: u64,
    pub team_count: u64,
    pub conversation_count: u64,
    pub message_count: u64,
    pub tool_call_count: u64,
    pub approval_count: u64,
    pub resource_count: u64,
    pub knowledge_count: u64,
    pub tool_count: u64,
    pub token_record_count: u64,
    pub total_tokens: u64,
    pub activity_count: u64,
    pub task_count: u64,
    pub active_task_count: u64,
    pub attention_task_count: u64,
    pub scheduled_task_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardTrendPoint {
    pub id: String,
    pub label: String,
    pub timestamp: u64,
    pub conversation_count: u64,
    pub message_count: u64,
    pub tool_call_count: u64,
    pub approval_count: u64,
    pub token_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardUserStat {
    pub user_id: String,
    pub display_name: String,
    pub activity_count: u64,
    pub conversation_count: u64,
    pub message_count: u64,
    pub tool_call_count: u64,
    pub approval_count: u64,
    pub token_count: u64,
    pub activity_trend: Vec<u64>,
    pub token_trend: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardRankingItem {
    pub id: String,
    pub label: String,
    pub value: u64,
    pub helper: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardConversationInsight {
    pub id: String,
    pub conversation_id: String,
    pub title: String,
    pub status: String,
    pub updated_at: u64,
    pub last_message_preview: Option<String>,
    pub message_count: u64,
    pub tool_call_count: u64,
    pub approval_count: u64,
    pub token_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardBreakdownItem {
    pub id: String,
    pub label: String,
    pub value: u64,
    pub helper: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetProfile {
    pub id: String,
    pub species: String,
    pub display_name: String,
    pub owner_user_id: String,
    pub avatar_label: String,
    pub summary: String,
    pub greeting: String,
    pub mood: String,
    pub favorite_snack: String,
    pub prompt_hints: Vec<String>,
    pub fallback_asset: String,
    pub rive_asset: Option<String>,
    pub state_machine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetMessage {
    pub id: String,
    pub pet_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPosition {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPresenceState {
    pub pet_id: String,
    pub is_visible: bool,
    pub chat_open: bool,
    pub motion_state: String,
    pub unread_count: u64,
    pub last_interaction_at: u64,
    pub position: PetPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavePetPresenceInput {
    pub pet_id: String,
    pub is_visible: Option<bool>,
    pub chat_open: Option<bool>,
    pub motion_state: Option<String>,
    pub unread_count: Option<u64>,
    pub last_interaction_at: Option<u64>,
    pub position: Option<PetPosition>,
}

fn default_pet_context_scope() -> String {
    "home".into()
}

fn default_pet_owner_user_id() -> String {
    "user-owner".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetConversationBinding {
    pub pet_id: String,
    pub workspace_id: String,
    #[serde(default = "default_pet_owner_user_id")]
    pub owner_user_id: String,
    #[serde(default = "default_pet_context_scope")]
    pub context_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BindPetConversationInput {
    pub pet_id: String,
    pub conversation_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetWorkspaceSnapshot {
    pub workspace_id: String,
    pub owner_user_id: String,
    pub context_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub profile: PetProfile,
    pub presence: PetPresenceState,
    pub binding: Option<PetConversationBinding>,
    pub messages: Vec<PetMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetDashboardSummary {
    pub pet_id: String,
    pub workspace_id: String,
    pub owner_user_id: String,
    pub species: String,
    pub mood: String,
    pub active_conversation_count: u64,
    pub knowledge_count: u64,
    pub memory_count: u64,
    pub reminder_count: u64,
    pub resource_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_interaction_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub location: Option<String>,
    pub origin: String,
    pub scope: String,
    pub visibility: String,
    pub owner_user_id: String,
    pub storage_path: Option<String>,
    pub content_type: Option<String>,
    pub byte_size: Option<u64>,
    pub preview_kind: String,
    pub status: String,
    pub updated_at: u64,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResourceInput {
    pub project_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceResourceInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceFolderUploadEntry {
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResourceFolderInput {
    pub project_id: Option<String>,
    pub files: Vec<WorkspaceResourceFolderUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceImportInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir_name: Option<String>,
    pub scope: String,
    pub visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub files: Vec<WorkspaceResourceFolderUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceContentDocument {
    pub resource_id: String,
    pub preview_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceChildrenRecord {
    pub name: String,
    pub relative_path: String,
    pub kind: String,
    pub preview_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoteWorkspaceResourceInput {
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDirectoryBrowserEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDirectoryBrowserResponse {
    pub current_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    pub entries: Vec<WorkspaceDirectoryBrowserEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub kind: String,
    pub scope: String,
    pub status: String,
    pub visibility: String,
    pub owner_user_id: Option<String>,
    pub source_type: String,
    pub source_ref: String,
    pub updated_at: u64,
}
