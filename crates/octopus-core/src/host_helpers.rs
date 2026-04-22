use crate::{
    ConnectionProfile, DesktopBackendConnection, HostState, HostUpdateCapabilities,
    HostUpdateStatus, HostWorkspaceConnectionRecord, ShellPreferences, DEFAULT_WORKSPACE_ID,
};

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
