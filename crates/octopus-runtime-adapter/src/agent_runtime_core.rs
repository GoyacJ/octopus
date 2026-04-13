use super::*;

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

fn usage_summary_from_tokens(total_tokens: Option<u32>) -> RuntimeUsageSummary {
    RuntimeUsageSummary {
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: total_tokens.unwrap_or(0),
    }
}

fn runtime_turn_checkpoint(
    content: &str,
    trace_context: &RuntimeTraceContext,
    usage_summary: RuntimeUsageSummary,
    pending_approval: Option<ApprovalRequestRecord>,
    current_iteration_index: u32,
    pending_mediation: Option<RuntimePendingMediationSummary>,
    capability_state_ref: Option<String>,
    capability_plan_summary: RuntimeCapabilityPlanSummary,
    last_execution_outcome: Option<RuntimeCapabilityExecutionOutcome>,
) -> RuntimeRunCheckpoint {
    RuntimeRunCheckpoint {
        serialized_session: json!({
            "pendingContent": content,
            "traceContext": trace_context,
        }),
        current_iteration_index,
        usage_summary,
        pending_approval,
        compaction_metadata: json!({}),
        pending_mediation,
        capability_state_ref,
        capability_plan_summary,
        last_execution_outcome,
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

fn approval_pending_mediation() -> RuntimePendingMediationSummary {
    RuntimePendingMediationSummary {
        capability_id: Some("runtime.turn".into()),
        tool_name: Some("runtime.turn".into()),
        provider_key: None,
        mediation_kind: "approval".into(),
        reason: Some("execution permission requires explicit approval".into()),
    }
}

fn execution_outcome(
    outcome: &str,
    detail: Option<String>,
    requires_approval: bool,
    requires_auth: bool,
) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: Some("runtime.turn".into()),
        tool_name: Some("runtime.turn".into()),
        provider_key: None,
        dispatch_kind: Some("runtime_turn".into()),
        outcome: outcome.into(),
        detail,
        requires_approval,
        requires_auth,
        concurrency_policy: Some("serialized".into()),
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
        if matches!(
            run_context.actor_manifest,
            actor_manifest::CompiledActorManifest::Team(_)
        ) {
            return Err(AppError::runtime("team_runtime_not_enabled"));
        }
        let requires_approval =
            execution_target::requires_approval(&run_context.requested_permission_mode)?;
        let execution = if requires_approval {
            None
        } else {
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
            requires_approval,
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
        let capability_projection = adapter
            .project_capability_state_async(
                &actor_manifest,
                &session_policy.config_snapshot_id,
                capability_state_ref,
                &capability_store,
            )
            .await?;

        let execution = if decision_status == "approved" {
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
        checkpoint,
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

fn apply_submit_state(
    adapter: &RuntimeAdapter,
    run_context: &run_context::RunContext,
    input: &SubmitRuntimeTurnInput,
    requires_approval: bool,
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
        status: if requires_approval {
            "waiting_approval".into()
        } else {
            "completed".into()
        },
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
        if requires_approval {
            format!(
                "Capability plan prepared for {}. Turn suspended pending approval for permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        } else {
            format!(
                "Capability plan prepared for {}. Turn executing with permission mode {}.",
                actor_label, run_context.requested_permission_mode
            )
        },
        if requires_approval {
            "warning"
        } else {
            "success"
        },
    );
    aggregate.detail.trace.push(submitted_trace.clone());

    let pending_mediation = requires_approval.then(approval_pending_mediation);
    let last_execution_outcome = Some(if requires_approval {
        execution_outcome(
            "require_approval",
            Some(format!(
                "permission mode {} requires explicit approval",
                run_context.requested_permission_mode
            )),
            true,
            false,
        )
    } else {
        execution_outcome("allow", None, false, false)
    });

    let approval = requires_approval.then(|| {
        let approval = ApprovalRequestRecord {
            id: format!("approval-{}", Uuid::new_v4()),
            session_id: run_context.session_id.clone(),
            conversation_id: run_context.conversation_id.clone(),
            run_id: aggregate.detail.run.id.clone(),
            tool_name: "runtime.turn".into(),
            summary: "Turn requires approval".into(),
            detail: format!(
                "Permission mode {} requires explicit approval.",
                run_context.requested_permission_mode
            ),
            risk_level: "medium".into(),
            created_at: run_context.now,
            status: "pending".into(),
            approval_layer: Some("execution-permission".into()),
            target_kind: Some("runtime-turn".into()),
            target_ref: Some(actor_ref.clone()),
            escalation_reason: Some("session ceiling requires approval".into()),
        };
        aggregate.detail.run.checkpoint = runtime_turn_checkpoint(
            &input.content,
            &run_context.trace_context,
            aggregate.detail.run.usage_summary.clone(),
            Some(approval.clone()),
            1,
            pending_mediation.clone(),
            Some(run_context.capability_state_ref.clone()),
            run_context.capability_plan_summary.clone(),
            last_execution_outcome.clone(),
        );
        approval
    });
    aggregate.detail.pending_approval = approval.clone();

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
    if !requires_approval {
        aggregate.detail.run.checkpoint = runtime_turn_checkpoint(
            &input.content,
            &run_context.trace_context,
            usage_summary.clone(),
            None,
            1,
            pending_mediation.clone(),
            Some(run_context.capability_state_ref.clone()),
            run_context.capability_plan_summary.clone(),
            last_execution_outcome.clone(),
        );
    }

    aggregate.detail.summary.status = if requires_approval {
        "waiting_approval".into()
    } else {
        "completed".into()
    };
    aggregate.detail.summary.updated_at = run_context.now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .unwrap_or_else(|| input.content.clone()),
    );
    aggregate.detail.summary.active_run_id = aggregate.detail.run.id.clone();
    aggregate.detail.summary.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.summary.memory_summary = run_context.actor_manifest.memory_summary();
    aggregate.detail.summary.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
    aggregate.detail.summary.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.summary.last_execution_outcome = last_execution_outcome.clone();

    aggregate.detail.run.status = if requires_approval {
        "waiting_approval".into()
    } else {
        "completed".into()
    };
    aggregate.detail.run.current_step = if requires_approval {
        "awaiting_approval".into()
    } else {
        "completed".into()
    };
    aggregate.detail.run.updated_at = run_context.now;
    aggregate.detail.run.configured_model_id =
        Some(run_context.resolved_target.configured_model_id.clone());
    aggregate.detail.run.configured_model_name =
        Some(run_context.resolved_target.configured_model_name.clone());
    aggregate.detail.run.model_id = Some(run_context.resolved_target.registry_model_id.clone());
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(if requires_approval {
        "approval".into()
    } else {
        "idle".into()
    });
    aggregate.detail.run.actor_ref = actor_ref.clone();
    aggregate.detail.run.approval_state = if requires_approval {
        "pending".into()
    } else {
        "not-required".into()
    };
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
    aggregate.detail.run.resolved_target = Some(run_context.resolved_target.clone());
    aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
    aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
    aggregate.detail.run.resolved_actor_kind = requested_actor_kind;
    aggregate.detail.run.resolved_actor_id = requested_actor_id;
    aggregate.detail.run.resolved_actor_label = Some(actor_label);
    aggregate.detail.capability_summary = run_context.capability_plan_summary.clone();
    aggregate.detail.provider_state_summary = run_context.provider_state_summary.clone();
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.capability_state_ref = Some(run_context.capability_state_ref.clone());
    aggregate.detail.last_execution_outcome = last_execution_outcome;
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
    let approval = pending.clone();
    let pending_mediation = None;
    let last_execution_outcome = Some(if decision_status == "approved" {
        execution_outcome("allow", None, false, false)
    } else {
        execution_outcome(
            "deny",
            Some("approval request was rejected".into()),
            true,
            false,
        )
    });

    if let Some(message) = aggregate
        .detail
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.sender_type == "user" && message.status == "waiting_approval")
    {
        message.status = if decision_status == "approved" {
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
    aggregate.detail.run.status = if decision_status == "approved" {
        "completed".into()
    } else {
        "blocked".into()
    };
    aggregate.detail.run.current_step = if decision_status == "approved" {
        "completed".into()
    } else {
        "approval_rejected".into()
    };
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(if decision_status == "approved" {
        "idle".into()
    } else {
        "blocked".into()
    });
    aggregate.detail.run.approval_state = decision_status.into();
    aggregate.detail.run.usage_summary = usage_summary.clone();
    aggregate.detail.run.artifact_refs = if execution.is_some() {
        vec![format!("artifact-{}", aggregate.detail.run.id)]
    } else {
        Vec::new()
    };
    aggregate.detail.run.checkpoint.pending_approval = None;
    aggregate.detail.run.checkpoint.usage_summary = usage_summary.clone();
    aggregate.detail.run.checkpoint.serialized_session = json!({
        "pendingContent": aggregate
            .detail
            .messages
            .iter()
            .rev()
            .find(|message| message.sender_type == "user")
            .map(|message| message.content.clone())
            .unwrap_or_default(),
        "traceContext": aggregate.detail.run.trace_context.clone(),
    });
    aggregate.detail.run.checkpoint.pending_mediation = pending_mediation.clone();
    aggregate.detail.run.checkpoint.capability_state_ref =
        Some(capability_projection.capability_state_ref.clone());
    aggregate.detail.run.checkpoint.capability_plan_summary =
        capability_projection.plan_summary.clone();
    aggregate.detail.run.checkpoint.last_execution_outcome = last_execution_outcome.clone();

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
    aggregate.detail.summary.memory_summary = actor_manifest.memory_summary();
    aggregate.detail.summary.provider_state_summary =
        capability_projection.provider_state_summary.clone();
    aggregate.detail.summary.pending_mediation = pending_mediation.clone();
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
    aggregate.detail.capability_summary = capability_projection.plan_summary;
    aggregate.detail.provider_state_summary = capability_projection.provider_state_summary;
    aggregate.detail.pending_mediation = pending_mediation;
    aggregate.detail.capability_state_ref = aggregate.detail.run.capability_state_ref.clone();
    aggregate.detail.last_execution_outcome = last_execution_outcome;
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
