use octopus_core::{
  ConnectionProfile,
  DesktopBackendConnection,
  HealthcheckBackendStatus,
  HealthcheckStatus,
  HostState,
  PreferencesPort,
  ShellBootstrap,
  ShellPreferences,
};

use crate::{error::ShellResult, state::ShellState};

pub type ShellBootstrapPayload = ShellBootstrap;
pub type HealthcheckStatusPayload = HealthcheckStatus;

pub fn bootstrap_shell_payload(state: &ShellState) -> ShellResult<ShellBootstrapPayload> {
  Ok(ShellBootstrapPayload {
    host_state: state.host_state.clone(),
    preferences: state.preferences_service.load_preferences()?,
    connections: state.connections.clone(),
    backend: Some(state.backend_supervisor.connection()),
  })
}

pub fn get_host_state_payload(state: &ShellState) -> HostState {
  state.host_state.clone()
}

pub fn load_shell_preferences(state: &ShellState) -> ShellResult<ShellPreferences> {
  state.preferences_service.load_preferences()
}

pub fn save_shell_preferences(state: &ShellState, preferences: ShellPreferences) -> ShellResult<ShellPreferences> {
  state.preferences_service.save_preferences(&preferences)
}

pub fn list_connections_payload(state: &ShellState) -> Vec<ConnectionProfile> {
  state.connections.clone()
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
