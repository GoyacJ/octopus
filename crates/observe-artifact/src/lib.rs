use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

pub const ARTIFACT_TYPE_EXECUTION_OUTPUT: &str = "execution_output";
pub const TRACE_STAGE_TASK_INTAKE: &str = "task_intake";
pub const TRACE_STAGE_GOVERNANCE_EVALUATION: &str = "governance_evaluation";
pub const TRACE_STAGE_RUN_ORCHESTRATOR: &str = "run_orchestrator";
pub const TRACE_STAGE_EXECUTION_ACTION: &str = "execution_action";
pub const TRACE_STAGE_ARTIFACT_STORE: &str = "artifact_store";

pub const AUDIT_EVENT_TASK_CREATED: &str = "task_created";
pub const AUDIT_EVENT_RUN_CREATED: &str = "run_created";
pub const AUDIT_EVENT_RUN_STARTED: &str = "run_started";
pub const AUDIT_EVENT_RUN_COMPLETED: &str = "run_completed";
pub const AUDIT_EVENT_RUN_FAILED: &str = "run_failed";
pub const AUDIT_EVENT_RUN_RETRY_REQUESTED: &str = "run_retry_requested";
pub const AUDIT_EVENT_RUN_TERMINATED: &str = "run_terminated";
pub const AUDIT_EVENT_ARTIFACT_CREATED: &str = "artifact_created";
pub const AUDIT_EVENT_APPROVAL_REQUESTED: &str = "approval_requested";
pub const AUDIT_EVENT_APPROVAL_APPROVED: &str = "approval_approved";
pub const AUDIT_EVENT_APPROVAL_REJECTED: &str = "approval_rejected";
pub const AUDIT_EVENT_APPROVAL_EXPIRED: &str = "approval_expired";
pub const AUDIT_EVENT_APPROVAL_CANCELLED: &str = "approval_cancelled";
pub const AUDIT_EVENT_RUN_BLOCKED: &str = "run_blocked";
pub const AUDIT_EVENT_POLICY_DENIED: &str = "policy_denied";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InboxItemRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub approval_request_id: String,
    pub item_type: String,
    pub status: String,
    pub dedupe_key: String,
    pub title: String,
    pub message: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

impl InboxItemRecord {
    pub fn approval_request(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        approval_request_id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let approval_request_id = approval_request_id.into();
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            approval_request_id: approval_request_id.clone(),
            item_type: "approval_request".to_string(),
            status: "open".to_string(),
            dedupe_key: format!("inbox:{approval_request_id}"),
            title: title.into(),
            message: message.into(),
            created_at: now.clone(),
            updated_at: now,
            resolved_at: None,
        }
    }

    pub fn mark_resolved(&mut self) {
        let now = current_timestamp();
        self.status = "resolved".to_string();
        self.updated_at = now.clone();
        self.resolved_at = Some(now);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub approval_request_id: String,
    pub status: String,
    pub dedupe_key: String,
    pub title: String,
    pub message: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NotificationRecord {
    pub fn approval_request(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        approval_request_id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let approval_request_id = approval_request_id.into();
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            approval_request_id: approval_request_id.clone(),
            status: "delivered".to_string(),
            dedupe_key: format!("notification:{approval_request_id}"),
            title: title.into(),
            message: message.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDecisionLogRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub capability_id: String,
    pub decision: String,
    pub reason: String,
    pub estimated_cost: i64,
    pub approval_request_id: Option<String>,
    pub created_at: String,
}

impl PolicyDecisionLogRecord {
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        capability_id: impl Into<String>,
        decision: impl Into<String>,
        reason: impl Into<String>,
        estimated_cost: i64,
        approval_request_id: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            capability_id: capability_id.into(),
            decision: decision.into(),
            reason: reason.into(),
            estimated_cost,
            approval_request_id,
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
    async fn upsert_inbox_item(&self, item: &InboxItemRecord) -> Result<(), ObservationStoreError>;
    async fn list_inbox_items_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<InboxItemRecord>, ObservationStoreError>;
    async fn list_inbox_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<InboxItemRecord>, ObservationStoreError>;
    async fn upsert_notification(
        &self,
        notification: &NotificationRecord,
    ) -> Result<(), ObservationStoreError>;
    async fn list_notifications_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<NotificationRecord>, ObservationStoreError>;
    async fn list_notifications_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<NotificationRecord>, ObservationStoreError>;
    async fn insert_policy_decision(
        &self,
        decision: &PolicyDecisionLogRecord,
    ) -> Result<(), ObservationStoreError>;
    async fn list_policy_decisions_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<PolicyDecisionLogRecord>, ObservationStoreError>;
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

    fn inbox_item_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<InboxItemRecord, sqlx::Error> {
        Ok(InboxItemRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            approval_request_id: row.try_get("approval_request_id")?,
            item_type: row.try_get("item_type")?,
            status: row.try_get("status")?,
            dedupe_key: row.try_get("dedupe_key")?,
            title: row.try_get("title")?,
            message: row.try_get("message")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            resolved_at: row.try_get("resolved_at")?,
        })
    }

    fn notification_from_row(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<NotificationRecord, sqlx::Error> {
        Ok(NotificationRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            approval_request_id: row.try_get("approval_request_id")?,
            status: row.try_get("status")?,
            dedupe_key: row.try_get("dedupe_key")?,
            title: row.try_get("title")?,
            message: row.try_get("message")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn policy_decision_from_row(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<PolicyDecisionLogRecord, sqlx::Error> {
        Ok(PolicyDecisionLogRecord {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            project_id: row.try_get("project_id")?,
            run_id: row.try_get("run_id")?,
            task_id: row.try_get("task_id")?,
            capability_id: row.try_get("capability_id")?,
            decision: row.try_get("decision")?,
            reason: row.try_get("reason")?,
            estimated_cost: row.try_get("estimated_cost")?,
            approval_request_id: row.try_get("approval_request_id")?,
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

    async fn upsert_inbox_item(&self, item: &InboxItemRecord) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO inbox_items (
                id, workspace_id, project_id, run_id, approval_request_id, item_type, status,
                dedupe_key, title, message, created_at, updated_at, resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(dedupe_key) DO UPDATE SET
                status = excluded.status,
                title = excluded.title,
                message = excluded.message,
                updated_at = excluded.updated_at,
                resolved_at = excluded.resolved_at
            "#,
        )
        .bind(&item.id)
        .bind(&item.workspace_id)
        .bind(&item.project_id)
        .bind(&item.run_id)
        .bind(&item.approval_request_id)
        .bind(&item.item_type)
        .bind(&item.status)
        .bind(&item.dedupe_key)
        .bind(&item.title)
        .bind(&item.message)
        .bind(&item.created_at)
        .bind(&item.updated_at)
        .bind(&item.resolved_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_inbox_items_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<InboxItemRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, approval_request_id, item_type, status,
                   dedupe_key, title, message, created_at, updated_at, resolved_at
            FROM inbox_items
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::inbox_item_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }

    async fn list_inbox_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<InboxItemRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, approval_request_id, item_type, status,
                   dedupe_key, title, message, created_at, updated_at, resolved_at
            FROM inbox_items
            WHERE workspace_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::inbox_item_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }

    async fn upsert_notification(
        &self,
        notification: &NotificationRecord,
    ) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO notifications (
                id, workspace_id, project_id, run_id, approval_request_id, status, dedupe_key,
                title, message, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(dedupe_key) DO UPDATE SET
                status = excluded.status,
                title = excluded.title,
                message = excluded.message,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&notification.id)
        .bind(&notification.workspace_id)
        .bind(&notification.project_id)
        .bind(&notification.run_id)
        .bind(&notification.approval_request_id)
        .bind(&notification.status)
        .bind(&notification.dedupe_key)
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(&notification.created_at)
        .bind(&notification.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_notifications_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<NotificationRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, approval_request_id, status, dedupe_key,
                   title, message, created_at, updated_at
            FROM notifications
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::notification_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }

    async fn list_notifications_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<NotificationRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, approval_request_id, status, dedupe_key,
                   title, message, created_at, updated_at
            FROM notifications
            WHERE workspace_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::notification_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }

    async fn insert_policy_decision(
        &self,
        decision: &PolicyDecisionLogRecord,
    ) -> Result<(), ObservationStoreError> {
        sqlx::query(
            r#"
            INSERT INTO policy_decision_logs (
                id, workspace_id, project_id, run_id, task_id, capability_id, decision, reason,
                estimated_cost, approval_request_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&decision.id)
        .bind(&decision.workspace_id)
        .bind(&decision.project_id)
        .bind(&decision.run_id)
        .bind(&decision.task_id)
        .bind(&decision.capability_id)
        .bind(&decision.decision)
        .bind(&decision.reason)
        .bind(decision.estimated_cost)
        .bind(&decision.approval_request_id)
        .bind(&decision.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_policy_decisions_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<PolicyDecisionLogRecord>, ObservationStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, capability_id, decision, reason,
                   estimated_cost, approval_request_id, created_at
            FROM policy_decision_logs
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::policy_decision_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ObservationStoreError::from)
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
