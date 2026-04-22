use serde::{Deserialize, Serialize};

use crate::{
    default_project_permission_allow, default_project_permission_inherit, AvatarUploadPayload,
    TaskSummary,
};

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
