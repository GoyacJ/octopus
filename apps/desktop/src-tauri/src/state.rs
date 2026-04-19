use std::{fs, path::Path, sync::Arc};

use octopus_core::{
    default_connection_stubs, default_host_state, default_preferences, ConnectionProfile,
    DesktopBackendConnection, HostState, PreferencesPort, ShellPreferences, DEFAULT_PROJECT_ID,
    DEFAULT_WORKSPACE_ID,
};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

use crate::{
    backend::BackendSupervisor,
    error::ShellResult,
    services::{NotificationService, PreferencesService, WorkspaceConnectionRegistryService},
    updates::AppUpdateService,
};

#[derive(Clone)]
pub struct ShellState {
    pub host_state: HostState,
    pub preferences_service: PreferencesService,
    pub local_connections: Vec<ConnectionProfile>,
    pub workspace_connection_registry_service: WorkspaceConnectionRegistryService,
    pub notification_service: NotificationService,
    pub backend_supervisor: BackendSupervisor,
    pub app_update_service: AppUpdateService,
}

impl ShellState {
    pub fn new(
        host_state: HostState,
        preferences_service: PreferencesService,
        workspace_connection_registry_service: WorkspaceConnectionRegistryService,
    ) -> Self {
        let preferences_path = preferences_service.path().to_path_buf();
        let workspace_root =
            resolve_workspace_root_for_backend(&preferences_path, host_state.cargo_workspace);
        let notification_service =
            NotificationService::new(resolve_notification_db_path(&preferences_path));

        Self::with_connections(
            host_state,
            preferences_service,
            workspace_connection_registry_service,
            notification_service,
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
        workspace_connection_registry_service: WorkspaceConnectionRegistryService,
        notification_service: NotificationService,
        local_connections: Vec<ConnectionProfile>,
        backend_supervisor: BackendSupervisor,
    ) -> Self {
        Self {
            host_state,
            preferences_service,
            local_connections,
            workspace_connection_registry_service,
            notification_service,
            backend_supervisor,
            app_update_service: AppUpdateService::new(),
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
    let workspace_connections_path = preferences_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("shell-workspace-connections.json");
    let host_state = default_host_state(
        app.package_info().version.to_string(),
        detect_cargo_workspace(),
    );
    let preferences_service = PreferencesService::new(
        preferences_path,
        default_preferences(DEFAULT_WORKSPACE_ID, DEFAULT_PROJECT_ID),
    );
    let workspace_connection_registry_service =
        WorkspaceConnectionRegistryService::new(workspace_connections_path);

    Ok(ShellState::new(
        host_state,
        preferences_service,
        workspace_connection_registry_service,
    ))
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

fn default_workspace_root_for_backend(
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

fn resolve_workspace_root_from_config(root: std::path::PathBuf) -> std::path::PathBuf {
    let config_path = root.join("config").join("workspace.toml");
    fs::read_to_string(config_path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let trimmed = line.trim();
                let (key, value) = trimmed.split_once('=')?;
                if key.trim() != "mapped_directory" {
                    return None;
                }
                Some(
                    value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string(),
                )
            })
        })
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or(root)
}

pub(crate) fn resolve_workspace_root_for_backend(
    preferences_path: &Path,
    cargo_workspace: bool,
) -> std::path::PathBuf {
    resolve_workspace_root_from_config(default_workspace_root_for_backend(
        preferences_path,
        cargo_workspace,
    ))
}

fn resolve_notification_db_path(preferences_path: &Path) -> std::path::PathBuf {
    preferences_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("data/main.db")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{resolve_notification_db_path, resolve_workspace_root_for_backend};

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

    #[test]
    fn resolves_persisted_mapped_directory_when_workspace_config_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let shell_root = temp.path().join("shell-root");
        let mapped_root = temp.path().join("mapped-root");
        std::fs::create_dir_all(shell_root.join("config")).expect("shell config dir");
        std::fs::write(
            shell_root.join("config").join("workspace.toml"),
            format!(
                "mapped_directory = {:?}\n",
                mapped_root.display().to_string()
            ),
        )
        .expect("write workspace config");

        assert_eq!(
            resolve_workspace_root_for_backend(&shell_root.join("shell-preferences.json"), false),
            mapped_root,
        );
    }

    #[test]
    fn resolves_notification_db_next_to_preferences_root() {
        let preferences_path = PathBuf::from("/tmp/octopus-shell/shell-preferences.json");

        assert_eq!(
            resolve_notification_db_path(&preferences_path),
            PathBuf::from("/tmp/octopus-shell/data/main.db"),
        );
    }
}
