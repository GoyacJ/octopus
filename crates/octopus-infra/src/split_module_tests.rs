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
fn workspace_bootstrap_does_not_seed_editable_agents_teams_or_automations() {
    let temp = tempfile::tempdir().expect("tempdir");
    let initialized = bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");
    let connection = Connection::open(&initialized.db_path).expect("db");

    let agent_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
        .expect("agent count");
    let team_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM teams", [], |row| row.get(0))
        .expect("team count");
    let automation_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM automations", [], |row| row.get(0))
        .expect("automation count");

    assert_eq!(agent_count, 0);
    assert_eq!(team_count, 0);
    assert_eq!(automation_count, 0);
    assert!(std::fs::read_dir(&initialized.managed_skills_dir)
        .expect("managed skills dir")
        .next()
        .is_none());
}
