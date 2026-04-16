mod asset_bundle;
mod asset_records;
mod runtime_policy;

use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use asset_bundle::*;
pub use asset_records::*;
pub use runtime_policy::*;

pub const DEFAULT_WORKSPACE_ID: &str = "ws-local";
pub const DEFAULT_PROJECT_ID: &str = "proj-redesign";
pub const RUNTIME_PERMISSION_READ_ONLY: &str = "read-only";
pub const RUNTIME_PERMISSION_WORKSPACE_WRITE: &str = "workspace-write";
pub const RUNTIME_PERMISSION_DANGER_FULL_ACCESS: &str = "danger-full-access";

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

impl AppError {
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

#[must_use]
pub fn normalize_runtime_permission_mode_label(value: &str) -> Option<&'static str> {
    match value.trim() {
        "readonly" | "read-only" => Some(RUNTIME_PERMISSION_READ_ONLY),
        "auto" | "ask" | "workspace-write" => Some(RUNTIME_PERMISSION_WORKSPACE_WRITE),
        "danger-full-access" => Some(RUNTIME_PERMISSION_DANGER_FULL_ACCESS),
        _ => None,
    }
}

#[must_use]
pub fn create_default_notification_unread_summary() -> NotificationUnreadSummary {
    NotificationUnreadSummary {
        total: 0,
        by_scope: NotificationUnreadScopeSummary {
            app: 0,
            workspace: 0,
            user: 0,
        },
    }
}

#[must_use]
pub fn normalize_notification_filter_scope(scope: Option<&str>) -> Option<&str> {
    match scope.map(str::trim) {
        Some("app") => Some("app"),
        Some("workspace") => Some("workspace"),
        Some("user") => Some("user"),
        _ => None,
    }
}

#[must_use]
pub fn notification_list_response_from_records(
    notifications: Vec<NotificationRecord>,
) -> NotificationListResponse {
    let mut unread = create_default_notification_unread_summary();

    for notification in &notifications {
        if notification.read_at.is_some() {
            continue;
        }

        unread.total += 1;
        match notification.scope_kind.as_str() {
            "workspace" => unread.by_scope.workspace += 1,
            "user" => unread.by_scope.user += 1,
            _ => unread.by_scope.app += 1,
        }
    }

    NotificationListResponse {
        notifications,
        unread,
    }
}

pub trait PreferencesPort: Send + Sync {
    fn load_preferences(&self) -> Result<ShellPreferences, AppError>;
    fn save_preferences(
        &self,
        preferences: &ShellPreferences,
    ) -> Result<ShellPreferences, AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostState {
    pub platform: String,
    pub mode: String,
    pub app_version: String,
    pub cargo_workspace: bool,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackendConnection {
    pub base_url: Option<String>,
    pub auth_token: Option<String>,
    pub state: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckBackendStatus {
    pub state: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckStatus {
    pub status: String,
    pub host: String,
    pub mode: String,
    pub cargo_workspace: bool,
    pub backend: HealthcheckBackendStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellPreferences {
    pub theme: String,
    pub locale: String,
    pub font_size: u32,
    pub font_family: String,
    pub font_style: String,
    pub compact_sidebar: bool,
    pub left_sidebar_collapsed: bool,
    pub right_sidebar_collapsed: bool,
    pub update_channel: String,
    pub default_workspace_id: String,
    pub last_visited_route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostReleaseSummary {
    pub version: String,
    pub channel: String,
    pub published_at: String,
    pub notes: Option<String>,
    pub notes_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostUpdateCapabilities {
    pub can_check: bool,
    pub can_download: bool,
    pub can_install: bool,
    pub supports_channels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostUpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostUpdateStatus {
    pub current_version: String,
    pub current_channel: String,
    pub state: String,
    pub latest_release: Option<HostReleaseSummary>,
    pub last_checked_at: Option<u64>,
    pub progress: Option<HostUpdateProgress>,
    pub capabilities: HostUpdateCapabilities,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRecord {
    pub id: String,
    pub scope_kind: String,
    pub scope_owner_id: Option<String>,
    pub level: String,
    pub title: String,
    pub body: String,
    pub source: String,
    pub created_at: u64,
    pub read_at: Option<u64>,
    pub toast_visible_until: Option<u64>,
    pub route_to: Option<String>,
    pub action_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationInput {
    pub scope_kind: String,
    pub scope_owner_id: Option<String>,
    pub level: String,
    pub title: String,
    pub body: String,
    pub source: String,
    pub toast_duration_ms: Option<u64>,
    pub route_to: Option<String>,
    pub action_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationFilter {
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationUnreadScopeSummary {
    pub app: u64,
    pub workspace: u64,
    pub user: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationUnreadSummary {
    pub total: u64,
    pub by_scope: NotificationUnreadScopeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationRecord>,
    pub unread: NotificationUnreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellBootstrap {
    pub host_state: HostState,
    pub preferences: ShellPreferences,
    pub connections: Vec<ConnectionProfile>,
    pub backend: Option<DesktopBackendConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub mode: String,
    pub label: String,
    pub workspace_id: String,
    pub base_url: Option<String>,
    pub state: String,
    pub last_sync_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostWorkspaceConnectionRecord {
    pub workspace_connection_id: String,
    pub workspace_id: String,
    pub label: String,
    pub base_url: String,
    pub transport_security: String,
    pub auth_mode: String,
    pub last_used_at: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostWorkspaceConnectionInput {
    pub workspace_id: String,
    pub label: String,
    pub base_url: String,
    pub transport_security: String,
    pub auth_mode: String,
}

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
    pub slug: String,
    pub deployment: String,
    pub bootstrap_status: String,
    pub owner_user_id: Option<String>,
    pub host: String,
    pub listen_address: String,
    pub default_project_id: String,
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
    pub source_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentAssignments {
    pub agent_ids: Vec<String>,
    pub team_ids: Vec<String>,
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
    pub agents: String,
    pub resources: String,
    pub tools: String,
    pub knowledge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPermissionOverrides {
    pub agents: String,
    pub resources: String,
    pub tools: String,
    pub knowledge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLinkedWorkspaceAssets {
    pub agent_ids: Vec<String>,
    pub resource_ids: Vec<String>,
    pub tool_source_keys: Vec<String>,
    pub knowledge_ids: Vec<String>,
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
    pub owner_user_id: String,
    pub member_user_ids: Vec<String>,
    pub permission_overrides: ProjectPermissionOverrides,
    pub linked_workspace_assets: ProjectLinkedWorkspaceAssets,
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
    pub linked_workspace_assets: Option<ProjectLinkedWorkspaceAssets>,
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
    pub linked_workspace_assets: Option<ProjectLinkedWorkspaceAssets>,
    pub assignments: Option<ProjectWorkspaceAssignments>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogRecord {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub provider: String,
    pub description: String,
    pub recommended_for: String,
    pub availability: String,
    pub default_permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialRecord {
    pub id: String,
    pub workspace_id: String,
    pub provider: String,
    pub name: String,
    pub base_url: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDescriptor {
    pub surface: String,
    pub protocol_family: String,
    pub transport: Vec<String>,
    pub auth_strategy: String,
    pub base_url: String,
    pub base_url_policy: String,
    pub enabled: bool,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRegistryRecord {
    pub provider_id: String,
    pub label: String,
    pub enabled: bool,
    pub surfaces: Vec<SurfaceDescriptor>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSurfaceBinding {
    pub surface: String,
    pub protocol_family: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRegistryRecord {
    pub model_id: String,
    pub provider_id: String,
    pub label: String,
    pub description: String,
    pub family: String,
    pub track: String,
    pub enabled: bool,
    pub recommended_for: String,
    pub availability: String,
    pub default_permission: String,
    pub surface_bindings: Vec<ModelSurfaceBinding>,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialBinding {
    pub credential_ref: String,
    pub provider_id: String,
    pub label: String,
    pub base_url: Option<String>,
    pub status: String,
    pub configured: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSelection {
    pub configured_model_id: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelTokenQuota {
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelTokenUsage {
    pub used_tokens: u64,
    pub remaining_tokens: Option<u64>,
    pub exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelRecord {
    pub configured_model_id: String,
    pub name: String,
    pub provider_id: String,
    pub model_id: String,
    pub credential_ref: Option<String>,
    pub base_url: Option<String>,
    pub token_quota: Option<ConfiguredModelTokenQuota>,
    pub token_usage: ConfiguredModelTokenUsage,
    pub enabled: bool,
    pub source: String,
    pub status: String,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRegistryDiagnostics {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogSnapshot {
    pub providers: Vec<ProviderRegistryRecord>,
    pub models: Vec<ModelRegistryRecord>,
    pub configured_models: Vec<ConfiguredModelRecord>,
    pub credential_bindings: Vec<CredentialBinding>,
    pub default_selections: BTreeMap<String, DefaultSelection>,
    pub diagnostics: ModelRegistryDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAssetManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackageManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
    pub package_kind: String,
    pub active: bool,
    pub shadowed_by: Option<String>,
    pub source_origin: String,
    pub workspace_owned: bool,
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerPackageManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
    pub package_kind: String,
    pub server_name: String,
    pub endpoint: String,
    pub tool_names: Vec<String>,
    pub prompt_names: Vec<String>,
    pub resource_uris: Vec<String>,
    pub scope: String,
    pub status_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManagementEntry {
    pub id: String,
    pub asset_id: String,
    pub capability_id: String,
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
    pub source_kind: String,
    pub execution_kind: String,
    pub description: String,
    pub required_permission: Option<String>,
    pub availability: String,
    pub source_key: String,
    pub display_path: String,
    pub disabled: bool,
    pub management: WorkspaceToolManagementCapabilities,
    pub builtin_key: Option<String>,
    pub active: Option<bool>,
    pub shadowed_by: Option<String>,
    pub source_origin: Option<String>,
    pub workspace_owned: Option<bool>,
    pub relative_path: Option<String>,
    pub server_name: Option<String>,
    pub endpoint: Option<String>,
    pub tool_names: Option<Vec<String>>,
    pub resource_uri: Option<String>,
    pub status_detail: Option<String>,
    pub scope: Option<String>,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManagementProjection {
    pub entries: Vec<CapabilityManagementEntry>,
    pub assets: Vec<CapabilityAssetManifest>,
    pub skill_packages: Vec<SkillPackageManifest>,
    pub mcp_server_packages: Vec<McpServerPackageManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolManagementCapabilities {
    pub can_disable: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolConsumerSummary {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub scope: String,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolCatalogEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_kind: Option<String>,
    pub description: String,
    pub required_permission: Option<String>,
    pub availability: String,
    pub source_key: String,
    pub display_path: String,
    pub disabled: bool,
    pub management: WorkspaceToolManagementCapabilities,
    pub builtin_key: Option<String>,
    pub active: Option<bool>,
    pub shadowed_by: Option<String>,
    pub source_origin: Option<String>,
    pub workspace_owned: Option<bool>,
    pub relative_path: Option<String>,
    pub server_name: Option<String>,
    pub endpoint: Option<String>,
    pub tool_names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,
    pub status_detail: Option<String>,
    pub scope: Option<String>,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAssetDisablePatch {
    pub source_key: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceSkillInput {
    pub slug: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceSkillInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillTreeNode {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub children: Option<Vec<WorkspaceSkillTreeNode>>,
    pub byte_size: Option<u64>,
    pub is_text: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillDocument {
    pub id: String,
    pub source_key: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub display_path: String,
    pub root_path: String,
    pub tree: Vec<WorkspaceSkillTreeNode>,
    pub source_origin: String,
    pub workspace_owned: bool,
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillTreeDocument {
    pub skill_id: String,
    pub source_key: String,
    pub display_path: String,
    pub root_path: String,
    pub tree: Vec<WorkspaceSkillTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillFileDocument {
    pub skill_id: String,
    pub source_key: String,
    pub path: String,
    pub display_path: String,
    pub byte_size: u64,
    pub is_text: bool,
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceSkillFileInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileUploadPayload {
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDirectoryUploadEntry {
    pub relative_path: String,
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceSkillArchiveInput {
    pub slug: String,
    pub archive: WorkspaceFileUploadPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceSkillFolderInput {
    pub slug: String,
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopyWorkspaceSkillToManagedInput {
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWorkspaceMcpServerInput {
    pub server_name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMcpServerDocument {
    pub server_name: String,
    pub source_key: String,
    pub display_path: String,
    pub scope: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRecord {
    pub id: String,
    pub workspace_id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_mode: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub description: String,
    pub cadence: String,
    pub owner_type: String,
    pub owner_id: String,
    pub status: String,
    pub next_run_at: Option<u64>,
    pub last_run_at: Option<u64>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRecordSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub status: String,
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentUserProfileRequest {
    pub username: String,
    pub display_name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentUserPasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentUserPasswordResponse {
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserRecord {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserUpsertRequest {
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub password: Option<String>,
    pub confirm_password: Option<String>,
    pub reset_password: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnitRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnitUpsertRequest {
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionUpsertRequest {
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupUpsertRequest {
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgAssignmentRecord {
    pub user_id: String,
    pub org_unit_id: String,
    pub is_primary: bool,
    pub position_ids: Vec<String>,
    pub user_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgAssignmentUpsertRequest {
    pub user_id: String,
    pub org_unit_id: String,
    pub is_primary: bool,
    pub position_ids: Vec<String>,
    pub user_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDefinition {
    pub code: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub resource_type: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRoleRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_codes: Vec<String>,
    pub source: String,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleUpsertRequest {
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleBindingRecord {
    pub id: String,
    pub role_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleBindingUpsertRequest {
    pub role_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceSummary {
    pub experience_level: String,
    pub member_count: u32,
    pub has_org_structure: bool,
    pub has_custom_roles: bool,
    pub has_advanced_policies: bool,
    pub has_menu_governance: bool,
    pub has_resource_governance: bool,
    pub recommended_landing_section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceSnapshot {
    pub experience_level: String,
    pub member_count: u32,
    pub has_org_structure: bool,
    pub has_custom_roles: bool,
    pub has_advanced_policies: bool,
    pub has_menu_governance: bool,
    pub has_resource_governance: bool,
    pub counts: AccessExperienceCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessSectionGrant {
    pub section: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRoleTemplate {
    pub code: String,
    pub name: String,
    pub description: String,
    pub managed_role_codes: Vec<String>,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRolePreset {
    pub code: String,
    pub name: String,
    pub description: String,
    pub recommended_for: String,
    pub template_codes: Vec<String>,
    pub capability_bundle_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessCapabilityBundle {
    pub code: String,
    pub name: String,
    pub description: String,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceCounts {
    pub custom_role_count: u32,
    pub org_unit_count: u32,
    pub data_policy_count: u32,
    pub resource_policy_count: u32,
    pub menu_policy_count: u32,
    pub protected_resource_count: u32,
    pub session_count: u32,
    pub audit_event_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceResponse {
    pub summary: AccessExperienceSummary,
    pub section_grants: Vec<AccessSectionGrant>,
    pub role_templates: Vec<AccessRoleTemplate>,
    pub role_presets: Vec<AccessRolePreset>,
    pub capability_bundles: Vec<AccessCapabilityBundle>,
    pub counts: AccessExperienceCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessMemberSummary {
    pub user: AccessUserRecord,
    pub primary_preset_code: Option<String>,
    pub primary_preset_name: String,
    pub effective_role_names: Vec<String>,
    pub has_org_assignments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserPresetUpdateRequest {
    pub preset_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicyRecord {
    pub id: String,
    pub name: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub scope_type: String,
    pub project_ids: Vec<String>,
    pub tags: Vec<String>,
    pub classifications: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicyUpsertRequest {
    pub name: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub scope_type: String,
    pub project_ids: Vec<String>,
    pub tags: Vec<String>,
    pub classifications: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicyRecord {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicyUpsertRequest {
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuDefinition {
    pub id: String,
    pub parent_id: Option<String>,
    pub label: String,
    pub route_name: Option<String>,
    pub source: String,
    pub status: String,
    pub order: i64,
    pub feature_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuPolicyRecord {
    pub menu_id: String,
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuPolicyUpsertRequest {
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateMenuPolicyRequest {
    pub menu_id: String,
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub id: String,
    pub code: String,
    pub label: String,
    pub required_permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuGateResult {
    pub menu_id: String,
    pub feature_code: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceActionGrant {
    pub resource_type: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessSessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub client_app_id: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedResourceDescriptor {
    pub id: String,
    pub resource_type: String,
    pub resource_subtype: Option<String>,
    pub name: String,
    pub project_id: Option<String>,
    pub tags: Vec<String>,
    pub classification: String,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedResourceMetadataUpsertRequest {
    pub resource_subtype: Option<String>,
    pub project_id: Option<String>,
    pub tags: Vec<String>,
    pub classification: String,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationSnapshot {
    pub principal: AccessUserRecord,
    pub effective_role_ids: Vec<String>,
    pub effective_roles: Vec<AccessRoleRecord>,
    pub effective_permission_codes: Vec<String>,
    pub org_assignments: Vec<UserOrgAssignmentRecord>,
    pub feature_codes: Vec<String>,
    pub visible_menu_ids: Vec<String>,
    pub menu_gates: Vec<MenuGateResult>,
    pub resource_action_grants: Vec<ResourceActionGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub subject_id: String,
    pub capability: String,
    pub project_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_subtype: Option<String>,
    pub tags: Vec<String>,
    pub classification: Option<String>,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditQuery {
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub outcome: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditListResponse {
    pub items: Vec<AuditRecord>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub session_id: String,
    pub status: String,
    pub current_step: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub model_id: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSource {
    pub scope: String,
    pub owner_id: Option<String>,
    pub display_path: String,
    pub source_key: String,
    pub exists: bool,
    pub loaded: bool,
    pub content_hash: Option<String>,
    pub document: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSecretReferenceStatus {
    pub scope: String,
    pub path: String,
    pub reference: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfiguredModelProbeInput {
    pub scope: String,
    pub configured_model_id: String,
    pub patch: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfiguredModelProbeResult {
    pub valid: bool,
    pub reachable: bool,
    pub configured_model_id: String,
    pub configured_model_name: Option<String>,
    pub request_id: Option<String>,
    pub consumed_tokens: Option<u32>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEffectiveConfig {
    pub effective_config: serde_json::Value,
    pub effective_config_hash: String,
    pub sources: Vec<RuntimeConfigSource>,
    pub validation: RuntimeConfigValidationResult,
    pub secret_references: Vec<RuntimeSecretReferenceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigPatch {
    pub scope: String,
    pub patch: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSnapshotSummary {
    pub id: String,
    pub effective_config_hash: String,
    pub started_from_scope_set: Vec<String>,
    pub source_refs: Vec<String>,
    pub created_at: u64,
    pub effective_config: Option<serde_json::Value>,
}

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
    pub activated_tools: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilitySurface {
    #[serde(default)]
    pub visible_tools: Vec<String>,
    #[serde(default)]
    pub deferred_tools: Vec<String>,
    #[serde(default)]
    pub discoverable_skills: Vec<String>,
    #[serde(default)]
    pub available_resources: Vec<String>,
    #[serde(default)]
    pub hidden_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityStateSnapshot {
    #[serde(default)]
    pub activated_tools: Vec<String>,
    #[serde(default)]
    pub granted_tools: Vec<String>,
    #[serde(default)]
    pub pending_tools: Vec<String>,
    #[serde(default)]
    pub approved_tools: Vec<String>,
    #[serde(default)]
    pub auth_resolved_tools: Vec<String>,
    #[serde(default)]
    pub hidden_tools: Vec<String>,
    #[serde(default)]
    pub injected_skill_message_count: u64,
    #[serde(default)]
    pub granted_tool_count: u64,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub effort_override: Option<String>,
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
    pub capability_state_ref: Option<String>,
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
    pub capability_state_ref: Option<String>,
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
    pub capability_state_ref: Option<String>,
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
    pub capability_state_ref: Option<String>,
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
    pub capability_state_ref: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub provider_id: String,
    pub credential_ref: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub default_surface: Option<String>,
    pub protocol_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedExecutionTarget {
    pub configured_model_id: String,
    pub configured_model_name: String,
    pub provider_id: String,
    pub registry_model_id: String,
    pub model_id: String,
    pub surface: String,
    pub protocol_family: String,
    pub credential_ref: Option<String>,
    pub base_url: Option<String>,
    pub max_output_tokens: Option<u32>,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBootstrap {
    pub provider: ProviderConfig,
    pub sessions: Vec<RuntimeSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeSessionInput {
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub title: String,
    pub session_kind: Option<String>,
    pub selected_actor_ref: String,
    pub selected_configured_model_id: Option<String>,
    pub execution_permission_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRuntimeTurnInput {
    pub content: String,
    pub permission_mode: Option<String>,
    #[serde(default)]
    pub recall_mode: Option<String>,
    #[serde(default)]
    pub ignored_memory_ids: Vec<String>,
    #[serde(default)]
    pub memory_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeApprovalInput {
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeAuthChallengeInput {
    pub resolution: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelRuntimeSubrunInput {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeMemoryProposalInput {
    pub decision: String,
    #[serde(default)]
    pub note: Option<String>,
}

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

#[must_use]
pub fn timestamp_now() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

#[must_use]
pub fn default_host_state(app_version: String, cargo_workspace: bool) -> HostState {
    HostState {
        platform: "tauri".into(),
        mode: "local".into(),
        app_version,
        cargo_workspace,
        shell: "tauri2".into(),
    }
}

#[must_use]
pub fn default_preferences(
    default_workspace_id: impl Into<String>,
    default_project_id: impl Into<String>,
) -> ShellPreferences {
    let workspace_id = default_workspace_id.into();
    let project_id = default_project_id.into();

    ShellPreferences {
        theme: "system".into(),
        locale: "zh-CN".into(),
        font_size: 14,
        font_family: "Inter, sans-serif".into(),
        font_style: "sans".into(),
        compact_sidebar: false,
        left_sidebar_collapsed: false,
        right_sidebar_collapsed: false,
        update_channel: "formal".into(),
        default_workspace_id: workspace_id.clone(),
        last_visited_route: format!("/workspaces/{workspace_id}/overview?project={project_id}"),
    }
}

#[must_use]
pub fn default_host_update_capabilities() -> HostUpdateCapabilities {
    HostUpdateCapabilities {
        can_check: true,
        can_download: false,
        can_install: false,
        supports_channels: true,
    }
}

#[must_use]
pub fn default_host_update_status(
    current_version: impl Into<String>,
    current_channel: impl Into<String>,
) -> HostUpdateStatus {
    HostUpdateStatus {
        current_version: current_version.into(),
        current_channel: current_channel.into(),
        state: "idle".into(),
        latest_release: None,
        last_checked_at: None,
        progress: None,
        capabilities: default_host_update_capabilities(),
        error_code: None,
        error_message: None,
    }
}

#[must_use]
pub fn default_connection_stubs() -> Vec<ConnectionProfile> {
    vec![ConnectionProfile {
        id: "conn-local".into(),
        mode: "local".into(),
        label: "Local Runtime".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        base_url: None,
        state: "local-ready".into(),
        last_sync_at: None,
    }]
}

#[must_use]
pub fn normalize_connection_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

#[must_use]
pub fn connection_mode_from_transport_security(transport_security: &str) -> String {
    match transport_security {
        "loopback" => "local".into(),
        "public" => "remote".into(),
        _ => "shared".into(),
    }
}

#[must_use]
pub fn transport_security_from_connection_mode(mode: &str) -> String {
    match mode {
        "local" => "loopback".into(),
        "remote" => "public".into(),
        _ => "trusted".into(),
    }
}

#[must_use]
pub fn workspace_connection_status_from_connection_profile(
    connection: &ConnectionProfile,
    backend: Option<&DesktopBackendConnection>,
) -> String {
    if connection.mode == "local" {
        return match backend.map(|item| item.state.as_str()) {
            Some("ready") => "connected".into(),
            Some("unavailable") => "unreachable".into(),
            _ => "disconnected".into(),
        };
    }

    match connection.state.as_str() {
        "connected" | "local-ready" => "connected".into(),
        "expired" => "expired".into(),
        "unreachable" => "unreachable".into(),
        _ => "disconnected".into(),
    }
}

#[must_use]
pub fn workspace_connection_base_url_from_profile(
    connection: &ConnectionProfile,
    backend: Option<&DesktopBackendConnection>,
) -> String {
    if let Some(base_url) = connection.base_url.as_ref() {
        return normalize_connection_base_url(base_url);
    }

    if connection.mode == "local" {
        if let Some(base_url) = backend.and_then(|item| item.base_url.as_ref()) {
            return normalize_connection_base_url(base_url);
        }
    }

    "http://127.0.0.1".into()
}

#[must_use]
pub fn host_workspace_connection_record_from_profile(
    connection: &ConnectionProfile,
    backend: Option<&DesktopBackendConnection>,
) -> HostWorkspaceConnectionRecord {
    HostWorkspaceConnectionRecord {
        workspace_connection_id: connection.id.clone(),
        workspace_id: connection.workspace_id.clone(),
        label: connection.label.clone(),
        base_url: workspace_connection_base_url_from_profile(connection, backend),
        transport_security: transport_security_from_connection_mode(&connection.mode),
        auth_mode: "session-token".into(),
        last_used_at: connection.last_sync_at,
        status: workspace_connection_status_from_connection_profile(connection, backend),
    }
}

#[must_use]
pub fn connection_profile_from_host_workspace_connection(
    connection: &HostWorkspaceConnectionRecord,
) -> ConnectionProfile {
    ConnectionProfile {
        id: connection.workspace_connection_id.clone(),
        mode: connection_mode_from_transport_security(&connection.transport_security),
        label: connection.label.clone(),
        workspace_id: connection.workspace_id.clone(),
        base_url: Some(normalize_connection_base_url(&connection.base_url)),
        state: match connection.status.as_str() {
            "connected" => "connected".into(),
            "expired" => "expired".into(),
            "unreachable" => "disconnected".into(),
            other => other.into(),
        },
        last_sync_at: connection.last_used_at,
    }
}
