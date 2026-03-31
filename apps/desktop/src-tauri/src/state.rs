use std::path::Path;

use octopus_core::{
  default_connection_stubs, default_host_state, default_preferences, ConnectionProfile, HostState, PreferencesPort,
  ShellPreferences, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use tauri::{AppHandle, Manager};

use crate::{error::ShellResult, services::PreferencesService};

#[derive(Debug, Clone)]
pub struct ShellState {
  pub host_state: HostState,
  pub preferences_service: PreferencesService,
  pub connections: Vec<ConnectionProfile>,
}

impl ShellState {
  pub fn new(host_state: HostState, preferences_service: PreferencesService) -> Self {
    Self::with_connections(host_state, preferences_service, default_connection_stubs())
  }

  pub fn with_connections(
    host_state: HostState,
    preferences_service: PreferencesService,
    connections: Vec<ConnectionProfile>,
  ) -> Self {
    Self {
      host_state,
      preferences_service,
      connections,
    }
  }

  pub fn default_preferences(&self) -> Result<ShellPreferences, octopus_core::AppError> {
    self.preferences_service.load_preferences()
  }
}

pub fn build_shell_state(app: &AppHandle) -> ShellResult<ShellState> {
  let preferences_path = app
    .path()
    .app_config_dir()
    .map_err(|error| octopus_core::AppError::Runtime(error.to_string()))?
    .join("shell-preferences.json");
  let host_state = default_host_state(app.package_info().version.to_string(), detect_cargo_workspace());
  let preferences_service = PreferencesService::new(
    preferences_path,
    default_preferences(DEFAULT_WORKSPACE_ID, DEFAULT_PROJECT_ID),
  );

  Ok(ShellState::new(host_state, preferences_service))
}

pub fn detect_cargo_workspace() -> bool {
  detect_cargo_workspace_from(Path::new(env!("CARGO_MANIFEST_DIR")))
}

pub fn detect_cargo_workspace_from(manifest_dir: &Path) -> bool {
  manifest_dir
    .ancestors()
    .nth(3)
    .map(|root| root.join("Cargo.toml").exists())
    .unwrap_or(false)
}
