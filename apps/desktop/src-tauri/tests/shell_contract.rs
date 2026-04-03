use std::{path::{Path, PathBuf}, process::Command, sync::Arc};

use octopus_core::{default_preferences, DesktopBackendConnection, HostState, ShellPreferences};
use octopus_desktop_shell::backend::BackendSupervisor;
use octopus_desktop_shell::bootstrap::{
  bootstrap_shell_payload, healthcheck_payload, load_shell_preferences, save_shell_preferences,
};
use octopus_desktop_shell::services::PreferencesService;
use octopus_desktop_shell::state::ShellState;
use parking_lot::RwLock;
use tempfile::tempdir;

fn test_host_state() -> HostState {
  HostState {
    platform: "tauri".into(),
    mode: "local".into(),
    app_version: "0.1.0-test".into(),
    cargo_workspace: true,
    shell: "tauri2".into(),
  }
}

fn test_state(root: PathBuf) -> ShellState {
  ShellState::new(
    test_host_state(),
    PreferencesService::new(
      root.join("preferences.json"),
      default_preferences("ws-local", "proj-redesign"),
    ),
  )
}

fn test_state_with_supervisor(root: PathBuf, backend_supervisor: BackendSupervisor) -> ShellState {
  ShellState::with_connections(
    test_host_state(),
    PreferencesService::new(
      root.join("preferences.json"),
      default_preferences("ws-local", "proj-redesign"),
    ),
    vec![],
    backend_supervisor,
  )
}

fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .ancestors()
    .nth(3)
    .map(Path::to_path_buf)
    .expect("workspace root")
}

fn ensure_backend_binary_exists() {
  let status = Command::new("cargo")
    .current_dir(repo_root())
    .args(["build", "-p", "octopus-desktop-backend"])
    .status()
    .expect("backend build should start");

  assert!(status.success(), "backend build should succeed");
}

#[test]
fn bootstrap_uses_defaults_when_preferences_file_is_missing() {
  let temp = tempdir().expect("tempdir");
  let state = test_state(temp.path().to_path_buf());

  let payload = bootstrap_shell_payload(&state).expect("bootstrap payload");

  assert_eq!(payload.host_state.platform, "tauri");
  assert_eq!(payload.preferences.default_workspace_id, "ws-local");
  assert_eq!(payload.preferences.last_visited_route, "/workspaces/ws-local/overview?project=proj-redesign");
  let backend = payload.backend.expect("shell backend payload");
  assert_eq!(backend.state, "unavailable");
  assert_eq!(backend.transport, "http");
  assert_eq!(backend.base_url, None);
  assert_eq!(backend.auth_token, None);
}

#[test]
fn save_then_load_preferences_roundtrips_to_disk() {
  let temp = tempdir().expect("tempdir");
  let state = test_state(temp.path().to_path_buf());
  let preferences = ShellPreferences {
    theme: "dark".into(),
    locale: "en-US".into(),
    compact_sidebar: true,
    left_sidebar_collapsed: true,
    right_sidebar_collapsed: false,
    default_workspace_id: "ws-enterprise".into(),
    last_visited_route: "/workspaces/ws-enterprise/overview?project=proj-launch".into(),
  };

  save_shell_preferences(&state, preferences.clone()).expect("save preferences");
  let loaded = load_shell_preferences(&state).expect("load preferences");

  assert_eq!(loaded, preferences);
}

#[test]
fn healthcheck_reflects_tauri_local_workspace_status() {
  let temp = tempdir().expect("tempdir");
  let state = test_state(temp.path().to_path_buf());

  let payload = healthcheck_payload(&state);

  assert_eq!(payload.status, "ok");
  assert_eq!(payload.host, "tauri");
  assert_eq!(payload.mode, "local");
  assert!(payload.cargo_workspace);
  assert_eq!(payload.backend.state, "unavailable");
  assert_eq!(payload.backend.transport, "http");
}

#[tokio::test]
async fn bootstrap_and_healthcheck_reflect_ready_backend_after_supervisor_start() {
  ensure_backend_binary_exists();

  let temp = tempdir().expect("tempdir");
  let runtime_root = temp.path().join("runtime");
  let supervisor = BackendSupervisor::new(
    Arc::new(RwLock::new(DesktopBackendConnection {
      base_url: None,
      auth_token: None,
      state: "unavailable".into(),
      transport: "http".into(),
    })),
    runtime_root,
  );
  let state = test_state_with_supervisor(temp.path().to_path_buf(), supervisor.clone());

  let connection = supervisor
    .start_dev(&state.host_state, state.preferences_service.path())
    .await
    .expect("backend should start in dev mode");

  let bootstrap = bootstrap_shell_payload(&state).expect("bootstrap payload");
  let health = healthcheck_payload(&state);
  let backend = bootstrap.backend.expect("shell backend payload");

  assert_eq!(connection.state, "ready");
  assert_eq!(backend.state, "ready");
  assert_eq!(backend.transport, "http");
  assert_eq!(health.backend.state, "ready");
  assert_eq!(health.backend.transport, "http");
  assert!(backend.base_url.is_some());
  assert!(backend.auth_token.is_some());
  assert_eq!(backend.base_url, connection.base_url);
  assert_eq!(backend.auth_token, connection.auth_token);

  supervisor.shutdown();
}
