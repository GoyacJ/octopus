use serde::{Deserialize, Serialize};

use crate::{AppError, DEFAULT_WORKSPACE_ID};

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
