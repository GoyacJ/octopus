use octopus_core::{ConnectionProfile, HostState, PreferencesPort, ShellPreferences};
use serde::Serialize;

use crate::{error::ShellResult, state::ShellState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostBackendConnectionPayload {
  pub base_url: Option<String>,
  pub auth_token: Option<String>,
  pub state: String,
  pub transport: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellBootstrapPayload {
  pub host_state: HostState,
  pub preferences: ShellPreferences,
  pub connections: Vec<ConnectionProfile>,
  pub backend: Option<HostBackendConnectionPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckBackendPayload {
  pub state: String,
  pub transport: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckStatusPayload {
  pub status: String,
  pub host: String,
  pub mode: String,
  pub cargo_workspace: bool,
  pub backend: HealthcheckBackendPayload,
}

pub fn bootstrap_shell_payload(state: &ShellState) -> ShellResult<ShellBootstrapPayload> {
  Ok(ShellBootstrapPayload {
    host_state: state.host_state.clone(),
    preferences: state.preferences_service.load_preferences()?,
    connections: state.connections.clone(),
    backend: Some(host_backend_payload(&state.backend_supervisor.connection())),
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

pub fn healthcheck_payload(state: &ShellState) -> HealthcheckStatusPayload {
  let backend = state.backend_supervisor.connection();

  HealthcheckStatusPayload {
    status: "ok".into(),
    host: state.host_state.platform.clone(),
    mode: state.host_state.mode.clone(),
    cargo_workspace: state.host_state.cargo_workspace,
    backend: HealthcheckBackendPayload {
      state: backend.state,
      transport: backend.transport,
    },
  }
}

fn host_backend_payload(connection: &octopus_core::DesktopBackendConnection) -> HostBackendConnectionPayload {
  HostBackendConnectionPayload {
    base_url: connection.base_url.clone(),
    auth_token: connection.auth_token.clone(),
    state: connection.state.clone(),
    transport: connection.transport.clone(),
  }
}
