use octopus_core::{
    connection_profile_from_host_workspace_connection,
    host_workspace_connection_record_from_profile, normalize_connection_base_url,
    ConnectionProfile, CreateHostWorkspaceConnectionInput, CreateNotificationInput,
    DesktopBackendConnection, HealthcheckBackendStatus, HealthcheckStatus, HostState,
    HostWorkspaceConnectionRecord, NotificationFilter, NotificationListResponse,
    NotificationRecord, NotificationUnreadSummary, PreferencesPort, ShellBootstrap,
    ShellPreferences, timestamp_now,
};

use crate::{error::ShellResult, state::ShellState};

pub type ShellBootstrapPayload = ShellBootstrap;
pub type HealthcheckStatusPayload = HealthcheckStatus;

fn list_remote_workspace_connections(
    state: &ShellState,
) -> ShellResult<Vec<HostWorkspaceConnectionRecord>> {
    state
        .workspace_connection_registry_service
        .load_connections()
}

pub fn list_workspace_connections(
    state: &ShellState,
) -> ShellResult<Vec<HostWorkspaceConnectionRecord>> {
    let backend = state.backend_supervisor.connection();
    let mut connections = state
        .local_connections
        .iter()
        .map(|connection| host_workspace_connection_record_from_profile(connection, Some(&backend)))
        .collect::<Vec<_>>();
    connections.extend(list_remote_workspace_connections(state)?);
    Ok(connections)
}

pub fn create_workspace_connection(
    state: &ShellState,
    input: CreateHostWorkspaceConnectionInput,
) -> ShellResult<HostWorkspaceConnectionRecord> {
    let connection = HostWorkspaceConnectionRecord {
        workspace_connection_id: format!(
            "conn-remote-{}-{}",
            input.workspace_id,
            timestamp_now()
        ),
        workspace_id: input.workspace_id,
        label: input.label,
        base_url: normalize_connection_base_url(&input.base_url),
        transport_security: input.transport_security,
        auth_mode: input.auth_mode,
        last_used_at: Some(timestamp_now()),
        status: "connected".into(),
    };

    state
        .workspace_connection_registry_service
        .upsert_connection(connection)
}

pub fn delete_workspace_connection(
    state: &ShellState,
    workspace_connection_id: &str,
) -> ShellResult<()> {
    if state
        .local_connections
        .iter()
        .any(|connection| connection.id == workspace_connection_id)
    {
        return Err(octopus_core::AppError::invalid_input(
            "local workspace connection cannot be deleted",
        ));
    }

    state
        .workspace_connection_registry_service
        .delete_connection(workspace_connection_id)?;
    Ok(())
}

pub fn bootstrap_shell_payload(state: &ShellState) -> ShellResult<ShellBootstrapPayload> {
    let connections = list_workspace_connections(state)?
        .iter()
        .map(connection_profile_from_host_workspace_connection)
        .collect::<Vec<ConnectionProfile>>();

    Ok(ShellBootstrapPayload {
        host_state: state.host_state.clone(),
        preferences: state.preferences_service.load_preferences()?,
        connections,
        backend: Some(state.backend_supervisor.connection()),
    })
}

pub fn get_host_state_payload(state: &ShellState) -> HostState {
    state.host_state.clone()
}

pub fn load_shell_preferences(state: &ShellState) -> ShellResult<ShellPreferences> {
    state.preferences_service.load_preferences()
}

pub fn save_shell_preferences(
    state: &ShellState,
    preferences: ShellPreferences,
) -> ShellResult<ShellPreferences> {
    state.preferences_service.save_preferences(&preferences)
}

pub fn list_connections_payload(state: &ShellState) -> Vec<ConnectionProfile> {
    state.local_connections.clone()
}

pub fn get_backend_connection_payload(state: &ShellState) -> DesktopBackendConnection {
    state.backend_supervisor.connection()
}

pub fn healthcheck_payload(state: &ShellState) -> HealthcheckStatusPayload {
    let backend = state.backend_supervisor.connection();

    HealthcheckStatusPayload {
        status: "ok".into(),
        host: state.host_state.platform.clone(),
        mode: state.host_state.mode.clone(),
        cargo_workspace: state.host_state.cargo_workspace,
        backend: HealthcheckBackendStatus {
            state: backend.state,
            transport: backend.transport,
        },
    }
}

pub fn list_notifications(
    state: &ShellState,
    filter: NotificationFilter,
) -> ShellResult<NotificationListResponse> {
    state.notification_service.list_notifications(filter)
}

pub fn create_notification(
    state: &ShellState,
    input: CreateNotificationInput,
) -> ShellResult<NotificationRecord> {
    state.notification_service.create_notification(input)
}

pub fn mark_notification_read(
    state: &ShellState,
    id: &str,
) -> ShellResult<NotificationRecord> {
    state.notification_service.mark_notification_read(id)
}

pub fn mark_all_notifications_read(
    state: &ShellState,
    filter: NotificationFilter,
) -> ShellResult<NotificationUnreadSummary> {
    state.notification_service.mark_all_notifications_read(filter)
}

pub fn dismiss_notification_toast(
    state: &ShellState,
    id: &str,
) -> ShellResult<NotificationRecord> {
    state.notification_service.dismiss_notification_toast(id)
}

pub fn get_notification_unread_summary(
    state: &ShellState,
) -> ShellResult<NotificationUnreadSummary> {
    state.notification_service.unread_summary()
}
