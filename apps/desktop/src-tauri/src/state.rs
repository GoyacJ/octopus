use std::{path::Path, sync::Arc};

use octopus_core::{
    default_connection_stubs, default_host_state, default_preferences, ConnectionProfile,
    DesktopBackendConnection, HostState, PreferencesPort, ShellPreferences, DEFAULT_PROJECT_ID,
    DEFAULT_WORKSPACE_ID,
};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

use crate::{backend::BackendSupervisor, error::ShellResult, services::PreferencesService};

#[derive(Clone)]
pub struct ShellState {
    pub host_state: HostState,
    pub preferences_service: PreferencesService,
    pub connections: Vec<ConnectionProfile>,
    pub backend_supervisor: BackendSupervisor,
}

impl ShellState {
    pub fn new(host_state: HostState, preferences_service: PreferencesService) -> Self {
        let preferences_path = preferences_service.path().to_path_buf();
        let workspace_root =
            resolve_workspace_root_for_backend(&preferences_path, host_state.cargo_workspace);

        Self::with_connections(
            host_state,
            preferences_service,
            default_connection_stubs(),
            BackendSupervisor::new(
                Arc::new(RwLock::new(DesktopBackendConnection {
                    base_url: None,
                    auth_token: None,
                    state: "unavailable".into(),
                    transport: "http".into(),
                })),
                workspace_root,
            ),
        )
    }

    pub fn with_connections(
        host_state: HostState,
        preferences_service: PreferencesService,
        connections: Vec<ConnectionProfile>,
        backend_supervisor: BackendSupervisor,
    ) -> Self {
        Self {
            host_state,
            preferences_service,
            connections,
            backend_supervisor,
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
    let host_state = default_host_state(
        app.package_info().version.to_string(),
        detect_cargo_workspace(),
    );
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

fn resolve_workspace_root_for_backend(
    preferences_path: &Path,
    cargo_workspace: bool,
) -> std::path::PathBuf {
    if cargo_workspace {
        return Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(".").to_path_buf());
    }

    preferences_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(".").to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::resolve_workspace_root_for_backend;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .map(Path::to_path_buf)
            .expect("workspace root")
    }

    #[test]
    fn resolves_repo_root_for_backend_when_running_inside_cargo_workspace() {
        let preferences_path = PathBuf::from("/tmp/octopus-shell/shell-preferences.json");

        assert_eq!(
            resolve_workspace_root_for_backend(&preferences_path, true),
            repo_root(),
        );
    }

    #[test]
    fn falls_back_to_preferences_parent_when_not_in_cargo_workspace() {
        let preferences_path = PathBuf::from("/tmp/octopus-shell/shell-preferences.json");

        assert_eq!(
            resolve_workspace_root_for_backend(&preferences_path, false),
            PathBuf::from("/tmp/octopus-shell"),
        );
    }
}
