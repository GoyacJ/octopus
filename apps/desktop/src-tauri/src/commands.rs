use octopus_core::{ConnectionProfile, HealthcheckStatus, HostState, ShellBootstrap, ShellPreferences};
use tauri::State;

use crate::{
  bootstrap::{
    bootstrap_shell_payload, get_host_state_payload, healthcheck_payload, list_connections_payload, load_shell_preferences,
    save_shell_preferences,
  },
  error::into_command_error,
  state::ShellState,
};

#[tauri::command]
pub fn bootstrap_shell(state: State<'_, ShellState>) -> Result<ShellBootstrap, String> {
  bootstrap_shell_payload(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub fn get_host_state(state: State<'_, ShellState>) -> HostState {
  get_host_state_payload(state.inner())
}

#[tauri::command]
pub fn load_preferences(state: State<'_, ShellState>) -> Result<ShellPreferences, String> {
  load_shell_preferences(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub fn save_preferences(state: State<'_, ShellState>, preferences: ShellPreferences) -> Result<ShellPreferences, String> {
  save_shell_preferences(state.inner(), preferences).map_err(into_command_error)
}

#[tauri::command]
pub fn list_connections_stub(state: State<'_, ShellState>) -> Vec<ConnectionProfile> {
  list_connections_payload(state.inner())
}

#[tauri::command]
pub fn healthcheck(state: State<'_, ShellState>) -> HealthcheckStatus {
  healthcheck_payload(state.inner())
}
