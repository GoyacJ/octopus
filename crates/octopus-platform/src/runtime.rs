use async_trait::async_trait;
use octopus_core::{
    AppError, CreateRuntimeSessionInput, ModelCatalogSnapshot, ResolveRuntimeApprovalInput,
    ResolveRuntimeAuthChallengeInput, ResolveRuntimeMemoryProposalInput, RuntimeBootstrap,
    RuntimeConfigPatch, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeRunSnapshot, RuntimeSessionDetail, RuntimeSessionSummary, SubmitRuntimeTurnInput,
};

#[async_trait]
pub trait RuntimeSessionService: Send + Sync {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError>;
    async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError>;
    async fn create_session(
        &self,
        input: CreateRuntimeSessionInput,
        user_id: &str,
    ) -> Result<RuntimeSessionDetail, AppError>;
    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError>;
    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait RuntimeExecutionService: Send + Sync {
    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn resolve_auth_challenge(
        &self,
        session_id: &str,
        challenge_id: &str,
        input: ResolveRuntimeAuthChallengeInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn resolve_memory_proposal(
        &self,
        session_id: &str,
        proposal_id: &str,
        input: ResolveRuntimeMemoryProposalInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>, AppError>;
}

#[async_trait]
pub trait RuntimeConfigService: Send + Sync {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn get_project_config(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn get_user_config(&self, user_id: &str) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError>;
    async fn save_config(
        &self,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
}

#[async_trait]
pub trait ModelRegistryService: Send + Sync {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError>;
}

#[async_trait]
pub trait ToolExecutionService: Send + Sync {}

#[async_trait]
pub trait AutomationService: Send + Sync {}

#[async_trait]
pub trait RuntimeProjectionService: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn sample_workflow_summary() -> octopus_core::RuntimeWorkflowSummary {
        octopus_core::RuntimeWorkflowSummary {
            workflow_run_id: "workflow-1".into(),
            label: "Team workflow".into(),
            status: "background_running".into(),
            total_steps: 3,
            completed_steps: 1,
            current_step_id: Some("step-1".into()),
            current_step_label: Some("Worker review".into()),
            background_capable: true,
            updated_at: 20,
        }
    }

    fn sample_run() -> RuntimeRunSnapshot {
        RuntimeRunSnapshot {
            id: "run-1".into(),
            session_id: "session-1".into(),
            conversation_id: "conversation-1".into(),
            status: "running".into(),
            current_step: "workflow_step".into(),
            started_at: 10,
            updated_at: 20,
            selected_memory: Vec::new(),
            freshness_summary: None,
            pending_memory_proposal: None,
            memory_state_ref: "memory-state-1".into(),
            configured_model_id: Some("quota-model".into()),
            configured_model_name: Some("Quota Model".into()),
            model_id: Some("provider-model".into()),
            consumed_tokens: Some(42),
            next_action: Some("await_workflow".into()),
            config_snapshot_id: "config-1".into(),
            effective_config_hash: "hash-1".into(),
            started_from_scope_set: vec!["workspace".into()],
            run_kind: "primary".into(),
            parent_run_id: None,
            actor_ref: "team:workspace-core".into(),
            delegated_by_tool_call_id: Some("tool-call-1".into()),
            workflow_run: Some("workflow-1".into()),
            workflow_run_detail: Some(octopus_core::RuntimeWorkflowRunDetail {
                workflow_run_id: "workflow-1".into(),
                status: "background_running".into(),
                current_step_id: Some("step-1".into()),
                current_step_label: Some("Worker review".into()),
                total_steps: 3,
                completed_steps: 1,
                background_capable: true,
            }),
            mailbox_ref: Some("mailbox-1".into()),
            handoff_ref: Some("handoff-1".into()),
            background_state: Some("background_running".into()),
            worker_dispatch: Some(octopus_core::RuntimeWorkerDispatchSummary {
                total_subruns: 1,
                active_subruns: 1,
                completed_subruns: 0,
                failed_subruns: 0,
            }),
            approval_state: "not-required".into(),
            approval_target: None,
            auth_target: None,
            usage_summary: octopus_core::RuntimeUsageSummary::default(),
            artifact_refs: vec!["runtime-artifact-run-1".into()],
            trace_context: octopus_core::RuntimeTraceContext::default(),
            checkpoint: octopus_core::RuntimeRunCheckpoint::default(),
            capability_plan_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            pending_mediation: None,
            capability_state_ref: Some("capability-state-1".into()),
            last_execution_outcome: None,
            last_mediation_outcome: None,
            resolved_target: None,
            requested_actor_kind: Some("team".into()),
            requested_actor_id: Some("team:workspace-core".into()),
            resolved_actor_kind: Some("team".into()),
            resolved_actor_id: Some("team:workspace-core".into()),
            resolved_actor_label: Some("Workspace Core".into()),
        }
    }

    fn sample_detail() -> RuntimeSessionDetail {
        let run = sample_run();
        RuntimeSessionDetail {
            summary: RuntimeSessionSummary {
                id: "session-1".into(),
                conversation_id: "conversation-1".into(),
                project_id: "project-1".into(),
                title: "Phase 4".into(),
                session_kind: "project".into(),
                status: "running".into(),
                updated_at: 20,
                last_message_preview: Some("Workflow in progress".into()),
                config_snapshot_id: "config-1".into(),
                effective_config_hash: "hash-1".into(),
                started_from_scope_set: vec!["workspace".into()],
                selected_actor_ref: "team:workspace-core".into(),
                manifest_revision: "manifest-1".into(),
                session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
                active_run_id: run.id.clone(),
                subrun_count: 1,
                workflow: Some(sample_workflow_summary()),
                pending_mailbox: Some(octopus_core::RuntimeMailboxSummary {
                    mailbox_ref: "mailbox-1".into(),
                    channel: "team-mailbox".into(),
                    status: "pending".into(),
                    pending_count: 1,
                    total_messages: 1,
                    updated_at: 20,
                }),
                background_run: Some(octopus_core::RuntimeBackgroundRunSummary {
                    run_id: run.id.clone(),
                    workflow_run_id: Some("workflow-1".into()),
                    status: "background_running".into(),
                    background_capable: true,
                    updated_at: 20,
                }),
                memory_summary: octopus_core::RuntimeMemorySummary::default(),
                memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
                pending_memory_proposal_count: 0,
                memory_state_ref: "memory-state-1".into(),
                capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
                provider_state_summary: Vec::new(),
                auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
                pending_mediation: None,
                policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
                capability_state_ref: Some("capability-state-1".into()),
                last_execution_outcome: None,
            },
            selected_actor_ref: "team:workspace-core".into(),
            manifest_revision: "manifest-1".into(),
            session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
            active_run_id: run.id.clone(),
            subrun_count: 1,
            workflow: Some(sample_workflow_summary()),
            pending_mailbox: Some(octopus_core::RuntimeMailboxSummary {
                mailbox_ref: "mailbox-1".into(),
                channel: "team-mailbox".into(),
                status: "pending".into(),
                pending_count: 1,
                total_messages: 1,
                updated_at: 20,
            }),
            background_run: Some(octopus_core::RuntimeBackgroundRunSummary {
                run_id: run.id.clone(),
                workflow_run_id: Some("workflow-1".into()),
                status: "background_running".into(),
                background_capable: true,
                updated_at: 20,
            }),
            memory_summary: octopus_core::RuntimeMemorySummary::default(),
            memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
            pending_memory_proposal_count: 0,
            memory_state_ref: "memory-state-1".into(),
            capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
            pending_mediation: None,
            policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
            capability_state_ref: Some("capability-state-1".into()),
            last_execution_outcome: None,
            run,
            subruns: vec![octopus_core::RuntimeSubrunSummary {
                run_id: "subrun-1".into(),
                parent_run_id: Some("run-1".into()),
                actor_ref: "agent:worker".into(),
                label: "Worker".into(),
                status: "running".into(),
                run_kind: "subrun".into(),
                delegated_by_tool_call_id: Some("tool-call-1".into()),
                workflow_run_id: Some("workflow-1".into()),
                mailbox_ref: Some("mailbox-1".into()),
                handoff_ref: Some("handoff-1".into()),
                started_at: 11,
                updated_at: 20,
            }],
            handoffs: vec![octopus_core::RuntimeHandoffSummary {
                handoff_ref: "handoff-1".into(),
                mailbox_ref: "mailbox-1".into(),
                sender_actor_ref: "team:workspace-core".into(),
                receiver_actor_ref: "agent:worker".into(),
                state: "pending".into(),
                artifact_refs: vec!["runtime-artifact-run-1".into()],
                updated_at: 20,
            }],
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        }
    }

    struct RuntimeHarness {
        detail: RuntimeSessionDetail,
        run: RuntimeRunSnapshot,
        events: broadcast::Sender<RuntimeEventEnvelope>,
    }

    impl RuntimeHarness {
        fn new(detail: RuntimeSessionDetail, run: RuntimeRunSnapshot) -> Self {
            let (events, _) = broadcast::channel(4);
            Self {
                detail,
                run,
                events,
            }
        }
    }

    #[async_trait]
    impl RuntimeSessionService for RuntimeHarness {
        async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError> {
            Ok(RuntimeBootstrap {
                provider: octopus_core::ProviderConfig {
                    provider_id: "provider-1".into(),
                    credential_ref: None,
                    base_url: None,
                    default_model: Some("quota-model".into()),
                    default_surface: None,
                    protocol_family: None,
                },
                sessions: vec![self.detail.summary.clone()],
            })
        }

        async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError> {
            Ok(vec![self.detail.summary.clone()])
        }

        async fn create_session(
            &self,
            _input: CreateRuntimeSessionInput,
            _user_id: &str,
        ) -> Result<RuntimeSessionDetail, AppError> {
            Ok(self.detail.clone())
        }

        async fn get_session(&self, _session_id: &str) -> Result<RuntimeSessionDetail, AppError> {
            Ok(self.detail.clone())
        }

        async fn list_events(
            &self,
            _session_id: &str,
            _after: Option<&str>,
        ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
            Ok(Vec::new())
        }

        async fn delete_session(&self, _session_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[async_trait]
    impl RuntimeExecutionService for RuntimeHarness {
        async fn submit_turn(
            &self,
            _session_id: &str,
            _input: SubmitRuntimeTurnInput,
        ) -> Result<RuntimeRunSnapshot, AppError> {
            Ok(self.run.clone())
        }

        async fn resolve_approval(
            &self,
            _session_id: &str,
            _approval_id: &str,
            _input: ResolveRuntimeApprovalInput,
        ) -> Result<RuntimeRunSnapshot, AppError> {
            Ok(self.run.clone())
        }

        async fn resolve_auth_challenge(
            &self,
            _session_id: &str,
            _challenge_id: &str,
            _input: ResolveRuntimeAuthChallengeInput,
        ) -> Result<RuntimeRunSnapshot, AppError> {
            Ok(self.run.clone())
        }

        async fn resolve_memory_proposal(
            &self,
            _session_id: &str,
            _proposal_id: &str,
            _input: ResolveRuntimeMemoryProposalInput,
        ) -> Result<RuntimeRunSnapshot, AppError> {
            Ok(self.run.clone())
        }

        async fn subscribe_events(
            &self,
            _session_id: &str,
        ) -> Result<tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>, AppError> {
            Ok(self.events.subscribe())
        }
    }

    #[tokio::test]
    async fn runtime_services_preserve_phase_four_typed_surface() {
        let detail = sample_detail();
        let run = detail.run.clone();
        let harness = Arc::new(RuntimeHarness::new(detail.clone(), run.clone()));
        let session_service: Arc<dyn RuntimeSessionService> = harness.clone();
        let execution_service: Arc<dyn RuntimeExecutionService> = harness;

        let listed = session_service
            .list_sessions()
            .await
            .expect("list sessions");
        assert_eq!(
            listed[0]
                .workflow
                .as_ref()
                .map(|item| item.workflow_run_id.as_str()),
            Some("workflow-1")
        );
        assert_eq!(
            listed[0]
                .pending_mailbox
                .as_ref()
                .map(|item| item.mailbox_ref.as_str()),
            Some("mailbox-1")
        );
        assert_eq!(
            listed[0]
                .background_run
                .as_ref()
                .map(|item| item.status.as_str()),
            Some("background_running")
        );

        let loaded = session_service
            .get_session("session-1")
            .await
            .expect("get session");
        assert_eq!(
            loaded
                .workflow
                .as_ref()
                .map(|item| item.workflow_run_id.as_str()),
            Some("workflow-1")
        );
        assert_eq!(
            loaded.subruns[0].workflow_run_id.as_deref(),
            Some("workflow-1")
        );
        assert_eq!(
            loaded.handoffs[0].artifact_refs,
            vec!["runtime-artifact-run-1"]
        );

        let submitted = execution_service
            .submit_turn(
                "session-1",
                SubmitRuntimeTurnInput {
                    content: "continue".into(),
                    permission_mode: None,
                    recall_mode: None,
                    ignored_memory_ids: Vec::new(),
                    memory_intent: None,
                },
            )
            .await
            .expect("submit turn");
        assert_eq!(submitted.workflow_run.as_deref(), Some("workflow-1"));
        assert_eq!(
            submitted
                .workflow_run_detail
                .as_ref()
                .and_then(|detail| detail.current_step_id.as_deref()),
            Some("step-1")
        );
        assert_eq!(
            submitted.background_state.as_deref(),
            Some("background_running")
        );
        assert_eq!(
            submitted
                .worker_dispatch
                .as_ref()
                .map(|dispatch| dispatch.total_subruns),
            Some(1)
        );
    }
}
