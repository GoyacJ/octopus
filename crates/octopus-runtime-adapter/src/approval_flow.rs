use super::*;

type ApprovalResolutionState = (
    ApprovalRequestRecord,
    Option<RuntimeTraceItem>,
    Option<RuntimeMessage>,
    RuntimeRunSnapshot,
    String,
    String,
);

pub(crate) fn approval_decision_status(decision: &str) -> Result<&'static str, AppError> {
    match decision {
        "approve" => Ok("approved"),
        "reject" => Ok("rejected"),
        _ => Err(AppError::invalid_input(
            "approval decision must be approve or reject",
        )),
    }
}

fn load_pending_approval_context(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    decision_status: &str,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<ResolvedExecutionTarget>,
        Option<String>,
    ),
    AppError,
> {
    let sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    let aggregate = sessions
        .get(session_id)
        .ok_or_else(|| AppError::not_found("runtime session"))?;
    let pending = aggregate
        .detail
        .pending_approval
        .as_ref()
        .ok_or_else(|| AppError::not_found("runtime approval"))?;
    if pending.id != approval_id {
        return Err(AppError::not_found("runtime approval"));
    }

    if decision_status == "approved" {
        let pending_input = aggregate
            .detail
            .messages
            .iter()
            .rev()
            .find(|message| message.sender_type == "user" && message.status == "waiting_approval")
            .map(|message| message.content.clone())
            .ok_or_else(|| AppError::runtime("pending approval input is unavailable"))?;
        let resolved_target = aggregate
            .detail
            .run
            .resolved_target
            .clone()
            .ok_or_else(|| AppError::runtime("resolved execution target is unavailable"))?;
        Ok((
            Some(pending_input),
            aggregate.detail.run.resolved_actor_kind.clone(),
            aggregate.detail.run.resolved_actor_id.clone(),
            Some(resolved_target),
            Some(aggregate.detail.run.config_snapshot_id.clone()),
        ))
    } else {
        Ok((None, None, None, None, None))
    }
}

fn apply_approval_resolution_state(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    now: u64,
    decision_status: &str,
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
        session_id: session_id.into(),
        conversation_id: aggregate.detail.summary.conversation_id.clone(),
        sender_type: "assistant".into(),
        sender_label: aggregate
            .detail
            .run
            .resolved_actor_label
            .clone()
            .or_else(|| {
                aggregate
                    .detail
                    .run
                    .resolved_target
                    .as_ref()
                    .map(|target| target.provider_id.clone())
            })
            .unwrap_or_else(|| "assistant".into()),
        content: response.content.clone(),
        timestamp: now,
        configured_model_id: aggregate
            .detail
            .run
            .resolved_target
            .as_ref()
            .map(|target| target.configured_model_id.clone()),
        configured_model_name: aggregate
            .detail
            .run
            .resolved_target
            .as_ref()
            .map(|target| target.configured_model_name.clone()),
        model_id: aggregate
            .detail
            .run
            .resolved_target
            .as_ref()
            .map(|target| target.registry_model_id.clone()),
        status: "completed".into(),
        requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
        requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
        resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
        resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
        resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
        used_default_actor: Some(aggregate.detail.run.resolved_actor_id.is_none()),
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
            "Approved turn executed and produced {} characters.",
            response.content.chars().count()
        ),
        tone: "success".into(),
        timestamp: now,
        actor: aggregate
            .detail
            .run
            .resolved_actor_label
            .clone()
            .unwrap_or_else(|| "assistant".into()),
        actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
        actor_id: aggregate.detail.run.resolved_actor_id.clone(),
        related_message_id: assistant_message.as_ref().map(|message| message.id.clone()),
        related_tool_name: None,
    });
    if let Some(trace) = execution_trace.as_ref() {
        aggregate.detail.trace.push(trace.clone());
    }

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

    aggregate.detail.pending_approval = None;
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

pub(super) async fn resolve_approval(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    input: ResolveRuntimeApprovalInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    let now = timestamp_now();
    let decision_status = approval_decision_status(&input.decision)?;

    let (pending_input, pending_actor_kind, pending_actor_id, resolved_target, config_snapshot_id) =
        load_pending_approval_context(adapter, session_id, approval_id, decision_status)?;

    let (resolved_target, configured_model) =
        match (resolved_target.as_ref(), config_snapshot_id.as_deref()) {
            (Some(target), Some(snapshot_id)) => {
                let (resolved_target, configured_model) =
                    adapter.resolve_approved_execution(snapshot_id, &target.configured_model_id)?;
                (Some(resolved_target), Some(configured_model))
            }
            _ => (None, None),
        };
    let execution = match (
        pending_input.as_deref(),
        resolved_target.as_ref(),
        configured_model.as_ref(),
    ) {
        (Some(content), Some(target), Some(configured_model)) => {
            let response = adapter
                .execute_resolved_turn(
                    target,
                    content,
                    pending_actor_kind.as_deref(),
                    pending_actor_id.as_deref(),
                )
                .await?;
            let _ = adapter.resolve_consumed_tokens(configured_model, &response)?;
            Some(response)
        }
        _ => None,
    };
    let consumed_tokens = match (execution.as_ref(), configured_model.as_ref()) {
        (Some(response), Some(configured_model)) => {
            adapter.resolve_consumed_tokens(configured_model, response)?
        }
        _ => None,
    };

    let (approval, execution_trace, assistant_message, run, conversation_id, project_id) =
        apply_approval_resolution_state(
            adapter,
            session_id,
            approval_id,
            now,
            decision_status,
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
