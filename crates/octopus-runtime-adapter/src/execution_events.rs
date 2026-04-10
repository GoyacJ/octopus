use super::*;

pub(crate) fn usage_cost_shape(total_tokens: Option<u32>) -> (&'static str, i64, &'static str) {
    match total_tokens {
        Some(total_tokens) => ("tokens", i64::from(total_tokens), "tokens"),
        None => ("turns", 1, "count"),
    }
}

pub(super) async fn record_submit_turn_activity(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    resolved_target: &ResolvedExecutionTarget,
    submitted_trace: &RuntimeTraceItem,
    execution_trace: Option<&RuntimeTraceItem>,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
) -> Result<(), AppError> {
    adapter
        .state
        .observation
        .append_trace(TraceEventRecord {
            id: submitted_trace.id.clone(),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            run_id: Some(run.id.clone()),
            session_id: Some(session_id.into()),
            event_kind: "turn_submitted".into(),
            title: submitted_trace.title.clone(),
            detail: submitted_trace.detail.clone(),
            created_at: now,
        })
        .await?;
    if let Some(execution_trace) = execution_trace {
        adapter
            .state
            .observation
            .append_trace(TraceEventRecord {
                id: execution_trace.id.clone(),
                workspace_id: adapter.state.workspace_id.clone(),
                project_id: Some(project_id.to_string()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "turn_executed".into(),
                title: execution_trace.title.clone(),
                detail: execution_trace.detail.clone(),
                created_at: now,
            })
            .await?;
    }
    adapter
        .state
        .observation
        .append_audit(AuditRecord {
            id: format!("audit-{}", Uuid::new_v4()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            actor_type: "session".into(),
            actor_id: session_id.into(),
            action: "runtime.submit_turn".into(),
            resource: run.id.clone(),
            outcome: run.status.clone(),
            created_at: now,
        })
        .await?;

    let (metric, amount, unit) =
        usage_cost_shape(execution.and_then(|response| response.total_tokens));
    adapter
        .state
        .observation
        .append_cost(CostLedgerEntry {
            id: format!("cost-{}", Uuid::new_v4()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            run_id: Some(run.id.clone()),
            configured_model_id: Some(resolved_target.configured_model_id.clone()),
            metric: metric.into(),
            amount,
            unit: unit.into(),
            created_at: now,
        })
        .await?;

    if let Some(consumed_tokens) = consumed_tokens {
        adapter.increment_configured_model_usage(
            &resolved_target.configured_model_id,
            consumed_tokens,
            now,
        )?;
    }

    Ok(())
}

pub(super) async fn emit_submit_turn_events(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: String,
    project_id: String,
    run: RuntimeRunSnapshot,
    user_message: RuntimeMessage,
    submitted_trace: RuntimeTraceItem,
    assistant_message: Option<RuntimeMessage>,
    execution_trace: Option<RuntimeTraceItem>,
    approval: Option<ApprovalRequestRecord>,
) -> Result<(), AppError> {
    let mut events = vec![
        RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.message.created".into(),
            kind: Some("runtime.message.created".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "message": user_message.clone(),
            })),
            run: None,
            message: Some(user_message),
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
        },
        RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.trace.emitted".into(),
            kind: Some("runtime.trace.emitted".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "trace": submitted_trace.clone(),
            })),
            run: None,
            message: None,
            trace: Some(submitted_trace),
            approval: None,
            decision: None,
            summary: None,
            error: None,
        },
    ];

    if let Some(message) = assistant_message {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.message.created".into(),
            kind: Some("runtime.message.created".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "message": message.clone(),
            })),
            run: None,
            message: Some(message),
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });
    }

    if let Some(trace) = execution_trace {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.trace.emitted".into(),
            kind: Some("runtime.trace.emitted".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "trace": trace.clone(),
            })),
            run: None,
            message: None,
            trace: Some(trace),
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });
    }

    if let Some(approval) = approval {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.approval.requested".into(),
            kind: Some("runtime.approval.requested".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "approval": approval.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: Some(approval),
            decision: None,
            summary: None,
            error: None,
        });
    }

    events.push(RuntimeEventEnvelope {
        id: format!("evt-{}", Uuid::new_v4()),
        event_type: "runtime.run.updated".into(),
        kind: Some("runtime.run.updated".into()),
        workspace_id: adapter.state.workspace_id.clone(),
        project_id: optional_project_id(&project_id),
        session_id: session_id.into(),
        conversation_id,
        run_id: Some(run.id.clone()),
        emitted_at: now,
        sequence: 0,
        payload: Some(json!({
            "run": run.clone(),
        })),
        run: Some(run),
        message: None,
        trace: None,
        approval: None,
        decision: None,
        summary: None,
        error: None,
    });

    for event in events {
        adapter.emit_event(session_id, event).await?;
    }

    Ok(())
}

pub(super) async fn record_approval_resolution_activity(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    approval: &ApprovalRequestRecord,
    decision: &str,
    execution_trace: Option<&RuntimeTraceItem>,
    execution: Option<&ExecutionResponse>,
    consumed_tokens: Option<u32>,
) -> Result<(), AppError> {
    adapter
        .state
        .observation
        .append_trace(TraceEventRecord {
            id: format!("trace-{}", Uuid::new_v4()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            run_id: Some(run.id.clone()),
            session_id: Some(session_id.into()),
            event_kind: "approval_resolved".into(),
            title: "Approval resolved".into(),
            detail: decision.to_string(),
            created_at: now,
        })
        .await?;
    if let Some(execution_trace) = execution_trace {
        adapter
            .state
            .observation
            .append_trace(TraceEventRecord {
                id: execution_trace.id.clone(),
                workspace_id: adapter.state.workspace_id.clone(),
                project_id: Some(project_id.to_string()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "turn_executed".into(),
                title: execution_trace.title.clone(),
                detail: execution_trace.detail.clone(),
                created_at: now,
            })
            .await?;
    }
    adapter
        .state
        .observation
        .append_audit(AuditRecord {
            id: format!("audit-{}", Uuid::new_v4()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            actor_type: "session".into(),
            actor_id: session_id.into(),
            action: "runtime.resolve_approval".into(),
            resource: approval.id.clone(),
            outcome: decision.to_string(),
            created_at: now,
        })
        .await?;

    if let Some(response) = execution {
        let (metric, amount, unit) = usage_cost_shape(response.total_tokens);
        adapter
            .state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: adapter.state.workspace_id.clone(),
                project_id: Some(project_id.to_string()),
                run_id: Some(run.id.clone()),
                configured_model_id: run
                    .resolved_target
                    .as_ref()
                    .map(|target| target.configured_model_id.clone()),
                metric: metric.into(),
                amount,
                unit: unit.into(),
                created_at: now,
            })
            .await?;
    }

    if let (Some(consumed_tokens), Some(resolved_target)) =
        (consumed_tokens, run.resolved_target.as_ref())
    {
        adapter.increment_configured_model_usage(
            &resolved_target.configured_model_id,
            consumed_tokens,
            now,
        )?;
    }

    Ok(())
}

pub(super) async fn emit_approval_resolution_events(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: String,
    project_id: String,
    run: RuntimeRunSnapshot,
    approval: ApprovalRequestRecord,
    decision: String,
    assistant_message: Option<RuntimeMessage>,
    execution_trace: Option<RuntimeTraceItem>,
) -> Result<(), AppError> {
    let mut events = vec![RuntimeEventEnvelope {
        id: format!("evt-{}", Uuid::new_v4()),
        event_type: "runtime.approval.resolved".into(),
        kind: Some("runtime.approval.resolved".into()),
        workspace_id: adapter.state.workspace_id.clone(),
        project_id: optional_project_id(&project_id),
        session_id: session_id.into(),
        conversation_id: conversation_id.clone(),
        run_id: Some(run.id.clone()),
        emitted_at: now,
        sequence: 0,
        payload: Some(json!({
            "approval": approval.clone(),
            "decision": decision.clone(),
            "run": run.clone(),
        })),
        run: Some(run.clone()),
        message: None,
        trace: None,
        approval: Some(approval.clone()),
        decision: Some(decision.clone()),
        summary: None,
        error: None,
    }];

    if let Some(message) = assistant_message {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.message.created".into(),
            kind: Some("runtime.message.created".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "message": message.clone(),
            })),
            run: None,
            message: Some(message),
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });
    }
    if let Some(trace) = execution_trace {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.trace.emitted".into(),
            kind: Some("runtime.trace.emitted".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "trace": trace.clone(),
            })),
            run: None,
            message: None,
            trace: Some(trace),
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });
    }
    events.push(RuntimeEventEnvelope {
        id: format!("evt-{}", Uuid::new_v4()),
        event_type: "runtime.run.updated".into(),
        kind: Some("runtime.run.updated".into()),
        workspace_id: adapter.state.workspace_id.clone(),
        project_id: optional_project_id(&project_id),
        session_id: session_id.into(),
        conversation_id,
        run_id: Some(run.id.clone()),
        emitted_at: now,
        sequence: 0,
        payload: Some(json!({
            "approval": approval.clone(),
            "decision": decision.clone(),
            "run": run.clone(),
        })),
        run: Some(run),
        message: None,
        trace: None,
        approval: Some(approval),
        decision: Some(decision),
        summary: None,
        error: None,
    });

    for event in events {
        adapter.emit_event(session_id, event).await?;
    }

    Ok(())
}
