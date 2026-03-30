mod database;
mod models;
mod services;

use std::path::Path;

use database::Slice1Database;
use services::{AutomationIntake, KnowledgeManager, RunOrchestrator, TaskIntake};
use thiserror::Error;

pub use models::{
    AssetKnowledgeSummaryRecord, AutomationDetailRecord, AutomationRecord, AutomationSummaryRecord,
    CandidateKnowledgeSummaryRecord, CreateAutomationInput, CreateAutomationReport,
    CreateTaskInput, CreateTriggerInput, CronTriggerConfig, DispatchManualEventInput,
    DispatchMcpEventInput, DispatchWebhookEventInput, KnowledgePromotionReport,
    KnowledgeSummaryRecord, ManualEventTriggerConfig, McpEventTriggerConfig,
    ModelSelectionDecisionRecord, ProjectKnowledgeIndexRecord, RunExecutionReport, RunRecord,
    RunSummaryRecord, TaskRecord, TriggerDeliveryRecord, TriggerDeliveryReport, TriggerRecord,
    TriggerSpec, WebhookTriggerConfig,
};
pub use octopus_domain_context::{ProjectContext, ProjectRecord};
pub use octopus_governance::{
    ApprovalDecision, ApprovalRequestRecord, BudgetPolicyRecord, CapabilityBindingRecord,
    CapabilityDescriptorRecord, CapabilityGrantRecord, CapabilityResolutionRecord,
    ModelCatalogItemRecord, ModelProfileRecord, ModelProviderRecord, TenantModelPolicyRecord,
};
pub use octopus_interop_mcp::{
    EnvironmentLeaseRecord, McpCredentialRecord, McpInvocationRecord, McpServerRecord,
};
pub use octopus_knowledge::{KnowledgeAssetRecord, KnowledgeCandidateRecord, KnowledgeSpaceRecord};
pub use octopus_observe_artifact::{
    ArtifactRecord, AuditRecord, InboxItemRecord, KnowledgeLineageRecord, NotificationRecord,
    PolicyDecisionLogRecord, TraceRecord,
};

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("task `{0}` not found")]
    TaskNotFound(String),
    #[error("run `{0}` not found")]
    RunNotFound(String),
    #[error("invalid run transition for `{run_id}`: `{from}` -> `{to}`")]
    InvalidRunTransition {
        run_id: String,
        from: String,
        to: String,
    },
    #[error("approval request `{0}` not found")]
    ApprovalRequestNotFound(String),
    #[error("automation `{0}` not found")]
    AutomationNotFound(String),
    #[error("automation `{automation_id}` cannot transition from `{from}` to `{to}`")]
    InvalidAutomationLifecycleTransition {
        automation_id: String,
        from: String,
        to: String,
    },
    #[error("trigger `{0}` not found")]
    TriggerNotFound(String),
    #[error("trigger delivery `{0}` not found")]
    TriggerDeliveryNotFound(String),
    #[error("trigger `{trigger_id}` has unsupported type `{trigger_type}`")]
    InvalidTriggerType {
        trigger_id: String,
        trigger_type: String,
    },
    #[error("cron trigger `{trigger_id}` has invalid schedule `{schedule}`")]
    InvalidCronSchedule {
        trigger_id: String,
        schedule: String,
    },
    #[error("webhook event for trigger `{trigger_id}` requires a non-empty idempotency key")]
    MissingWebhookIdempotencyKey { trigger_id: String },
    #[error("webhook event for trigger `{trigger_id}` has invalid secret")]
    InvalidWebhookSecret { trigger_id: String },
    #[error("mcp server `{0}` not found")]
    McpServerNotFound(String),
    #[error("mcp event `{event_name}` does not match trigger `{trigger_id}` selector")]
    McpEventMismatch {
        trigger_id: String,
        event_name: String,
    },
    #[error("trigger delivery `{delivery_id}` cannot transition from `{from}` to `{to}`")]
    InvalidTriggerDeliveryTransition {
        delivery_id: String,
        from: String,
        to: String,
    },
    #[error("knowledge candidate `{0}` not found")]
    KnowledgeCandidateNotFound(String),
    #[error("{0}")]
    KnowledgeSpaceNotFound(String),
    #[error(
        "knowledge candidate `{candidate_id}` has invalid state `{status}`; expected `{expected}`"
    )]
    InvalidKnowledgeCandidateState {
        candidate_id: String,
        status: String,
        expected: String,
    },
    #[error("approval request `{approval_id}` has invalid approval type `{approval_type}`")]
    InvalidApprovalType {
        approval_id: String,
        approval_type: String,
    },
    #[error("approval request `{approval_id}` has invalid target ref `{target_ref}`")]
    InvalidApprovalTargetRef {
        approval_id: String,
        target_ref: String,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Context(#[from] octopus_domain_context::ContextStoreError),
    #[error(transparent)]
    Governance(#[from] octopus_governance::GovernanceStoreError),
    #[error(transparent)]
    Interop(#[from] octopus_interop_mcp::InteropStoreError),
    #[error(transparent)]
    Knowledge(#[from] octopus_knowledge::KnowledgeStoreError),
    #[error(transparent)]
    Observation(#[from] octopus_observe_artifact::ObservationStoreError),
}

#[derive(Debug, Clone)]
pub struct Slice2Runtime {
    task_intake: TaskIntake,
    automation_intake: AutomationIntake,
    knowledge_manager: KnowledgeManager,
    run_orchestrator: RunOrchestrator,
}

pub type Slice1Runtime = Slice2Runtime;

impl Slice2Runtime {
    pub async fn open_at(path: &Path) -> Result<Self, RuntimeError> {
        let database = Slice1Database::open_at(path).await?;
        let knowledge_manager = KnowledgeManager::new(
            database.knowledge_store().clone(),
            database.observation_store().clone(),
        );
        Ok(Self {
            task_intake: TaskIntake::new(database.pool().clone(), database.context_store().clone()),
            automation_intake: AutomationIntake::new(
                database.pool().clone(),
                database.context_store().clone(),
            ),
            knowledge_manager: knowledge_manager.clone(),
            run_orchestrator: RunOrchestrator::new(
                database.pool().clone(),
                database.governance_store().clone(),
                database.interop_store().clone(),
                knowledge_manager,
                database.observation_store().clone(),
            ),
        })
    }

    pub async fn ensure_project_context(
        &self,
        workspace_id: &str,
        workspace_slug: &str,
        workspace_display_name: &str,
        project_id: &str,
        project_slug: &str,
        project_display_name: &str,
    ) -> Result<octopus_domain_context::ProjectContext, RuntimeError> {
        self.task_intake
            .ensure_project_context(
                workspace_id,
                workspace_slug,
                workspace_display_name,
                project_id,
                project_slug,
                project_display_name,
            )
            .await
    }

    pub async fn upsert_capability_descriptor(
        &self,
        record: CapabilityDescriptorRecord,
    ) -> Result<(), RuntimeError> {
        self.run_orchestrator
            .upsert_capability_descriptor(record)
            .await
    }

    pub async fn upsert_capability_binding(
        &self,
        record: CapabilityBindingRecord,
    ) -> Result<(), RuntimeError> {
        self.run_orchestrator
            .upsert_capability_binding(record)
            .await
    }

    pub async fn upsert_capability_grant(
        &self,
        record: CapabilityGrantRecord,
    ) -> Result<(), RuntimeError> {
        self.run_orchestrator.upsert_capability_grant(record).await
    }

    pub async fn upsert_budget_policy(
        &self,
        record: BudgetPolicyRecord,
    ) -> Result<(), RuntimeError> {
        self.run_orchestrator.upsert_budget_policy(record).await
    }

    pub async fn upsert_mcp_server(&self, record: McpServerRecord) -> Result<(), RuntimeError> {
        self.run_orchestrator.upsert_mcp_server(record).await
    }

    pub async fn upsert_mcp_credential(
        &self,
        record: McpCredentialRecord,
        secret: &str,
    ) -> Result<(), RuntimeError> {
        self.run_orchestrator
            .upsert_mcp_credential(record, secret)
            .await
    }

    pub async fn create_task(&self, input: CreateTaskInput) -> Result<TaskRecord, RuntimeError> {
        self.task_intake.create_task(input).await
    }

    pub async fn fetch_project_context(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<octopus_domain_context::ProjectContext, RuntimeError> {
        self.task_intake
            .fetch_project_context(workspace_id, project_id)
            .await
    }

    pub async fn list_projects(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<octopus_domain_context::ProjectRecord>, RuntimeError> {
        self.task_intake.list_projects(workspace_id).await
    }

    pub async fn fetch_task(&self, task_id: &str) -> Result<TaskRecord, RuntimeError> {
        self.task_intake.fetch_task(task_id).await
    }

    pub async fn ensure_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
        display_name: &str,
        owner_ref: &str,
    ) -> Result<KnowledgeSpaceRecord, RuntimeError> {
        self.knowledge_manager
            .ensure_project_knowledge_space(workspace_id, project_id, display_name, owner_ref)
            .await
    }

    pub async fn create_automation(
        &self,
        input: CreateAutomationInput,
    ) -> Result<AutomationRecord, RuntimeError> {
        Ok(self
            .automation_intake
            .create_automation_with_trigger(input, CreateTriggerInput::manual_event())
            .await?
            .automation)
    }

    pub async fn create_automation_with_trigger(
        &self,
        input: CreateAutomationInput,
        trigger: CreateTriggerInput,
    ) -> Result<CreateAutomationReport, RuntimeError> {
        self.automation_intake
            .create_automation_with_trigger(input, trigger)
            .await
    }

    pub async fn fetch_automation(
        &self,
        automation_id: &str,
    ) -> Result<Option<AutomationRecord>, RuntimeError> {
        self.automation_intake.fetch_automation(automation_id).await
    }

    pub async fn list_automations(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<AutomationSummaryRecord>, RuntimeError> {
        self.automation_intake
            .list_automations(workspace_id, project_id)
            .await
    }

    pub async fn load_automation_detail(
        &self,
        automation_id: &str,
    ) -> Result<AutomationDetailRecord, RuntimeError> {
        self.automation_intake
            .load_automation_detail(automation_id)
            .await
    }

    pub async fn activate_automation(
        &self,
        automation_id: &str,
    ) -> Result<AutomationRecord, RuntimeError> {
        self.automation_intake
            .transition_automation_status(automation_id, "active")
            .await
    }

    pub async fn pause_automation(
        &self,
        automation_id: &str,
    ) -> Result<AutomationRecord, RuntimeError> {
        self.automation_intake
            .transition_automation_status(automation_id, "paused")
            .await
    }

    pub async fn archive_automation(
        &self,
        automation_id: &str,
    ) -> Result<AutomationRecord, RuntimeError> {
        self.automation_intake
            .transition_automation_status(automation_id, "archived")
            .await
    }

    pub async fn fetch_trigger(
        &self,
        trigger_id: &str,
    ) -> Result<Option<TriggerRecord>, RuntimeError> {
        self.automation_intake.fetch_trigger(trigger_id).await
    }

    pub async fn fetch_trigger_delivery(
        &self,
        delivery_id: &str,
    ) -> Result<Option<TriggerDeliveryRecord>, RuntimeError> {
        self.automation_intake
            .fetch_trigger_delivery(delivery_id)
            .await
    }

    pub async fn start_task(&self, task_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        let task = self.task_intake.fetch_task(task_id).await?;
        self.run_orchestrator.start_task(&task).await
    }

    pub async fn list_runs(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<RunSummaryRecord>, RuntimeError> {
        self.run_orchestrator
            .list_runs(workspace_id, project_id)
            .await
    }

    pub async fn dispatch_manual_event(
        &self,
        input: DispatchManualEventInput,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let (trigger, automation) = self.load_trigger_and_automation(&input.trigger_id).await?;
        if !matches!(trigger.spec, TriggerSpec::ManualEvent { .. }) {
            return Err(RuntimeError::InvalidTriggerType {
                trigger_id: trigger.id.clone(),
                trigger_type: trigger.trigger_type().to_string(),
            });
        }
        self.dispatch_trigger_delivery(automation, trigger, input.dedupe_key, input.payload)
            .await
    }

    pub async fn dispatch_webhook_event(
        &self,
        input: DispatchWebhookEventInput,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let (trigger, automation) = self.load_trigger_and_automation(&input.trigger_id).await?;
        let Some(config) = trigger.spec.webhook_config() else {
            return Err(RuntimeError::InvalidTriggerType {
                trigger_id: trigger.id.clone(),
                trigger_type: trigger.trigger_type().to_string(),
            });
        };
        if input.idempotency_key.trim().is_empty() {
            return Err(RuntimeError::MissingWebhookIdempotencyKey {
                trigger_id: trigger.id.clone(),
            });
        }
        if !self
            .automation_intake
            .verify_webhook_secret(config, &input.secret)
        {
            return Err(RuntimeError::InvalidWebhookSecret {
                trigger_id: trigger.id.clone(),
            });
        }
        let dedupe_key = format!("webhook:{}:{}", trigger.id, input.idempotency_key);
        self.dispatch_trigger_delivery(automation, trigger, dedupe_key, input.payload)
            .await
    }

    pub async fn dispatch_mcp_event(
        &self,
        input: DispatchMcpEventInput,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let (trigger, automation) = self.load_trigger_and_automation(&input.trigger_id).await?;
        let Some(config) = trigger.spec.mcp_event_config() else {
            return Err(RuntimeError::InvalidTriggerType {
                trigger_id: trigger.id.clone(),
                trigger_type: trigger.trigger_type().to_string(),
            });
        };
        self.ensure_mcp_server_exists(&input.server_id).await?;
        if config.server_id != input.server_id
            || !self
                .automation_intake
                .mcp_event_matches(config, &input.event_name)
        {
            return Err(RuntimeError::McpEventMismatch {
                trigger_id: trigger.id.clone(),
                event_name: input.event_name,
            });
        }
        let dedupe_key = format!("mcp_event:{}:{}", trigger.id, input.dedupe_key);
        self.dispatch_trigger_delivery(automation, trigger, dedupe_key, input.payload)
            .await
    }

    pub async fn tick_due_triggers(
        &self,
        now: &str,
    ) -> Result<Vec<TriggerDeliveryReport>, RuntimeError> {
        let due_triggers = self.automation_intake.list_due_cron_triggers(now).await?;
        let mut reports = Vec::with_capacity(due_triggers.len());

        for trigger in due_triggers {
            let Some(config) = trigger.spec.cron_config() else {
                continue;
            };
            let scheduled_at = config.next_fire_at.clone();
            let automation = self
                .automation_intake
                .fetch_automation(&trigger.automation_id)
                .await?
                .ok_or_else(|| RuntimeError::AutomationNotFound(trigger.automation_id.clone()))?;
            let next_fire_at = self.automation_intake.compute_next_cron_fire(
                &trigger.id,
                config,
                &scheduled_at,
            )?;
            self.automation_intake
                .update_cron_next_fire_at(&trigger.id, &next_fire_at)
                .await?;
            let refreshed_trigger = self
                .automation_intake
                .fetch_trigger(&trigger.id)
                .await?
                .ok_or_else(|| RuntimeError::TriggerNotFound(trigger.id.clone()))?;
            let dedupe_key = format!("cron:{}:{}", trigger.id, scheduled_at);
            let payload = serde_json::json!({
                "trigger_type": "cron",
                "scheduled_at": scheduled_at,
                "observed_at": now,
            });
            reports.push(
                self.dispatch_trigger_delivery(automation, refreshed_trigger, dedupe_key, payload)
                    .await?,
            );
        }

        Ok(reports)
    }

    pub async fn resolve_approval(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
        actor_ref: &str,
        note: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let approval = self
            .run_orchestrator
            .fetch_approval_request(approval_id)
            .await?
            .ok_or_else(|| RuntimeError::ApprovalRequestNotFound(approval_id.to_string()))?;
        let task = self.task_intake.fetch_task(&approval.task_id).await?;
        let report = self
            .run_orchestrator
            .resolve_approval(approval_id, &task, decision, actor_ref, note)
            .await?;
        if let Some(trigger_delivery_id) = report.run.trigger_delivery_id.clone() {
            self.automation_intake
                .sync_delivery_from_run(&trigger_delivery_id, &report.run)
                .await?;
        }
        Ok(report)
    }

    pub async fn fetch_approval_request(
        &self,
        approval_id: &str,
    ) -> Result<Option<ApprovalRequestRecord>, RuntimeError> {
        self.run_orchestrator
            .fetch_approval_request(approval_id)
            .await
    }

    pub async fn request_knowledge_promotion(
        &self,
        candidate_id: &str,
        actor_ref: &str,
        note: &str,
    ) -> Result<ApprovalRequestRecord, RuntimeError> {
        self.run_orchestrator
            .request_knowledge_promotion(candidate_id, actor_ref, note)
            .await
    }

    pub async fn retry_run(&self, run_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        let run = self
            .run_orchestrator
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;
        let task = self.task_intake.fetch_task(&run.task_id).await?;
        let report = self.run_orchestrator.retry_run(run_id, &task).await?;
        if let Some(trigger_delivery_id) = report.run.trigger_delivery_id.clone() {
            self.automation_intake
                .sync_delivery_from_run(&trigger_delivery_id, &report.run)
                .await?;
        }
        Ok(report)
    }

    pub async fn retry_trigger_delivery(
        &self,
        delivery_id: &str,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let delivery = self
            .automation_intake
            .fetch_trigger_delivery(delivery_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerDeliveryNotFound(delivery_id.to_string()))?;
        if delivery.status != "failed" {
            return Err(RuntimeError::InvalidTriggerDeliveryTransition {
                delivery_id: delivery.id,
                from: delivery.status,
                to: "retry_scheduled".to_string(),
            });
        }
        let trigger = self
            .automation_intake
            .fetch_trigger(&delivery.trigger_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerNotFound(delivery.trigger_id.clone()))?;
        let automation = self
            .automation_intake
            .fetch_automation(&trigger.automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(trigger.automation_id.clone()))?;
        let run_id = delivery.run_id.clone().ok_or_else(|| {
            RuntimeError::InvalidTriggerDeliveryTransition {
                delivery_id: delivery.id.clone(),
                from: delivery.status.clone(),
                to: "retry_scheduled".to_string(),
            }
        })?;
        let run = self
            .run_orchestrator
            .fetch_run(&run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.clone()))?;
        if !run.can_retry() {
            return Err(RuntimeError::InvalidTriggerDeliveryTransition {
                delivery_id: delivery.id.clone(),
                from: delivery.status.clone(),
                to: "retry_scheduled".to_string(),
            });
        }
        let task = self.task_intake.fetch_task(&run.task_id).await?;
        self.automation_intake
            .mark_delivery_retry_scheduled(delivery)
            .await?;
        let run_report = self.run_orchestrator.retry_run(&run_id, &task).await?;
        let delivery = self
            .automation_intake
            .sync_delivery_from_run(delivery_id, &run_report.run)
            .await?;

        Ok(TriggerDeliveryReport {
            automation,
            trigger,
            delivery,
            task,
            run_report,
        })
    }

    pub async fn terminate_run(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let run = self.run_orchestrator.terminate_run(run_id, reason).await?;
        if let Some(trigger_delivery_id) = run.trigger_delivery_id.clone() {
            self.automation_intake
                .sync_delivery_from_run(&trigger_delivery_id, &run)
                .await?;
        }
        self.run_orchestrator.load_run_report(&run.id).await
    }

    pub async fn fetch_run(&self, run_id: &str) -> Result<Option<RunRecord>, RuntimeError> {
        self.run_orchestrator.fetch_run(run_id).await
    }

    pub async fn load_run_report(&self, run_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        self.run_orchestrator.load_run_report(run_id).await
    }

    pub async fn list_model_providers(&self) -> Result<Vec<ModelProviderRecord>, RuntimeError> {
        self.run_orchestrator.list_model_providers().await
    }

    pub async fn list_model_catalog_items(
        &self,
    ) -> Result<Vec<ModelCatalogItemRecord>, RuntimeError> {
        self.run_orchestrator.list_model_catalog_items().await
    }

    pub async fn list_model_profiles(&self) -> Result<Vec<ModelProfileRecord>, RuntimeError> {
        self.run_orchestrator.list_model_profiles().await
    }

    pub async fn get_workspace_model_policy(
        &self,
        workspace_id: &str,
    ) -> Result<Option<TenantModelPolicyRecord>, RuntimeError> {
        self.run_orchestrator
            .fetch_tenant_model_policy(workspace_id)
            .await
    }

    pub async fn record_model_selection_decision(
        &self,
        record: ModelSelectionDecisionRecord,
    ) -> Result<ModelSelectionDecisionRecord, RuntimeError> {
        self.run_orchestrator
            .record_model_selection_decision(record)
            .await
    }

    pub async fn fetch_model_selection_decision_by_run(
        &self,
        run_id: &str,
    ) -> Result<Option<ModelSelectionDecisionRecord>, RuntimeError> {
        self.run_orchestrator
            .fetch_model_selection_decision_by_run(run_id)
            .await
    }

    pub async fn promote_knowledge_candidate(
        &self,
        candidate_id: &str,
        actor_ref: &str,
        note: &str,
    ) -> Result<KnowledgePromotionReport, RuntimeError> {
        self.knowledge_manager
            .promote_knowledge_candidate(candidate_id, actor_ref, note)
            .await
    }

    pub async fn retry_knowledge_capture(
        &self,
        run_id: &str,
    ) -> Result<RunExecutionReport, RuntimeError> {
        let run = self
            .run_orchestrator
            .fetch_run(run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;
        let task = self.task_intake.fetch_task(&run.task_id).await?;
        let artifacts = self.run_orchestrator.list_artifacts_by_run(run_id).await?;
        let artifact = artifacts
            .into_iter()
            .find(|artifact| artifact.artifact_type == "execution_output")
            .ok_or_else(|| RuntimeError::RunNotFound(run_id.to_string()))?;
        self.knowledge_manager
            .retry_capture(&run, &task, &artifact)
            .await?;
        self.load_run_report(run_id).await
    }

    pub async fn list_approval_requests_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ApprovalRequestRecord>, RuntimeError> {
        self.run_orchestrator
            .list_approval_requests_by_run(run_id)
            .await
    }

    pub async fn list_artifacts_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ArtifactRecord>, RuntimeError> {
        self.run_orchestrator.list_artifacts_by_run(run_id).await
    }

    pub async fn list_capability_resolutions(
        &self,
        workspace_id: &str,
        project_id: &str,
        estimated_cost: i64,
    ) -> Result<Vec<CapabilityResolutionRecord>, RuntimeError> {
        self.run_orchestrator
            .list_capability_resolutions(workspace_id, project_id, estimated_cost)
            .await
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerRecord>, RuntimeError> {
        self.run_orchestrator.list_mcp_servers().await
    }

    pub async fn list_mcp_credentials(&self) -> Result<Vec<McpCredentialRecord>, RuntimeError> {
        self.run_orchestrator.list_mcp_credentials().await
    }

    pub async fn list_mcp_invocations_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<McpInvocationRecord>, RuntimeError> {
        self.run_orchestrator
            .list_mcp_invocations_by_run(run_id)
            .await
    }

    pub async fn list_environment_leases_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<EnvironmentLeaseRecord>, RuntimeError> {
        self.run_orchestrator
            .list_environment_leases_by_run(run_id)
            .await
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
        self.run_orchestrator
            .request_environment_lease(
                run_id,
                task_id,
                capability_id,
                environment_type,
                sandbox_tier,
                ttl_seconds,
            )
            .await
    }

    pub async fn heartbeat_environment_lease(
        &self,
        lease_id: &str,
        ttl_seconds: i64,
    ) -> Result<EnvironmentLeaseRecord, RuntimeError> {
        self.run_orchestrator
            .heartbeat_environment_lease(lease_id, ttl_seconds)
            .await
    }

    pub async fn release_environment_lease(
        &self,
        lease_id: &str,
    ) -> Result<EnvironmentLeaseRecord, RuntimeError> {
        self.run_orchestrator
            .release_environment_lease(lease_id)
            .await
    }

    pub async fn list_inbox_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<InboxItemRecord>, RuntimeError> {
        self.run_orchestrator
            .list_inbox_items_by_workspace(workspace_id)
            .await
    }

    pub async fn list_notifications_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<NotificationRecord>, RuntimeError> {
        self.run_orchestrator
            .list_notifications_by_workspace(workspace_id)
            .await
    }

    pub async fn list_policy_decisions_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<PolicyDecisionLogRecord>, RuntimeError> {
        self.run_orchestrator
            .list_policy_decisions_by_run(run_id)
            .await
    }

    pub async fn list_knowledge_lineage_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeLineageRecord>, RuntimeError> {
        self.knowledge_manager
            .list_knowledge_lineage_by_run(run_id)
            .await
    }

    pub async fn fetch_project_knowledge_space(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<KnowledgeSpaceRecord>, RuntimeError> {
        self.knowledge_manager
            .fetch_project_knowledge_space(workspace_id, project_id)
            .await
    }

    pub async fn get_project_knowledge_index(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<ProjectKnowledgeIndexRecord, RuntimeError> {
        self.knowledge_manager
            .get_project_knowledge_index(workspace_id, project_id)
            .await
    }

    pub async fn fetch_knowledge_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<KnowledgeCandidateRecord>, RuntimeError> {
        self.knowledge_manager
            .fetch_knowledge_candidate(candidate_id)
            .await
    }

    pub async fn list_knowledge_candidates_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeCandidateRecord>, RuntimeError> {
        self.knowledge_manager
            .list_knowledge_candidates_by_run(run_id)
            .await
    }

    pub async fn list_knowledge_assets_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<KnowledgeAssetRecord>, RuntimeError> {
        self.knowledge_manager
            .list_knowledge_assets_by_run(run_id)
            .await
    }

    pub async fn list_trigger_deliveries_by_automation(
        &self,
        automation_id: &str,
    ) -> Result<Vec<TriggerDeliveryRecord>, RuntimeError> {
        self.automation_intake
            .list_trigger_deliveries_by_automation(automation_id)
            .await
    }

    async fn load_trigger_and_automation(
        &self,
        trigger_id: &str,
    ) -> Result<(TriggerRecord, AutomationRecord), RuntimeError> {
        let trigger = self
            .automation_intake
            .fetch_trigger(trigger_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerNotFound(trigger_id.to_string()))?;
        let automation = self
            .automation_intake
            .fetch_automation(&trigger.automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(trigger.automation_id.clone()))?;
        Ok((trigger, automation))
    }

    async fn ensure_mcp_server_exists(&self, server_id: &str) -> Result<(), RuntimeError> {
        let servers = self.run_orchestrator.list_mcp_servers().await?;
        if servers.iter().any(|server| server.id == server_id) {
            return Ok(());
        }
        Err(RuntimeError::McpServerNotFound(server_id.to_string()))
    }

    async fn dispatch_trigger_delivery(
        &self,
        automation: AutomationRecord,
        trigger: TriggerRecord,
        dedupe_key: String,
        payload: serde_json::Value,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let mut delivery = if let Some(existing) = self
            .automation_intake
            .find_trigger_delivery_by_dedupe_key(&dedupe_key)
            .await?
        {
            if let Some(run_id) = existing.run_id.clone() {
                let run_report = self.run_orchestrator.load_run_report(&run_id).await?;
                let task = self.task_intake.fetch_task(&run_report.run.task_id).await?;
                return Ok(TriggerDeliveryReport {
                    automation,
                    trigger,
                    delivery: existing,
                    task,
                    run_report,
                });
            }
            existing
        } else {
            self.automation_intake
                .create_trigger_delivery(&trigger.id, &dedupe_key, payload)
                .await?
        };
        delivery = self
            .automation_intake
            .transition_delivery_to_delivering(delivery)
            .await?;

        let task = self
            .task_intake
            .create_task(CreateTaskInput {
                workspace_id: automation.workspace_id.clone(),
                project_id: automation.project_id.clone(),
                source_kind: "automation".into(),
                automation_id: Some(automation.id.clone()),
                title: automation.title.clone(),
                instruction: automation.instruction.clone(),
                action: automation.action.clone(),
                capability_id: automation.capability_id.clone(),
                estimated_cost: automation.estimated_cost,
                idempotency_key: format!("task:trigger_delivery:{}", delivery.id),
            })
            .await?;

        let run_report = self
            .run_orchestrator
            .start_task_with_trigger_delivery(&task, Some(delivery.id.as_str()))
            .await?;
        let delivery = self
            .automation_intake
            .sync_delivery_from_run(&delivery.id, &run_report.run)
            .await?;

        Ok(TriggerDeliveryReport {
            automation,
            trigger,
            delivery,
            task,
            run_report,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RunRecord;

    #[test]
    fn run_retry_requires_failed_status_resume_token_and_remaining_attempts() {
        let mut run = RunRecord {
            id: "run-1".into(),
            task_id: "task-1".into(),
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            automation_id: None,
            trigger_delivery_id: None,
            run_type: "task".into(),
            status: "failed".into(),
            approval_request_id: None,
            idempotency_key: "run:task:task-1".into(),
            attempt_count: 1,
            max_attempts: 2,
            checkpoint_seq: 2,
            resume_token: Some("resume:run-1:2".into()),
            last_error: Some("network_glitch".into()),
            created_at: "2026-03-26T10:00:00Z".into(),
            updated_at: "2026-03-26T10:00:01Z".into(),
            started_at: Some("2026-03-26T10:00:00Z".into()),
            completed_at: None,
            terminated_at: None,
        };

        assert!(run.can_retry());

        run.resume_token = None;
        assert!(!run.can_retry());
    }

    #[test]
    fn run_terminate_is_disallowed_after_completion() {
        let run = RunRecord {
            id: "run-1".into(),
            task_id: "task-1".into(),
            workspace_id: "workspace-alpha".into(),
            project_id: "project-alpha".into(),
            automation_id: None,
            trigger_delivery_id: None,
            run_type: "task".into(),
            status: "completed".into(),
            approval_request_id: None,
            idempotency_key: "run:task:task-1".into(),
            attempt_count: 1,
            max_attempts: 2,
            checkpoint_seq: 2,
            resume_token: None,
            last_error: None,
            created_at: "2026-03-26T10:00:00Z".into(),
            updated_at: "2026-03-26T10:00:01Z".into(),
            started_at: Some("2026-03-26T10:00:00Z".into()),
            completed_at: Some("2026-03-26T10:00:01Z".into()),
            terminated_at: None,
        };

        assert!(!run.can_terminate());
    }
}
