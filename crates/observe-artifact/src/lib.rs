use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

pub const ARTIFACT_TYPE_EXECUTION_OUTPUT: &str = "execution_output";
pub const TRACE_STAGE_RUN_ORCHESTRATOR: &str = "run_orchestrator";
pub const TRACE_STAGE_EXECUTION_ACTION: &str = "execution_action";
pub const TRACE_STAGE_ARTIFACT_STORE: &str = "artifact_store";

pub const AUDIT_EVENT_RUN_CREATED: &str = "run_created";
pub const AUDIT_EVENT_RUN_STARTED: &str = "run_started";
pub const AUDIT_EVENT_RUN_COMPLETED: &str = "run_completed";
pub const AUDIT_EVENT_RUN_FAILED: &str = "run_failed";
pub const AUDIT_EVENT_RUN_RETRY_REQUESTED: &str = "run_retry_requested";
pub const AUDIT_EVENT_RUN_TERMINATED: &str = "run_terminated";
pub const AUDIT_EVENT_ARTIFACT_CREATED: &str = "artifact_created";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub artifact_type: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ArtifactRecord {
    pub fn execution_output(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            artifact_type: ARTIFACT_TYPE_EXECUTION_OUTPUT.to_string(),
            content: content.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub event_type: String,
    pub message: String,
    pub created_at: String,
}

impl AuditRecord {
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        event_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            event_type: event_type.into(),
            message: message.into(),
            created_at: current_timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub stage: String,
    pub attempt: i64,
    pub message: String,
    pub created_at: String,
}

impl TraceRecord {
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        stage: impl Into<String>,
        attempt: i64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            stage: stage.into(),
            attempt,
            message: message.into(),
            created_at: current_timestamp(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ObservationStoreError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[async_trait]
pub trait ArtifactStore {
    async fn insert_artifact(&self, artifact: &ArtifactRecord)
        -> Result<(), ObservationStoreError>;
    async fn list_artifacts_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ArtifactRecord>, ObservationStoreError>;
}

#[async_trait]
pub trait ObservationWriter {
    async fn write_audit(&self, audit: &AuditRecord) -> Result<(), ObservationStoreError>;
    async fn write_trace(&self, trace: &TraceRecord) -> Result<(), ObservationStoreError>;
    async fn list_audits_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<AuditRecord>, ObservationStoreError>;
    async fn list_traces_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<TraceRecord>, ObservationStoreError>;
}

#[derive(Debug, Clone)]
pub struct SqliteObservationStore {
    pool: SqlitePool,
}

impl SqliteObservationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn artifact_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ArtifactRecord, sqlx::Error> {
        Ok(ArtifactRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            task_id: row.try_get("task_id")?,
            artifact_type: row.try_get("artifact_type")?,
            content: row.try_get("content")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn audit_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AuditRecord, sqlx::Error> {
        Ok(AuditRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            task_id: row.try_get("task_id")?,
            event_type: row.try_get("event_type")?,
            message: row.try_get("message")?,
            created_at: row.try_get("created_at")?,
        })
    }

    fn trace_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<TraceRecord, sqlx::Error> {
        Ok(TraceRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            task_id: row.try_get("task_id")?,
            stage: row.try_get("stage")?,
            attempt: row.try_get("attempt")?,
            message: row.try_get("message")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[async_trait]
impl ArtifactStore for SqliteObservationStore {
    async fn insert_artifact(
        &self,
        artifact: &ArtifactRecord,
    ) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO artifacts (
                id, workspace_id, project_id, run_id, task_id, artifact_type, content, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(&artifact.id)
        .bind(&artifact.workspace_id)
        .bind(&artifact.project_id)
        .bind(&artifact.run_id)
        .bind(&artifact.task_id)
        .bind(&artifact.artifact_type)
        .bind(&artifact.content)
        .bind(&artifact.created_at)
        .bind(&artifact.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_artifacts_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ArtifactRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, artifact_type, content, created_at, updated_at
            FROM artifacts
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::artifact_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }
}

#[async_trait]
impl ObservationWriter for SqliteObservationStore {
    async fn write_audit(&self, audit: &AuditRecord) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO audit_records (
                id, workspace_id, project_id, run_id, task_id, event_type, message, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(&audit.id)
        .bind(&audit.workspace_id)
        .bind(&audit.project_id)
        .bind(&audit.run_id)
        .bind(&audit.task_id)
        .bind(&audit.event_type)
        .bind(&audit.message)
        .bind(&audit.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn write_trace(&self, trace: &TraceRecord) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO trace_records (
                id, workspace_id, project_id, run_id, task_id, stage, attempt, message, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(&trace.id)
        .bind(&trace.workspace_id)
        .bind(&trace.project_id)
        .bind(&trace.run_id)
        .bind(&trace.task_id)
        .bind(&trace.stage)
        .bind(trace.attempt)
        .bind(&trace.message)
        .bind(&trace.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_audits_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<AuditRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, event_type, message, created_at
            FROM audit_records
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::audit_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }

    async fn list_traces_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<TraceRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, stage, attempt, message, created_at
            FROM trace_records
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::trace_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
