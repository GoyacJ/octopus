use super::{bootstrap, workspace_paths};
use rusqlite::Connection;

#[test]
fn split_modules_expose_workspace_bootstrap_api() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = workspace_paths::WorkspacePaths::new(temp.path());
    assert_eq!(paths.db_path, temp.path().join("data/main.db"));

    let initialized = bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");
    assert!(initialized.workspace_config.exists());
    assert!(initialized.app_registry_config.exists());
}

#[test]
fn workspace_bootstrap_does_not_seed_editable_agents_or_teams_and_drops_legacy_automations() {
    let temp = tempfile::tempdir().expect("tempdir");
    let initialized = bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");
    let connection = Connection::open(&initialized.db_path).expect("db");

    let agent_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
        .expect("agent count");
    let team_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM teams", [], |row| row.get(0))
        .expect("team count");
    assert_eq!(agent_count, 0);
    assert_eq!(team_count, 0);
    assert!(connection
        .prepare("SELECT 1 FROM automations LIMIT 1")
        .is_err());
    assert!(std::fs::read_dir(&initialized.managed_skills_dir)
        .expect("managed skills dir")
        .next()
        .is_none());
}

#[test]
fn workspace_bootstrap_hard_resets_legacy_access_control_tables_with_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = workspace_paths::WorkspacePaths::new(temp.path());
    paths.ensure_layout().expect("layout");

    let connection = Connection::open(&paths.db_path).expect("db");
    connection
        .execute_batch(
            "
            CREATE TABLE memberships (
              user_id TEXT NOT NULL,
              role_ids TEXT NOT NULL,
              scope_project_ids TEXT NOT NULL
            );
            INSERT INTO memberships (user_id, role_ids, scope_project_ids)
            VALUES ('user-owner', '[\"owner\"]', '[]');
            ",
        )
        .expect("seed legacy memberships");

    bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");

    let connection = Connection::open(&paths.db_path).expect("db");

    let memberships_missing = connection
        .prepare("SELECT 1 FROM memberships LIMIT 1")
        .is_err();
    let roles_missing = connection.prepare("SELECT 1 FROM roles LIMIT 1").is_err();
    let permissions_missing = connection
        .prepare("SELECT 1 FROM permissions LIMIT 1")
        .is_err();
    let users_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("users count");
    let role_bindings_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM role_bindings", [], |row| row.get(0))
        .expect("role bindings count");
    let data_policies_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM data_policies", [], |row| row.get(0))
        .expect("data policies count");

    assert!(memberships_missing);
    assert!(roles_missing);
    assert!(permissions_missing);
    assert_eq!(users_count, 0);
    assert_eq!(role_bindings_count, 0);
    assert_eq!(data_policies_count, 0);
}

#[test]
fn workspace_bootstrap_seeds_task_projection_tables_and_task_permission_defaults() {
    let temp = tempfile::tempdir().expect("tempdir");
    let initialized = bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");
    let connection = Connection::open(&initialized.db_path).expect("db");

    for table in [
        "project_tasks",
        "project_task_runs",
        "project_task_interventions",
        "project_task_scheduler_claims",
    ] {
        let exists: Option<String> = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(exists.as_deref(), Some(table));
    }

    let workspace_config: toml::Value = toml::from_str(
        &std::fs::read_to_string(&initialized.workspace_config).expect("read workspace config"),
    )
    .expect("parse workspace config");
    assert_eq!(
        workspace_config
            .get("project_default_permissions")
            .and_then(|value| value.get("tasks"))
            .and_then(toml::Value::as_str),
        Some("allow")
    );

    let default_project_permission_overrides_json: String = connection
        .query_row(
            "SELECT permission_overrides_json FROM projects WHERE id = 'proj-redesign'",
            [],
            |row| row.get(0),
        )
        .expect("default project permission overrides");
    let default_project_permission_overrides: serde_json::Value =
        serde_json::from_str(&default_project_permission_overrides_json)
            .expect("parse permission overrides json");
    assert_eq!(
        default_project_permission_overrides
            .get("tasks")
            .and_then(serde_json::Value::as_str),
        Some("inherit")
    );
}

#[test]
fn workspace_bootstrap_hard_resets_legacy_sessions_table_shape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = workspace_paths::WorkspacePaths::new(temp.path());
    paths.ensure_layout().expect("layout");

    let connection = Connection::open(&paths.db_path).expect("db");
    connection
        .execute_batch(
            "
            CREATE TABLE sessions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              client_app_id TEXT NOT NULL,
              token TEXT NOT NULL UNIQUE,
              status TEXT NOT NULL,
              role_ids TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER
            );
            ",
        )
        .expect("seed legacy sessions");

    bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");

    let connection = Connection::open(&paths.db_path).expect("db");
    let mut pragma = connection
        .prepare("PRAGMA table_info(sessions)")
        .expect("pragma sessions");
    let columns = pragma
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query columns")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect columns");

    assert!(!columns.iter().any(|column| column == "role_ids"));
    assert_eq!(
        columns,
        vec![
            "id",
            "workspace_id",
            "user_id",
            "client_app_id",
            "token",
            "status",
            "created_at",
            "expires_at",
        ]
    );
}
