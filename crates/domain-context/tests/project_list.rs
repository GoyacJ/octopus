use octopus_domain_context::{
    ContextRepository, ProjectRecord, SqliteContextStore, WorkspaceRecord,
};
use sqlx::sqlite::SqlitePoolOptions;

async fn open_store() -> SqliteContextStore {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE workspaces (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            slug TEXT NOT NULL,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    SqliteContextStore::new(pool)
}

fn workspace(id: &str, display_name: &str) -> WorkspaceRecord {
    WorkspaceRecord {
        id: id.to_string(),
        slug: id.to_string(),
        display_name: display_name.to_string(),
        created_at: "2026-03-29T10:00:00Z".to_string(),
        updated_at: "2026-03-29T10:00:00Z".to_string(),
    }
}

fn project(id: &str, workspace_id: &str, display_name: &str, updated_at: &str) -> ProjectRecord {
    ProjectRecord {
        id: id.to_string(),
        workspace_id: workspace_id.to_string(),
        slug: id.to_string(),
        display_name: display_name.to_string(),
        created_at: "2026-03-29T10:00:00Z".to_string(),
        updated_at: updated_at.to_string(),
    }
}

#[tokio::test]
async fn list_projects_is_workspace_scoped_and_sorted_latest_first() {
    let store = open_store().await;

    store
        .upsert_context(
            workspace("workspace-alpha", "Workspace Alpha"),
            project(
                "project-a",
                "workspace-alpha",
                "Project A",
                "2026-03-29T10:00:01Z",
            ),
        )
        .await
        .unwrap();
    store
        .upsert_context(
            workspace("workspace-alpha", "Workspace Alpha"),
            project(
                "project-b",
                "workspace-alpha",
                "Project B",
                "2026-03-29T10:00:01Z",
            ),
        )
        .await
        .unwrap();
    store
        .upsert_context(
            workspace("workspace-alpha", "Workspace Alpha"),
            project(
                "project-c",
                "workspace-alpha",
                "Project C",
                "2026-03-29T10:00:02Z",
            ),
        )
        .await
        .unwrap();
    store
        .upsert_context(
            workspace("workspace-bravo", "Workspace Bravo"),
            project(
                "project-bravo",
                "workspace-bravo",
                "Project Bravo",
                "2026-03-29T10:00:03Z",
            ),
        )
        .await
        .unwrap();

    let projects = store.list_projects("workspace-alpha").await.unwrap();
    let ids = projects
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["project-c", "project-b", "project-a"]);
}

#[tokio::test]
async fn list_projects_returns_empty_when_workspace_has_no_projects() {
    let store = open_store().await;

    let projects = store.list_projects("workspace-empty").await.unwrap();

    assert!(projects.is_empty());
}
