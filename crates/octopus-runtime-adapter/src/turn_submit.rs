use super::*;

type SubmitTurnState = (
    RuntimeMessage,
    RuntimeTraceItem,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    Option<ApprovalRequestRecord>,
    RuntimeRunSnapshot,
    String,
    String,
);

fn apply_submit_turn_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    input: &SubmitRuntimeTurnInput,
    requires_approval: bool,
    normalized_permission_mode: &str,
    resolved_target: &ResolvedExecutionTarget,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
    requested_actor_kind: Option<String>,
    requested_actor_id: Option<String>,
    resolved_actor_kind: Option<String>,
    resolved_actor_id: Option<String>,
    resolved_actor_label: Option<String>,
) -> Result<SubmitTurnState, AppError> {
    let mut sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;

    let user_message = RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.into(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        sender_type: "user".into(),
        sender_label: "User".into(),
        content: input.content.clone(),
        timestamp: now,
        configured_model_id: Some(resolved_target.configured_model_id.clone()),
        configured_model_name: Some(resolved_target.configured_model_name.clone()),
        model_id: Some(resolved_target.registry_model_id.clone()),
        status: if requires_approval {
            "waiting_approval".into()
        } else {
            "completed".into()
        },
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: resolved_actor_kind.clone(),
        resolved_actor_id: resolved_actor_id.clone(),
        resolved_actor_label: resolved_actor_label.clone(),
        used_default_actor: Some(resolved_actor_id.is_none()),
        resource_ids: Some(Vec::new()),
        attachments: Some(Vec::new()),
        artifacts: Some(Vec::new()),
        usage: None,
        tool_calls: None,
        process_entries: None,
    };
    aggregate.detail.messages.push(user_message.clone());

    let submitted_trace = RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.into(),
        run_id: aggregate.detail.run.id.clone(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        kind: "step".into(),
        title: "Turn submitted".into(),
        detail: if requires_approval {
            format!(
                "Permission mode {} requires explicit approval before execution.",
                normalized_permission_mode
            )
        } else {
            format!(
                "Turn submitted and completed with permission mode {}.",
                normalized_permission_mode
            )
        },
        tone: if requires_approval {
            "warning".into()
        } else {
            "success".into()
        },
        timestamp: now,
        actor: resolved_actor_label
            .clone()
            .unwrap_or_else(|| "user".into()),
        actor_kind: resolved_actor_kind.clone(),
        actor_id: resolved_actor_id.clone(),
        related_message_id: Some(user_message.id.clone()),
        related_tool_name: None,
    };
    aggregate.detail.trace.push(submitted_trace.clone());

    let approval = requires_approval.then(|| ApprovalRequestRecord {
        id: format!("approval-{}", Uuid::new_v4()),
        session_id: session_id.into(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        run_id: aggregate.detail.run.id.clone(),
        tool_name: "runtime.turn".into(),
        summary: "Turn requires approval".into(),
        detail: format!(
            "Permission mode {} requires explicit approval.",
            normalized_permission_mode
        ),
        risk_level: "medium".into(),
        created_at: now,
        status: "pending".into(),
    });
    aggregate.detail.pending_approval = approval.clone();

    let assistant_message = execution.map(|response| RuntimeMessage {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.into(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        sender_type: "assistant".into(),
        sender_label: resolved_actor_label
            .clone()
            .unwrap_or_else(|| resolved_target.provider_id.clone()),
        content: response.content.clone(),
        timestamp: now,
        configured_model_id: Some(resolved_target.configured_model_id.clone()),
        configured_model_name: Some(resolved_target.configured_model_name.clone()),
        model_id: Some(resolved_target.registry_model_id.clone()),
        status: "completed".into(),
        requested_actor_kind: requested_actor_kind.clone(),
        requested_actor_id: requested_actor_id.clone(),
        resolved_actor_kind: resolved_actor_kind.clone(),
        resolved_actor_id: resolved_actor_id.clone(),
        resolved_actor_label: resolved_actor_label.clone(),
        used_default_actor: Some(resolved_actor_id.is_none()),
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

    let execution_trace = execution.map(|response| RuntimeTraceItem {
        id: format!("trace-{}", Uuid::new_v4()),
        session_id: session_id.into(),
        run_id: aggregate.detail.run.id.clone(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        kind: "step".into(),
        title: "Model execution completed".into(),
        detail: format!(
            "Resolved {}:{} via {} and produced {} characters.",
            resolved_target.provider_id,
            resolved_target.configured_model_name,
            resolved_target.protocol_family,
            response.content.chars().count()
        ),
        tone: "success".into(),
        timestamp: now,
        actor: resolved_actor_label
            .clone()
            .unwrap_or_else(|| "assistant".into()),
        actor_kind: resolved_actor_kind.clone(),
        actor_id: resolved_actor_id.clone(),
        related_message_id: assistant_message.as_ref().map(|message| message.id.clone()),
        related_tool_name: None,
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

    aggregate.detail.summary.status = if requires_approval {
        "waiting_approval".into()
    } else {
        "completed".into()
    };
    aggregate.detail.summary.updated_at = now;
    aggregate.detail.summary.last_message_preview = Some(
        assistant_message
            .as_ref()
            .map(|message| message.content.clone())
            .unwrap_or_else(|| input.content.clone()),
    );
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
    aggregate.detail.run.updated_at = now;
    aggregate.detail.run.configured_model_id = Some(resolved_target.configured_model_id.clone());
    aggregate.detail.run.configured_model_name =
        Some(resolved_target.configured_model_name.clone());
    aggregate.detail.run.model_id = Some(resolved_target.registry_model_id.clone());
    aggregate.detail.run.consumed_tokens = consumed_tokens;
    aggregate.detail.run.next_action = Some(if requires_approval {
        "approval".into()
    } else {
        "idle".into()
    });
    aggregate.detail.run.resolved_target = Some(resolved_target.clone());
    aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
    aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
    aggregate.detail.run.resolved_actor_kind = resolved_actor_kind.clone();
    aggregate.detail.run.resolved_actor_id = resolved_actor_id.clone();
    aggregate.detail.run.resolved_actor_label = resolved_actor_label.clone();

    let run = aggregate.detail.run.clone();
    let conversation_id = aggregate.detail.summary.conversation_id.clone();
    let project_id = aggregate.detail.summary.project_id.clone();
    adapter.persist_session(session_id, aggregate)?;

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

pub(super) async fn submit_turn(
    adapter: &RuntimeAdapter,
    session_id: &str,
    input: SubmitRuntimeTurnInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    let now = timestamp_now();
    let requires_approval = execution_target::requires_approval(&input.permission_mode)?;
    let normalized_permission_mode =
        normalize_runtime_permission_mode_label(&input.permission_mode).ok_or_else(|| {
            AppError::invalid_input(format!(
                "unsupported permission mode: {}",
                input.permission_mode
            ))
        })?;
    let config_snapshot_id = {
        let sessions = adapter
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        aggregate.detail.run.config_snapshot_id.clone()
    };
    let (resolved_target, configured_model) =
        adapter.resolve_submit_execution(&config_snapshot_id, &input)?;

    let execution = if requires_approval {
        None
    } else {
        let response = adapter
            .execute_resolved_turn(
                &resolved_target,
                &input.content,
                input.actor_kind.as_deref(),
                input.actor_id.as_deref(),
            )
            .await?;
        let _ = adapter.resolve_consumed_tokens(&configured_model, &response)?;
        Some(response)
    };
    let consumed_tokens = execution
        .as_ref()
        .map(|response| adapter.resolve_consumed_tokens(&configured_model, response))
        .transpose()?
        .flatten();
    let requested_actor_kind = input.actor_kind.clone();
    let requested_actor_id = input.actor_id.clone();
    let resolved_actor_kind = input.actor_kind.clone();
    let resolved_actor_id = input.actor_id.clone();
    let resolved_actor_label = actor_context::resolve_actor_label(
        &adapter.state.paths,
        resolved_actor_kind.as_deref(),
        resolved_actor_id.as_deref(),
    );

    let (
        user_message,
        submitted_trace,
        execution_trace,
        assistant_message,
        approval,
        run,
        conversation_id,
        project_id,
    ) = apply_submit_turn_state(
        adapter,
        session_id,
        now,
        &input,
        requires_approval,
        &normalized_permission_mode,
        &resolved_target,
        execution.as_ref(),
        consumed_tokens,
        requested_actor_kind,
        requested_actor_id,
        resolved_actor_kind,
        resolved_actor_id,
        resolved_actor_label,
    )?;

    execution_events::record_submit_turn_activity(
        adapter,
        session_id,
        now,
        &project_id,
        &run,
        &resolved_target,
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
