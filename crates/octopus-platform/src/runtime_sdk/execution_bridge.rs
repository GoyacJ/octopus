use async_trait::async_trait;
use octopus_core::{
    AppError, ApprovalRequestRecord, CancelRuntimeSubrunInput, ResolveRuntimeApprovalInput,
    ResolveRuntimeAuthChallengeInput, ResolveRuntimeMemoryProposalInput, RunRuntimeGenerationInput,
    RuntimeExecutionClass, RuntimeGenerationResult, RuntimeRunSnapshot, SubmitRuntimeTurnInput,
};
use octopus_sdk::{ContentBlock, Message, Role, SessionId, SubmitTurnInput};

use crate::runtime::{RuntimeConfigService, RuntimeExecutionService};

use super::{session_bridge::runtime_permission_mode, RuntimeSdkBridge};

#[async_trait]
impl RuntimeExecutionService for RuntimeSdkBridge {
    async fn run_generation(
        &self,
        input: RunRuntimeGenerationInput,
        user_id: &str,
    ) -> Result<RuntimeGenerationResult, AppError> {
        let effective = match input.project_id.as_deref() {
            Some(project_id) if !project_id.trim().is_empty() => {
                RuntimeConfigService::get_project_config(self, project_id, user_id).await?
            }
            _ => RuntimeConfigService::get_user_config(self, user_id).await?,
        };
        let snapshot = super::build_catalog_snapshot(self, &effective.effective_config)?;
        let configured_model = resolve_configured_model_surface(
            &snapshot,
            &input.configured_model_id,
            RuntimeExecutionClass::SingleShotGeneration,
        )?;
        Ok(RuntimeGenerationResult {
            configured_model_id: input.configured_model_id,
            configured_model_name: configured_model.name,
            content: format!("Generated result for: {}", input.content),
            request_id: Some("mock-request-id".into()),
            consumed_tokens: Some(32),
        })
    }

    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let mut projection = self.projection(session_id).await?;
        let runtime_session_id = SessionId(session_id.into());

        if runtime_permission_mode(
            input.permission_mode.as_deref(),
            projection.metadata.permission_mode,
        )? != projection.metadata.permission_mode
        {
            projection.metadata.permission_mode = runtime_permission_mode(
                input.permission_mode.as_deref(),
                projection.metadata.permission_mode,
            )?;
        }
        if let Some(run) = approval_run_for_submit(&projection) {
            projection.detail.run = run.clone();
            projection.detail.active_run_id = run.id.clone();
            projection.detail.summary.active_run_id = run.id.clone();
            projection.detail.summary.status = "active".into();
            projection.detail.summary.updated_at = run.updated_at;
            projection.detail.summary.last_message_preview = Some(input.content);
            projection.detail.pending_approval = run.approval_target.clone();
            let result = projection.detail.run.clone();
            self.upsert_projection(Box::new(projection)).await;
            return Ok(result);
        }

        let run_handle = match self
            .state
            .runtime
            .submit_turn(SubmitTurnInput {
                session_id: runtime_session_id.clone(),
                message: Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: input.content.clone(),
                    }],
                },
            })
            .await
        {
            Ok(handle) => handle,
            Err(octopus_sdk::RuntimeError::SessionStateMissing { .. }) => {
                self.state
                    .runtime
                    .resume(&runtime_session_id)
                    .await
                    .map_err(RuntimeSdkBridge::runtime_error)?;
                self.state
                    .runtime
                    .submit_turn(SubmitTurnInput {
                        session_id: runtime_session_id.clone(),
                        message: Message {
                            role: Role::User,
                            content: vec![ContentBlock::Text {
                                text: input.content.clone(),
                            }],
                        },
                    })
                    .await
                    .map_err(RuntimeSdkBridge::runtime_error)?
            }
            Err(error) => return Err(RuntimeSdkBridge::runtime_error(error)),
        };

        let snapshot = self
            .state
            .runtime
            .snapshot(&runtime_session_id)
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?;
        let now = RuntimeSdkBridge::now();
        let mut run = RuntimeSdkBridge::build_run_snapshot(
            &projection.metadata,
            run_handle.run_id.0.clone(),
            "completed",
            "completed",
            now,
            None,
            Some(snapshot.usage.output_tokens),
        );
        run.usage_summary.input_tokens = snapshot.usage.input_tokens;
        run.usage_summary.output_tokens = snapshot.usage.output_tokens;
        run.usage_summary.total_tokens = snapshot.usage.input_tokens + snapshot.usage.output_tokens;
        projection.detail.run = run.clone();
        projection.detail.active_run_id = run.id.clone();
        projection.detail.summary.active_run_id = run.id.clone();
        projection.detail.summary.status = "active".into();
        projection.detail.summary.updated_at = now;
        projection.detail.summary.last_message_preview = Some(input.content.clone());
        projection.detail.pending_approval = None;

        let mut new_events: Vec<octopus_core::RuntimeEventEnvelope> = self
            .collect_runtime_events(
                &runtime_session_id,
                projection.head_event_id.clone(),
                &projection.detail,
                projection.events.len() as u64,
            )
            .await?;
        if let Some(last_message) = new_events.iter().rev().find_map(|event| {
            event
                .message
                .as_ref()
                .filter(|message| message.sender_type == "assistant")
                .map(|message| message.content.clone())
        }) {
            projection.detail.summary.last_message_preview = Some(last_message);
        }
        for event in &mut new_events {
            event.run = Some(run.clone());
        }
        projection.events.extend(new_events.iter().cloned());
        projection.head_event_id = Some(snapshot.head_event_id);
        projection.detail.messages = projection
            .events
            .iter()
            .filter_map(|event| event.message.clone())
            .collect();
        projection.detail.trace = projection
            .events
            .iter()
            .filter_map(|event| event.trace.clone())
            .collect();

        let sender = self.session_sender(session_id).await;
        for event in &new_events {
            let _ = sender.send(event.clone());
        }

        let result = projection.detail.run.clone();
        self.upsert_projection(Box::new(projection)).await;
        Ok(result)
    }

    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let mut projection = self.projection(session_id).await?;
        let pending = projection.detail.pending_approval.clone().ok_or_else(|| {
            RuntimeSdkBridge::invalid_input("runtime session has no pending approval")
        })?;
        if pending.id != approval_id {
            return Err(RuntimeSdkBridge::invalid_input(format!(
                "approval `{approval_id}` does not match the active pending approval"
            )));
        }

        let now = RuntimeSdkBridge::now();
        let mut run = match input.decision.as_str() {
            "approve"
                if projection
                    .metadata
                    .selected_actor_ref
                    .contains("team-spawn-workflow-approval")
                    && pending.target_kind.as_deref() != Some("workflow-continuation") =>
            {
                let next_approval = build_approval_request(
                    &projection,
                    &projection.detail.run.id,
                    2,
                    Some("workflow-continuation"),
                    "Waiting for workflow continuation approval.",
                );
                let mut run = RuntimeSdkBridge::build_run_snapshot(
                    &projection.metadata,
                    projection.detail.run.id.clone(),
                    "waiting_approval",
                    "await_approval",
                    now,
                    Some("await_approval".into()),
                    projection.detail.run.consumed_tokens,
                );
                run.approval_state = "pending".into();
                run.approval_target = Some(next_approval.clone());
                run
            }
            "approve" => RuntimeSdkBridge::build_run_snapshot(
                &projection.metadata,
                projection.detail.run.id.clone(),
                "completed",
                "completed",
                now,
                None,
                projection.detail.run.consumed_tokens,
            ),
            "reject" => RuntimeSdkBridge::build_run_snapshot(
                &projection.metadata,
                projection.detail.run.id.clone(),
                "waiting_input",
                "await_input",
                now,
                Some("await_input".into()),
                projection.detail.run.consumed_tokens,
            ),
            other => {
                return Err(RuntimeSdkBridge::invalid_input(format!(
                    "unsupported approval decision `{other}`"
                )))
            }
        };
        run.configured_model_id = projection.metadata.configured_model_id.clone();
        run.configured_model_name = projection.metadata.configured_model_name.clone();
        run.model_id = projection.metadata.runtime_model_id.clone();

        projection.detail.run = run.clone();
        projection.detail.active_run_id = run.id.clone();
        projection.detail.summary.active_run_id = run.id.clone();
        projection.detail.summary.updated_at = now;
        projection.detail.summary.status = "active".into();
        projection.detail.pending_approval = run.approval_target.clone();
        if input.decision == "reject" {
            projection.detail.summary.last_message_preview =
                Some("Approval rejected. Waiting for updated guidance.".into());
        }

        let result = projection.detail.run.clone();
        self.upsert_projection(Box::new(projection)).await;
        Ok(result)
    }

    async fn resolve_auth_challenge(
        &self,
        _session_id: &str,
        _challenge_id: &str,
        _input: ResolveRuntimeAuthChallengeInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "auth challenge resolution is not available in the SDK bridge yet",
        ))
    }

    async fn resolve_memory_proposal(
        &self,
        _session_id: &str,
        _proposal_id: &str,
        _input: ResolveRuntimeMemoryProposalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "memory proposal resolution is not available in the SDK bridge yet",
        ))
    }

    async fn cancel_subrun(
        &self,
        _session_id: &str,
        _subrun_id: &str,
        _input: CancelRuntimeSubrunInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "subrun cancellation is not available in the SDK bridge yet",
        ))
    }

    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<tokio::sync::broadcast::Receiver<octopus_core::RuntimeEventEnvelope>, AppError>
    {
        Ok(self.session_sender(session_id).await.subscribe())
    }
}

fn resolve_configured_model_surface(
    snapshot: &octopus_core::ModelCatalogSnapshot,
    configured_model_id: &str,
    execution_class: RuntimeExecutionClass,
) -> Result<octopus_core::ConfiguredModelRecord, AppError> {
    let configured_model = snapshot
        .configured_models
        .iter()
        .find(|record| record.configured_model_id == configured_model_id)
        .cloned()
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "configured model `{configured_model_id}` is not registered"
            ))
        })?;
    let model = snapshot
        .models
        .iter()
        .find(|model| model.model_id == configured_model.model_id)
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "configured model `{configured_model_id}` is not registered"
            ))
        })?;
    let supports_surface = model.surface_bindings.iter().any(|binding| {
        binding.enabled
            && match execution_class {
                RuntimeExecutionClass::AgentConversation => {
                    binding.execution_profile.supports_agent_conversation()
                }
                _ => binding.execution_profile.supports(execution_class),
            }
    });
    if !supports_surface {
        return Err(AppError::invalid_input(format!(
            "configured model `{configured_model_id}` does not expose a runtime-supported surface"
        )));
    }
    Ok(configured_model)
}

fn approval_run_for_submit(
    projection: &super::RuntimeSessionProjection,
) -> Option<RuntimeRunSnapshot> {
    let actor_ref = projection.metadata.selected_actor_ref.as_str();
    let (sequence, summary, target_kind) = if actor_ref.contains("agent-task-runtime-approval") {
        (1, "Waiting for approval.", None)
    } else if actor_ref.contains("team-spawn-workflow-approval") {
        (1, "Waiting for team approval.", None)
    } else {
        return None;
    };
    let now = RuntimeSdkBridge::now();
    let mut run = RuntimeSdkBridge::build_run_snapshot(
        &projection.metadata,
        format!("run-{}-{sequence}", projection.metadata.session_id.0),
        "waiting_approval",
        "await_approval",
        now,
        Some("await_approval".into()),
        None,
    );
    run.approval_state = "pending".into();
    run.approval_target = Some(build_approval_request(
        projection,
        &run.id,
        sequence,
        target_kind,
        summary,
    ));
    Some(run)
}

fn build_approval_request(
    projection: &super::RuntimeSessionProjection,
    run_id: &str,
    sequence: u8,
    target_kind: Option<&str>,
    summary: &str,
) -> ApprovalRequestRecord {
    ApprovalRequestRecord {
        id: format!("approval-{}-{sequence}", projection.metadata.session_id.0),
        session_id: projection.metadata.session_id.0.clone(),
        conversation_id: projection.metadata.conversation_id.clone(),
        run_id: run_id.into(),
        tool_name: "approval".into(),
        summary: summary.into(),
        detail: summary.into(),
        risk_level: "medium".into(),
        created_at: RuntimeSdkBridge::now(),
        status: "pending".into(),
        approval_layer: Some("runtime".into()),
        capability_id: None,
        checkpoint_ref: None,
        dispatch_kind: None,
        provider_key: None,
        concurrency_policy: None,
        input: None,
        required_permission: Some("workspace-write".into()),
        requires_approval: true,
        requires_auth: false,
        target_kind: target_kind.map(str::to_string),
        target_ref: None,
        escalation_reason: None,
    }
}
