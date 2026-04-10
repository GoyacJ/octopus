use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
pub struct RegisterWorkspaceOwnerRequest {
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
pub struct RegisterWorkspaceOwnerResponse {
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
pub struct WorkspaceMembershipRecord {
    pub workspace_id: String,
    pub user_id: String,
    pub role_ids: Vec<String>,
    pub scope_mode: String,
    pub scope_project_ids: Vec<String>,
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
    pub role_ids: Vec<String>,
    pub scope_project_ids: Vec<String>,
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
pub struct ProjectRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub status: String,
    pub description: String,
    pub assignments: Option<ProjectWorkspaceAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: String,
    pub assignments: Option<ProjectWorkspaceAssignments>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: String,
    pub status: String,
    pub assignments: Option<ProjectWorkspaceAssignments>,
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
pub struct WorkspaceOverviewSnapshot {
    pub workspace: WorkspaceSummary,
    pub metrics: Vec<WorkspaceMetricRecord>,
    pub projects: Vec<ProjectRecord>,
    pub recent_conversations: Vec<ConversationRecord>,
    pub recent_activity: Vec<WorkspaceActivityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDashboardSnapshot {
    pub project: ProjectRecord,
    pub metrics: Vec<WorkspaceMetricRecord>,
    pub recent_conversations: Vec<ConversationRecord>,
    pub recent_activity: Vec<WorkspaceActivityRecord>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetConversationBinding {
    pub pet_id: String,
    pub workspace_id: String,
    pub project_id: String,
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
    pub profile: PetProfile,
    pub presence: PetPresenceState,
    pub binding: Option<PetConversationBinding>,
    pub messages: Vec<PetMessage>,
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
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResourceFileUploadEntry {
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
    pub files: Vec<WorkspaceResourceFileUploadEntry>,
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
    pub status: String,
    pub source_type: String,
    pub source_ref: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar: Option<String>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub integration_source: Option<WorkspaceLinkIntegrationSource>,
    pub description: String,
    pub status: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar: Option<String>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub leader_agent_id: Option<String>,
    pub member_agent_ids: Vec<String>,
    pub integration_source: Option<WorkspaceLinkIntegrationSource>,
    pub description: String,
    pub status: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLinkIntegrationSource {
    pub kind: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertAgentInput {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertTeamInput {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub leader_agent_id: Option<String>,
    pub member_agent_ids: Vec<String>,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentLinkRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub linked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTeamLinkRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub team_id: String,
    pub linked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentLinkInput {
    pub project_id: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTeamLinkInput {
    pub project_id: String,
    pub team_id: String,
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
pub struct WorkspaceToolCatalogSnapshot {
    pub entries: Vec<WorkspaceToolCatalogEntry>,
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
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
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
    pub status_detail: Option<String>,
    pub scope: Option<String>,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolDisablePatch {
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
pub struct ImportWorkspaceAgentBundlePreviewInput {
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundleInput {
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssue {
    pub severity: String,
    pub scope: String,
    pub source_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAgentPreviewItem {
    pub source_id: String,
    pub agent_id: Option<String>,
    pub name: String,
    pub department: String,
    pub action: String,
    pub skill_slugs: Vec<String>,
    pub mcp_server_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedTeamPreviewItem {
    pub source_id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub action: String,
    pub leader_name: Option<String>,
    pub member_names: Vec<String>,
    pub agent_source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedSkillPreviewItem {
    pub slug: String,
    pub skill_id: String,
    pub name: String,
    pub action: String,
    pub content_hash: String,
    pub file_count: u64,
    pub source_ids: Vec<String>,
    pub departments: Vec<String>,
    pub agent_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMcpPreviewItem {
    pub server_name: String,
    pub action: String,
    pub content_hash: Option<String>,
    pub source_ids: Vec<String>,
    pub consumer_names: Vec<String>,
    pub referenced_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAvatarPreviewItem {
    pub source_id: String,
    pub owner_kind: String,
    pub owner_name: String,
    pub file_name: String,
    pub generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundlePreview {
    pub departments: Vec<String>,
    pub department_count: u64,
    pub detected_agent_count: u64,
    pub importable_agent_count: u64,
    pub detected_team_count: u64,
    pub importable_team_count: u64,
    pub create_count: u64,
    pub update_count: u64,
    pub skip_count: u64,
    pub failure_count: u64,
    pub unique_skill_count: u64,
    pub unique_mcp_count: u64,
    pub avatar_count: u64,
    pub filtered_file_count: u64,
    pub agents: Vec<ImportedAgentPreviewItem>,
    pub teams: Vec<ImportedTeamPreviewItem>,
    pub skills: Vec<ImportedSkillPreviewItem>,
    pub mcps: Vec<ImportedMcpPreviewItem>,
    pub avatars: Vec<ImportedAvatarPreviewItem>,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundleResult {
    pub departments: Vec<String>,
    pub department_count: u64,
    pub detected_agent_count: u64,
    pub importable_agent_count: u64,
    pub detected_team_count: u64,
    pub importable_team_count: u64,
    pub create_count: u64,
    pub update_count: u64,
    pub skip_count: u64,
    pub failure_count: u64,
    pub unique_skill_count: u64,
    pub unique_mcp_count: u64,
    pub avatar_count: u64,
    pub filtered_file_count: u64,
    pub agents: Vec<ImportedAgentPreviewItem>,
    pub teams: Vec<ImportedTeamPreviewItem>,
    pub skills: Vec<ImportedSkillPreviewItem>,
    pub mcps: Vec<ImportedMcpPreviewItem>,
    pub avatars: Vec<ImportedAvatarPreviewItem>,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportWorkspaceAgentBundleInput {
    pub mode: String,
    pub agent_ids: Vec<String>,
    pub team_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportWorkspaceAgentBundleResult {
    pub root_dir_name: String,
    pub file_count: u64,
    pub agent_count: u64,
    pub team_count: u64,
    pub skill_count: u64,
    pub mcp_count: u64,
    pub avatar_count: u64,
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
    pub issues: Vec<ImportIssue>,
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
    pub role_ids: Vec<String>,
    pub scope_project_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceUserRequest {
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub role_ids: Vec<String>,
    pub scope_project_ids: Vec<String>,
    pub avatar: Option<AvatarUploadPayload>,
    pub use_default_avatar: Option<bool>,
    pub password: Option<String>,
    pub confirm_password: Option<String>,
    pub use_default_password: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceUserRequest {
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub role_ids: Vec<String>,
    pub scope_project_ids: Vec<String>,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
    pub password: Option<String>,
    pub confirm_password: Option<String>,
    pub reset_password_to_default: Option<bool>,
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
pub struct RoleRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub code: String,
    pub description: String,
    pub status: String,
    pub permission_ids: Vec<String>,
    pub menu_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub code: String,
    pub description: String,
    pub status: String,
    pub kind: String,
    pub target_type: Option<String>,
    pub target_ids: Vec<String>,
    pub action: Option<String>,
    pub member_permission_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuRecord {
    pub id: String,
    pub workspace_id: String,
    pub parent_id: Option<String>,
    pub source: String,
    pub label: String,
    pub route_name: Option<String>,
    pub status: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCenterAlertRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCenterOverviewSnapshot {
    pub workspace_id: String,
    pub current_user: UserRecordSummary,
    pub role_names: Vec<String>,
    pub metrics: Vec<WorkspaceMetricRecord>,
    pub alerts: Vec<PermissionCenterAlertRecord>,
    pub quick_links: Vec<MenuRecord>,
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

fn default_runtime_session_kind() -> String {
    "project".into()
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
    pub configured_model_id: Option<String>,
    pub configured_model_name: Option<String>,
    pub model_id: Option<String>,
    pub consumed_tokens: Option<u32>,
    pub next_action: Option<String>,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub started_from_scope_set: Vec<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub emitted_at: u64,
    pub sequence: u64,
    pub payload: Option<serde_json::Value>,
    pub run: Option<RuntimeRunSnapshot>,
    pub message: Option<RuntimeMessage>,
    pub trace: Option<RuntimeTraceItem>,
    pub approval: Option<ApprovalRequestRecord>,
    pub decision: Option<String>,
    pub summary: Option<RuntimeSessionSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionDetail {
    pub summary: RuntimeSessionSummary,
    pub run: RuntimeRunSnapshot,
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
    pub project_id: String,
    pub title: String,
    pub session_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRuntimeTurnInput {
    pub content: String,
    pub model_id: Option<String>,
    pub configured_model_id: Option<String>,
    pub permission_mode: String,
    pub actor_kind: Option<String>,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeApprovalInput {
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub status: String,
    pub latest_version: u32,
    pub updated_at: u64,
    pub storage_path: Option<String>,
    pub content_hash: Option<String>,
    pub byte_size: Option<u64>,
    pub content_type: Option<String>,
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
