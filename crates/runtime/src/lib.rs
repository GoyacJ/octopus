mod database;
mod models;
mod services;

use std::path::Path;

use database::Slice1Database;
use octopus_observe_artifact::{
    ArtifactRecord, InboxItemRecord, NotificationRecord, PolicyDecisionLogRecord,
};
use services::{AutomationIntake, RunOrchestrator, TaskIntake};
use thiserror::Error;

pub use models::{
    AutomationRecord, CreateAutomationInput, CreateTaskInput, DispatchManualEventInput,
    RunExecutionReport, RunRecord, TaskRecord, TriggerDeliveryRecord, TriggerDeliveryReport,
    TriggerRecord,
};
pub use octopus_governance::{
    ApprovalDecision, ApprovalRequestRecord, BudgetPolicyRecord, CapabilityBindingRecord,
    CapabilityDescriptorRecord, CapabilityGrantRecord,
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
    #[error("trigger `{0}` not found")]
    TriggerNotFound(String),
    #[error("trigger delivery `{0}` not found")]
    TriggerDeliveryNotFound(String),
    #[error("trigger `{trigger_id}` has unsupported type `{trigger_type}`")]
    InvalidTriggerType { trigger_id: String, trigger_type: String },
    #[error("trigger delivery `{delivery_id}` cannot transition from `{from}` to `{to}`")]
    InvalidTriggerDeliveryTransition {
        delivery_id: String,
        from: String,
        to: String,
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
    Observation(#[from] octopus_observe_artifact::ObservationStoreError),
}

#[derive(Debug, Clone)]
pub struct Slice2Runtime {
    task_intake: TaskIntake,
    automation_intake: AutomationIntake,
    run_orchestrator: RunOrchestrator,
}

pub type Slice1Runtime = Slice2Runtime;

impl Slice2Runtime {
    pub async fn open_at(path: &Path) -> Result<Self, RuntimeError> {
        let database = Slice1Database::open_at(path).await?;
        Ok(Self {
            task_intake: TaskIntake::new(database.pool().clone(), database.context_store().clone()),
            automation_intake: AutomationIntake::new(
                database.pool().clone(),
                database.context_store().clone(),
            ),
            run_orchestrator: RunOrchestrator::new(
                database.pool().clone(),
                database.governance_store().clone(),
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

    pub async fn create_task(&self, input: CreateTaskInput) -> Result<TaskRecord, RuntimeError> {
        self.task_intake.create_task(input).await
    }

    pub async fn create_automation(
        &self,
        input: CreateAutomationInput,
    ) -> Result<AutomationRecord, RuntimeError> {
        self.automation_intake.create_automation(input).await
    }

    pub async fn fetch_automation(
        &self,
        automation_id: &str,
    ) -> Result<Option<AutomationRecord>, RuntimeError> {
        self.automation_intake.fetch_automation(automation_id).await
    }

    pub async fn start_task(&self, task_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        let task = self.task_intake.fetch_task(task_id).await?;
        self.run_orchestrator.start_task(&task).await
    }

    pub async fn dispatch_manual_event(
        &self,
        input: DispatchManualEventInput,
    ) -> Result<TriggerDeliveryReport, RuntimeError> {
        let trigger = self
            .automation_intake
            .fetch_trigger(&input.trigger_id)
            .await?
            .ok_or_else(|| RuntimeError::TriggerNotFound(input.trigger_id.clone()))?;
        if trigger.trigger_type != "manual_event" {
            return Err(RuntimeError::InvalidTriggerType {
                trigger_id: trigger.id,
                trigger_type: trigger.trigger_type,
            });
        }

        let automation = self
            .automation_intake
            .fetch_automation(&trigger.automation_id)
            .await?
            .ok_or_else(|| RuntimeError::AutomationNotFound(trigger.automation_id.clone()))?;

        let mut delivery = if let Some(existing) = self
            .automation_intake
            .find_trigger_delivery_by_dedupe_key(&input.dedupe_key)
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
                .create_trigger_delivery(&input.trigger_id, &input.dedupe_key, input.payload)
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
    ) -> Result<RunRecord, RuntimeError> {
        self.run_orchestrator.terminate_run(run_id, reason).await
    }

    pub async fn fetch_run(&self, run_id: &str) -> Result<Option<RunRecord>, RuntimeError> {
        self.run_orchestrator.fetch_run(run_id).await
    }

    pub async fn load_run_report(&self, run_id: &str) -> Result<RunExecutionReport, RuntimeError> {
        self.run_orchestrator.load_run_report(run_id).await
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

    pub async fn list_trigger_deliveries_by_automation(
        &self,
        automation_id: &str,
    ) -> Result<Vec<TriggerDeliveryRecord>, RuntimeError> {
        self.automation_intake
            .list_trigger_deliveries_by_automation(automation_id)
            .await
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
