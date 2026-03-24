//! SQLite-backed event log and projection store for the Phase 3 MVP slice.

use std::str::FromStr;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use octopus_application::{
    AuditEventRecord, EventEnvelope, InboxItemRecord, Phase3Store, ResumeReceipt, RunContext,
    RunCreationBundle, RunRecord, RunResumeBundle, TimelineEventRecord,
};
use sqlx::{
    sqlite::{SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};

#[derive(Clone)]
pub struct SqlitePhase3Store {
    pool: SqlitePool,
}

impl SqlitePhase3Store {
    pub async fn connect(connection_string: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(connection_string)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS event_envelopes (
              id TEXT PRIMARY KEY,
              run_id TEXT NOT NULL,
              object_type TEXT NOT NULL,
              event_type TEXT NOT NULL,
              actor_id TEXT NOT NULL,
              surface TEXT NOT NULL,
              resume_token TEXT,
              idempotency_key TEXT,
              risk_level TEXT NOT NULL,
              budget_context TEXT NOT NULL,
              summary TEXT NOT NULL,
              occurred_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS runs (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              interaction_type TEXT NOT NULL,
              status TEXT NOT NULL,
              summary TEXT NOT NULL,
              input TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS inbox_items (
              id TEXT PRIMARY KEY,
              run_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              status TEXT NOT NULL,
              title TEXT NOT NULL,
              prompt TEXT NOT NULL,
              response_type TEXT NOT NULL,
              options_json TEXT NOT NULL,
              resume_token TEXT NOT NULL,
              created_at TEXT NOT NULL,
              resolved_at TEXT,
              FOREIGN KEY(run_id) REFERENCES runs(id)
            );

            CREATE TABLE IF NOT EXISTS timeline_events (
              id TEXT PRIMARY KEY,
              run_id TEXT NOT NULL,
              event_type TEXT NOT NULL,
              summary TEXT NOT NULL,
              occurred_at TEXT NOT NULL,
              FOREIGN KEY(run_id) REFERENCES runs(id)
            );

            CREATE TABLE IF NOT EXISTS audit_events (
              id TEXT PRIMARY KEY,
              actor_id TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              action TEXT NOT NULL,
              summary TEXT NOT NULL,
              occurred_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS resume_receipts (
              run_id TEXT NOT NULL,
              idempotency_key TEXT NOT NULL,
              final_status TEXT NOT NULL,
              recorded_at TEXT NOT NULL,
              PRIMARY KEY(run_id, idempotency_key),
              FOREIGN KEY(run_id) REFERENCES runs(id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Phase3Store for SqlitePhase3Store {
    async fn create_run(&self, bundle: RunCreationBundle) -> Result<RunContext> {
        let mut tx = self.pool.begin().await?;

        insert_run(&mut tx, &bundle.run).await?;
        insert_inbox_item(&mut tx, &bundle.inbox_item).await?;
        insert_event_envelopes(&mut tx, &bundle.event_envelopes).await?;
        insert_timeline_events(&mut tx, &bundle.timeline_events).await?;
        insert_audit_events(&mut tx, &bundle.audit_events).await?;

        tx.commit().await?;

        Ok(RunContext {
            run: bundle.run,
            pending_inbox_item: Some(bundle.inbox_item),
        })
    }

    async fn get_run_context(&self, run_id: &str) -> Result<Option<RunContext>> {
        let run = self.get_run(run_id).await?;
        let Some(run) = run else {
            return Ok(None);
        };

        let pending = sqlx::query(
            r#"
            SELECT *
            FROM inbox_items
            WHERE run_id = ? AND status = 'pending'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .map(parse_inbox_item)
        .transpose()?;

        Ok(Some(RunContext {
            run,
            pending_inbox_item: pending,
        }))
    }

    async fn list_runs(&self) -> Result<Vec<RunRecord>> {
        let rows = sqlx::query("SELECT * FROM runs ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(parse_run).collect()
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>> {
        sqlx::query("SELECT * FROM runs WHERE id = ?")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await?
            .map(parse_run)
            .transpose()
    }

    async fn list_run_timeline(&self, run_id: &str) -> Result<Vec<TimelineEventRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM timeline_events WHERE run_id = ? ORDER BY occurred_at ASC, id ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(parse_timeline_event).collect()
    }

    async fn list_inbox_items(&self) -> Result<Vec<InboxItemRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM inbox_items ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(parse_inbox_item).collect()
    }

    async fn list_audit_events(&self) -> Result<Vec<AuditEventRecord>> {
        let rows = sqlx::query("SELECT * FROM audit_events ORDER BY occurred_at DESC, id DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(parse_audit_event).collect()
    }

    async fn append_audit_events(&self, events: &[AuditEventRecord]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        insert_audit_events(&mut tx, events).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn find_resume_receipt(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ResumeReceipt>> {
        sqlx::query(
            "SELECT * FROM resume_receipts WHERE run_id = ? AND idempotency_key = ? LIMIT 1",
        )
        .bind(run_id)
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?
        .map(parse_resume_receipt)
        .transpose()
    }

    async fn apply_resume(&self, bundle: RunResumeBundle) -> Result<RunContext> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE runs
            SET status = ?, summary = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(bundle.run.status.as_str())
        .bind(&bundle.run.summary)
        .bind(&bundle.run.updated_at)
        .bind(&bundle.run.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE inbox_items
            SET status = ?, resolved_at = ?
            WHERE id = ?
            "#,
        )
        .bind(bundle.inbox_item.status.as_str())
        .bind(&bundle.inbox_item.resolved_at)
        .bind(&bundle.inbox_item.id)
        .execute(&mut *tx)
        .await?;

        insert_event_envelopes(&mut tx, &bundle.event_envelopes).await?;
        insert_timeline_events(&mut tx, &bundle.timeline_events).await?;
        insert_audit_events(&mut tx, &bundle.audit_events).await?;

        sqlx::query(
            r#"
            INSERT INTO resume_receipts (run_id, idempotency_key, final_status, recorded_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&bundle.receipt.run_id)
        .bind(&bundle.receipt.idempotency_key)
        .bind(bundle.receipt.final_status.as_str())
        .bind(&bundle.receipt.recorded_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(RunContext {
            run: bundle.run,
            pending_inbox_item: None,
        })
    }
}

async fn insert_run(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, run: &RunRecord) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO runs (
          id, workspace_id, agent_id, interaction_type, status, summary, input, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&run.id)
    .bind(&run.workspace_id)
    .bind(&run.agent_id)
    .bind(run.interaction_type.as_str())
    .bind(run.status.as_str())
    .bind(&run.summary)
    .bind(&run.input)
    .bind(&run.created_at)
    .bind(&run.updated_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_inbox_item(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    item: &InboxItemRecord,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO inbox_items (
          id, run_id, kind, status, title, prompt, response_type, options_json, resume_token, created_at, resolved_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&item.id)
    .bind(&item.run_id)
    .bind(item.kind.as_str())
    .bind(item.status.as_str())
    .bind(&item.title)
    .bind(&item.prompt)
    .bind(item.response_type.as_str())
    .bind(serde_json::to_string(&item.options)?)
    .bind(&item.resume_token)
    .bind(&item.created_at)
    .bind(&item.resolved_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_event_envelopes(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    events: &[EventEnvelope],
) -> Result<()> {
    for event in events {
        sqlx::query(
            r#"
            INSERT INTO event_envelopes (
              id, run_id, object_type, event_type, actor_id, surface, resume_token,
              idempotency_key, risk_level, budget_context, summary, occurred_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.id)
        .bind(&event.run_id)
        .bind(&event.object_type)
        .bind(&event.event_type)
        .bind(&event.actor_id)
        .bind(&event.surface)
        .bind(&event.resume_token)
        .bind(&event.idempotency_key)
        .bind(&event.risk_level)
        .bind(&event.budget_context)
        .bind(&event.summary)
        .bind(&event.occurred_at)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn insert_timeline_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    events: &[TimelineEventRecord],
) -> Result<()> {
    for event in events {
        sqlx::query(
            r#"
            INSERT INTO timeline_events (id, run_id, event_type, summary, occurred_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.id)
        .bind(&event.run_id)
        .bind(&event.event_type)
        .bind(&event.summary)
        .bind(&event.occurred_at)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn insert_audit_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    events: &[AuditEventRecord],
) -> Result<()> {
    for event in events {
        sqlx::query(
            r#"
            INSERT INTO audit_events (id, actor_id, subject_type, subject_id, action, summary, occurred_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.id)
        .bind(&event.actor_id)
        .bind(&event.subject_type)
        .bind(&event.subject_id)
        .bind(&event.action)
        .bind(&event.summary)
        .bind(&event.occurred_at)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

fn parse_run(row: SqliteRow) -> Result<RunRecord> {
    Ok(RunRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        agent_id: row.try_get("agent_id")?,
        interaction_type: parse_enum(&row, "interaction_type")?,
        status: parse_enum(&row, "status")?,
        summary: row.try_get("summary")?,
        input: row.try_get("input")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_inbox_item(row: SqliteRow) -> Result<InboxItemRecord> {
    Ok(InboxItemRecord {
        id: row.try_get("id")?,
        run_id: row.try_get("run_id")?,
        kind: parse_enum(&row, "kind")?,
        status: parse_enum(&row, "status")?,
        title: row.try_get("title")?,
        prompt: row.try_get("prompt")?,
        response_type: parse_enum(&row, "response_type")?,
        options: serde_json::from_str(&row.try_get::<String, _>("options_json")?)?,
        resume_token: row.try_get("resume_token")?,
        created_at: row.try_get("created_at")?,
        resolved_at: row.try_get("resolved_at")?,
    })
}

fn parse_timeline_event(row: SqliteRow) -> Result<TimelineEventRecord> {
    Ok(TimelineEventRecord {
        id: row.try_get("id")?,
        run_id: row.try_get("run_id")?,
        event_type: row.try_get("event_type")?,
        summary: row.try_get("summary")?,
        occurred_at: row.try_get("occurred_at")?,
    })
}

fn parse_audit_event(row: SqliteRow) -> Result<AuditEventRecord> {
    Ok(AuditEventRecord {
        id: row.try_get("id")?,
        actor_id: row.try_get("actor_id")?,
        subject_type: row.try_get("subject_type")?,
        subject_id: row.try_get("subject_id")?,
        action: row.try_get("action")?,
        summary: row.try_get("summary")?,
        occurred_at: row.try_get("occurred_at")?,
    })
}

fn parse_resume_receipt(row: SqliteRow) -> Result<ResumeReceipt> {
    Ok(ResumeReceipt {
        run_id: row.try_get("run_id")?,
        idempotency_key: row.try_get("idempotency_key")?,
        final_status: parse_enum(&row, "final_status")?,
        recorded_at: row.try_get("recorded_at")?,
    })
}

fn parse_enum<T>(row: &SqliteRow, column: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let raw: String = row.try_get(column)?;
    raw.parse::<T>()
        .map_err(|err| anyhow!("failed to parse {column}={raw}: {err}"))
}
