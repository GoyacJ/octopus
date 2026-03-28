use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeSpaceRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub owner_ref: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl KnowledgeSpaceRecord {
    pub fn project_scope(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        display_name: impl Into<String>,
        owner_ref: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: Some(project_id.into()),
            owner_ref: owner_ref.into(),
            display_name: display_name.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeCandidateRecord {
    pub id: String,
    pub knowledge_space_id: String,
    pub source_run_id: String,
    pub source_task_id: String,
    pub source_artifact_id: String,
    pub capability_id: String,
    pub status: String,
    pub content: String,
    pub provenance_source: String,
    pub source_trust_level: String,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
}

impl KnowledgeCandidateRecord {
    pub fn new(
        knowledge_space_id: impl Into<String>,
        source_run_id: impl Into<String>,
        source_task_id: impl Into<String>,
        source_artifact_id: impl Into<String>,
        capability_id: impl Into<String>,
        content: impl Into<String>,
        provenance_source: impl Into<String>,
        source_trust_level: impl Into<String>,
        dedupe_key: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            knowledge_space_id: knowledge_space_id.into(),
            source_run_id: source_run_id.into(),
            source_task_id: source_task_id.into(),
            source_artifact_id: source_artifact_id.into(),
            capability_id: capability_id.into(),
            status: "candidate".to_string(),
            content: content.into(),
            provenance_source: provenance_source.into(),
            source_trust_level: source_trust_level.into(),
            dedupe_key: dedupe_key.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeAssetRecord {
    pub id: String,
    pub knowledge_space_id: String,
    pub source_candidate_id: String,
    pub capability_id: String,
    pub status: String,
    pub content: String,
    pub trust_level: String,
    pub created_at: String,
    pub updated_at: String,
}

impl KnowledgeAssetRecord {
    pub fn from_candidate(candidate: &KnowledgeCandidateRecord) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            knowledge_space_id: candidate.knowledge_space_id.clone(),
            source_candidate_id: candidate.id.clone(),
            capability_id: candidate.capability_id.clone(),
            status: "verified_shared".to_string(),
            content: candidate.content.clone(),
            trust_level: "verified".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeCaptureRetryRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub artifact_id: String,
    pub capability_id: String,
    pub last_error: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

impl KnowledgeCaptureRetryRecord {
    pub fn pending(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        artifact_id: impl Into<String>,
        capability_id: impl Into<String>,
        last_error: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            artifact_id: artifact_id.into(),
            capability_id: capability_id.into(),
            last_error: last_error.into(),
            status: "pending".to_string(),
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

#[derive(Debug, Error)]
pub enum KnowledgeStoreError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct SqliteKnowledgeStore {
    pool: SqlitePool,
}

impl SqliteKnowledgeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn ensure_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
        display_name: &str,
        owner_ref: &str,
    ) -> Result<KnowledgeSpaceRecord, KnowledgeStoreError> {
        let record =
            KnowledgeSpaceRecord::project_scope(workspace_id, project_id, display_name, owner_ref);
        sqlx::query(
            r#"
            INSERT INTO knowledge_spaces (
                id, workspace_id, project_id, owner_ref, display_name, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(workspace_id, project_id) DO UPDATE SET
                owner_ref = excluded.owner_ref,
                display_name = excluded.display_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.workspace_id)
        .bind(&record.project_id)
        .bind(&record.owner_ref)
        .bind(&record.display_name)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(self
            .fetch_project_knowledge_space(workspace_id, project_id)
            .await?
            .expect("knowledge space should exist after upsert"))
    }

    pub async fn fetch_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<KnowledgeSpaceRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, owner_ref, display_name, created_at, updated_at
            FROM knowledge_spaces
            WHERE workspace_id = ?1 AND project_id = ?2
            "#,
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_space_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn fetch_knowledge_space(
        &self,
        knowledge_space_id: &str,
    ) -> Result<Option<KnowledgeSpaceRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, owner_ref, display_name, created_at, updated_at
            FROM knowledge_spaces
            WHERE id = ?1
            "#,
        )
        .bind(knowledge_space_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_space_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn create_knowledge_candidate(
        &self,
        candidate: &KnowledgeCandidateRecord,
    ) -> Result<(), KnowledgeStoreError> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_candidates (
                id, knowledge_space_id, source_run_id, source_task_id, source_artifact_id,
                capability_id, status, content, provenance_source, source_trust_level,
                dedupe_key, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(&candidate.id)
        .bind(&candidate.knowledge_space_id)
        .bind(&candidate.source_run_id)
        .bind(&candidate.source_task_id)
        .bind(&candidate.source_artifact_id)
        .bind(&candidate.capability_id)
        .bind(&candidate.status)
        .bind(&candidate.content)
        .bind(&candidate.provenance_source)
        .bind(&candidate.source_trust_level)
        .bind(&candidate.dedupe_key)
        .bind(&candidate.created_at)
        .bind(&candidate.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_knowledge_candidate_by_dedupe_key(
        &self,
        dedupe_key: &str,
    ) -> Result<Option<KnowledgeCandidateRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_run_id, source_task_id, source_artifact_id,
                   capability_id, status, content, provenance_source, source_trust_level,
                   dedupe_key, created_at, updated_at
            FROM knowledge_candidates
            WHERE dedupe_key = ?1
            "#,
        )
        .bind(dedupe_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_candidate_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn fetch_knowledge_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<KnowledgeCandidateRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_run_id, source_task_id, source_artifact_id,
                   capability_id, status, content, provenance_source, source_trust_level,
                   dedupe_key, created_at, updated_at
            FROM knowledge_candidates
            WHERE id = ?1
            "#,
        )
        .bind(candidate_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_candidate_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn update_knowledge_candidate_status(
        &self,
        candidate_id: &str,
        status: &str,
    ) -> Result<(), KnowledgeStoreError> {
        sqlx::query(
            r#"
            UPDATE knowledge_candidates
            SET status = ?2, updated_at = ?3
            WHERE id = ?1
            "#,
        )
        .bind(candidate_id)
        .bind(status)
        .bind(current_timestamp())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_knowledge_candidates_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeCandidateRecord>, KnowledgeStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_run_id, source_task_id, source_artifact_id,
                   capability_id, status, content, provenance_source, source_trust_level,
                   dedupe_key, created_at, updated_at
            FROM knowledge_candidates
            WHERE source_run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(knowledge_candidate_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn list_knowledge_candidates_by_space(
        &self,
        knowledge_space_id: &str,
    ) -> Result<Vec<KnowledgeCandidateRecord>, KnowledgeStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_run_id, source_task_id, source_artifact_id,
                   capability_id, status, content, provenance_source, source_trust_level,
                   dedupe_key, created_at, updated_at
            FROM knowledge_candidates
            WHERE knowledge_space_id = ?1
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(knowledge_space_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(knowledge_candidate_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn upsert_knowledge_asset(
        &self,
        asset: &KnowledgeAssetRecord,
    ) -> Result<(), KnowledgeStoreError> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_assets (
                id, knowledge_space_id, source_candidate_id, capability_id, status, content,
                trust_level, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(source_candidate_id) DO UPDATE SET
                capability_id = excluded.capability_id,
                status = excluded.status,
                content = excluded.content,
                trust_level = excluded.trust_level,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&asset.id)
        .bind(&asset.knowledge_space_id)
        .bind(&asset.source_candidate_id)
        .bind(&asset.capability_id)
        .bind(&asset.status)
        .bind(&asset.content)
        .bind(&asset.trust_level)
        .bind(&asset.created_at)
        .bind(&asset.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_knowledge_asset(
        &self,
        asset_id: &str,
    ) -> Result<Option<KnowledgeAssetRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_candidate_id, capability_id, status, content,
                   trust_level, created_at, updated_at
            FROM knowledge_assets
            WHERE id = ?1
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_asset_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn fetch_knowledge_asset_by_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<KnowledgeAssetRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_candidate_id, capability_id, status, content,
                   trust_level, created_at, updated_at
            FROM knowledge_assets
            WHERE source_candidate_id = ?1
            "#,
        )
        .bind(candidate_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_asset_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn list_shared_assets_for_project_capability(
        &self,
        workspace_id: &str,
        project_id: &str,
        capability_id: &str,
    ) -> Result<Vec<KnowledgeAssetRecord>, KnowledgeStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT a.id, a.knowledge_space_id, a.source_candidate_id, a.capability_id, a.status,
                   a.content, a.trust_level, a.created_at, a.updated_at
            FROM knowledge_assets a
            INNER JOIN knowledge_spaces s ON s.id = a.knowledge_space_id
            WHERE s.workspace_id = ?1
              AND s.project_id = ?2
              AND a.capability_id = ?3
              AND a.status = 'verified_shared'
            ORDER BY a.created_at, a.id
            "#,
        )
        .bind(workspace_id)
        .bind(project_id)
        .bind(capability_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(knowledge_asset_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn list_knowledge_assets_by_space(
        &self,
        knowledge_space_id: &str,
    ) -> Result<Vec<KnowledgeAssetRecord>, KnowledgeStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, knowledge_space_id, source_candidate_id, capability_id, status, content,
                   trust_level, created_at, updated_at
            FROM knowledge_assets
            WHERE knowledge_space_id = ?1
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(knowledge_space_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(knowledge_asset_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn upsert_capture_retry(
        &self,
        retry: &KnowledgeCaptureRetryRecord,
    ) -> Result<(), KnowledgeStoreError> {
        sqlx::query(
            r#"
            INSERT INTO knowledge_capture_retries (
                id, workspace_id, project_id, run_id, task_id, artifact_id, capability_id,
                last_error, status, created_at, updated_at, resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(run_id) DO UPDATE SET
                artifact_id = excluded.artifact_id,
                capability_id = excluded.capability_id,
                last_error = excluded.last_error,
                status = excluded.status,
                updated_at = excluded.updated_at,
                resolved_at = excluded.resolved_at
            "#,
        )
        .bind(&retry.id)
        .bind(&retry.workspace_id)
        .bind(&retry.project_id)
        .bind(&retry.run_id)
        .bind(&retry.task_id)
        .bind(&retry.artifact_id)
        .bind(&retry.capability_id)
        .bind(&retry.last_error)
        .bind(&retry.status)
        .bind(&retry.created_at)
        .bind(&retry.updated_at)
        .bind(&retry.resolved_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_capture_retry_by_run(
        &self,
        run_id: &str,
    ) -> Result<Option<KnowledgeCaptureRetryRecord>, KnowledgeStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, artifact_id, capability_id,
                   last_error, status, created_at, updated_at, resolved_at
            FROM knowledge_capture_retries
            WHERE run_id = ?1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| knowledge_capture_retry_from_row(&row))
            .transpose()
            .map_err(KnowledgeStoreError::from)
    }

    pub async fn resolve_capture_retry(&self, run_id: &str) -> Result<(), KnowledgeStoreError> {
        if let Some(mut retry) = self.fetch_capture_retry_by_run(run_id).await? {
            if retry.status != "resolved" {
                retry.mark_resolved();
                sqlx::query(
                    r#"
                    UPDATE knowledge_capture_retries
                    SET status = ?2, updated_at = ?3, resolved_at = ?4
                    WHERE run_id = ?1
                    "#,
                )
                .bind(run_id)
                .bind(&retry.status)
                .bind(&retry.updated_at)
                .bind(&retry.resolved_at)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }
}

fn knowledge_space_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<KnowledgeSpaceRecord, sqlx::Error> {
    Ok(KnowledgeSpaceRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        owner_ref: row.try_get("owner_ref")?,
        display_name: row.try_get("display_name")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn knowledge_candidate_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<KnowledgeCandidateRecord, sqlx::Error> {
    Ok(KnowledgeCandidateRecord {
        id: row.try_get("id")?,
        knowledge_space_id: row.try_get("knowledge_space_id")?,
        source_run_id: row.try_get("source_run_id")?,
        source_task_id: row.try_get("source_task_id")?,
        source_artifact_id: row.try_get("source_artifact_id")?,
        capability_id: row.try_get("capability_id")?,
        status: row.try_get("status")?,
        content: row.try_get("content")?,
        provenance_source: row.try_get("provenance_source")?,
        source_trust_level: row.try_get("source_trust_level")?,
        dedupe_key: row.try_get("dedupe_key")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn knowledge_asset_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<KnowledgeAssetRecord, sqlx::Error> {
    Ok(KnowledgeAssetRecord {
        id: row.try_get("id")?,
        knowledge_space_id: row.try_get("knowledge_space_id")?,
        source_candidate_id: row.try_get("source_candidate_id")?,
        capability_id: row.try_get("capability_id")?,
        status: row.try_get("status")?,
        content: row.try_get("content")?,
        trust_level: row.try_get("trust_level")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn knowledge_capture_retry_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<KnowledgeCaptureRetryRecord, sqlx::Error> {
    Ok(KnowledgeCaptureRetryRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        run_id: row.try_get("run_id")?,
        task_id: row.try_get("task_id")?,
        artifact_id: row.try_get("artifact_id")?,
        capability_id: row.try_get("capability_id")?,
        last_error: row.try_get("last_error")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        resolved_at: row.try_get("resolved_at")?,
    })
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
