use super::*;
use octopus_core::RuntimeTargetPolicyDecision;

#[derive(Debug, Clone)]
pub(crate) struct AgentRuntimeCore;

type SubmitState = (
    RuntimeMessage,
    RuntimeTraceItem,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    Option<ApprovalRequestRecord>,
    RuntimeRunSnapshot,
    String,
    String,
);

type ApprovalResolutionState = (
    ApprovalRequestRecord,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
);

type AuthChallengeResolutionState = (
    RuntimeAuthChallengeSummary,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
);

type MemoryProposalResolutionState = (RuntimeMemoryProposal, RuntimeRunSnapshot, String, String);

fn usage_summary_from_tokens(total_tokens: Option<u32>) -> RuntimeUsageSummary {
    RuntimeUsageSummary {
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: total_tokens.unwrap_or(0),
    }
}

fn capability_execution_outcome(
    outcome: &str,
    detail: Option<String>,
    requires_approval: bool,
    requires_auth: bool,
) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: None,
        tool_name: None,
        provider_key: None,
        dispatch_kind: None,
        outcome: outcome.into(),
        detail,
        requires_approval,
        requires_auth,
        concurrency_policy: Some("serialized".into()),
    }
}

fn serialized_runtime_session(
    content: &str,
    trace_context: &RuntimeTraceContext,
) -> Value {
    json!({
        "content": content,
        "pendingContent": content,
        "traceContext": trace_context,
    })
}

fn build_runtime_checkpoint(
    serialized_session: Value,
    current_iteration_index: u32,
    usage_summary: RuntimeUsageSummary,
    pending_approval: Option<ApprovalRequestRecord>,
    pending_auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pending_mediation: Option<RuntimePendingMediationSummary>,
    capability_state_ref: Option<String>,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    last_mediation_outcome: Option<RuntimeMediationOutcome>,
    mediation_request: Option<&approval_broker::MediationRequest>,
    broker_decision: Option<&approval_broker::BrokerDecision>,
    checkpoint_artifact_ref: Option<String>,
) -> RuntimeRunCheckpoint {
    let broker_state = broker_decision.map(|decision| decision.state.clone());
    let (approval_layer, capability_id, provider_key, reason, required_permission, requires_approval, requires_auth, target_kind, target_ref) =
        if let Some(request) = mediation_request {
            (
                Some(request.approval_layer.clone()),
                request.capability_id.clone(),
                request.provider_key.clone(),
                request.escalation_reason.clone(),
                request.required_permission.clone(),
                Some(request.requires_approval),
                Some(request.requires_auth),
                Some(request.target_kind.clone()),
                Some(request.target_ref.clone()),
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
        };

    RuntimeRunCheckpoint {
        approval_layer,
        broker_decision: broker_state,
        capability_id,
        checkpoint_artifact_ref,
        serialized_session,
        current_iteration_index,
        usage_summary,
        pending_approval,
        pending_auth_challenge,
        compaction_metadata: json!({}),
        pending_mediation,
        provider_key,
        reason,
        required_permission,
        requires_approval,
        requires_auth,
        target_kind,
        target_ref,
        capability_state_ref,
        capability_plan_summary,
        last_execution_outcome,
        last_mediation_outcome,
    }
}

fn runtime_execution_mediation_request(
    run_context: &run_context::RunContext,
    summary: String,
    detail: String,
    requires_approval: bool,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> approval_broker::MediationRequest {
    approval_broker::MediationRequest {
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        run_id: run_context.run_id.clone(),
        tool_name: run_context.actor_manifest.label().to_string(),
        summary,
        detail,
        mediation_kind: "approval".into(),
        approval_layer: "execution-permission".into(),
        target_kind: "runtime-execution".into(),
        target_ref: run_context.actor_manifest.actor_ref().to_string(),
        capability_id: Some(run_context.actor_manifest.actor_ref().to_string()),
        provider_key: None,
        required_permission: Some(run_context.requested_permission_mode.clone()),
        escalation_reason: requires_approval
            .then(|| "session ceiling requires approval".into()),
        requires_approval,
        requires_auth: false,
        created_at,
        risk_level: "medium".into(),
        checkpoint_ref,
    }
}

fn runtime_provider_auth_mediation_request(
    run_context: &run_context::RunContext,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    provider_auth_mediation_request(
        &run_context.session_id,
        &run_context.conversation_id,
        &run_context.run_id,
        &run_context.actor_manifest,
        &run_context.provider_state_summary,
        &run_context.auth_state_summary,
        run_context.provider_auth_policy_decision.as_ref(),
        checkpoint_ref,
        created_at,
    )
}

fn provider_auth_mediation_request(
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    provider_state_summary: &[RuntimeCapabilityProviderState],
    auth_state_summary: &RuntimeAuthStateSummary,
    policy_decision: Option<&RuntimeTargetPolicyDecision>,
    checkpoint_ref: Option<String>,
    created_at: u64,
) -> Option<approval_broker::MediationRequest> {
    let provider_key = auth_state_summary
        .challenged_provider_keys
        .first()
        .cloned()
        .or_else(|| {
            provider_state_summary
                .iter()
                .find(|provider| provider.state == "auth_required")
                .map(|provider| provider.provider_key.clone())
        })?;

    Some(approval_broker::MediationRequest {
        session_id: session_id.to_string(),
        conversation_id: conversation_id.to_string(),
        run_id: run_id.to_string(),
        tool_name: actor_manifest.label().to_string(),
        summary: format!("{} requires provider authentication", actor_manifest.label()),
        detail: format!(
            "Resolve provider or MCP authentication for `{provider_key}` before execution can continue."
        ),
        mediation_kind: "auth".into(),
        approval_layer: policy_decision
            .map(|value| value.target_kind.clone())
            .unwrap_or_else(|| "provider-auth".into()),
        target_kind: "provider-auth".into(),
        target_ref: actor_manifest.actor_ref().to_string(),
        capability_id: policy_decision
            .and_then(|value| value.capability_id.clone())
            .or_else(|| Some(actor_manifest.actor_ref().to_string())),
        provider_key: Some(provider_key),
        required_permission: policy_decision.and_then(|value| value.required_permission.clone()),
        escalation_reason: policy_decision
            .and_then(|value| value.reason.clone())
            .or_else(|| Some("provider or MCP auth must resolve before execution can continue".into())),
        requires_approval: policy_decision.is_some_and(|value| value.requires_approval),
        requires_auth: true,
        created_at,
        risk_level: "medium".into(),
        checkpoint_ref,
    })
}

fn provider_auth_required(projection: &capability_planner_bridge::CapabilityProjection) -> bool {
    !projection.auth_state_summary.challenged_provider_keys.is_empty()
        || projection
            .provider_state_summary
            .iter()
            .any(|provider| provider.state == "auth_required")
}

fn apply_checkpoint_ref(
    approval: &mut Option<ApprovalRequestRecord>,
    auth_target: &mut Option<RuntimeAuthChallengeSummary>,
    pending_mediation: &mut Option<RuntimePendingMediationSummary>,
    last_mediation_outcome: &mut Option<RuntimeMediationOutcome>,
    checkpoint_ref: &str,
) {
    if let Some(approval) = approval.as_mut() {
        approval.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(challenge) = auth_target.as_mut() {
        challenge.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(mediation) = pending_mediation.as_mut() {
        mediation.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
    if let Some(outcome) = last_mediation_outcome.as_mut() {
        outcome.checkpoint_ref = Some(checkpoint_ref.to_string());
    }
}

fn finalize_mediation_checkpoint_ref(
    adapter: &RuntimeAdapter,
    session_id: &str,
    run_id: &str,
    approval: &mut Option<ApprovalRequestRecord>,
    auth_target: &mut Option<RuntimeAuthChallengeSummary>,
    pending_mediation: &mut Option<RuntimePendingMediationSummary>,
    last_mediation_outcome: &mut Option<RuntimeMediationOutcome>,
) -> Option<String> {
    let checkpoint_ref = pending_mediation
        .as_ref()
        .and_then(|mediation| mediation.mediation_id.as_deref())
        .map(|mediation_id| adapter.runtime_mediation_checkpoint_ref(session_id, run_id, mediation_id))
        .or_else(|| pending_mediation.as_ref().and_then(|mediation| mediation.checkpoint_ref.clone()))
        .or_else(|| approval.as_ref().and_then(|item| item.checkpoint_ref.clone()))
        .or_else(|| auth_target.as_ref().and_then(|item| item.checkpoint_ref.clone()))
        .or_else(|| last_mediation_outcome.as_ref().and_then(|item| item.checkpoint_ref.clone()));

    if let Some(checkpoint_ref) = checkpoint_ref.as_deref() {
        apply_checkpoint_ref(
            approval,
            auth_target,
            pending_mediation,
            last_mediation_outcome,
            checkpoint_ref,
        );
    }

    checkpoint_ref
}

fn memory_proposal_pending_mediation(
    run_context: &run_context::RunContext,
) -> Option<RuntimePendingMediationSummary> {
    let proposal = run_context.pending_memory_proposal.as_ref()?;
    Some(RuntimePendingMediationSummary {
        approval_id: None,
        approval_layer: Some("memory-review".into()),
        auth_challenge_id: None,
        capability_id: Some(run_context.actor_manifest.actor_ref().to_string()),
        checkpoint_ref: None,
        detail: Some(proposal.proposal_reason.clone()),
        escalation_reason: Some("durable memory writes remain proposal-only until review".into()),
        mediation_id: Some(proposal.proposal_id.clone()),
        mediation_kind: "memory".into(),
        provider_key: None,
        reason: Some(proposal.proposal_reason.clone()),
        required_permission: None,
        requires_approval: false,
        requires_auth: false,
        state: proposal.proposal_state.clone(),
        summary: Some(proposal.summary.clone()),
        target_kind: "memory-write".into(),
        target_ref: proposal.memory_id.clone(),
        tool_name: Some(run_context.actor_manifest.label().to_string()),
    })
}

fn memory_proposal_mediation_outcome(
    proposal: &RuntimeMemoryProposal,
    decision_status: &str,
    now: u64,
) -> RuntimeMediationOutcome {
    RuntimeMediationOutcome {
        approval_layer: Some("memory-review".into()),
        capability_id: None,
        checkpoint_ref: None,
        detail: Some(proposal.proposal_reason.clone()),
        mediation_id: Some(proposal.proposal_id.clone()),
        mediation_kind: "memory".into(),
        outcome: decision_status.into(),
        provider_key: None,
        reason: proposal
            .review
            .as_ref()
            .and_then(|review| review.note.clone())
            .or_else(|| Some(proposal.proposal_reason.clone())),
        requires_approval: false,
        requires_auth: false,
        resolved_at: Some(now),
        target_kind: "memory-write".into(),
        target_ref: proposal.memory_id.clone(),
        tool_name: Some(proposal.title.clone()),
    }
}

fn blocking_mediation_state(
    approval: Option<&ApprovalRequestRecord>,
    auth_target: Option<&RuntimeAuthChallengeSummary>,
) -> (&'static str, &'static str, &'static str) {
    if approval.is_some() {
        ("waiting_approval", "awaiting_approval", "approval")
    } else if auth_target.is_some() {
        ("waiting_input", "awaiting_auth", "auth")
    } else {
        ("completed", "completed", "idle")
    }
}

fn apply_runtime_resolution_checkpoint(
    usage_summary: RuntimeUsageSummary,
    serialized_session: Value,
    pending_approval: Option<ApprovalRequestRecord>,
    pending_auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pending_mediation: Option<RuntimePendingMediationSummary>,
    capability_state_ref: Option<String>,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
    last_mediation_outcome: Option<RuntimeMediationOutcome>,
) -> RuntimeRunCheckpoint {
    let checkpoint_artifact_ref = pending_approval
        .as_ref()
        .and_then(|approval| approval.checkpoint_ref.clone())
        .or_else(|| {
            pending_auth_challenge
                .as_ref()
                .and_then(|challenge| challenge.checkpoint_ref.clone())
        });
    RuntimeRunCheckpoint {
        usage_summary,
        serialized_session,
        current_iteration_index: 1,
        pending_approval,
        pending_auth_challenge,
        pending_mediation,
        capability_state_ref,
        capability_plan_summary,
        last_execution_outcome,
        last_mediation_outcome,
        checkpoint_artifact_ref,
        ..Default::default()
    }
}

fn build_submit_trace(
    session_id: &str,
    run_id: &str,
    conversation_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    now: u64,
    detail: String,
    tone: &str,
) -> RuntimeTraceItem {
    RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id: run_id.to_string(),
        conversation_id: conversation_id.to_string(),
        kind: "planner.step".into(),
        title: "Capability plan prepared".into(),
        detail,
        tone: tone.into(),
        timestamp: now,
        actor: actor_manifest.label().to_string(),
        actor_kind: Some(actor_manifest.actor_kind_label().into()),
        actor_id: Some(actor_manifest.actor_ref().to_string()),
        related_message_id: None,
        related_tool_name: None,
    }
}

fn build_execution_trace(
    session_id: &str,
    run_id: &str,
    conversation_id: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    resolved_target: &ResolvedExecutionTarget,
    response: &ExecutionResponse,
    now: u64,
    related_message_id: Option<String>,
) -> RuntimeTraceItem {
    RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id: run_id.to_string(),
        conversation_id: conversation_id.to_string(),
        kind: "model.step".into(),
        title: "Model loop completed".into(),
        detail: format!(
            "Resolved {}:{} via {} and produced {} characters.",
            resolved_target.provider_id,
            resolved_target.configured_model_name,
            resolved_target.protocol_family,
            response.content.chars().count()
        ),
        tone: "success".into(),
        timestamp: now,
        actor: actor_manifest.label().to_string(),
        actor_kind: Some(actor_manifest.actor_kind_label().into()),
        actor_id: Some(actor_manifest.actor_ref().to_string()),
        related_message_id,
        related_tool_name: None,
    }
}

impl AgentRuntimeCore {
    pub(crate) async fn submit_turn(
        adapter: &RuntimeAdapter,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let run_context = adapter.build_run_context(session_id, &input, now).await?;
        let requires_approval =
            execution_target::requires_approval(&run_context.requested_permission_mode)?;
        let provider_auth_required = !run_context.auth_state_summary.challenged_provider_keys.is_empty();
        let execution_mediation_request = runtime_execution_mediation_request(
            &run_context,
            if requires_approval {
                "Turn requires approval".into()
            } else {
                "Execution allowed".into()
            },
            if requires_approval {
                format!(
                    "Permission mode {} requires explicit approval before execution.",
                    run_context.requested_permission_mode
                )
            } else {
                format!(
                    "Permission mode {} is within the frozen session ceiling.",
                    run_context.requested_permission_mode
                )
            },
            requires_approval,
            None,
            now,
        );
        let auth_mediation_request = (!requires_approval && provider_auth_required).then(|| {
            runtime_provider_auth_mediation_request(
                &run_context,
                None,
                now,
            )
        }).flatten();
        let mediation_request = auth_mediation_request
            .clone()
            .unwrap_or_else(|| execution_mediation_request.clone());
        let broker_decision = if requires_approval {
            approval_broker::require_approval(&execution_mediation_request)
        } else if let Some(auth_request) = auth_mediation_request.as_ref() {
            approval_broker::require_auth(auth_request)
        } else {
            approval_broker::allow(&execution_mediation_request)
        };
        let execution = if broker_decision.state == "allow" {
            let system_prompt = run_context.actor_manifest.system_prompt();
            let response = adapter
                .execute_resolved_turn(
                    &run_context.resolved_target,
                    &input.content,
                    Some(system_prompt.as_str()),
                )
                .await?;
            let _ = adapter.resolve_consumed_tokens(&run_context.configured_model, &response)?;
            Some(response)
        } else {
            None
        };
        let consumed_tokens = execution
            .as_ref()
            .map(|response| {
                adapter.resolve_consumed_tokens(&run_context.configured_model, response)
            })
            .transpose()?
            .flatten();

        let (
            user_message,
            submitted_trace,
            execution_trace,
            assistant_message,
            approval,
            run,
            conversation_id,
            project_id,
        ) = apply_submit_state(
            adapter,
            &run_context,
            &input,
            &mediation_request,
            broker_decision,
            execution.as_ref(),
            consumed_tokens,
        )?;

        execution_events::record_submit_turn_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &run_context.resolved_target,
            &submitted_trace,
            execution_trace.as_ref(),
            execution.as_ref(),
            consumed_tokens,
        )
        .await?;
        execution_events::emit_submit_turn_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            user_message,
            submitted_trace,
            assistant_message,
            execution_trace,
            approval,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resume_after_approval(
        adapter: &RuntimeAdapter,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let decision_status = approval_flow::approval_decision_status(&input.decision)?;
        let (
            actor_manifest,
            session_policy,
            checkpoint,
            resolved_target,
            configured_model,
            capability_state_ref,
        ) = load_pending_checkpoint(adapter, session_id, approval_id)?;
        let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
        if decision_status == "approved" {
            capability_store.approve_tool(actor_manifest.label().to_string());
        }
        let capability_projection = adapter
            .project_capability_state_async(
                &actor_manifest,
                &session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref,
                &capability_store,
            )
            .await?;

        let execution = if decision_status == "approved" && !provider_auth_required(&capability_projection) {
            let content = checkpoint
                .serialized_session
                .get("content")
                .or_else(|| checkpoint.serialized_session.get("pendingContent"))
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::runtime("pending approval content is unavailable"))?;
            let system_prompt = actor_manifest.system_prompt();
            let response = adapter
                .execute_resolved_turn(&resolved_target, content, Some(system_prompt.as_str()))
                .await?;
            let _ = adapter.resolve_consumed_tokens(&configured_model, &response)?;
            Some(response)
        } else {
            None
        };
        let consumed_tokens = execution
            .as_ref()
            .map(|response| adapter.resolve_consumed_tokens(&configured_model, response))
            .transpose()?
            .flatten();
        let (approval, execution_trace, assistant_message, run, conversation_id, project_id) =
            apply_approval_resolution_state(
                adapter,
                session_id,
                approval_id,
                now,
                decision_status,
                &actor_manifest,
                &session_policy,
                capability_projection,
                execution.as_ref(),
                consumed_tokens,
            )?;

        execution_events::record_approval_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &approval,
            &input.decision,
            execution_trace.as_ref(),
            execution.as_ref(),
            consumed_tokens,
        )
        .await?;
        execution_events::emit_approval_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            approval,
            input.decision,
            assistant_message,
            execution_trace,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resolve_auth_challenge(
        adapter: &RuntimeAdapter,
        session_id: &str,
        challenge_id: &str,
        input: ResolveRuntimeAuthChallengeInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let resolution = approval_flow::auth_challenge_resolution_status(&input.resolution)?;
        let (
            actor_manifest,
            session_policy,
            checkpoint,
            resolved_target,
            configured_model,
            capability_state_ref,
        ) = load_pending_auth_checkpoint(adapter, session_id, challenge_id)?;
        let capability_store = adapter.load_capability_store(Some(&capability_state_ref))?;
        if resolution == "resolved" {
            capability_store.resolve_tool_auth(actor_manifest.label().to_string());
        }
        let capability_projection = adapter
            .project_capability_state_async(
                &actor_manifest,
                &session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref,
                &capability_store,
            )
            .await?;

        let execution = if resolution == "resolved" {
            let content = checkpoint
                .serialized_session
                .get("content")
                .or_else(|| checkpoint.serialized_session.get("pendingContent"))
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::runtime("pending auth content is unavailable"))?;
            let system_prompt = actor_manifest.system_prompt();
            let response = adapter
                .execute_resolved_turn(&resolved_target, content, Some(system_prompt.as_str()))
                .await?;
            let _ = adapter.resolve_consumed_tokens(&configured_model, &response)?;
            Some(response)
        } else {
            None
        };
        let consumed_tokens = execution
            .as_ref()
            .map(|response| adapter.resolve_consumed_tokens(&configured_model, response))
            .transpose()?
            .flatten();
        let (challenge, execution_trace, assistant_message, run, conversation_id, project_id) =
            apply_auth_challenge_resolution_state(
                adapter,
                session_id,
                challenge_id,
                now,
                resolution,
                &input,
                &actor_manifest,
                &session_policy,
                capability_projection,
                execution.as_ref(),
                consumed_tokens,
            )?;

        execution_events::record_auth_challenge_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &challenge,
            &input.resolution,
            execution_trace.as_ref(),
            execution.as_ref(),
            consumed_tokens,
        )
        .await?;
        execution_events::emit_auth_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            challenge,
            input.resolution,
            assistant_message,
            execution_trace,
        )
        .await?;

        Ok(run)
    }

    pub(crate) async fn resolve_memory_proposal(
        adapter: &RuntimeAdapter,
        session_id: &str,
        proposal_id: &str,
        input: ResolveRuntimeMemoryProposalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let decision_status = approval_flow::memory_proposal_decision_status(&input.decision)?;
        let (proposal, run, conversation_id, project_id) = apply_memory_proposal_resolution_state(
            adapter,
            session_id,
            proposal_id,
            now,
            decision_status,
            &input,
        )?;

        execution_events::record_memory_proposal_resolution_activity(
            adapter,
            session_id,
            now,
            &project_id,
            &run,
            &proposal,
            &input.decision,
        )
        .await?;
        execution_events::emit_memory_proposal_resolution_events(
            adapter,
            session_id,
            now,
            conversation_id,
            project_id,
            run.clone(),
            proposal,
            input.decision,
        )
        .await?;

        Ok(run)
    }
}

fn load_pending_checkpoint(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
) -> Result<
    (
        actor_manifest::CompiledActorManifest,
        session_policy::CompiledSessionPolicy,
        RuntimeRunCheckpoint,
        ResolvedExecutionTarget,
        ConfiguredModelRecord,
        String,
    ),
    AppError,
> {
    let (
        approval,
        session_policy_snapshot_ref,
        persisted_checkpoint,
        configured_model_id,
        capability_state_ref,
    ) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let approval = aggregate
            .detail
            .pending_approval
            .clone()
            .ok_or_else(|| AppError::not_found("runtime approval"))?;
        if approval.id != approval_id {
            return Err(AppError::not_found("runtime approval"));
        }
        let checkpoint = aggregate.detail.run.checkpoint.clone();
        (
            approval,
            aggregate.metadata.session_policy_snapshot_ref.clone(),
            checkpoint.clone(),
            aggregate.detail.run.configured_model_id.clone(),
            aggregate
                .detail
                .run
                .capability_state_ref
                .clone()
                .or_else(|| aggregate.detail.capability_state_ref.clone())
                .unwrap_or_else(|| format!("{}-capability-state", aggregate.detail.run.id)),
        )
    };
    let checkpoint = adapter
        .load_runtime_artifact::<RuntimeRunCheckpoint>(
            persisted_checkpoint.checkpoint_artifact_ref.as_deref(),
        )?
        .unwrap_or(persisted_checkpoint);
    let session_policy = adapter.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
    let actor_manifest =
        adapter.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)?;
    let configured_model_id = configured_model_id
        .or_else(|| session_policy.selected_configured_model_id.clone())
        .filter(|configured_model_id| !configured_model_id.is_empty())
        .ok_or_else(|| AppError::runtime("configured model is unavailable for approval resume"))?;
    let (resolved_target, configured_model) = adapter
        .resolve_approved_execution(&session_policy.config_snapshot_id, &configured_model_id)?;
    if approval.status != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime approval `{approval_id}` is already {}",
            approval.status
        )));
    }
    Ok((
        actor_manifest,
        session_policy,
        checkpoint,
        resolved_target,
        configured_model,
        capability_state_ref,
    ))
}

fn load_pending_auth_checkpoint(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
) -> Result<
    (
        actor_manifest::CompiledActorManifest,
        session_policy::CompiledSessionPolicy,
        RuntimeRunCheckpoint,
        ResolvedExecutionTarget,
        ConfiguredModelRecord,
        String,
    ),
    AppError,
> {
    let (
        challenge,
        session_policy_snapshot_ref,
        persisted_checkpoint,
        configured_model_id,
        capability_state_ref,
    ) = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        let challenge = aggregate
            .detail
            .run
            .checkpoint
            .pending_auth_challenge
            .clone()
            .or_else(|| aggregate.detail.run.auth_target.clone())
            .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
        if challenge.id != challenge_id {
            return Err(AppError::not_found("runtime auth challenge"));
        }
        let checkpoint = aggregate.detail.run.checkpoint.clone();
        (
            challenge,
            aggregate.metadata.session_policy_snapshot_ref.clone(),
            checkpoint.clone(),
            aggregate.detail.run.configured_model_id.clone(),
            aggregate
                .detail
                .run
                .capability_state_ref
                .clone()
                .or_else(|| aggregate.detail.capability_state_ref.clone())
                .unwrap_or_else(|| format!("{}-capability-state", aggregate.detail.run.id)),
        )
    };
    let checkpoint = adapter
        .load_runtime_artifact::<RuntimeRunCheckpoint>(
            persisted_checkpoint.checkpoint_artifact_ref.as_deref(),
        )?
        .unwrap_or(persisted_checkpoint);
    let session_policy = adapter.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
    let actor_manifest =
        adapter.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)?;
    let configured_model_id = configured_model_id
        .or_else(|| session_policy.selected_configured_model_id.clone())
        .filter(|configured_model_id| !configured_model_id.is_empty())
        .ok_or_else(|| AppError::runtime("configured model is unavailable for auth resume"))?;
    let (resolved_target, configured_model) = adapter
        .resolve_approved_execution(&session_policy.config_snapshot_id, &configured_model_id)?;
    if challenge.status != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime auth challenge `{challenge_id}` is already {}",
            challenge.status
        )));
    }
    Ok((
        actor_manifest,
        session_policy,
        checkpoint,
        resolved_target,
        configured_model,
        capability_state_ref,
    ))
}

fn apply_submit_state(
    adapter: &RuntimeAdapter,
    run_context: &run_context::RunContext,
    input: &SubmitRuntimeTurnInput,
    mediation_request: &approval_broker::MediationRequest,
    broker_decision: approval_broker::BrokerDecision,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
) -> Result<SubmitState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(&run_context.session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let actor_ref = run_context.actor_manifest.actor_ref().to_string();
    let actor_label = run_context.actor_manifest.label().to_string();
    let requested_actor_kind = Some(run_context.actor_manifest.actor_kind_label().to_string());
    let requested_actor_id = Some(actor_ref.clone());
    let mut approval = broker_decision.approval.clone();
    let mut auth_target = broker_decision.auth_challenge.clone();
    let blocking_pending_mediation = broker_decision.pending_mediation.clone();
    let memory_pending_mediation = if blocking_pending_mediation.is_none() {
        memory_proposal_pending_mediation(run_context)
    } else {
        None
    };
    let mut pending_mediation = blocking_pending_mediation
        .clone()
        .or(memory_pending_mediation.clone());
    let last_execution_outcome = Some(broker_decision.execution_outcome.clone());
    let mut last_mediation_outcome = broker_decision.mediation_outcome.clone();
    let has_blocking_mediation = approval.is_some() || auth_target.is_some();
    let (run_status, current_step, next_action) =
        blocking_mediation_state(approval.as_ref(), auth_target.as_ref());
    let serialized_session = serialized_runtime_session(&input.content, &run_context.trace_context);

    let user_message = RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        sender_type: "user".into(),
        sender_label: "User".into(),
        content: input.content.clone(),
        timestamp: run_context.now,
        configured_model_id: Some(run_context.resolved_target.configured_model_id.clone()),
        configured_model_name: Some(run_context.resolved_target.configured_model_name.clone()),
        model_id: Some(run_context.resolved_target.registry_model_id.clone()),
        status: run_status.into(),
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: requested_actor_kind.clone(),
        resolved_actor_id: requested_actor_id.clone(),
        resolved_actor_label: Some(actor_label.clone()),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(Vec::new()),
        usage: None,
        tool_calls: None,
        process_entries: None,
    };
    aggregate.detail.messages.push(user_message.clone());

    let submitted_trace = build_submit_trace(
        &run_context.session_id,
        &aggregate.detail.run.id,
        &run_context.conversation_id,
        &run_context.actor_manifest,
        run_context.now,
        if approval.is_some() {
            format!(
                "Capability plan prepared for {}. Turn suspended pending approval for permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        } else if auth_target.is_some() {
            format!(
                "Capability plan prepared for {}. Turn suspended pending provider authentication.",
                actor_label
            )
        } else {
            format!(
                "Capability plan prepared for {}. Turn executing with permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        },
        if has_blocking_mediation {
            "warning"
        } else {
            "success"
        },
    );
    aggregate.detail.trace.push(submitted_trace.clone());

    let assistant_message = execution.map(|response| RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: run_context.session_id.clone(),
        conversation_id: run_context.conversation_id.clone(),
        sender_type: "assistant".into(),
        sender_label: actor_label.clone(),
        content: response.content.clone(),
        timestamp: run_context.now,
        configured_model_id: Some(run_context.resolved_target.configured_model_id.clone()),
        configured_model_name: Some(run_context.resolved_target.configured_model_name.clone()),
        model_id: Some(run_context.resolved_target.registry_model_id.clone()),
        status: "completed".into(),
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: requested_actor_kind.clone(),
        resolved_actor_id: requested_actor_id.clone(),
        resolved_actor_label: Some(actor_label.clone()),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(vec![format!("artifact-{}", aggregate.detail.run.id)]),
        usage: None,
        tool_calls: None,
        process_entries: None,
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            &run_context.session_id,
            &aggregate.detail.run.id,
            &run_context.conversation_id,
            &run_context.actor_manifest,
            &run_context.resolved_target,
            response,
            run_context.now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    let usage_summary = usage_summary_from_tokens(
        consumed_tokens.or_else(|| execution.and_then(|item| item.total_tokens)),
    );
    let checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
        adapter,
        &run_context.session_id,
        &run_context.run_id,
        &mut approval,
        &mut auth_target,
        &mut pending_mediation,
        &mut last_mediation_outcome,
    );
    let mut checkpoint = build_runtime_checkpoint(
        serialized_session.clone(),
        1,
        usage_summary.clone(),
        approval.clone(),
        auth_target.clone(),
        pending_mediation.clone(),
        Some(run_context.capability_state_ref.clone()),
        run_context.capability_plan_summary.clone(),
        last_execution_outcome.clone(),
        last_mediation_outcome.clone(),
        Some(mediation_request),
        Some(&broker_decision),
        checkpoint_artifact_ref.clone(),
    );
    if let Some(mediation_id) = pending_mediation
        .as_ref()
        .and_then(|mediation| mediation.mediation_id.as_deref())
    {
        let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
            &run_context.session_id,
            &run_context.run_id,
            mediation_id,
            &checkpoint,
        )?;
        checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
        apply_checkpoint_ref(
            &mut approval,
            &mut auth_target,
            &mut pending_mediation,
            &mut last_mediation_outcome,
            &storage_path,
        );
        checkpoint.pending_approval = approval.clone();
        checkpoint.pending_auth_challenge = auth_target.clone();
        checkpoint.pending_mediation = pending_mediation.clone();
        checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
    }
    aggregate.detail.run.checkpoint = checkpoint;

    aggregate.detail.summary.status = run_status.into();
    aggregate.detail.summary.updated_at = run_context.now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .unwrap_or_else(|| input.content.clone()),
    );
    aggregate.detail.summary.active_run_id = aggregate.detail.run.id.clone();
    aggregate.detail.summary.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.summary.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        u64::from(run_context.pending_memory_proposal.is_some());
    aggregate.detail.summary.memory_state_ref =
        run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.summary.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.policy_decision_summary =
        run_context.policy_decision_summary.clone();
    aggregate.detail.summary.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.run.status = run_status.into();
    aggregate.detail.run.current_step = current_step.into();
    aggregate.detail.run.updated_at = run_context.now;
    aggregate.detail.run.configured_model_id =
        Some(run_context.resolved_target.configured_model_id.clone());
    aggregate.detail.run.configured_model_name =
        Some(run_context.resolved_target.configured_model_name.clone());
    aggregate.detail.run.model_id = Some(run_context.resolved_target.registry_model_id.clone());
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(next_action.into());
    aggregate.detail.run.selected_memory = run_context.memory_selection.selected_memory.clone();
    aggregate.detail.run.freshness_summary =
        Some(run_context.memory_selection.freshness_summary.clone());
    aggregate.detail.run.pending_memory_proposal = run_context.pending_memory_proposal.clone();
    aggregate.detail.run.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.run.actor_ref = actor_ref.clone();
    aggregate.detail.run.approval_state = if approval.is_some() {
        "pending".into()
    } else if auth_target.is_some() {
        "auth-required".into()
    } else {
        "not-required".into()
    };
    aggregate.detail.run.approval_target = approval.clone();
    aggregate.detail.run.auth_target = auth_target.clone();
    aggregate.detail.run.usage_summary = usage_summary;
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![format!("artifact-{}", aggregate.detail.run.id)]
    } else {
        Vec::new()
    };
    aggregate.detail.run.trace_context = run_context.trace_context.clone();
    aggregate.detail.run.capability_plan_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.run.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.pending_approval = approval.clone();
    aggregate.detail.run.resolved_target = Some(run_context.resolved_target.clone());
    aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
    aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
    aggregate.detail.run.resolved_actor_kind = requested_actor_kind;
    aggregate.detail.run.resolved_actor_id = requested_actor_id;
    aggregate.detail.run.resolved_actor_label = Some(actor_label);
    aggregate.detail.memory_summary = run_context.memory_selection.summary.clone();
    aggregate.detail.memory_selection_summary =
        run_context.memory_selection.selection_summary.clone();
    aggregate.detail.pending_memory_proposal_count =
        u64::from(run_context.pending_memory_proposal.is_some());
    aggregate.detail.memory_state_ref = run_context.memory_selection.memory_state_ref.clone();
    aggregate.detail.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.auth_state_summary = run_context.auth_state_summary.clone();
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.policy_decision_summary = run_context.policy_decision_summary.clone();
    aggregate.detail.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(team_manifest) = &run_context.actor_manifest
    {
        team_runtime::apply_team_runtime_projection(
            &mut aggregate.detail,
            team_manifest,
            run_context.now,
        );
    }
    sync_runtime_session_detail(&mut aggregate.detail);

    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_session(&run_context.session_id, aggregate)?;

    Ok((
        user_message,
        submitted_trace,
        execution_trace,
        assistant_message,
        approval,
        run,
        conversation_id,
        project_id,
    ))
}

fn apply_approval_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    now: u64,
    decision_status: &str,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
) -> Result<ApprovalResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let pending = aggregate
        .detail
        .pending_approval
        .as_mut()
        .ok_or_else(|| AppError::not_found("runtime approval"))?;
    if pending.id != approval_id {
        return Err(AppError::not_found("runtime approval"));
    }
    pending.status = decision_status.into();
    let mut approval = pending.clone();
    let mut auth_target = None;
    let mut pending_mediation = None;
    let mut checkpoint_artifact_ref = aggregate
        .detail
        .run
        .checkpoint
        .checkpoint_artifact_ref
        .clone()
        .or_else(|| approval.checkpoint_ref.clone());
    let mut auth_state_summary = capability_projection.auth_state_summary.clone();
    let policy_decision_summary = capability_projection.policy_decision_summary.clone();
    let approved_requires_auth =
        decision_status == "approved" && provider_auth_required(&capability_projection);
    let mut last_execution_outcome = Some(if decision_status == "approved" {
        RuntimeCapabilityExecutionOutcome {
            capability_id: approval.capability_id.clone(),
            tool_name: Some(approval.tool_name.clone()),
            provider_key: approval.provider_key.clone(),
            dispatch_kind: approval.target_kind.clone(),
            outcome: "allow".into(),
            detail: None,
            requires_approval: approval.requires_approval,
            requires_auth: approval.requires_auth,
            concurrency_policy: Some("serialized".into()),
        }
    } else {
        RuntimeCapabilityExecutionOutcome {
            capability_id: approval.capability_id.clone(),
            tool_name: Some(approval.tool_name.clone()),
            provider_key: approval.provider_key.clone(),
            dispatch_kind: approval.target_kind.clone(),
            outcome: "deny".into(),
            detail: Some("approval request was rejected".into()),
            requires_approval: approval.requires_approval,
            requires_auth: approval.requires_auth,
            concurrency_policy: Some("serialized".into()),
        }
    });
    let mut last_mediation_outcome = Some(RuntimeMediationOutcome {
        approval_layer: approval.approval_layer.clone(),
        capability_id: approval.capability_id.clone(),
        checkpoint_ref: approval.checkpoint_ref.clone(),
        detail: Some(approval.detail.clone()),
        mediation_id: Some(approval.id.clone()),
        mediation_kind: "approval".into(),
        outcome: decision_status.into(),
        provider_key: approval.provider_key.clone(),
        reason: approval.escalation_reason.clone(),
        requires_approval: approval.requires_approval,
        requires_auth: approval.requires_auth,
        resolved_at: Some(now),
        target_kind: approval.target_kind.clone().unwrap_or_default(),
        target_ref: approval.target_ref.clone().unwrap_or_default(),
        tool_name: Some(approval.tool_name.clone()),
    });
    let mut checkpoint = None;

    if approved_requires_auth {
        let provider_auth_policy_decision = session_policy
            .target_decisions
            .get(&format!("provider-auth:{}", actor_manifest.actor_ref()));
        let auth_request = provider_auth_mediation_request(
            session_id,
            &aggregate.detail.summary.conversation_id,
            &aggregate.detail.run.id,
            actor_manifest,
            &capability_projection.provider_state_summary,
            &capability_projection.auth_state_summary,
            provider_auth_policy_decision,
            None,
            now,
        )
        .ok_or_else(|| AppError::runtime("provider auth mediation request missing target"))?;
        let broker_decision = approval_broker::require_auth(&auth_request);
        let mut checkpoint_approval = Some(approval.clone());

        auth_target = broker_decision.auth_challenge.clone();
        pending_mediation = broker_decision.pending_mediation.clone();
        last_execution_outcome = Some(broker_decision.execution_outcome.clone());
        last_mediation_outcome = broker_decision.mediation_outcome.clone();
        checkpoint_artifact_ref = finalize_mediation_checkpoint_ref(
            adapter,
            session_id,
            &aggregate.detail.run.id,
            &mut checkpoint_approval,
            &mut auth_target,
            &mut pending_mediation,
            &mut last_mediation_outcome,
        );

        let usage_summary = usage_summary_from_tokens(
            consumed_tokens.or_else(|| execution.and_then(|item| item.total_tokens)),
        );
        let serialized_session = serialized_runtime_session(
            &aggregate
                .detail
                .messages
                .iter()
                .rev()
                .find(|message| message.sender_type == "user")
                .map(|message| message.content.clone())
                .unwrap_or_default(),
            &aggregate.detail.run.trace_context,
        );
        let mut next_checkpoint = build_runtime_checkpoint(
            serialized_session,
            1,
            usage_summary,
            None,
            auth_target.clone(),
            pending_mediation.clone(),
            Some(capability_projection.capability_state_ref.clone()),
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
            Some(&auth_request),
            Some(&broker_decision),
            checkpoint_artifact_ref.clone(),
        );
        if let Some(mediation_id) = pending_mediation
            .as_ref()
            .and_then(|mediation| mediation.mediation_id.as_deref())
        {
            let (storage_path, _) = adapter.persist_runtime_mediation_checkpoint(
                session_id,
                &aggregate.detail.run.id,
                mediation_id,
                &next_checkpoint,
            )?;
            next_checkpoint.checkpoint_artifact_ref = Some(storage_path.clone());
            apply_checkpoint_ref(
                &mut checkpoint_approval,
                &mut auth_target,
                &mut pending_mediation,
                &mut last_mediation_outcome,
                &storage_path,
            );
            next_checkpoint.pending_auth_challenge = auth_target.clone();
            next_checkpoint.pending_mediation = pending_mediation.clone();
            next_checkpoint.last_mediation_outcome = last_mediation_outcome.clone();
        }
        if let Some(next_approval) = checkpoint_approval {
            approval = next_approval;
        }
        if let Some(challenge) = auth_target.as_ref() {
            if let Some(provider_key) = challenge.provider_key.clone() {
                if !auth_state_summary
                    .challenged_provider_keys
                    .contains(&provider_key)
                {
                    auth_state_summary
                        .challenged_provider_keys
                        .push(provider_key);
                }
            }
            auth_state_summary.last_challenge_at = Some(challenge.created_at);
            auth_state_summary.pending_challenge_count =
                auth_state_summary.challenged_provider_keys.len() as u64;
        }
        checkpoint = Some(next_checkpoint);
    }

    if let Some(message) = aggregate
        .detail
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.sender_type == "user" && message.status == "waiting_approval")
    {
        message.status = if approved_requires_auth {
            "waiting_input".into()
        } else if decision_status == "approved" {
            "completed".into()
        } else {
            "blocked".into()
        };
    }

    let assistant_message = execution.map(|response| RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        sender_type: "assistant".into(),
        sender_label: actor_manifest.label().to_string(),
        content: response.content.clone(),
        timestamp: now,
        configured_model_id: aggregate.detail.run.configured_model_id.clone(),
        configured_model_name: aggregate.detail.run.configured_model_name.clone(),
        model_id: aggregate.detail.run.model_id.clone(),
        status: "completed".into(),
        requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
        requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
        resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
        resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
        resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(vec![format!("artifact-{}", aggregate.detail.run.id)]),
        usage: None,
        tool_calls: None,
        process_entries: None,
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            session_id,
            &aggregate.detail.run.id,
            &aggregate.detail.summary.conversation_id,
            actor_manifest,
            aggregate
                .detail
                .run
                .resolved_target
                .as_ref()
                .expect("resolved target must exist when approval resumes"),
            response,
            now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    let usage_summary = usage_summary_from_tokens(
        consumed_tokens.or_else(|| execution.and_then(|item| item.total_tokens)),
    );
    let run_status = if approved_requires_auth {
        "waiting_input"
    } else if decision_status == "approved" {
        "completed"
    } else {
        "blocked"
    };
    let current_step = if approved_requires_auth {
        "awaiting_auth"
    } else if decision_status == "approved" {
        "completed"
    } else {
        "approval_rejected"
    };
    let next_action = if approved_requires_auth {
        "auth"
    } else if decision_status == "approved" {
        "idle"
    } else {
        "blocked"
    };
    let approval_state = if approved_requires_auth {
        "auth-required"
    } else {
        decision_status
    };

    aggregate.detail.run.status = run_status.into();
    aggregate.detail.run.current_step = current_step.into();
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(next_action.into());
    aggregate.detail.run.approval_state = approval_state.into();
    aggregate.detail.run.approval_target = None;
    aggregate.detail.run.auth_target = auth_target.clone();
    aggregate.detail.run.usage_summary = usage_summary.clone();
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![format!("artifact-{}", aggregate.detail.run.id)]
    } else {
        Vec::new()
    };
    aggregate.detail.run.checkpoint = if let Some(checkpoint) = checkpoint {
        checkpoint
    } else {
        let mut checkpoint = apply_runtime_resolution_checkpoint(
            usage_summary.clone(),
            serialized_runtime_session(
                &aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.sender_type == "user")
                    .map(|message| message.content.clone())
                    .unwrap_or_default(),
                &aggregate.detail.run.trace_context,
            ),
            None,
            None,
            pending_mediation.clone(),
            Some(capability_projection.capability_state_ref.clone()),
            capability_projection.plan_summary.clone(),
            last_execution_outcome.clone(),
            last_mediation_outcome.clone(),
        );
        checkpoint.approval_layer = approval.approval_layer.clone();
        checkpoint.broker_decision = Some(decision_status.into());
        checkpoint.capability_id = approval.capability_id.clone();
        checkpoint.checkpoint_artifact_ref = checkpoint_artifact_ref;
        checkpoint.provider_key = approval.provider_key.clone();
        checkpoint.reason = approval.escalation_reason.clone();
        checkpoint.required_permission = approval.required_permission.clone();
        checkpoint.requires_approval = Some(approval.requires_approval);
        checkpoint.requires_auth = Some(approval.requires_auth);
        checkpoint.target_kind = approval.target_kind.clone();
        checkpoint.target_ref = approval.target_ref.clone();
        checkpoint
    };

    aggregate.detail.summary.status = aggregate.detail.run.status.clone();
    aggregate.detail.summary.updated_at = now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .or_else(|| {
                aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.sender_type == "user")
                    .map(|message| message.content.clone())
            })
            .unwrap_or_default(),
    );
    aggregate.detail.summary.session_policy = session_policy.contract_snapshot();
    aggregate.detail.summary.capability_summary = capability_projection.plan_summary.clone();
    aggregate.detail.summary.memory_summary = aggregate.detail.memory_summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        aggregate.detail.memory_selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        aggregate.detail.pending_memory_proposal_count;
    aggregate.detail.summary.memory_state_ref = aggregate.detail.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.summary.auth_state_summary = auth_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.policy_decision_summary = policy_decision_summary.clone();
    aggregate.detail.summary.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.pending_approval = None;
    aggregate.detail.run.capability_plan_summary = capability_projection.plan_summary.clone();
    aggregate.detail.run.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(capability_projection.capability_state_ref);
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.capability_summary = capability_projection.plan_summary;
    aggregate.detail.provider_state_summary = capability_projection.provider_state_summary;
    aggregate.detail.auth_state_summary = auth_state_summary;
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.policy_decision_summary = policy_decision_summary;
    aggregate.detail.capability_state_ref = aggregate.detail.run.capability_state_ref.clone();
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest {
        team_runtime::apply_team_runtime_projection(&mut aggregate.detail, team_manifest, now);
    }
    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_session(session_id, aggregate)?;

    Ok((
        approval,
        execution_trace,
        assistant_message,
        run,
        conversation_id,
        project_id,
    ))
}

#[allow(clippy::too_many_arguments)]
fn apply_auth_challenge_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
    now: u64,
    resolution: &str,
    input: &ResolveRuntimeAuthChallengeInput,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    capability_projection: capability_planner_bridge::CapabilityProjection,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
) -> Result<AuthChallengeResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let pending = aggregate
        .detail
        .run
        .checkpoint
        .pending_auth_challenge
        .as_mut()
        .ok_or_else(|| AppError::not_found("runtime auth challenge"))?;
    if pending.id != challenge_id {
        return Err(AppError::not_found("runtime auth challenge"));
    }
    pending.status = resolution.into();
    pending.resolution = Some(input.resolution.clone());
    let challenge = pending.clone();
    let pending_mediation = None;
    let last_execution_outcome = Some(if resolution == "resolved" {
        capability_execution_outcome("allow", None, false, false)
    } else {
        capability_execution_outcome(
            "deny",
            Some(format!("auth challenge ended with status `{resolution}`")),
            false,
            true,
        )
    });
    let last_mediation_outcome = Some(RuntimeMediationOutcome {
        approval_layer: Some(challenge.approval_layer.clone()),
        capability_id: challenge.capability_id.clone(),
        checkpoint_ref: challenge.checkpoint_ref.clone(),
        detail: Some(challenge.detail.clone()),
        mediation_id: Some(challenge.id.clone()),
        mediation_kind: "auth".into(),
        outcome: resolution.into(),
        provider_key: challenge.provider_key.clone(),
        reason: Some(challenge.escalation_reason.clone()),
        requires_approval: challenge.requires_approval,
        requires_auth: challenge.requires_auth,
        resolved_at: Some(now),
        target_kind: challenge.target_kind.clone(),
        target_ref: challenge.target_ref.clone(),
        tool_name: challenge.tool_name.clone(),
    });

    let assistant_message = execution.map(|response| RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        sender_type: "assistant".into(),
        sender_label: actor_manifest.label().to_string(),
        content: response.content.clone(),
        timestamp: now,
        configured_model_id: aggregate.detail.run.configured_model_id.clone(),
        configured_model_name: aggregate.detail.run.configured_model_name.clone(),
        model_id: aggregate.detail.run.model_id.clone(),
        status: "completed".into(),
        requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
        requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
        resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
        resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
        resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
        used_default_actor: Some(false),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(vec![format!("artifact-{}", aggregate.detail.run.id)]),
        usage: None,
        tool_calls: None,
        process_entries: None,
    });
    if let Some(message) = assistant_message.as_ref() {
        aggregate.detail.messages.push(message.clone());
    }

    let execution_trace = execution.map(|response| {
        build_execution_trace(
            session_id,
            &aggregate.detail.run.id,
            &aggregate.detail.summary.conversation_id,
            actor_manifest,
            aggregate
                .detail
                .run
                .resolved_target
                .as_ref()
                .expect("resolved target must exist when auth challenge resumes"),
            response,
            now,
            assistant_message.as_ref().map(|message| message.id.clone()),
        )
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    let usage_summary = usage_summary_from_tokens(
        consumed_tokens.or_else(|| execution.and_then(|item| item.total_tokens)),
    );
    aggregate.detail.run.status = if resolution == "resolved" {
        "completed".into()
    } else {
        "blocked".into()
    };
    aggregate.detail.run.current_step = if resolution == "resolved" {
        "completed".into()
    } else {
        "auth_challenge_blocked".into()
    };
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(if resolution == "resolved" {
        "idle".into()
    } else {
        "blocked".into()
    });
    aggregate.detail.run.approval_state = resolution.into();
    aggregate.detail.run.auth_target = None;
    aggregate.detail.run.usage_summary = usage_summary.clone();
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![format!("artifact-{}", aggregate.detail.run.id)]
    } else {
        Vec::new()
    };
    aggregate.detail.run.checkpoint.pending_auth_challenge = None;
    aggregate.detail.run.checkpoint.usage_summary = usage_summary.clone();
    aggregate.detail.run.checkpoint.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.checkpoint.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.run.checkpoint.capability_plan_summary =
        capability_projection.plan_summary.clone();
    aggregate.detail.run.checkpoint.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.checkpoint.last_mediation_outcome = last_mediation_outcome.clone();

    aggregate.detail.summary.status = aggregate.detail.run.status.clone();
    aggregate.detail.summary.updated_at = now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .or_else(|| {
                aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.sender_type == "user")
                    .map(|message| message.content.clone())
            })
            .unwrap_or_default(),
    );
    aggregate.detail.summary.session_policy = session_policy.contract_snapshot();
    aggregate.detail.summary.capability_summary = capability_projection.plan_summary.clone();
    aggregate.detail.summary.memory_summary = aggregate.detail.memory_summary.clone();
    aggregate.detail.summary.memory_selection_summary =
        aggregate.detail.memory_selection_summary.clone();
    aggregate.detail.summary.pending_memory_proposal_count =
        aggregate.detail.pending_memory_proposal_count;
    aggregate.detail.summary.memory_state_ref = aggregate.detail.memory_state_ref.clone();
    aggregate.detail.summary.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.summary.auth_state_summary.last_challenge_at = Some(now);
    if let Some(provider_key) = challenge.provider_key.clone() {
        aggregate
            .detail
            .summary
            .auth_state_summary
            .challenged_provider_keys
            .retain(|value| value != &provider_key);
        aggregate.detail.summary.auth_state_summary.pending_challenge_count = aggregate
            .detail
            .summary
            .auth_state_summary
            .challenged_provider_keys
            .len() as u64;
        if resolution == "resolved" {
            if !aggregate
                .detail
                .summary
                .auth_state_summary
                .resolved_provider_keys
                .contains(&provider_key)
            {
                aggregate
                    .detail
                    .summary
                    .auth_state_summary
                    .resolved_provider_keys
                    .push(provider_key);
            }
        } else if !aggregate
            .detail
            .summary
            .auth_state_summary
            .failed_provider_keys
            .contains(&provider_key)
        {
            aggregate
                .detail
                .summary
                .auth_state_summary
                .failed_provider_keys
                .push(provider_key);
        }
    }

    aggregate.detail.run.capability_plan_summary = capability_projection.plan_summary.clone();
    aggregate.detail.run.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.run.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.capability_state_ref = Some(capability_projection.capability_state_ref);
    aggregate.detail.run.last_execution_outcome = last_execution_outcome.clone();
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.capability_summary = capability_projection.plan_summary;
    aggregate.detail.provider_state_summary = capability_projection.provider_state_summary;
    aggregate.detail.auth_state_summary = aggregate.detail.summary.auth_state_summary.clone();
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.capability_state_ref = aggregate.detail.run.capability_state_ref.clone();
    aggregate.detail.last_execution_outcome = last_execution_outcome;
    if let actor_manifest::CompiledActorManifest::Team(team_manifest) = actor_manifest {
        team_runtime::apply_team_runtime_projection(&mut aggregate.detail, team_manifest, now);
    }
    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_session(session_id, aggregate)?;

    Ok((
        challenge,
        execution_trace,
        assistant_message,
        run,
        conversation_id,
        project_id,
    ))
}

fn apply_memory_proposal_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    proposal_id: &str,
    now: u64,
    decision_status: &str,
    input: &ResolveRuntimeMemoryProposalInput,
) -> Result<MemoryProposalResolutionState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let proposal = aggregate
        .detail
        .run
        .pending_memory_proposal
        .as_mut()
        .ok_or_else(|| AppError::not_found("runtime memory proposal"))?;
    if proposal.proposal_id != proposal_id {
        return Err(AppError::not_found("runtime memory proposal"));
    }
    if proposal.proposal_state != "pending" {
        return Err(AppError::invalid_input(format!(
            "runtime memory proposal `{proposal_id}` is already {}",
            proposal.proposal_state
        )));
    }

    proposal.proposal_state = decision_status.into();
    proposal.review = Some(RuntimeMemoryProposalReview {
        decision: input.decision.clone(),
        reviewed_at: now,
        reviewer_ref: Some(format!("session:{session_id}")),
        note: input.note.clone(),
    });
    let resolved_proposal = proposal.clone();
    let last_mediation_outcome =
        Some(memory_proposal_mediation_outcome(&resolved_proposal, decision_status, now));

    if matches!(decision_status, "approved" | "revalidated") {
        let record = memory_writer::build_persisted_memory_record(
            &resolved_proposal,
            &aggregate.detail.summary.project_id,
            now,
        );
        let body = memory_writer::build_persisted_memory_body(
            &resolved_proposal,
            input.note.as_deref(),
            now,
        );
        adapter.persist_runtime_memory_record(&record, &body)?;
    }

    aggregate.detail.summary.updated_at = now;
    aggregate.detail.run.updated_at = now;
    aggregate.detail.summary.pending_memory_proposal_count = 0;
    aggregate.detail.pending_memory_proposal_count = 0;
    aggregate.detail.summary.pending_mediation = None;
    aggregate.detail.pending_mediation = None;
    let next_memory_state_ref =
        memory_runtime::runtime_memory_state_ref(&aggregate.detail.run.id, now);
    aggregate.detail.summary.memory_state_ref = next_memory_state_ref.clone();
    aggregate.detail.memory_state_ref = next_memory_state_ref.clone();
    aggregate.detail.run.memory_state_ref = next_memory_state_ref;

    if matches!(decision_status, "approved" | "revalidated") {
        aggregate.detail.memory_summary.durable_memory_count += 1;
        aggregate.detail.summary.memory_summary.durable_memory_count += 1;
        if let Some(item) = aggregate
            .detail
            .run
            .selected_memory
            .iter_mut()
            .find(|item| item.memory_id == resolved_proposal.memory_id)
        {
            item.title = resolved_proposal.title.clone();
            item.summary = resolved_proposal.summary.clone();
            item.kind = resolved_proposal.kind.clone();
            item.scope = resolved_proposal.scope.clone();
            item.freshness_state = if decision_status == "revalidated" {
                "revalidated".into()
            } else {
                "fresh".into()
            };
            item.last_validated_at = Some(now);
        }
        if let Some(freshness_summary) = aggregate.detail.run.freshness_summary.as_mut() {
            freshness_summary.fresh_count = aggregate
                .detail
                .run
                .selected_memory
                .iter()
                .filter(|item| matches!(item.freshness_state.as_str(), "fresh" | "revalidated"))
                .count() as u64;
            freshness_summary.stale_count =
                aggregate.detail.run.selected_memory.len() as u64 - freshness_summary.fresh_count;
        }
    }

    aggregate.detail.run.pending_mediation = None;
    aggregate.detail.run.last_mediation_outcome = last_mediation_outcome.clone();
    aggregate.detail.run.checkpoint.pending_mediation = None;
    aggregate.detail.run.checkpoint.last_mediation_outcome = last_mediation_outcome;

    sync_runtime_session_detail(&mut aggregate.detail);
    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_session(session_id, aggregate)?;

    Ok((resolved_proposal, run, conversation_id, project_id))
}
