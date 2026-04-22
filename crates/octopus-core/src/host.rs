use serde::{Deserialize, Serialize};

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
