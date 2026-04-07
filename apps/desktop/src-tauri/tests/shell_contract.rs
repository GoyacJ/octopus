use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use octopus_core::{
    default_preferences, CreateNotificationInput, DesktopBackendConnection, HostState,
    NotificationFilter, RuntimeEffectiveConfig, ShellPreferences,
};
use octopus_desktop_shell::backend::BackendSupervisor;
use octopus_desktop_shell::bootstrap::{
    bootstrap_shell_payload, create_notification, delete_workspace_connection,
    dismiss_notification_toast, get_backend_connection_payload, healthcheck_payload,
    list_notifications, list_workspace_connections, load_shell_preferences,
    mark_all_notifications_read, mark_notification_read, save_shell_preferences,
    create_workspace_connection,
};
use octopus_desktop_shell::services::{
    NotificationService, PreferencesService, WorkspaceConnectionRegistryService,
};
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
        WorkspaceConnectionRegistryService::new(root.join("workspace-connections.json")),
    )
}

fn test_state_with_supervisor(root: PathBuf, backend_supervisor: BackendSupervisor) -> ShellState {
    ShellState::with_connections(
        test_host_state(),
        PreferencesService::new(
            root.join("preferences.json"),
            default_preferences("ws-local", "proj-redesign"),
        ),
        WorkspaceConnectionRegistryService::new(root.join("workspace-connections.json")),
        NotificationService::new(root.join("data").join("main.db")),
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
    assert_eq!(
        payload.preferences.last_visited_route,
        "/workspaces/ws-local/overview?project=proj-redesign"
    );
    let backend = payload.backend.expect("shell backend payload");
    assert_eq!(backend.state, "unavailable");
    assert_eq!(backend.transport, "http");
    assert_eq!(backend.base_url, None);
    assert_eq!(backend.auth_token, None);
    assert_eq!(payload.connections.len(), 1);
    assert_eq!(payload.connections[0].id, "conn-local");
}

#[test]
fn workspace_connections_roundtrip_and_deduplicate_by_base_url_and_workspace_id() {
    let temp = tempdir().expect("tempdir");
    let state = test_state(temp.path().to_path_buf());

    let created = create_workspace_connection(
        &state,
        octopus_core::CreateHostWorkspaceConnectionInput {
            workspace_id: "ws-enterprise".into(),
            label: "Enterprise Workspace".into(),
            base_url: "https://enterprise.example.test/".into(),
            transport_security: "trusted".into(),
            auth_mode: "session-token".into(),
        },
    )
    .expect("create workspace connection");

    let deduped = create_workspace_connection(
        &state,
        octopus_core::CreateHostWorkspaceConnectionInput {
            workspace_id: "ws-enterprise".into(),
            label: "Enterprise Workspace".into(),
            base_url: "https://enterprise.example.test".into(),
            transport_security: "trusted".into(),
            auth_mode: "session-token".into(),
        },
    )
    .expect("dedupe workspace connection");

    assert_eq!(created.workspace_connection_id, deduped.workspace_connection_id);

    let listed = list_workspace_connections(&state).expect("list workspace connections");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].workspace_connection_id, "conn-local");
    assert_eq!(listed[1].workspace_connection_id, created.workspace_connection_id);
}

#[test]
fn deleting_remote_workspace_connection_keeps_local_default_connection() {
    let temp = tempdir().expect("tempdir");
    let state = test_state(temp.path().to_path_buf());

    let created = create_workspace_connection(
        &state,
        octopus_core::CreateHostWorkspaceConnectionInput {
            workspace_id: "ws-enterprise".into(),
            label: "Enterprise Workspace".into(),
            base_url: "https://enterprise.example.test".into(),
            transport_security: "trusted".into(),
            auth_mode: "session-token".into(),
        },
    )
    .expect("create workspace connection");

    delete_workspace_connection(&state, &created.workspace_connection_id)
        .expect("delete workspace connection");

    let listed = list_workspace_connections(&state).expect("list workspace connections");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].workspace_connection_id, "conn-local");
}

#[test]
fn save_then_load_preferences_roundtrips_to_disk() {
    let temp = tempdir().expect("tempdir");
    let state = test_state(temp.path().to_path_buf());
    let preferences = ShellPreferences {
        theme: "dark".into(),
        locale: "en-US".into(),
        font_size: 16,
        font_family: "SF Pro Display".into(),
        font_style: "sans".into(),
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

#[test]
fn notifications_persist_across_state_reloads_and_compute_unread_counts() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let state = test_state(root.clone());

    let created = create_notification(
        &state,
        CreateNotificationInput {
            scope_kind: "workspace".into(),
            scope_owner_id: Some("ws-local".into()),
            level: "success".into(),
            title: "Workspace synced".into(),
            body: "The workspace is ready.".into(),
            source: "workspace-store".into(),
            toast_duration_ms: Some(30_000),
            route_to: Some("/workspaces/ws-local/overview".into()),
            action_label: Some("Open workspace".into()),
        },
    )
    .expect("create notification");

    let reloaded = test_state(root);
    let listed = list_notifications(
        &reloaded,
        NotificationFilter {
            scope: Some("all".into()),
        },
    )
    .expect("list notifications");

    assert_eq!(listed.notifications.len(), 1);
    assert_eq!(listed.notifications[0].id, created.id);
    assert_eq!(listed.unread.total, 1);
    assert_eq!(listed.unread.by_scope.workspace, 1);
}

#[test]
fn notifications_support_mark_read_mark_all_read_and_toast_dismiss_without_deleting_history() {
    let temp = tempdir().expect("tempdir");
    let state = test_state(temp.path().to_path_buf());

    let created = create_notification(
        &state,
        CreateNotificationInput {
            scope_kind: "app".into(),
            scope_owner_id: None,
            level: "info".into(),
            title: "Heads up".into(),
            body: "A background task completed.".into(),
            source: "runtime".into(),
            toast_duration_ms: Some(30_000),
            route_to: None,
            action_label: None,
        },
    )
    .expect("create notification");

    let marked = mark_notification_read(&state, &created.id).expect("mark notification read");
    assert!(marked.read_at.is_some());

    let dismissed =
        dismiss_notification_toast(&state, &created.id).expect("dismiss notification toast");
    assert_eq!(dismissed.id, created.id);
    assert_eq!(dismissed.toast_visible_until, None);

    let summary = mark_all_notifications_read(
        &state,
        NotificationFilter {
            scope: Some("app".into()),
        },
    )
    .expect("mark all notifications read");

    assert_eq!(summary.total, 0);

    let listed = list_notifications(
        &state,
        NotificationFilter {
            scope: Some("all".into()),
        },
    )
    .expect("list notifications");

    assert_eq!(listed.notifications.len(), 1);
    assert_eq!(listed.notifications[0].id, created.id);
    assert!(listed.notifications[0].read_at.is_some());
    assert_eq!(listed.notifications[0].toast_visible_until, None);
}

#[test]
fn backend_connection_payload_exposes_supervisor_state() {
    let temp = tempdir().expect("tempdir");
    let state = test_state(temp.path().to_path_buf());

    let connection = get_backend_connection_payload(&state);

    assert_eq!(connection.state, "unavailable");
    assert_eq!(connection.transport, "http");
    assert_eq!(connection.base_url, None);
    assert_eq!(connection.auth_token, None);
}

#[tokio::test]
async fn bootstrap_and_healthcheck_reflect_ready_backend_after_supervisor_start() {
    ensure_backend_binary_exists();

    let temp = tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace-root");
    let supervisor = BackendSupervisor::new(
        Arc::new(RwLock::new(DesktopBackendConnection {
            base_url: None,
            auth_token: None,
            state: "unavailable".into(),
            transport: "http".into(),
        })),
        workspace_root,
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

#[tokio::test]
async fn backend_host_token_does_not_grant_workspace_api_access() {
    ensure_backend_binary_exists();

    let temp = tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace-root");
    let supervisor = BackendSupervisor::new(
        Arc::new(RwLock::new(DesktopBackendConnection {
            base_url: None,
            auth_token: None,
            state: "unavailable".into(),
            transport: "http".into(),
        })),
        workspace_root,
    );
    let state = test_state_with_supervisor(temp.path().to_path_buf(), supervisor.clone());

    let connection = supervisor
        .start_dev(&state.host_state, state.preferences_service.path())
        .await
        .expect("backend should start in dev mode");

    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/v1/workspace",
            connection.base_url.expect("backend base url")
        ))
        .bearer_auth(connection.auth_token.expect("backend auth token"))
        .send()
        .await
        .expect("workspace request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    supervisor.shutdown();
}

#[tokio::test]
async fn runtime_config_sources_expose_workspace_relative_metadata_without_absolute_paths() {
    ensure_backend_binary_exists();

    let temp = tempdir().expect("tempdir");
    let workspace_root = temp.path().join("nested").join("workspace-root");
    let supervisor = BackendSupervisor::new(
        Arc::new(RwLock::new(DesktopBackendConnection {
            base_url: None,
            auth_token: None,
            state: "unavailable".into(),
            transport: "http".into(),
        })),
        workspace_root.clone(),
    );
    let state = test_state_with_supervisor(temp.path().to_path_buf(), supervisor.clone());

    let connection = supervisor
        .start_dev(&state.host_state, state.preferences_service.path())
        .await
        .expect("backend should start in dev mode");

    let config = reqwest::Client::new()
        .get(format!(
            "{}/api/v1/runtime/config",
            connection.base_url.expect("backend base url")
        ))
        .send()
        .await
        .expect("runtime config request")
        .json::<RuntimeEffectiveConfig>()
        .await
        .expect("runtime config payload");

    let workspace_source = config
        .sources
        .iter()
        .find(|source| source.source_key == "workspace")
        .expect("workspace runtime config source");

    assert_eq!(workspace_source.scope, "workspace");
    assert_eq!(
        workspace_source.display_path,
        "config/runtime/workspace.json"
    );
    assert_eq!(workspace_source.owner_id, None);

    let serialized = serde_json::to_string(&config).expect("runtime config serialized");
    assert!(!serialized.contains(&workspace_root.display().to_string()));

    supervisor.shutdown();
}
