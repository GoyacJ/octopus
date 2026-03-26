use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl WorkspaceRecord {
    pub fn new(
        id: impl Into<String>,
        slug: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            slug: slug.into(),
            display_name: display_name.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRecord {
    pub id: String,
    pub workspace_id: String,
    pub slug: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ProjectRecord {
    pub fn new(
        id: impl Into<String>,
        workspace_id: impl Into<String>,
        slug: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            workspace_id: workspace_id.into(),
            slug: slug.into(),
            display_name: display_name.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectContext {
    pub workspace: WorkspaceRecord,
    pub project: ProjectRecord,
}

#[derive(Debug, Error)]
pub enum ContextStoreError {
    #[error(
        "workspace `{workspace_id}` and project `{project_id}` do not belong to the same scope"
    )]
    WorkspaceProjectMismatch {
        workspace_id: String,
        project_id: String,
    },
    #[error("workspace `{0}` not found")]
    WorkspaceNotFound(String),
    #[error("project `{project_id}` not found in workspace `{workspace_id}`")]
    ProjectNotFound {
        workspace_id: String,
        project_id: String,
    },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[async_trait]
pub trait ContextRepository {
    async fn upsert_context(
        &self,
        workspace: WorkspaceRecord,
        project: ProjectRecord,
    ) -> Result<ProjectContext, ContextStoreError>;

    async fn fetch_project_context(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<ProjectContext, ContextStoreError>;
}

#[derive(Debug, Clone)]
pub struct SqliteContextStore {
    pool: SqlitePool,
}

impl SqliteContextStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn workspace_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<WorkspaceRecord, sqlx::Error> {
        Ok(WorkspaceRecord {
            id: row.try_get("id")?,
            slug: row.try_get("slug")?,
            display_name: row.try_get("display_name")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn project_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ProjectRecord, sqlx::Error> {
        Ok(ProjectRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            slug: row.try_get("slug")?,
            display_name: row.try_get("display_name")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[async_trait]
impl ContextRepository for SqliteContextStore {
    async fn upsert_context(
        &self,
        workspace: WorkspaceRecord,
        project: ProjectRecord,
    ) -> Result<ProjectContext, ContextStoreError> {
        if workspace.id != project.workspace_id {
            return Err(ContextStoreError::WorkspaceProjectMismatch {
                workspace_id: workspace.id,
                project_id: project.id,
            });
        }

        sqlx::query(
            r#"
            INSERT INTO workspaces (id, slug, display_name, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                slug = excluded.slug,
                display_name = excluded.display_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&workspace.id)
        .bind(&workspace.slug)
        .bind(&workspace.display_name)
        .bind(&workspace.created_at)
        .bind(&workspace.updated_at)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO projects (id, workspace_id, slug, display_name, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                workspace_id = excluded.workspace_id,
                slug = excluded.slug,
                display_name = excluded.display_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&project.id)
        .bind(&project.workspace_id)
        .bind(&project.slug)
        .bind(&project.display_name)
        .bind(&project.created_at)
        .bind(&project.updated_at)
        .execute(&self.pool)
        .await?;

        self.fetch_project_context(&workspace.id, &project.id).await
    }

    async fn fetch_project_context(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<ProjectContext, ContextStoreError> {
        let workspace_row = sqlx::query(
            "SELECT id, slug, display_name, created_at, updated_at FROM workspaces WHERE id = ?1",
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ContextStoreError::WorkspaceNotFound(workspace_id.to_string()))?;

        let project_row = sqlx::query(
            r#"
            SELECT id, workspace_id, slug, display_name, created_at, updated_at
            FROM projects
            WHERE id = ?1 AND workspace_id = ?2
            "#,
        )
        .bind(project_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ContextStoreError::ProjectNotFound {
            workspace_id: workspace_id.to_string(),
            project_id: project_id.to_string(),
        })?;

        Ok(ProjectContext {
            workspace: Self::workspace_from_row(&workspace_row)?,
            project: Self::project_from_row(&project_row)?,
        })
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
