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
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        },
        RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "planner.completed".into(),
            kind: Some("planner.completed".into()),
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
            trace: Some(submitted_trace.clone()),
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        },
        RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "trace.emitted".into(),
            kind: Some("trace.emitted".into()),
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
            trace: Some(submitted_trace.clone()),
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        },
    ];

    if !run.capability_plan_summary.hidden_capabilities.is_empty() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "policy.exposure_denied".into(),
            kind: Some("policy.exposure_denied".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            outcome: Some("deny".into()),
            payload: Some(json!({
                "hiddenCapabilities": run.capability_plan_summary.hidden_capabilities.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }

    if !run.capability_plan_summary.deferred_tools.is_empty() || run.pending_mediation.is_some() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "policy.surface_deferred".into(),
            kind: Some("policy.surface_deferred".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            outcome: Some(
                run.pending_mediation
                    .as_ref()
                    .map(|mediation| mediation.state.clone())
                    .unwrap_or_else(|| "deferred".into()),
            ),
            payload: Some(json!({
                "deferredTools": run.capability_plan_summary.deferred_tools.clone(),
                "pendingMediation": run.pending_mediation.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }

    if let Some(message) = assistant_message {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "model.started".into(),
            kind: Some("model.started".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
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
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
    }

    if let Some(trace) = execution_trace {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "model.completed".into(),
            kind: Some("model.completed".into()),
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
            trace: Some(trace.clone()),
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "trace.emitted".into(),
            kind: Some("trace.emitted".into()),
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
            trace: Some(trace.clone()),
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
    }

    if let Some(approval) = approval {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "approval.requested".into(),
            kind: Some("approval.requested".into()),
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
            approval: Some(approval.clone()),
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
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
            approval: Some(approval.clone()),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }

    if let Some(challenge) = run.auth_target.clone() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "auth.challenge_requested".into(),
            kind: Some("auth.challenge_requested".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            outcome: Some(challenge.status.clone()),
            payload: Some(json!({
                "authChallenge": challenge.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            auth_challenge: Some(challenge),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }

    if !run.selected_memory.is_empty() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "memory.selected".into(),
            kind: Some("memory.selected".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "selectedMemory": run.selected_memory.clone(),
                "memorySelectionSummary": {
                    "selectedCount": run.selected_memory.len(),
                },
                "freshnessSummary": run.freshness_summary.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            memory_proposal: None,
            memory_selection_summary: Some(RuntimeMemorySelectionSummary {
                total_candidate_count: run.selected_memory.len() as u64,
                selected_count: run.selected_memory.len() as u64,
                ignored_count: 0,
                recall_mode: "default".into(),
                selected_memory_ids: run
                    .selected_memory
                    .iter()
                    .map(|item| item.memory_id.clone())
                    .collect(),
            }),
            freshness_summary: run.freshness_summary.clone(),
            selected_memory: Some(run.selected_memory.clone()),
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
    }

    if let Some(memory_proposal) = run.pending_memory_proposal.clone() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "memory.proposed".into(),
            kind: Some("memory.proposed".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "memoryProposal": memory_proposal.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            memory_proposal: Some(memory_proposal),
            memory_selection_summary: None,
            freshness_summary: run.freshness_summary.clone(),
            selected_memory: Some(run.selected_memory.clone()),
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        });
    }

    if let (Some(workflow_run_id), Some(workflow_detail)) =
        (run.workflow_run.clone(), run.workflow_run_detail.clone())
    {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "workflow.started".into(),
            kind: Some("workflow.started".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some(run.status.clone()),
            payload: Some(json!({ "workflow": workflow_detail.clone() })),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "workflow.step.started".into(),
            kind: Some("workflow.step.started".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some("started".into()),
            payload: Some(json!({
                "workflowRunId": workflow_run_id,
                "stepId": workflow_detail.current_step_id,
                "stepLabel": workflow_detail.current_step_label,
            })),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "workflow.step.completed".into(),
            kind: Some("workflow.step.completed".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some(run.status.clone()),
            payload: Some(json!({ "workflow": workflow_detail.clone() })),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: if run.status == "failed" {
                "workflow.failed".into()
            } else {
                "workflow.completed".into()
            },
            kind: Some(if run.status == "failed" {
                "workflow.failed".into()
            } else {
                "workflow.completed".into()
            }),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_detail.workflow_run_id.clone()),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some(run.status.clone()),
            payload: Some(json!({ "workflow": workflow_detail })),
            ..Default::default()
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
        run: Some(run.clone()),
        message: None,
        trace: None,
        approval: None,
        decision: None,
        summary: None,
        error: None,
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        ..Default::default()
    });

    for event in events {
        adapter.emit_event(session_id, event).await?;
    }

    Ok(())
}

pub(super) async fn record_memory_proposal_resolution_activity(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    proposal: &RuntimeMemoryProposal,
    decision: &str,
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
            event_kind: "memory_proposal_resolved".into(),
            title: "Runtime memory proposal resolved".into(),
            detail: format!("{} -> {}", proposal.proposal_id, decision),
            created_at: now,
        })
        .await?;
    adapter
        .state
        .observation
        .append_audit(AuditRecord {
            id: format!("audit-{}", Uuid::new_v4()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: Some(project_id.to_string()),
            actor_type: "session".into(),
            actor_id: session_id.into(),
            action: "runtime.resolve_memory_proposal".into(),
            resource: proposal.proposal_id.clone(),
            outcome: decision.to_string(),
            created_at: now,
        })
        .await?;
    Ok(())
}

pub(super) async fn emit_memory_proposal_resolution_events(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: String,
    project_id: String,
    run: RuntimeRunSnapshot,
    proposal: RuntimeMemoryProposal,
    decision: String,
) -> Result<(), AppError> {
    let event_type = match decision.as_str() {
        "approve" => "memory.approved",
        "revalidate" => "memory.revalidated",
        _ => "memory.rejected",
    };
    let events = vec![
        RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: event_type.into(),
            kind: Some(event_type.into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "memoryProposal": proposal.clone(),
                "decision": decision.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            memory_proposal: Some(proposal.clone()),
            memory_selection_summary: None,
            freshness_summary: run.freshness_summary.clone(),
            selected_memory: Some(run.selected_memory.clone()),
            trace: None,
            approval: None,
            decision: Some(decision.clone()),
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            ..Default::default()
        },
        RuntimeEventEnvelope {
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
                "memoryProposal": proposal,
                "decision": decision,
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            memory_proposal: None,
            memory_selection_summary: None,
            freshness_summary: None,
            selected_memory: None,
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: None,
            provider_state_summary: None,
            pending_mediation: None,
            capability_state_ref: None,
            last_execution_outcome: None,
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        },
    ];

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

pub(super) async fn record_auth_challenge_resolution_activity(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    challenge: &RuntimeAuthChallengeSummary,
    resolution: &str,
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
            event_kind: "auth_challenge_resolved".into(),
            title: "Auth challenge resolved".into(),
            detail: resolution.to_string(),
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
            action: "runtime.resolve_auth_challenge".into(),
            resource: challenge.id.clone(),
            outcome: resolution.to_string(),
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
        event_type: "approval.resolved".into(),
        kind: Some("approval.resolved".into()),
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
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        last_mediation_outcome: run.last_mediation_outcome.clone(),
        ..Default::default()
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
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }
    if let Some(trace) = execution_trace {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "model.completed".into(),
            kind: Some("model.completed".into()),
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
            trace: Some(trace.clone()),
            approval: None,
            decision: None,
            summary: None,
            error: None,
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "trace.emitted".into(),
            kind: Some("trace.emitted".into()),
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
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }
    events.push(RuntimeEventEnvelope {
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
        approval: Some(approval.clone()),
        decision: Some(decision.clone()),
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        last_mediation_outcome: run.last_mediation_outcome.clone(),
        ..Default::default()
    });
    if let Some(challenge) = run.auth_target.clone() {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "auth.challenge_requested".into(),
            kind: Some("auth.challenge_requested".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            outcome: Some(challenge.status.clone()),
            payload: Some(json!({
                "approval": approval.clone(),
                "authChallenge": challenge.clone(),
                "decision": decision.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            approval: Some(approval.clone()),
            auth_challenge: Some(challenge),
            decision: Some(decision.clone()),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }
    if let (Some(workflow_run_id), Some(workflow_detail)) =
        (run.workflow_run.clone(), run.workflow_run_detail.clone())
    {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "workflow.step.completed".into(),
            kind: Some("workflow.step.completed".into()),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_run_id.clone()),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some(run.status.clone()),
            payload: Some(json!({ "workflow": workflow_detail.clone() })),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: if run.status == "failed" {
                "workflow.failed".into()
            } else {
                "workflow.completed".into()
            },
            kind: Some(if run.status == "failed" {
                "workflow.failed".into()
            } else {
                "workflow.completed".into()
            }),
            workspace_id: adapter.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            emitted_at: now,
            sequence: 0,
            iteration: Some(run.checkpoint.current_iteration_index),
            workflow_run_id: Some(workflow_run_id),
            workflow_step_id: workflow_detail.current_step_id.clone(),
            actor_ref: Some(run.actor_ref.clone()),
            tool_use_id: run.delegated_by_tool_call_id.clone(),
            outcome: Some(run.status.clone()),
            payload: Some(json!({ "workflow": workflow_detail })),
            ..Default::default()
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
        run: Some(run.clone()),
        message: None,
        trace: None,
        approval: Some(approval),
        decision: Some(decision),
        summary: None,
        error: None,
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        ..Default::default()
    });

    for event in events {
        adapter.emit_event(session_id, event).await?;
    }

    Ok(())
}

pub(super) async fn emit_auth_resolution_events(
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: String,
    project_id: String,
    run: RuntimeRunSnapshot,
    challenge: RuntimeAuthChallengeSummary,
    resolution: String,
    assistant_message: Option<RuntimeMessage>,
    execution_trace: Option<RuntimeTraceItem>,
) -> Result<(), AppError> {
    let auth_event_type = if resolution == "resolved" {
        "auth.resolved"
    } else {
        "auth.failed"
    };
    let mut events = vec![RuntimeEventEnvelope {
        id: format!("evt-{}", Uuid::new_v4()),
        event_type: auth_event_type.into(),
        kind: Some(auth_event_type.into()),
        workspace_id: adapter.state.workspace_id.clone(),
        project_id: optional_project_id(&project_id),
        session_id: session_id.into(),
        conversation_id: conversation_id.clone(),
        run_id: Some(run.id.clone()),
        emitted_at: now,
        sequence: 0,
        outcome: Some(resolution.clone()),
        payload: Some(json!({
            "authChallenge": challenge.clone(),
            "resolution": resolution.clone(),
            "run": run.clone(),
        })),
        run: Some(run.clone()),
        auth_challenge: Some(challenge.clone()),
        decision: Some(resolution.clone()),
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        last_mediation_outcome: run.last_mediation_outcome.clone(),
        ..Default::default()
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
            message: Some(message),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
    }
    if let Some(trace) = execution_trace {
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "model.completed".into(),
            kind: Some("model.completed".into()),
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
            trace: Some(trace.clone()),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
        });
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "trace.emitted".into(),
            kind: Some("trace.emitted".into()),
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
            trace: Some(trace),
            capability_plan_summary: Some(run.capability_plan_summary.clone()),
            provider_state_summary: Some(run.provider_state_summary.clone()),
            pending_mediation: run.pending_mediation.clone(),
            capability_state_ref: run.capability_state_ref.clone(),
            last_execution_outcome: run.last_execution_outcome.clone(),
            last_mediation_outcome: run.last_mediation_outcome.clone(),
            ..Default::default()
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
        run: Some(run.clone()),
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        last_mediation_outcome: run.last_mediation_outcome.clone(),
        ..Default::default()
    });

    for event in events {
        adapter.emit_event(session_id, event).await?;
    }

    Ok(())
}
