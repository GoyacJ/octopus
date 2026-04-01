use std::path::PathBuf;

use octopus_core::{default_preferences, HostState, ShellPreferences};
use octopus_desktop_shell::bootstrap::{
  bootstrap_shell_payload, healthcheck_payload, load_shell_preferences, save_shell_preferences,
};
use octopus_desktop_shell::services::PreferencesService;
use octopus_desktop_shell::state::ShellState;
use tempfile::tempdir;

fn test_state(root: PathBuf) -> ShellState {
  ShellState::new(
    HostState {
      platform: "tauri".into(),
      mode: "local".into(),
      app_version: "0.1.0-test".into(),
      cargo_workspace: true,
      shell: "tauri2".into(),
    },
    PreferencesService::new(
      root.join("preferences.json"),
      default_preferences("ws-local", "proj-redesign"),
    ),
  )
}

#[test]
fn bootstrap_uses_defaults_when_preferences_file_is_missing() {
  let temp = tempdir().expect("tempdir");
  let state = test_state(temp.path().to_path_buf());

  let payload = bootstrap_shell_payload(&state).expect("bootstrap payload");

  assert_eq!(payload.host_state.platform, "tauri");
  assert_eq!(payload.preferences.default_workspace_id, "ws-local");
  assert_eq!(payload.preferences.last_visited_route, "/workspaces/ws-local/overview?project=proj-redesign");
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
}
