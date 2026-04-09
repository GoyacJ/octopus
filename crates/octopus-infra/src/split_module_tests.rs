use super::{bootstrap, workspace_paths};

#[test]
fn split_modules_expose_workspace_bootstrap_api() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = workspace_paths::WorkspacePaths::new(temp.path());
    assert_eq!(paths.db_path, temp.path().join("data/main.db"));

    let initialized = bootstrap::initialize_workspace(temp.path()).expect("workspace initialized");
    assert!(initialized.workspace_config.exists());
    assert!(initialized.app_registry_config.exists());
}
