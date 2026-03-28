use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;

use octopus_domain_context::{
    ContextRepository, ProjectContext, ProjectRecord, SqliteContextStore, WorkspaceRecord,
};
use octopus_execution::{ExecutionAction, ExecutionEngine, ExecutionOutcome};
use octopus_governance::{
    ApprovalDecision, ApprovalRequestRecord, BudgetPolicyRecord, CapabilityBindingRecord,
    CapabilityDescriptorRecord, CapabilityGrantRecord, GovernanceDecision, SqliteGovernanceStore,
    TaskGovernanceInput, APPROVAL_TYPE_EXECUTION, APPROVAL_TYPE_KNOWLEDGE_PROMOTION,
    DECISION_ALLOW, DECISION_DENY, DECISION_REQUIRE_APPROVAL,
};
use octopus_interop_mcp::{
    EnvironmentLeaseRecord, GatewayExecutionOutcome, GatewayRequest, McpCredentialRecord,
    McpGateway, McpInvocationRecord, McpServerRecord, SqliteInteropStore, KNOWLEDGE_GATE_ELIGIBLE,
};
use octopus_knowledge::{
    KnowledgeAssetRecord, KnowledgeCandidateRecord, KnowledgeCaptureRetryRecord,
    KnowledgeSpaceRecord, SqliteKnowledgeStore,
};
use octopus_observe_artifact::{
    ArtifactRecord, ArtifactStore, AuditRecord, InboxItemRecord, KnowledgeLineageRecord,
    NotificationRecord, ObservationWriter, PolicyDecisionLogRecord, SqliteObservationStore,
    TraceRecord, AUDIT_EVENT_APPROVAL_APPROVED, AUDIT_EVENT_APPROVAL_CANCELLED,
    AUDIT_EVENT_APPROVAL_EXPIRED, AUDIT_EVENT_APPROVAL_REJECTED, AUDIT_EVENT_APPROVAL_REQUESTED,
    AUDIT_EVENT_ARTIFACT_CREATED, AUDIT_EVENT_KNOWLEDGE_ASSET_PROMOTED,
    AUDIT_EVENT_KNOWLEDGE_CANDIDATE_CREATED, AUDIT_EVENT_KNOWLEDGE_CAPTURE_FAILED,
    AUDIT_EVENT_KNOWLEDGE_CAPTURE_GATED, AUDIT_EVENT_KNOWLEDGE_CAPTURE_RETRIED,
    AUDIT_EVENT_KNOWLEDGE_RECALLED, AUDIT_EVENT_POLICY_DENIED, AUDIT_EVENT_RUN_BLOCKED,
    AUDIT_EVENT_RUN_COMPLETED, AUDIT_EVENT_RUN_CREATED, AUDIT_EVENT_RUN_FAILED,
    AUDIT_EVENT_RUN_RETRY_REQUESTED, AUDIT_EVENT_RUN_STARTED, AUDIT_EVENT_RUN_TERMINATED,
    TRACE_STAGE_ARTIFACT_STORE, TRACE_STAGE_EXECUTION_ACTION, TRACE_STAGE_GOVERNANCE_EVALUATION,
    TRACE_STAGE_KNOWLEDGE_CAPTURE, TRACE_STAGE_KNOWLEDGE_PROMOTION, TRACE_STAGE_KNOWLEDGE_RECALL,
    TRACE_STAGE_RUN_ORCHESTRATOR, TRACE_STAGE_TRIGGER_DELIVERY,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::{
    models::{
        current_timestamp, AutomationDetailRecord, AutomationRecord, AutomationSummaryRecord,
        CreateAutomationInput, CreateAutomationReport, CreateTaskInput, CreateTriggerInput,
        CronTriggerConfig, KnowledgePromotionReport, McpEventTriggerConfig, RunExecutionReport,
        RunRecord, RunSummaryRecord, TaskRecord, TriggerDeliveryRecord, TriggerRecord,
        TriggerSpec, WebhookTriggerConfig,
    },
    RuntimeError,
};

#[derive(Debug, Clone)]
pub struct TaskIntake {
    pool: SqlitePool,
    context_store: SqliteContextStore,
}

impl TaskIntake {
    pub fn new(pool: SqlitePool, context_store: SqliteContextStore) -> Self {
        Self {
            pool,
            context_store,
        }
    }

    pub async fn ensure_project_context(
        &self,
        workspace_id: &str,
        workspace_slug: &str,
        workspace_display_name: &str,
        project_id: &str,
        project_slug: &str,
        project_display_name: &str,
    ) -> Result<ProjectContext, RuntimeError> {
        let workspace = WorkspaceRecord::new(workspace_id, workspace_slug, workspace_display_name);
        let project =
            ProjectRecord::new(project_id, workspace_id, project_slug, project_display_name);
        Ok(self
            .context_store
            .upsert_context(workspace, project)
            .await?)
    }

    pub async fn create_task(&self, input: CreateTaskInput) -> Result<TaskRecord, RuntimeError> {
        self.context_store
            .fetch_project_context(&input.workspace_id, &input.project_id)
            .await?;

        if let Some(existing) = self
            .find_task_by_idempotency_key(&input.idempotency_key)
            .await?
        {
            return Ok(existing);
        }

        let task = TaskRecord::new(input);
        let action_json = serde_json::to_string(&task.action)?;

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, workspace_id, project_id, source_kind, automation_id, title, instruction,
                action_json, capability_id, estimated_cost, idempotency_key, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(&task.id)
        .bind(&task.workspace_id)
        .bind(&task.project_id)
        .bind(&task.source_kind)
        .bind(&task.automation_id)
        .bind(&task.title)
        .bind(&task.instruction)
        .bind(action_json)
        .bind(&task.capability_id)
        .bind(task.estimated_cost)
        .bind(&task.idempotency_key)
        .bind(&task.created_at)
        .bind(&task.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn fetch_project_context(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<ProjectContext, RuntimeError> {
        Ok(self
            .context_store
            .fetch_project_context(workspace_id, project_id)
            .await?)
    }

    pub async fn fetch_task(&self, task_id: &str) -> Result<TaskRecord, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, source_kind, automation_id, title, instruction,
                   action_json, capability_id, estimated_cost, idempotency_key, created_at, updated_at
            FROM tasks
            WHERE id = ?1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

        task_from_row(&row)
    }

    async fn find_task_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<TaskRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, source_kind, automation_id, title, instruction,
                   action_json, capability_id, estimated_cost, idempotency_key, created_at, updated_at
            FROM tasks
            WHERE idempotency_key = ?1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| task_from_row(&row)).transpose()
    }
}

#[derive(Debug, Clone)]
pub struct AutomationIntake {
    pool: SqlitePool,
    context_store: SqliteContextStore,
}

impl AutomationIntake {
    pub fn new(pool: SqlitePool, context_store: SqliteContextStore) -> Self {
        Self {
            pool,
            context_store,
        }
    }

    pub async fn create_automation_with_trigger(
        &self,
        input: CreateAutomationInput,
        trigger_input: CreateTriggerInput,
    ) -> Result<CreateAutomationReport, RuntimeError> {
        self.context_store
            .fetch_project_context(&input.workspace_id, &input.project_id)
            .await?;

        let (trigger, webhook_secret) = self.build_trigger("pending", trigger_input);
        let automation = AutomationRecord::new(input, trigger.id.clone());
        let trigger = TriggerRecord {
            automation_id: automation.id.clone(),
            ..trigger
        };

        sqlx::query(
            r#"
            INSERT INTO automations (
                id, workspace_id, project_id, trigger_id, status, title, instruction,
                action_json, capability_id, estimated_cost, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&automation.id)
        .bind(&automation.workspace_id)
        .bind(&automation.project_id)
        .bind(&automation.trigger_id)
        .bind(&automation.status)
        .bind(&automation.title)
        .bind(&automation.instruction)
        .bind(serde_json::to_string(&automation.action)?)
        .bind(&automation.capability_id)
        .bind(automation.estimated_cost)
        .bind(&automation.created_at)
        .bind(&automation.updated_at)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO triggers (
                id, automation_id, trigger_type, schedule, timezone, next_fire_at,
                ingress_mode, secret_header_name, secret_hint, webhook_secret_hash,
                server_id, event_name, event_pattern, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind(&trigger.id)
        .bind(&trigger.automation_id)
        .bind(trigger.trigger_type())
        .bind(trigger_schedule(&trigger))
        .bind(trigger_timezone(&trigger))
        .bind(trigger_next_fire_at(&trigger))
        .bind(trigger_ingress_mode(&trigger))
        .bind(trigger_secret_header_name(&trigger))
        .bind(trigger_secret_hint(&trigger))
        .bind(trigger_secret_hash(&trigger))
        .bind(trigger_server_id(&trigger))
        .bind(trigger_event_name(&trigger))
        .bind(trigger_event_pattern(&trigger))
        .bind(&trigger.created_at)
        .bind(&trigger.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(CreateAutomationReport {
            automation,
            trigger,
            webhook_secret,
        })
    }

    fn build_trigger(
        &self,
        automation_id: impl Into<String>,
        trigger_input: CreateTriggerInput,
    ) -> (TriggerRecord, Option<String>) {
        let automation_id = automation_id.into();
        match trigger_input {
            CreateTriggerInput::ManualEvent => (TriggerRecord::manual_event(automation_id), None),
            CreateTriggerInput::Cron {
                schedule,
                timezone,
                next_fire_at,
            } => (
                TriggerRecord::new(
                    automation_id,
                    TriggerSpec::Cron {
                        config: CronTriggerConfig {
                            schedule,
                            timezone,
                            next_fire_at,
                        },
                    },
                ),
                None,
            ),
            CreateTriggerInput::Webhook {
                ingress_mode,
                secret_header_name,
                secret_hint,
                secret_plaintext,
            } => {
                let plaintext = secret_plaintext.unwrap_or_else(generate_webhook_secret);
                let hint = secret_hint.or_else(|| default_secret_hint(&plaintext));
                let secret_hash = Some(hash_webhook_secret(&plaintext));
                (
                    TriggerRecord::new(
                        automation_id,
                        TriggerSpec::Webhook {
                            config: WebhookTriggerConfig {
                                ingress_mode,
                                secret_header_name,
                                secret_hint: hint,
                                secret_present: true,
                                secret_hash,
                            },
                        },
                    ),
                    Some(plaintext),
                )
            }
            CreateTriggerInput::McpEvent {
                server_id,
                event_name,
                event_pattern,
            } => (
                TriggerRecord::new(
                    automation_id,
                    TriggerSpec::McpEvent {
                        config: McpEventTriggerConfig {
                            server_id,
                            event_name,
                            event_pattern,
                        },
                    },
                ),
                None,
            ),
        }
    }

    pub fn verify_webhook_secret(&self, config: &WebhookTriggerConfig, secret: &str) -> bool {
        let Some(expected_hash) = config.secret_hash.as_deref() else {
            return false;
        };
        hash_webhook_secret(secret) == expected_hash
    }

    pub fn mcp_event_matches(&self, config: &McpEventTriggerConfig, event_name: &str) -> bool {
        if let Some(expected_name) = config.event_name.as_deref() {
            if expected_name == event_name {
                return true;
            }
        }

        if let Some(pattern) = config.event_pattern.as_deref() {
            return wildcard_match(pattern, event_name);
        }

        false
    }

    pub fn compute_next_cron_fire(
        &self,
        trigger_id: &str,
        config: &CronTriggerConfig,
        after: &str,
    ) -> Result<String, RuntimeError> {
        let timezone = Tz::from_str(config.timezone.as_str()).map_err(|_| {
            RuntimeError::InvalidCronSchedule {
                trigger_id: trigger_id.to_string(),
                schedule: config.schedule.clone(),
            }
        })?;
        let schedule = Schedule::from_str(config.schedule.as_str()).map_err(|_| {
            RuntimeError::InvalidCronSchedule {
                trigger_id: trigger_id.to_string(),
                schedule: config.schedule.clone(),
            }
        })?;
        let after = DateTime::parse_from_rfc3339(after)
            .map_err(|_| RuntimeError::InvalidCronSchedule {
                trigger_id: trigger_id.to_string(),
                schedule: config.schedule.clone(),
            })?
            .with_timezone(&Utc)
            .with_timezone(&timezone);
        let next =
            schedule
                .after(&after)
                .next()
                .ok_or_else(|| RuntimeError::InvalidCronSchedule {
                    trigger_id: trigger_id.to_string(),
                    schedule: config.schedule.clone(),
                })?;
        Ok(next.with_timezone(&Utc).to_rfc3339())
    }

    pub async fn list_due_cron_triggers(
        &self,
        now: &str,
    ) -> Result<Vec<TriggerRecord>, RuntimeError> {
        let now = DateTime::parse_from_rfc3339(now)
            .map_err(|_| RuntimeError::InvalidCronSchedule {
                trigger_id: "cron".to_string(),
                schedule: now.to_string(),
            })?
            .with_timezone(&Utc);
        let rows = sqlx::query(
            r#"
            SELECT id, automation_id, trigger_type, schedule, timezone, next_fire_at,
                   ingress_mode, secret_header_name, secret_hint, webhook_secret_hash,
                   server_id, event_name, event_pattern, created_at, updated_at
            FROM triggers
            WHERE trigger_type = 'cron'
            ORDER BY next_fire_at, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut due = Vec::new();
        for row in rows {
            let trigger = trigger_from_row(&row)?;
            let Some(config) = trigger.spec.cron_config() else {
                continue;
            };
            let next_fire_at = DateTime::parse_from_rfc3339(config.next_fire_at.as_str())
                .map_err(|_| RuntimeError::InvalidCronSchedule {
                    trigger_id: trigger.id.clone(),
                    schedule: config.schedule.clone(),
                })?
                .with_timezone(&Utc);
            if next_fire_at <= now {
                due.push(trigger);
            }
        }

        Ok(due)
    }

    pub async fn update_cron_next_fire_at(
        &self,
        trigger_id: &str,
        next_fire_at: &str,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE triggers
            SET next_fire_at = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
        )
        .bind(trigger_id)
        .bind(next_fire_at)
        .bind(current_timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn fetch_automation(
        &self,
        automation_id: &str,
    ) -> Result<Option<AutomationRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, trigger_id, status, title, instruction,
                   action_json, capability_id, estimated_cost, created_at, updated_at
            FROM automations
            WHERE id = ?1
            "#,
        )
        .bind(automation_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| automation_from_row(&row)).transpose()
    }

    pub async fn list_automations(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<AutomationSummaryRecord>, RuntimeError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, trigger_id, status, title, instruction,
                   action_json, capability_id, estimated_cost, created_at, updated_at
            FROM automations
            WHERE workspace_id = ?1 AND project_id = ?2
            ORDER BY updated_at DESC, id DESC
            "#,
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let automation = automation_from_row(&row)?;
            summaries.push(self.build_automation_summary(automation, 3).await?);
        }
        Ok(summaries)
    }

    pub async fn load_automation_detail(
        &self,
        automation_id: &str,
    ) -> Result<AutomationDetailRecord, RuntimeError> {
        let automation = self
            .fetch_automation(automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(automation_id.to_string()))?;
        self.build_automation_summary(automation, 10).await
    }

    pub async fn transition_automation_status(
        &self,
        automation_id: &str,
        next_status: &str,
    ) -> Result<AutomationRecord, RuntimeError> {
        let automation = self
            .fetch_automation(automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(automation_id.to_string()))?;
        if !automation.can_transition_to(next_status) {
            return Err(RuntimeError::InvalidAutomationLifecycleTransition {
                automation_id: automation.id,
                from: automation.status,
                to: next_status.to_string(),
            });
        }

        let updated_at = current_timestamp();
        sqlx::query(
            r#"
            UPDATE automations
            SET status = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
        )
        .bind(automation_id)
        .bind(next_status)
        .bind(&updated_at)
        .execute(&self.pool)
        .await?;

        self.fetch_automation(automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(automation_id.to_string()))
    }

    pub async fn fetch_trigger(
        &self,
        trigger_id: &str,
    ) -> Result<Option<TriggerRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, automation_id, trigger_type, schedule, timezone, next_fire_at,
                   ingress_mode, secret_header_name, secret_hint, webhook_secret_hash,
                   server_id, event_name, event_pattern, created_at, updated_at
            FROM triggers
            WHERE id = ?1
            "#,
        )
        .bind(trigger_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| trigger_from_row(&row)).transpose()
    }

    pub async fn create_trigger_delivery(
        &self,
        trigger_id: &str,
        dedupe_key: &str,
        payload: Value,
    ) -> Result<TriggerDeliveryRecord, RuntimeError> {
        let delivery =
            TriggerDeliveryRecord::new(trigger_id.to_string(), dedupe_key.to_string(), payload);
        sqlx::query(
            r#"
            INSERT INTO trigger_deliveries (
                id, trigger_id, run_id, status, dedupe_key, payload_json, attempt_count,
                last_error, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&delivery.id)
        .bind(&delivery.trigger_id)
        .bind(&delivery.run_id)
        .bind(&delivery.status)
        .bind(&delivery.dedupe_key)
        .bind(serde_json::to_string(&delivery.payload)?)
        .bind(delivery.attempt_count)
        .bind(&delivery.last_error)
        .bind(&delivery.created_at)
        .bind(&delivery.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(delivery)
    }

    pub async fn find_trigger_delivery_by_dedupe_key(
        &self,
        dedupe_key: &str,
    ) -> Result<Option<TriggerDeliveryRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, trigger_id, run_id, status, dedupe_key, payload_json, attempt_count,
                   last_error, created_at, updated_at
            FROM trigger_deliveries
            WHERE dedupe_key = ?1
            "#,
        )
        .bind(dedupe_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| trigger_delivery_from_row(&row)).transpose()
    }

    pub async fn fetch_trigger_delivery(
        &self,
        delivery_id: &str,
    ) -> Result<Option<TriggerDeliveryRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, trigger_id, run_id, status, dedupe_key, payload_json, attempt_count,
                   last_error, created_at, updated_at
            FROM trigger_deliveries
            WHERE id = ?1
            "#,
        )
        .bind(delivery_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| trigger_delivery_from_row(&row)).transpose()
    }

    pub async fn list_trigger_deliveries_by_automation(
        &self,
        automation_id: &str,
    ) -> Result<Vec<TriggerDeliveryRecord>, RuntimeError> {
        let rows = sqlx::query(
            r#"
            SELECT td.id, td.trigger_id, td.run_id, td.status, td.dedupe_key, td.payload_json,
                   td.attempt_count, td.last_error, td.created_at, td.updated_at
            FROM trigger_deliveries td
            JOIN triggers t ON t.id = td.trigger_id
            WHERE t.automation_id = ?1
            ORDER BY td.updated_at DESC, td.id DESC
            "#,
        )
        .bind(automation_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(trigger_delivery_from_row)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn update_trigger_delivery(
        &self,
        delivery: &TriggerDeliveryRecord,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE trigger_deliveries
            SET run_id = ?2,
                status = ?3,
                payload_json = ?4,
                attempt_count = ?5,
                last_error = ?6,
                updated_at = ?7
            WHERE id = ?1
            "#,
        )
        .bind(&delivery.id)
        .bind(&delivery.run_id)
        .bind(&delivery.status)
        .bind(serde_json::to_string(&delivery.payload)?)
        .bind(delivery.attempt_count)
        .bind(&delivery.last_error)
        .bind(&delivery.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn transition_delivery_to_delivering(
        &self,
        mut delivery: TriggerDeliveryRecord,
    ) -> Result<TriggerDeliveryRecord, RuntimeError> {
        delivery.status = "delivering".to_string();
        delivery.attempt_count += 1;
        delivery.last_error = None;
        delivery.updated_at = current_timestamp();
        self.update_trigger_delivery(&delivery).await?;
        Ok(delivery)
    }

    pub async fn mark_delivery_retry_scheduled(
        &self,
        mut delivery: TriggerDeliveryRecord,
    ) -> Result<TriggerDeliveryRecord, RuntimeError> {
        delivery.status = "retry_scheduled".to_string();
        delivery.attempt_count += 1;
        delivery.updated_at = current_timestamp();
        self.update_trigger_delivery(&delivery).await?;
        Ok(delivery)
    }

    pub async fn sync_delivery_from_run(
        &self,
        delivery_id: &str,
        run: &RunRecord,
    ) -> Result<TriggerDeliveryRecord, RuntimeError> {
        let mut delivery = self
            .fetch_trigger_delivery(delivery_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerDeliveryNotFound(delivery_id.to_string()))?;

        delivery.run_id = Some(run.id.clone());
        delivery.last_error = run.last_error.clone();
        delivery.status = match run.status.as_str() {
            "completed" => "succeeded".to_string(),
            "waiting_approval" | "running" | "resuming" | "created" => "delivering".to_string(),
            "failed" | "blocked" | "terminated" | "cancelled" => "failed".to_string(),
            _ => delivery.status,
        };
        delivery.updated_at = current_timestamp();
        self.update_trigger_delivery(&delivery).await?;
        Ok(delivery)
    }

    async fn list_recent_trigger_deliveries_by_automation(
        &self,
        automation_id: &str,
        limit: i64,
    ) -> Result<Vec<TriggerDeliveryRecord>, RuntimeError> {
        let rows = sqlx::query(
            r#"
            SELECT td.id, td.trigger_id, td.run_id, td.status, td.dedupe_key, td.payload_json,
                   td.attempt_count, td.last_error, td.created_at, td.updated_at
            FROM trigger_deliveries td
            JOIN triggers t ON t.id = td.trigger_id
            WHERE t.automation_id = ?1
            ORDER BY td.updated_at DESC, td.id DESC
            LIMIT ?2
            "#,
        )
        .bind(automation_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(trigger_delivery_from_row)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn fetch_task_record(&self, task_id: &str) -> Result<Option<TaskRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, source_kind, automation_id, title, instruction,
                   action_json, capability_id, estimated_cost, idempotency_key, created_at, updated_at
            FROM tasks
            WHERE id = ?1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| task_from_row(&row)).transpose()
    }

    async fn fetch_run_record(&self, run_id: &str) -> Result<Option<RunRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, task_id, workspace_id, project_id, automation_id, trigger_delivery_id,
                   run_type, status, approval_request_id, idempotency_key, attempt_count,
                   max_attempts, checkpoint_seq, resume_token, last_error, created_at, updated_at,
                   started_at, completed_at, terminated_at
            FROM runs
            WHERE id = ?1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| run_from_row(&row)).transpose()
    }

    async fn build_automation_summary(
        &self,
        automation: AutomationRecord,
        recent_limit: i64,
    ) -> Result<AutomationSummaryRecord, RuntimeError> {
        let trigger = self
            .fetch_trigger(&automation.trigger_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerNotFound(automation.trigger_id.clone()))?;
        let recent_deliveries = self
            .list_recent_trigger_deliveries_by_automation(&automation.id, recent_limit)
            .await?;
        let last_run_summary = match recent_deliveries.first().and_then(|delivery| delivery.run_id.clone())
        {
            Some(run_id) => {
                let run = self
                    .fetch_run_record(&run_id)
                    .await?
                    .ok_or_else(|| RuntimeError::RunNotFound(run_id.clone()))?;
                let task = self
                    .fetch_task_record(&run.task_id)
                    .await?
                    .ok_or_else(|| RuntimeError::TaskNotFound(run.task_id.clone()))?;
                Some(RunSummaryRecord::new(&run, &task))
            }
            None => None,
        };

        Ok(AutomationSummaryRecord {
            automation,
            trigger,
            recent_deliveries,
            last_run_summary,
        })
    }
}

fn trigger_schedule(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Cron { config } => Some(config.schedule.as_str()),
        _ => None,
    }
}

fn trigger_timezone(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Cron { config } => Some(config.timezone.as_str()),
        _ => None,
    }
}

fn trigger_next_fire_at(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Cron { config } => Some(config.next_fire_at.as_str()),
        _ => None,
    }
}

fn trigger_ingress_mode(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Webhook { config } => Some(config.ingress_mode.as_str()),
        _ => None,
    }
}

fn trigger_secret_header_name(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Webhook { config } => Some(config.secret_header_name.as_str()),
        _ => None,
    }
}

fn trigger_secret_hint(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Webhook { config } => config.secret_hint.as_deref(),
        _ => None,
    }
}

fn trigger_secret_hash(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::Webhook { config } => config.secret_hash.as_deref(),
        _ => None,
    }
}

fn trigger_server_id(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::McpEvent { config } => Some(config.server_id.as_str()),
        _ => None,
    }
}

fn trigger_event_name(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::McpEvent { config } => config.event_name.as_deref(),
        _ => None,
    }
}

fn trigger_event_pattern(trigger: &TriggerRecord) -> Option<&str> {
    match &trigger.spec {
        TriggerSpec::McpEvent { config } => config.event_pattern.as_deref(),
        _ => None,
    }
}

fn generate_webhook_secret() -> String {
    format!(
        "whsec_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn default_secret_hint(secret: &str) -> Option<String> {
    let hint = secret.chars().rev().take(4).collect::<Vec<_>>();
    if hint.is_empty() {
        None
    } else {
        Some(hint.into_iter().rev().collect())
    }
}

fn hash_webhook_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == value;
    }

    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');
    let mut remainder = value;

    for (index, part) in pattern
        .split('*')
        .filter(|segment| !segment.is_empty())
        .enumerate()
    {
        if index == 0 && anchored_start {
            let Some(stripped) = remainder.strip_prefix(part) else {
                return false;
            };
            remainder = stripped;
            continue;
        }

        let Some(found_at) = remainder.find(part) else {
            return false;
        };
        remainder = &remainder[(found_at + part.len())..];
    }

    if anchored_end {
        let last = pattern
            .split('*')
            .filter(|segment| !segment.is_empty())
            .next_back()
            .unwrap_or_default();
        value.ends_with(last)
    } else {
        true
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgeManager {
    knowledge_store: SqliteKnowledgeStore,
    observation_store: SqliteObservationStore,
}

impl KnowledgeManager {
    pub fn new(
        knowledge_store: SqliteKnowledgeStore,
        observation_store: SqliteObservationStore,
    ) -> Self {
        Self {
            knowledge_store,
            observation_store,
        }
    }

    pub async fn ensure_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
        display_name: &str,
        owner_ref: &str,
    ) -> Result<KnowledgeSpaceRecord, RuntimeError> {
        Ok(self
            .knowledge_store
            .ensure_project_knowledge_space(workspace_id, project_id, display_name, owner_ref)
            .await?)
    }

    pub async fn fetch_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<KnowledgeSpaceRecord>, RuntimeError> {
        Ok(self
            .knowledge_store
            .fetch_project_knowledge_space(workspace_id, project_id)
            .await?)
    }

    pub async fn list_knowledge_candidates_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeCandidateRecord>, RuntimeError> {
        Ok(self
            .knowledge_store
            .list_knowledge_candidates_by_run(run_id)
            .await?)
    }

    pub async fn fetch_knowledge_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<KnowledgeCandidateRecord>, RuntimeError> {
        Ok(self
            .knowledge_store
            .fetch_knowledge_candidate(candidate_id)
            .await?)
    }

    pub async fn list_knowledge_lineage_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeLineageRecord>, RuntimeError> {
        Ok(self
            .observation_store
            .list_knowledge_lineage_by_run(run_id)
            .await?)
    }

    pub async fn list_recalled_knowledge_assets_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeAssetRecord>, RuntimeError> {
        let lineage = self
            .observation_store
            .list_knowledge_lineage_by_run(run_id)
            .await?;
        let mut assets = Vec::new();
        for record in lineage {
            if record.relation_type != "recalled_by" {
                continue;
            }
            let Some(asset_id) = record.source_ref.strip_prefix("knowledge_asset:") else {
                continue;
            };
            if let Some(asset) = self.knowledge_store.fetch_knowledge_asset(asset_id).await? {
                assets.push(asset);
            }
        }
        Ok(assets)
    }

    pub async fn list_knowledge_assets_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeAssetRecord>, RuntimeError> {
        let mut seen = HashSet::new();
        let mut assets = Vec::new();

        for asset in self.list_recalled_knowledge_assets_by_run(run_id).await? {
            if seen.insert(asset.id.clone()) {
                assets.push(asset);
            }
        }

        for candidate in self
            .knowledge_store
            .list_knowledge_candidates_by_run(run_id)
            .await?
        {
            if let Some(asset) = self
                .knowledge_store
                .fetch_knowledge_asset_by_candidate(&candidate.id)
                .await?
            {
                if seen.insert(asset.id.clone()) {
                    assets.push(asset);
                }
            }
        }

        assets.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(assets)
    }

    pub async fn apply_recall(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
    ) -> Result<Vec<KnowledgeAssetRecord>, RuntimeError> {
        let assets = self
            .knowledge_store
            .list_shared_assets_for_project_capability(
                &run.workspace_id,
                &run.project_id,
                &task.capability_id,
            )
            .await?;

        if assets.is_empty() {
            return Ok(Vec::new());
        }

        for asset in &assets {
            self.observation_store
                .insert_knowledge_lineage(&KnowledgeLineageRecord::new(
                    &run.workspace_id,
                    &run.project_id,
                    &run.id,
                    &run.task_id,
                    format!("knowledge_asset:{}", asset.id),
                    format!("run:{}", run.id),
                    "recalled_by",
                ))
                .await?;
        }
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_KNOWLEDGE_RECALLED,
                format!("Recalled {} shared knowledge assets", assets.len()),
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_KNOWLEDGE_RECALL,
                std::cmp::max(run.attempt_count, 1),
                format!("Recalled {} shared knowledge assets", assets.len()),
            ))
            .await?;

        Ok(assets)
    }

    pub async fn capture_from_artifact(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
        artifact: &ArtifactRecord,
    ) -> Result<Option<KnowledgeCandidateRecord>, RuntimeError> {
        if artifact.knowledge_gate_status != KNOWLEDGE_GATE_ELIGIBLE {
            self.knowledge_store.resolve_capture_retry(&run.id).await?;
            self.observation_store
                .write_audit(&AuditRecord::new(
                    &run.workspace_id,
                    &run.project_id,
                    &run.id,
                    &run.task_id,
                    AUDIT_EVENT_KNOWLEDGE_CAPTURE_GATED,
                    format!(
                        "Knowledge capture gated for artifact {}: {}",
                        artifact.id, artifact.knowledge_gate_status
                    ),
                ))
                .await?;
            self.observation_store
                .write_trace(&TraceRecord::new(
                    &run.workspace_id,
                    &run.project_id,
                    &run.id,
                    &run.task_id,
                    TRACE_STAGE_KNOWLEDGE_CAPTURE,
                    std::cmp::max(run.attempt_count, 1),
                    format!(
                        "Knowledge capture gated for artifact {}: {}",
                        artifact.id, artifact.knowledge_gate_status
                    ),
                ))
                .await?;
            return Ok(None);
        }

        let dedupe_key = format!("knowledge_candidate:artifact:{}", artifact.id);
        if let Some(existing) = self
            .knowledge_store
            .find_knowledge_candidate_by_dedupe_key(&dedupe_key)
            .await?
        {
            self.knowledge_store.resolve_capture_retry(&run.id).await?;
            return Ok(Some(existing));
        }

        let Some(space) = self
            .knowledge_store
            .fetch_project_knowledge_space(&run.workspace_id, &run.project_id)
            .await?
        else {
            self.record_capture_failure(run, task, artifact, "project_knowledge_space_missing")
                .await?;
            return Ok(None);
        };

        let candidate = KnowledgeCandidateRecord::new(
            &space.id,
            &run.id,
            &run.task_id,
            &artifact.id,
            &task.capability_id,
            &artifact.content,
            &artifact.provenance_source,
            &artifact.trust_level,
            dedupe_key,
        );
        self.knowledge_store
            .create_knowledge_candidate(&candidate)
            .await?;
        self.knowledge_store.resolve_capture_retry(&run.id).await?;
        self.observation_store
            .insert_knowledge_lineage(&KnowledgeLineageRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                format!("artifact:{}", artifact.id),
                format!("knowledge_candidate:{}", candidate.id),
                "derived_from",
            ))
            .await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_KNOWLEDGE_CANDIDATE_CREATED,
                "Knowledge candidate captured from execution artifact",
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_KNOWLEDGE_CAPTURE,
                std::cmp::max(run.attempt_count, 1),
                "Knowledge candidate captured from execution artifact",
            ))
            .await?;

        Ok(Some(candidate))
    }

    pub async fn retry_capture(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
        artifact: &ArtifactRecord,
    ) -> Result<(), RuntimeError> {
        let existing_candidates = self
            .knowledge_store
            .list_knowledge_candidates_by_run(&run.id)
            .await?;
        if !existing_candidates.is_empty() {
            return Ok(());
        }

        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_KNOWLEDGE_CAPTURE_RETRIED,
                "Knowledge capture retried explicitly",
            ))
            .await?;

        let _ = self.capture_from_artifact(run, task, artifact).await?;
        Ok(())
    }

    pub async fn promote_knowledge_candidate(
        &self,
        candidate_id: &str,
        actor_ref: &str,
        note: &str,
    ) -> Result<KnowledgePromotionReport, RuntimeError> {
        let mut candidate = self
            .knowledge_store
            .fetch_knowledge_candidate(candidate_id)
            .await?
            .ok_or_else(|| RuntimeError::KnowledgeCandidateNotFound(candidate_id.to_string()))?;
        let space = self
            .knowledge_store
            .fetch_knowledge_space(&candidate.knowledge_space_id)
            .await?
            .ok_or_else(|| RuntimeError::KnowledgeCandidateNotFound(candidate_id.to_string()))?;

        let asset = if let Some(existing) = self
            .knowledge_store
            .fetch_knowledge_asset_by_candidate(candidate_id)
            .await?
        {
            existing
        } else {
            let created = KnowledgeAssetRecord::from_candidate(&candidate);
            self.knowledge_store
                .upsert_knowledge_asset(&created)
                .await?;
            created
        };

        if candidate.status != "verified_shared" {
            self.knowledge_store
                .update_knowledge_candidate_status(candidate_id, "verified_shared")
                .await?;
            candidate.status = "verified_shared".to_string();
            candidate.updated_at = current_timestamp();
        }

        let lineage = KnowledgeLineageRecord::new(
            &space.workspace_id,
            space.project_id.as_deref().unwrap_or_default(),
            &candidate.source_run_id,
            &candidate.source_task_id,
            format!("knowledge_candidate:{}", candidate.id),
            format!("knowledge_asset:{}", asset.id),
            "promoted_from",
        );
        self.observation_store
            .insert_knowledge_lineage(&lineage)
            .await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &space.workspace_id,
                space.project_id.as_deref().unwrap_or_default(),
                &candidate.source_run_id,
                &candidate.source_task_id,
                AUDIT_EVENT_KNOWLEDGE_ASSET_PROMOTED,
                format!("Knowledge candidate promoted by {actor_ref}: {note}"),
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &space.workspace_id,
                space.project_id.as_deref().unwrap_or_default(),
                &candidate.source_run_id,
                &candidate.source_task_id,
                TRACE_STAGE_KNOWLEDGE_PROMOTION,
                1,
                format!("Knowledge candidate promoted by {actor_ref}"),
            ))
            .await?;

        Ok(KnowledgePromotionReport {
            knowledge_space: space,
            candidate,
            asset,
            lineage,
        })
    }

    async fn record_capture_failure(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
        artifact: &ArtifactRecord,
        reason: &str,
    ) -> Result<(), RuntimeError> {
        self.knowledge_store
            .upsert_capture_retry(&KnowledgeCaptureRetryRecord::pending(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                &artifact.id,
                &task.capability_id,
                reason,
            ))
            .await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_KNOWLEDGE_CAPTURE_FAILED,
                format!("Knowledge capture failed: {reason}"),
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_KNOWLEDGE_CAPTURE,
                std::cmp::max(run.attempt_count, 1),
                format!("Knowledge capture failed: {reason}"),
            ))
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RunOrchestrator {
    pool: SqlitePool,
    governance_store: SqliteGovernanceStore,
    interop_store: SqliteInteropStore,
    mcp_gateway: McpGateway,
    knowledge_manager: KnowledgeManager,
    observation_store: SqliteObservationStore,
}

impl RunOrchestrator {
    pub fn new(
        pool: SqlitePool,
        governance_store: SqliteGovernanceStore,
        interop_store: SqliteInteropStore,
        knowledge_manager: KnowledgeManager,
        observation_store: SqliteObservationStore,
    ) -> Self {
        let mcp_gateway = McpGateway::new(interop_store.clone());
        Self {
            pool,
            governance_store,
            interop_store,
            mcp_gateway,
            knowledge_manager,
            observation_store,
        }
    }

    pub async fn upsert_capability_descriptor(
        &self,
        record: CapabilityDescriptorRecord,
    ) -> Result<(), RuntimeError> {
        self.governance_store
            .upsert_capability_descriptor(&record)
            .await?;
        Ok(())
    }

    pub async fn upsert_capability_binding(
        &self,
        record: CapabilityBindingRecord,
    ) -> Result<(), RuntimeError> {
        self.governance_store
            .upsert_capability_binding(&record)
            .await?;
        Ok(())
    }

    pub async fn upsert_capability_grant(
        &self,
        record: CapabilityGrantRecord,
    ) -> Result<(), RuntimeError> {
        self.governance_store
            .upsert_capability_grant(&record)
            .await?;
        Ok(())
    }

    pub async fn upsert_budget_policy(
        &self,
        record: BudgetPolicyRecord,
    ) -> Result<(), RuntimeError> {
        self.governance_store.upsert_budget_policy(&record).await?;
        Ok(())
    }

    pub async fn upsert_mcp_server(&self, record: McpServerRecord) -> Result<(), RuntimeError> {
        self.interop_store.upsert_mcp_server(&record).await?;
        Ok(())
    }

    pub async fn upsert_mcp_credential(
        &self,
        record: McpCredentialRecord,
        secret: &str,
    ) -> Result<(), RuntimeError> {
        self.interop_store
            .upsert_mcp_credential(&record, secret)
            .await?;
        Ok(())
    }

    pub async fn list_visible_capabilities(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<CapabilityDescriptorRecord>, RuntimeError> {
        Ok(self
            .governance_store
            .list_visible_capability_descriptors(workspace_id, project_id)
            .await?)
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerRecord>, RuntimeError> {
        Ok(self.interop_store.list_mcp_servers().await?)
    }

    pub async fn list_mcp_credentials(&self) -> Result<Vec<McpCredentialRecord>, RuntimeError> {
        Ok(self.interop_store.list_mcp_credentials().await?)
    }

    pub async fn list_mcp_invocations_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<McpInvocationRecord>, RuntimeError> {
        Ok(self
            .interop_store
            .list_mcp_invocations_by_run(run_id)
            .await?)
    }

    pub async fn list_environment_leases_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<EnvironmentLeaseRecord>, RuntimeError> {
        Ok(self
            .interop_store
            .list_environment_leases_by_run(run_id)
            .await?)
    }

    pub async fn request_environment_lease(
        &self,
        run_id: &str,
        task_id: &str,
        capability_id: &str,
        environment_type: &str,
        sandbox_tier: &str,
        ttl_seconds: i64,
    ) -> Result<EnvironmentLeaseRecord, RuntimeError> {
        let run = self
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;
        Ok(self
            .interop_store
            .request_environment_lease(
                &run.workspace_id,
                &run.project_id,
                run_id,
                task_id,
                capability_id,
                environment_type,
                sandbox_tier,
                ttl_seconds,
            )
            .await?)
    }

    pub async fn heartbeat_environment_lease(
        &self,
        lease_id: &str,
        ttl_seconds: i64,
    ) -> Result<EnvironmentLeaseRecord, RuntimeError> {
        Ok(self
            .interop_store
            .heartbeat_environment_lease(lease_id, ttl_seconds)
            .await?)
    }

    pub async fn release_environment_lease(
        &self,
        lease_id: &str,
    ) -> Result<EnvironmentLeaseRecord, RuntimeError> {
        Ok(self
            .interop_store
            .release_environment_lease(lease_id)
            .await?)
    }

    pub async fn start_task(&self, task: &TaskRecord) -> Result<RunExecutionReport, RuntimeError> {
        self.start_task_with_trigger_delivery(task, None).await
    }

    pub async fn start_task_with_trigger_delivery(
        &self,
        task: &TaskRecord,
        trigger_delivery_id: Option<&str>,
    ) -> Result<RunExecutionReport, RuntimeError> {
        if let Some(existing) = self
            .fetch_run_by_idempotency_key(&format!("run:task:{}", task.id))
            .await?
        {
            return self.load_run_report(&existing.id).await;
        }

        let run = RunRecord::new(task, trigger_delivery_id.map(str::to_string));
        self.insert_run(&run).await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_RUN_CREATED,
                "Run created for task",
            ))
            .await?;
        if run.trigger_delivery_id.is_some() {
            self.observation_store
                .write_trace(&TraceRecord::new(
                    &run.workspace_id,
                    &run.project_id,
                    &run.id,
                    &run.task_id,
                    TRACE_STAGE_TRIGGER_DELIVERY,
                    1,
                    "Trigger delivery started automation execution",
                ))
                .await?;
        }

        let decision = self
            .governance_store
            .evaluate_task(&TaskGovernanceInput {
                workspace_id: task.workspace_id.clone(),
                project_id: task.project_id.clone(),
                capability_id: task.capability_id.clone(),
                estimated_cost: task.estimated_cost,
            })
            .await?;

        match decision {
            GovernanceDecision::Allow { reason } => {
                self.record_policy_decision(&run, task, DECISION_ALLOW, reason.as_str(), None)
                    .await?;
                self.execute_run(run, task).await
            }
            GovernanceDecision::RequireApproval { reason } => {
                self.wait_for_approval(run, task, reason.as_str()).await
            }
            GovernanceDecision::Deny { reason } => self.deny_run(run, task, reason.as_str()).await,
        }
    }

    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        task: &TaskRecord,
        decision: ApprovalDecision,
        actor_ref: &str,
        note: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let approval = self
            .governance_store
            .resolve_approval_request(approval_id, decision, actor_ref, note)
            .await?;
        let run = self
            .fetch_run(&approval.run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(approval.run_id.clone()))?;

        match approval.approval_type.as_str() {
            APPROVAL_TYPE_EXECUTION => self.resolve_execution_approval(approval, run, task).await,
            APPROVAL_TYPE_KNOWLEDGE_PROMOTION => {
                self.resolve_knowledge_promotion_approval(approval, run, actor_ref, note)
                    .await
            }
            other => Err(RuntimeError::InvalidApprovalType {
                approval_id: approval.id,
                approval_type: other.to_string(),
            }),
        }
    }

    async fn resolve_execution_approval(
        &self,
        approval: ApprovalRequestRecord,
        mut run: RunRecord,
        task: &TaskRecord,
    ) -> Result<RunExecutionReport, RuntimeError> {
        match approval.status.as_str() {
            "approved" => {
                if run.status == "completed" {
                    return self.load_run_report(&run.id).await;
                }
                if !matches!(run.status.as_str(), "waiting_approval" | "resuming") {
                    return Err(RuntimeError::InvalidRunTransition {
                        run_id: run.id,
                        from: run.status,
                        to: "resuming".to_string(),
                    });
                }

                self.resolve_inbox_for_approval(&run.id, &approval.id)
                    .await?;
                self.record_policy_decision(
                    &run,
                    task,
                    DECISION_ALLOW,
                    "approval_approved",
                    Some(approval.id.clone()),
                )
                .await?;

                run.status = "resuming".to_string();
                run.approval_request_id = Some(approval.id.clone());
                run.resume_token = None;
                run.last_error = None;
                run.updated_at = current_timestamp();
                run.checkpoint_seq += 1;
                self.update_run(&run).await?;
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_APPROVAL_APPROVED,
                        "Approval approved and run resumed",
                    ))
                    .await?;

                self.execute_run(run, task).await
            }
            "rejected" | "expired" | "cancelled" => {
                if run.status == "blocked" {
                    return self.load_run_report(&run.id).await;
                }
                if run.status != "waiting_approval" {
                    return Err(RuntimeError::InvalidRunTransition {
                        run_id: run.id,
                        from: run.status,
                        to: "blocked".to_string(),
                    });
                }

                self.resolve_inbox_for_approval(&run.id, &approval.id)
                    .await?;
                let rejection_reason = format!("approval_{}", approval.status);
                self.record_policy_decision(
                    &run,
                    task,
                    DECISION_DENY,
                    rejection_reason.as_str(),
                    Some(approval.id.clone()),
                )
                .await?;

                run.status = "blocked".to_string();
                run.approval_request_id = Some(approval.id.clone());
                run.resume_token = None;
                run.last_error = Some(format!("approval_{}", approval.status));
                run.updated_at = current_timestamp();
                run.checkpoint_seq += 1;
                self.update_run(&run).await?;

                let event_type = match approval.status.as_str() {
                    "rejected" => AUDIT_EVENT_APPROVAL_REJECTED,
                    "expired" => AUDIT_EVENT_APPROVAL_EXPIRED,
                    _ => AUDIT_EVENT_APPROVAL_CANCELLED,
                };
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        event_type,
                        format!("Approval {} and run blocked", approval.status),
                    ))
                    .await?;
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_RUN_BLOCKED,
                        format!("Run blocked because approval {}", approval.status),
                    ))
                    .await?;

                self.load_run_report(&run.id).await
            }
            other => Err(RuntimeError::InvalidRunTransition {
                run_id: run.id,
                from: other.to_string(),
                to: "resolve_approval".to_string(),
            }),
        }
    }

    async fn resolve_knowledge_promotion_approval(
        &self,
        approval: ApprovalRequestRecord,
        run: RunRecord,
        actor_ref: &str,
        note: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let candidate_id = approval
            .target_ref
            .strip_prefix("knowledge_candidate:")
            .ok_or_else(|| RuntimeError::InvalidApprovalTargetRef {
                approval_id: approval.id.clone(),
                target_ref: approval.target_ref.clone(),
            })?;

        self.resolve_inbox_for_approval(&run.id, &approval.id).await?;

        match approval.status.as_str() {
            "approved" => {
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_APPROVAL_APPROVED,
                        format!(
                            "Knowledge promotion approval approved for candidate {candidate_id}"
                        ),
                    ))
                    .await?;
                self.knowledge_manager
                    .promote_knowledge_candidate(candidate_id, actor_ref, note)
                    .await?;
                self.load_run_report(&run.id).await
            }
            "rejected" | "expired" | "cancelled" => {
                let event_type = match approval.status.as_str() {
                    "rejected" => AUDIT_EVENT_APPROVAL_REJECTED,
                    "expired" => AUDIT_EVENT_APPROVAL_EXPIRED,
                    _ => AUDIT_EVENT_APPROVAL_CANCELLED,
                };
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        event_type,
                        format!(
                            "Knowledge promotion approval {} for candidate {}",
                            approval.status, candidate_id
                        ),
                    ))
                    .await?;
                self.load_run_report(&run.id).await
            }
            other => Err(RuntimeError::InvalidRunTransition {
                run_id: run.id,
                from: other.to_string(),
                to: "resolve_approval".to_string(),
            }),
        }
    }

    pub async fn retry_run(
        &self,
        run_id: &str,
        task: &TaskRecord,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let mut run = self
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;

        if !run.can_retry() {
            return Err(RuntimeError::InvalidRunTransition {
                run_id: run_id.to_string(),
                from: run.status,
                to: "resuming".to_string(),
            });
        }

        run.status = "resuming".to_string();
        run.checkpoint_seq += 1;
        run.updated_at = current_timestamp();
        self.update_run(&run).await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_RUN_RETRY_REQUESTED,
                "Run retry requested",
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_RUN_ORCHESTRATOR,
                run.attempt_count + 1,
                "Retry requested for failed run",
            ))
            .await?;

        self.execute_run(run, task).await
    }

    pub async fn terminate_run(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<RunRecord, RuntimeError> {
        let mut run = self
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;

        if !run.can_terminate() {
            return Err(RuntimeError::InvalidRunTransition {
                run_id: run_id.to_string(),
                from: run.status,
                to: "terminated".to_string(),
            });
        }

        run.status = "terminated".to_string();
        run.resume_token = None;
        run.last_error = Some(reason.to_string());
        run.terminated_at = Some(current_timestamp());
        run.updated_at = run.terminated_at.clone().unwrap();
        run.checkpoint_seq += 1;
        self.update_run(&run).await?;

        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_RUN_TERMINATED,
                format!("Run terminated: {reason}"),
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_RUN_ORCHESTRATOR,
                std::cmp::max(run.attempt_count, 1),
                format!("Run terminated: {reason}"),
            ))
            .await?;

        Ok(run)
    }

    pub async fn fetch_run(&self, run_id: &str) -> Result<Option<RunRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, task_id, workspace_id, project_id, automation_id, trigger_delivery_id,
                run_type, status, approval_request_id, idempotency_key, attempt_count,
                max_attempts, checkpoint_seq, resume_token, last_error, created_at, updated_at,
                started_at, completed_at, terminated_at
            FROM runs
            WHERE id = ?1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| run_from_row(&row)).transpose()
    }

    pub async fn fetch_approval_request(
        &self,
        approval_id: &str,
    ) -> Result<Option<ApprovalRequestRecord>, RuntimeError> {
        Ok(self
            .governance_store
            .fetch_approval_request(approval_id)
            .await?)
    }

    pub async fn request_knowledge_promotion(
        &self,
        candidate_id: &str,
        actor_ref: &str,
        note: &str,
    ) -> Result<ApprovalRequestRecord, RuntimeError> {
        let target_ref = knowledge_candidate_target_ref(candidate_id);
        if let Some(existing) = self
            .governance_store
            .find_open_approval_request_by_target_ref(
                APPROVAL_TYPE_KNOWLEDGE_PROMOTION,
                target_ref.as_str(),
            )
            .await?
        {
            self.sync_governance_surface_records(
                &existing,
                "Knowledge promotion approval required",
                format!("Knowledge promotion requested by {actor_ref}: {note}").as_str(),
            )
            .await?;
            return Ok(existing);
        }

        let candidate = self
            .knowledge_manager
            .fetch_knowledge_candidate(candidate_id)
            .await?
            .ok_or_else(|| RuntimeError::KnowledgeCandidateNotFound(candidate_id.to_string()))?;
        if candidate.status != "candidate" {
            return Err(RuntimeError::InvalidKnowledgeCandidateState {
                candidate_id: candidate.id,
                status: candidate.status,
                expected: "candidate".to_string(),
            });
        }

        let run = self
            .fetch_run(&candidate.source_run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(candidate.source_run_id.clone()))?;
        let approval = ApprovalRequestRecord::new_knowledge_promotion(
            &run.workspace_id,
            &run.project_id,
            &run.id,
            &run.task_id,
            candidate_id,
            "knowledge_promotion_requested",
        );
        self.governance_store
            .create_approval_request(&approval)
            .await?;
        self.sync_governance_surface_records(
            &approval,
            "Knowledge promotion approval required",
            format!("Knowledge promotion requested by {actor_ref}: {note}").as_str(),
        )
        .await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_APPROVAL_REQUESTED,
                format!(
                    "Knowledge promotion requested for candidate {candidate_id} by {actor_ref}: {note}"
                ),
            ))
            .await?;

        Ok(approval)
    }

    pub async fn list_approval_requests_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ApprovalRequestRecord>, RuntimeError> {
        Ok(self
            .governance_store
            .list_approval_requests_by_run(run_id)
            .await?)
    }

    pub async fn list_artifacts_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ArtifactRecord>, RuntimeError> {
        Ok(self.observation_store.list_artifacts_by_run(run_id).await?)
    }

    pub async fn list_inbox_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<InboxItemRecord>, RuntimeError> {
        Ok(self
            .observation_store
            .list_inbox_items_by_workspace(workspace_id)
            .await?)
    }

    pub async fn list_notifications_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<NotificationRecord>, RuntimeError> {
        Ok(self
            .observation_store
            .list_notifications_by_workspace(workspace_id)
            .await?)
    }

    pub async fn list_policy_decisions_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<PolicyDecisionLogRecord>, RuntimeError> {
        Ok(self
            .observation_store
            .list_policy_decisions_by_run(run_id)
            .await?)
    }

    pub async fn load_run_report(&self, run_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        let run = self
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;
        let artifacts = self.observation_store.list_artifacts_by_run(run_id).await?;
        let audits = self.observation_store.list_audits_by_run(run_id).await?;
        let traces = self.observation_store.list_traces_by_run(run_id).await?;
        let approvals = self
            .governance_store
            .list_approval_requests_by_run(run_id)
            .await?;
        let inbox_items = self
            .observation_store
            .list_inbox_items_by_run(run_id)
            .await?;
        let notifications = self
            .observation_store
            .list_notifications_by_run(run_id)
            .await?;
        let policy_decisions = self
            .observation_store
            .list_policy_decisions_by_run(run_id)
            .await?;
        let knowledge_candidates = self
            .knowledge_manager
            .list_knowledge_candidates_by_run(run_id)
            .await?;
        let recalled_knowledge_assets = self
            .knowledge_manager
            .list_recalled_knowledge_assets_by_run(run_id)
            .await?;

        Ok(RunExecutionReport {
            run,
            artifacts,
            audits,
            traces,
            approvals,
            inbox_items,
            notifications,
            policy_decisions,
            knowledge_candidates,
            recalled_knowledge_assets,
        })
    }

    async fn wait_for_approval(
        &self,
        mut run: RunRecord,
        task: &TaskRecord,
        reason: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let approval = if let Some(existing) = self
            .governance_store
            .find_approval_request_by_dedupe_key(&format!("approval:{}", run.id))
            .await?
        {
            existing
        } else {
            let created = ApprovalRequestRecord::new_execution(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                reason,
            );
            self.governance_store
                .create_approval_request(&created)
                .await?;
            created
        };

        self.sync_governance_surface_records(
            &approval,
            "Approval required",
            "A run is waiting for approval",
        )
        .await?;

        self.record_policy_decision(
            &run,
            task,
            DECISION_REQUIRE_APPROVAL,
            reason,
            Some(approval.id.clone()),
        )
        .await?;

        run.status = "waiting_approval".to_string();
        run.approval_request_id = Some(approval.id.clone());
        run.resume_token = Some(format!("approval:{}", approval.id));
        run.last_error = None;
        run.updated_at = current_timestamp();
        run.checkpoint_seq += 1;
        self.update_run(&run).await?;

        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_APPROVAL_REQUESTED,
                "Approval requested before execution",
            ))
            .await?;

        self.load_run_report(&run.id).await
    }

    async fn deny_run(
        &self,
        mut run: RunRecord,
        task: &TaskRecord,
        reason: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        self.record_policy_decision(&run, task, DECISION_DENY, reason, None)
            .await?;

        run.status = "blocked".to_string();
        run.resume_token = None;
        run.last_error = Some(reason.to_string());
        run.updated_at = current_timestamp();
        run.checkpoint_seq += 1;
        self.update_run(&run).await?;

        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_POLICY_DENIED,
                format!("Run denied before execution: {reason}"),
            ))
            .await?;
        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_RUN_BLOCKED,
                format!("Run blocked before execution: {reason}"),
            ))
            .await?;

        self.load_run_report(&run.id).await
    }

    async fn execute_run(
        &self,
        mut run: RunRecord,
        task: &TaskRecord,
    ) -> Result<RunExecutionReport, RuntimeError> {
        run.status = "running".to_string();
        run.attempt_count += 1;
        run.checkpoint_seq += 1;
        run.started_at.get_or_insert_with(current_timestamp);
        run.updated_at = current_timestamp();
        self.update_run(&run).await?;

        self.observation_store
            .write_audit(&AuditRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                AUDIT_EVENT_RUN_STARTED,
                "Run execution started",
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_RUN_ORCHESTRATOR,
                run.attempt_count,
                "Run entered execution",
            ))
            .await?;
        let _ = self.knowledge_manager.apply_recall(&run, task).await?;

        let action_outcome = match &task.action {
            ExecutionAction::ConnectorCall {
                tool_name,
                arguments,
            } => ActionExecutionOutcome::from_gateway(
                self.mcp_gateway
                    .execute(GatewayRequest {
                        workspace_id: run.workspace_id.clone(),
                        project_id: run.project_id.clone(),
                        run_id: run.id.clone(),
                        task_id: run.task_id.clone(),
                        capability_id: task.capability_id.clone(),
                        tool_name: tool_name.clone(),
                        arguments: arguments.clone(),
                        attempt: run.attempt_count,
                    })
                    .await?,
            ),
            _ => ActionExecutionOutcome::from_builtin(
                ExecutionEngine::execute(&task.action, run.attempt_count as u32),
                &task.capability_id,
            ),
        };

        match action_outcome {
            ActionExecutionOutcome::Succeeded(success) => {
                self.observation_store
                    .write_trace(&TraceRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        TRACE_STAGE_EXECUTION_ACTION,
                        run.attempt_count,
                        "Execution action succeeded",
                    ))
                    .await?;

                let artifact = ArtifactRecord::execution_output(
                    &run.workspace_id,
                    &run.project_id,
                    &run.id,
                    &run.task_id,
                    success.content,
                )
                .with_provenance(
                    success.artifact_provenance,
                    success.source_descriptor_id,
                    success.source_invocation_id,
                    success.trust_level,
                    success.knowledge_gate_status,
                );
                self.observation_store.insert_artifact(&artifact).await?;
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_ARTIFACT_CREATED,
                        "Execution artifact persisted",
                    ))
                    .await?;
                self.observation_store
                    .write_trace(&TraceRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        TRACE_STAGE_ARTIFACT_STORE,
                        run.attempt_count,
                        "Execution artifact stored",
                    ))
                    .await?;

                run.status = "completed".to_string();
                run.resume_token = None;
                run.last_error = None;
                run.completed_at = Some(current_timestamp());
                run.updated_at = run.completed_at.clone().unwrap();
                run.checkpoint_seq += 1;
                self.update_run(&run).await?;
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_RUN_COMPLETED,
                        "Run completed successfully",
                    ))
                    .await?;
                let _ = self
                    .knowledge_manager
                    .capture_from_artifact(&run, task, &artifact)
                    .await?;
            }
            ActionExecutionOutcome::Failed(failure) => {
                self.observation_store
                    .write_trace(&TraceRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        TRACE_STAGE_EXECUTION_ACTION,
                        run.attempt_count,
                        format!("Execution action failed: {}", failure.message),
                    ))
                    .await?;

                run.status = "failed".to_string();
                run.last_error = Some(failure.message.clone());
                run.resume_token = if failure.retryable && run.attempt_count < run.max_attempts {
                    Some(format!("resume:{}:{}", run.id, run.attempt_count + 1))
                } else {
                    None
                };
                run.updated_at = current_timestamp();
                run.checkpoint_seq += 1;
                self.update_run(&run).await?;
                self.observation_store
                    .write_audit(&AuditRecord::new(
                        &run.workspace_id,
                        &run.project_id,
                        &run.id,
                        &run.task_id,
                        AUDIT_EVENT_RUN_FAILED,
                        format!("Run failed: {}", failure.message),
                    ))
                    .await?;
            }
        }

        self.load_run_report(&run.id).await
    }

    async fn record_policy_decision(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
        decision: &str,
        reason: &str,
        approval_request_id: Option<String>,
    ) -> Result<(), RuntimeError> {
        self.observation_store
            .insert_policy_decision(&PolicyDecisionLogRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                &task.capability_id,
                decision,
                reason,
                task.estimated_cost,
                approval_request_id,
            ))
            .await?;
        self.observation_store
            .write_trace(&TraceRecord::new(
                &run.workspace_id,
                &run.project_id,
                &run.id,
                &run.task_id,
                TRACE_STAGE_GOVERNANCE_EVALUATION,
                std::cmp::max(run.attempt_count, 1),
                format!("Governance decision: {decision} ({reason})"),
            ))
            .await?;
        Ok(())
    }

    async fn resolve_inbox_for_approval(
        &self,
        run_id: &str,
        approval_request_id: &str,
    ) -> Result<(), RuntimeError> {
        let inbox_items = self
            .observation_store
            .list_inbox_items_by_run(run_id)
            .await?;
        for mut item in inbox_items {
            if item.approval_request_id == approval_request_id && item.status != "resolved" {
                item.mark_resolved();
                self.observation_store.upsert_inbox_item(&item).await?;
            }
        }
        Ok(())
    }

    async fn sync_governance_surface_records(
        &self,
        approval: &ApprovalRequestRecord,
        title: &str,
        message: &str,
    ) -> Result<(), RuntimeError> {
        let mut inbox = InboxItemRecord::approval_request(
            &approval.workspace_id,
            &approval.project_id,
            &approval.run_id,
            &approval.id,
            &approval.target_ref,
            title,
            message,
        );
        inbox.status = "open".to_string();
        self.observation_store.upsert_inbox_item(&inbox).await?;

        let notification = NotificationRecord::approval_request(
            &approval.workspace_id,
            &approval.project_id,
            &approval.run_id,
            &approval.id,
            &approval.target_ref,
            title,
            message,
        );
        self.observation_store
            .upsert_notification(&notification)
            .await?;

        Ok(())
    }

    async fn insert_run(&self, run: &RunRecord) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            INSERT INTO runs (
                id, task_id, workspace_id, project_id, automation_id, trigger_delivery_id,
                run_type, status, approval_request_id, idempotency_key, attempt_count,
                max_attempts, checkpoint_seq, resume_token, last_error, created_at, updated_at,
                started_at, completed_at, terminated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            "#,
        )
        .bind(&run.id)
        .bind(&run.task_id)
        .bind(&run.workspace_id)
        .bind(&run.project_id)
        .bind(&run.automation_id)
        .bind(&run.trigger_delivery_id)
        .bind(&run.run_type)
        .bind(&run.status)
        .bind(&run.approval_request_id)
        .bind(&run.idempotency_key)
        .bind(run.attempt_count)
        .bind(run.max_attempts)
        .bind(run.checkpoint_seq)
        .bind(&run.resume_token)
        .bind(&run.last_error)
        .bind(&run.created_at)
        .bind(&run.updated_at)
        .bind(&run.started_at)
        .bind(&run.completed_at)
        .bind(&run.terminated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_run(&self, run: &RunRecord) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE runs
            SET automation_id = ?2,
                trigger_delivery_id = ?3,
                status = ?4,
                approval_request_id = ?5,
                attempt_count = ?6,
                max_attempts = ?7,
                checkpoint_seq = ?8,
                resume_token = ?9,
                last_error = ?10,
                updated_at = ?11,
                started_at = ?12,
                completed_at = ?13,
                terminated_at = ?14
            WHERE id = ?1
            "#,
        )
        .bind(&run.id)
        .bind(&run.automation_id)
        .bind(&run.trigger_delivery_id)
        .bind(&run.status)
        .bind(&run.approval_request_id)
        .bind(run.attempt_count)
        .bind(run.max_attempts)
        .bind(run.checkpoint_seq)
        .bind(&run.resume_token)
        .bind(&run.last_error)
        .bind(&run.updated_at)
        .bind(&run.started_at)
        .bind(&run.completed_at)
        .bind(&run.terminated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fetch_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<RunRecord>, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, task_id, workspace_id, project_id, automation_id, trigger_delivery_id,
                run_type, status, approval_request_id, idempotency_key, attempt_count,
                max_attempts, checkpoint_seq, resume_token, last_error, created_at, updated_at,
                started_at, completed_at, terminated_at
            FROM runs
            WHERE idempotency_key = ?1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| run_from_row(&row)).transpose()
    }
}

fn task_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<TaskRecord, RuntimeError> {
    let action_json: String = row.try_get("action_json")?;
    Ok(TaskRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        source_kind: row.try_get("source_kind")?,
        automation_id: row.try_get("automation_id")?,
        title: row.try_get("title")?,
        instruction: row.try_get("instruction")?,
        action: serde_json::from_str(&action_json)?,
        capability_id: row.try_get("capability_id")?,
        estimated_cost: row.try_get("estimated_cost")?,
        idempotency_key: row.try_get("idempotency_key")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn automation_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AutomationRecord, RuntimeError> {
    let action_json: String = row.try_get("action_json")?;
    Ok(AutomationRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        trigger_id: row.try_get("trigger_id")?,
        status: row.try_get("status")?,
        title: row.try_get("title")?,
        instruction: row.try_get("instruction")?,
        action: serde_json::from_str(&action_json)?,
        capability_id: row.try_get("capability_id")?,
        estimated_cost: row.try_get("estimated_cost")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn trigger_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<TriggerRecord, RuntimeError> {
    let trigger_id: String = row.try_get("id")?;
    let trigger_type: String = row.try_get("trigger_type")?;
    let spec = match trigger_type.as_str() {
        "manual_event" => TriggerSpec::manual_event(),
        "cron" => TriggerSpec::Cron {
            config: CronTriggerConfig {
                schedule: row.try_get("schedule")?,
                timezone: row.try_get("timezone")?,
                next_fire_at: row.try_get("next_fire_at")?,
            },
        },
        "webhook" => {
            let secret_hash: Option<String> = row.try_get("webhook_secret_hash")?;
            TriggerSpec::Webhook {
                config: WebhookTriggerConfig {
                    ingress_mode: row.try_get("ingress_mode")?,
                    secret_header_name: row.try_get("secret_header_name")?,
                    secret_hint: row.try_get("secret_hint")?,
                    secret_present: secret_hash.is_some(),
                    secret_hash,
                },
            }
        }
        "mcp_event" => TriggerSpec::McpEvent {
            config: McpEventTriggerConfig {
                server_id: row.try_get("server_id")?,
                event_name: row.try_get("event_name")?,
                event_pattern: row.try_get("event_pattern")?,
            },
        },
        other => {
            return Err(RuntimeError::InvalidTriggerType {
                trigger_id,
                trigger_type: other.to_string(),
            });
        }
    };

    Ok(TriggerRecord {
        id: trigger_id,
        automation_id: row.try_get("automation_id")?,
        spec,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn trigger_delivery_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<TriggerDeliveryRecord, RuntimeError> {
    let payload_json: String = row.try_get("payload_json")?;
    Ok(TriggerDeliveryRecord {
        id: row.try_get("id")?,
        trigger_id: row.try_get("trigger_id")?,
        run_id: row.try_get("run_id")?,
        status: row.try_get("status")?,
        dedupe_key: row.try_get("dedupe_key")?,
        payload: serde_json::from_str(&payload_json)?,
        attempt_count: row.try_get("attempt_count")?,
        last_error: row.try_get("last_error")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn run_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<RunRecord, RuntimeError> {
    Ok(RunRecord {
        id: row.try_get("id")?,
        task_id: row.try_get("task_id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        automation_id: row.try_get("automation_id")?,
        trigger_delivery_id: row.try_get("trigger_delivery_id")?,
        run_type: row.try_get("run_type")?,
        status: row.try_get("status")?,
        approval_request_id: row.try_get("approval_request_id")?,
        idempotency_key: row.try_get("idempotency_key")?,
        attempt_count: row.try_get("attempt_count")?,
        max_attempts: row.try_get("max_attempts")?,
        checkpoint_seq: row.try_get("checkpoint_seq")?,
        resume_token: row.try_get("resume_token")?,
        last_error: row.try_get("last_error")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        terminated_at: row.try_get("terminated_at")?,
    })
}

#[derive(Debug, Clone)]
struct ActionExecutionSuccess {
    content: String,
    artifact_provenance: String,
    source_descriptor_id: String,
    source_invocation_id: Option<String>,
    trust_level: String,
    knowledge_gate_status: String,
}

#[derive(Debug, Clone)]
struct ActionExecutionFailure {
    message: String,
    retryable: bool,
}

#[derive(Debug, Clone)]
enum ActionExecutionOutcome {
    Succeeded(ActionExecutionSuccess),
    Failed(ActionExecutionFailure),
}

impl ActionExecutionOutcome {
    fn from_builtin(outcome: ExecutionOutcome, capability_id: &str) -> Self {
        match outcome {
            ExecutionOutcome::Succeeded(success) => Self::Succeeded(ActionExecutionSuccess {
                content: success.content,
                artifact_provenance: "builtin".to_string(),
                source_descriptor_id: capability_id.to_string(),
                source_invocation_id: None,
                trust_level: "trusted".to_string(),
                knowledge_gate_status: KNOWLEDGE_GATE_ELIGIBLE.to_string(),
            }),
            ExecutionOutcome::Failed(failure) => Self::Failed(ActionExecutionFailure {
                message: failure.message,
                retryable: failure.retryable,
            }),
        }
    }

    fn from_gateway(outcome: GatewayExecutionOutcome) -> Self {
        match outcome {
            GatewayExecutionOutcome::Succeeded(success) => {
                Self::Succeeded(ActionExecutionSuccess {
                    content: success.content,
                    artifact_provenance: success.artifact_metadata.provenance_source,
                    source_descriptor_id: success.artifact_metadata.source_descriptor_id,
                    source_invocation_id: Some(success.artifact_metadata.source_invocation_id),
                    trust_level: success.artifact_metadata.trust_level,
                    knowledge_gate_status: success.artifact_metadata.knowledge_gate_status,
                })
            }
            GatewayExecutionOutcome::Failed(failure) => Self::Failed(ActionExecutionFailure {
                message: failure.message,
                retryable: failure.retryable,
            }),
        }
    }
}

fn knowledge_candidate_target_ref(candidate_id: &str) -> String {
    format!("knowledge_candidate:{candidate_id}")
}
