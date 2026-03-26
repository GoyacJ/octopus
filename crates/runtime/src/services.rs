use octopus_domain_context::{
    ContextRepository, ProjectContext, ProjectRecord, SqliteContextStore, WorkspaceRecord,
};
use octopus_execution::{ExecutionEngine, ExecutionOutcome};
use octopus_governance::{
    ApprovalDecision, ApprovalRequestRecord, BudgetPolicyRecord, CapabilityBindingRecord,
    CapabilityDescriptorRecord, CapabilityGrantRecord, GovernanceDecision, SqliteGovernanceStore,
    TaskGovernanceInput, DECISION_ALLOW, DECISION_DENY, DECISION_REQUIRE_APPROVAL,
};
use octopus_observe_artifact::{
    ArtifactRecord, ArtifactStore, AuditRecord, InboxItemRecord, NotificationRecord,
    ObservationWriter, PolicyDecisionLogRecord, SqliteObservationStore, TraceRecord,
    AUDIT_EVENT_APPROVAL_APPROVED, AUDIT_EVENT_APPROVAL_CANCELLED, AUDIT_EVENT_APPROVAL_EXPIRED,
    AUDIT_EVENT_APPROVAL_REJECTED, AUDIT_EVENT_APPROVAL_REQUESTED, AUDIT_EVENT_ARTIFACT_CREATED,
    AUDIT_EVENT_POLICY_DENIED, AUDIT_EVENT_RUN_BLOCKED, AUDIT_EVENT_RUN_COMPLETED,
    AUDIT_EVENT_RUN_CREATED, AUDIT_EVENT_RUN_FAILED, AUDIT_EVENT_RUN_RETRY_REQUESTED,
    AUDIT_EVENT_RUN_STARTED, AUDIT_EVENT_RUN_TERMINATED, TRACE_STAGE_ARTIFACT_STORE,
    TRACE_STAGE_EXECUTION_ACTION, TRACE_STAGE_GOVERNANCE_EVALUATION, TRACE_STAGE_RUN_ORCHESTRATOR,
};
use sqlx::{Row, SqlitePool};

use crate::{
    models::{current_timestamp, CreateTaskInput, RunExecutionReport, RunRecord, TaskRecord},
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
                id, workspace_id, project_id, title, instruction, action_json, capability_id,
                estimated_cost, idempotency_key, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&task.id)
        .bind(&task.workspace_id)
        .bind(&task.project_id)
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

    pub async fn fetch_task(&self, task_id: &str) -> Result<TaskRecord, RuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, title, instruction, action_json, capability_id,
                   estimated_cost, idempotency_key, created_at, updated_at
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
            SELECT id, workspace_id, project_id, title, instruction, action_json, capability_id,
                   estimated_cost, idempotency_key, created_at, updated_at
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
pub struct RunOrchestrator {
    pool: SqlitePool,
    governance_store: SqliteGovernanceStore,
    observation_store: SqliteObservationStore,
}

impl RunOrchestrator {
    pub fn new(
        pool: SqlitePool,
        governance_store: SqliteGovernanceStore,
        observation_store: SqliteObservationStore,
    ) -> Self {
        Self {
            pool,
            governance_store,
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

    pub async fn start_task(&self, task: &TaskRecord) -> Result<RunExecutionReport, RuntimeError> {
        if let Some(existing) = self
            .fetch_run_by_idempotency_key(&format!("run:task:{}", task.id))
            .await?
        {
            return self.load_run_report(&existing.id).await;
        }

        let run = RunRecord::new(task);
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
        let mut run = self
            .fetch_run(&approval.run_id)
            .await?
            .ok_or_else(|| RuntimeError::RunNotFound(approval.run_id.clone()))?;

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
                id, task_id, workspace_id, project_id, run_type, status, approval_request_id,
                idempotency_key, attempt_count, max_attempts, checkpoint_seq, resume_token,
                last_error, created_at, updated_at, started_at, completed_at, terminated_at
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

        Ok(RunExecutionReport {
            run,
            artifacts,
            audits,
            traces,
            approvals,
            inbox_items,
            notifications,
            policy_decisions,
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

        let mut inbox = InboxItemRecord::approval_request(
            &run.workspace_id,
            &run.project_id,
            &run.id,
            &approval.id,
            "Approval required",
            "A run needs approval before execution",
        );
        inbox.status = "open".to_string();
        self.observation_store.upsert_inbox_item(&inbox).await?;

        let notification = NotificationRecord::approval_request(
            &run.workspace_id,
            &run.project_id,
            &run.id,
            &approval.id,
            "Approval required",
            "A run is waiting for approval",
        );
        self.observation_store
            .upsert_notification(&notification)
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

        match ExecutionEngine::execute(&task.action, run.attempt_count as u32) {
            ExecutionOutcome::Succeeded(success) => {
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
            }
            ExecutionOutcome::Failed(failure) => {
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

    async fn insert_run(&self, run: &RunRecord) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            INSERT INTO runs (
                id, task_id, workspace_id, project_id, run_type, status, approval_request_id,
                idempotency_key, attempt_count, max_attempts, checkpoint_seq, resume_token,
                last_error, created_at, updated_at, started_at, completed_at, terminated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            "#,
        )
        .bind(&run.id)
        .bind(&run.task_id)
        .bind(&run.workspace_id)
        .bind(&run.project_id)
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
            SET status = ?2,
                approval_request_id = ?3,
                attempt_count = ?4,
                max_attempts = ?5,
                checkpoint_seq = ?6,
                resume_token = ?7,
                last_error = ?8,
                updated_at = ?9,
                started_at = ?10,
                completed_at = ?11,
                terminated_at = ?12
            WHERE id = ?1
            "#,
        )
        .bind(&run.id)
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
                id, task_id, workspace_id, project_id, run_type, status, approval_request_id,
                idempotency_key, attempt_count, max_attempts, checkpoint_seq, resume_token,
                last_error, created_at, updated_at, started_at, completed_at, terminated_at
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

fn run_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<RunRecord, RuntimeError> {
    Ok(RunRecord {
        id: row.try_get("id")?,
        task_id: row.try_get("task_id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
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
