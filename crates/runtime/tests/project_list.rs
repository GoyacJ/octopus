use octopus_runtime::Slice1Runtime;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

async fn open_runtime() -> (tempfile::TempDir, std::path::PathBuf, Slice1Runtime) {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = tempdir.path().join("project-list.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    (tempdir, db_path, runtime)
}

async fn update_project_timestamp(
    db_path: &std::path::Path,
    project_id: &str,
    updated_at: &str,
) {
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(false)
            .foreign_keys(true),
    )
    .await
    .unwrap();

    sqlx::query("UPDATE projects SET updated_at = ?1 WHERE id = ?2")
        .bind(updated_at)
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn runtime_lists_workspace_projects_sorted_latest_first() {
    let (_tempdir, db_path, runtime) = open_runtime().await;

    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-a",
            "project-a",
            "Project A",
        )
        .await
        .unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-b",
            "project-b",
            "Project B",
        )
        .await
        .unwrap();
    runtime
        .ensure_project_context(
            "workspace-bravo",
            "workspace-bravo",
            "Workspace Bravo",
            "project-bravo",
            "project-bravo",
            "Project Bravo",
        )
        .await
        .unwrap();

    update_project_timestamp(&db_path, "project-a", "2026-03-29T10:00:01Z").await;
    update_project_timestamp(&db_path, "project-b", "2026-03-29T10:00:02Z").await;
    update_project_timestamp(&db_path, "project-bravo", "2026-03-29T10:00:03Z").await;

    let projects = runtime.list_projects("workspace-alpha").await.unwrap();
    let ids = projects
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["project-b", "project-a"]);
}

#[tokio::test]
async fn runtime_project_list_returns_empty_for_missing_workspace() {
    let (_tempdir, _db_path, runtime) = open_runtime().await;

    let projects = runtime.list_projects("workspace-empty").await.unwrap();

    assert!(projects.is_empty());
}
