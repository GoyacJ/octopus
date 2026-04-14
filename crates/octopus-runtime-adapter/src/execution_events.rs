use super::*;

pub(crate) fn usage_cost_shape(total_tokens: Option<u32>) -> (&'static str, i64, &'static str) {
    match total_tokens {
        Some(total_tokens) => ("tokens", i64::from(total_tokens), "tokens"),
        None => ("turns", 1, "count"),
    }
}

fn base_run_event(
    adapter: &RuntimeAdapter,
    session_id: &str,
    project_id: &str,
    conversation_id: &str,
    run: &RuntimeRunSnapshot,
    now: u64,
    event_type: &str,
) -> RuntimeEventEnvelope {
    RuntimeEventEnvelope {
        id: format!("evt-{}", Uuid::new_v4()),
        event_type: event_type.into(),
        kind: Some(event_type.into()),
        workspace_id: adapter.state.workspace_id.clone(),
        project_id: optional_project_id(project_id),
        session_id: session_id.into(),
        conversation_id: conversation_id.into(),
        run_id: Some(run.id.clone()),
        parent_run_id: run.parent_run_id.clone(),
        emitted_at: now,
        sequence: 0,
        capability_plan_summary: Some(run.capability_plan_summary.clone()),
        provider_state_summary: Some(run.provider_state_summary.clone()),
        pending_mediation: run.pending_mediation.clone(),
        capability_state_ref: run.capability_state_ref.clone(),
        last_execution_outcome: run.last_execution_outcome.clone(),
        last_mediation_outcome: run.last_mediation_outcome.clone(),
        ..Default::default()
    }
}

fn model_streamed(message: &RuntimeMessage) -> bool {
    !message.content.trim().is_empty()
}

fn capability_dispatch_kind_label(dispatch_kind: tools::CapabilityDispatchKind) -> &'static str {
    match dispatch_kind {
        tools::CapabilityDispatchKind::BuiltinOrPlugin => "builtin_or_plugin",
        tools::CapabilityDispatchKind::RuntimeCapability => "runtime_capability",
    }
}

fn capability_concurrency_policy_label(
    policy: tools::CapabilityConcurrencyPolicy,
) -> &'static str {
    match policy {
        tools::CapabilityConcurrencyPolicy::ParallelRead => "parallel_read",
        tools::CapabilityConcurrencyPolicy::Serialized => "serialized",
    }
}

fn capability_family(
    record: &agent_runtime_core::RuntimeLoopCapabilityEvent,
) -> &'static str {
    let Some(capability) = record.capability.as_ref() else {
        return "tool";
    };
    match capability.source_kind {
        tools::CapabilitySourceKind::McpTool
        | tools::CapabilitySourceKind::McpPrompt
        | tools::CapabilitySourceKind::McpResource => "mcp",
        tools::CapabilitySourceKind::LocalSkill
        | tools::CapabilitySourceKind::BundledSkill
        | tools::CapabilitySourceKind::PluginSkill => "skill",
        _ if capability.execution_kind == tools::CapabilityExecutionKind::PromptSkill => "skill",
        _ => "tool",
    }
}

fn capability_phase_outcome(phase: tools::CapabilityExecutionPhase) -> &'static str {
    match phase {
        tools::CapabilityExecutionPhase::Started => "started",
        tools::CapabilityExecutionPhase::Completed => "completed",
        tools::CapabilityExecutionPhase::Failed => "failed",
        tools::CapabilityExecutionPhase::BlockedApproval => "blocked_approval",
        tools::CapabilityExecutionPhase::BlockedAuth => "blocked_auth",
        tools::CapabilityExecutionPhase::Denied => "denied",
        tools::CapabilityExecutionPhase::Cancelled => "cancelled",
        tools::CapabilityExecutionPhase::Interrupted => "interrupted",
        tools::CapabilityExecutionPhase::Degraded => "degraded",
    }
}

fn subrun_summaries_for_run(
    adapter: &RuntimeAdapter,
    session_id: &str,
    run_id: &str,
) -> Result<Vec<RuntimeSubrunSummary>, AppError> {
    let sessions = adapter
        .state
        .sessions
        .lock()
        .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
    Ok(sessions
        .get(session_id)
        .map(|aggregate| {
            aggregate
                .detail
                .subruns
                .iter()
                .filter(|subrun| subrun.parent_run_id.as_deref() == Some(run_id))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn append_runtime_loop_events(
    events: &mut Vec<RuntimeEventEnvelope>,
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: &str,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    assistant_message: Option<&RuntimeMessage>,
    execution_trace: Option<&RuntimeTraceItem>,
    model_iterations: &[agent_runtime_core::RuntimeLoopModelIteration],
    capability_events: &[agent_runtime_core::RuntimeLoopCapabilityEvent],
    subruns: &[RuntimeSubrunSummary],
) {
    let fallback_iterations = if model_iterations.is_empty()
        && (assistant_message.is_some() || execution_trace.is_some())
    {
        vec![agent_runtime_core::RuntimeLoopModelIteration {
            iteration: run.checkpoint.current_iteration_index.max(1),
            streamed: assistant_message.is_some_and(model_streamed),
        }]
    } else {
        Vec::new()
    };
    let iterations = if model_iterations.is_empty() {
        fallback_iterations.as_slice()
    } else {
        model_iterations
    };

    for iteration in iterations {
        let mut started = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            "model.started",
        );
        started.iteration = Some(iteration.iteration);
        started.outcome = Some("started".into());
        started.run = Some(run.clone());
        started.payload = Some(json!({
            "iteration": iteration.iteration,
            "run": run.clone(),
        }));
        events.push(started);

        if iteration.streamed {
            let mut streaming = base_run_event(
                adapter,
                session_id,
                project_id,
                conversation_id,
                run,
                now,
                "model.streaming",
            );
            streaming.iteration = Some(iteration.iteration);
            streaming.outcome = Some("streaming".into());
            streaming.payload = Some(json!({
                "iteration": iteration.iteration,
                "run": run.clone(),
            }));
            if let Some(message) = assistant_message.cloned() {
                streaming.message = Some(message);
            }
            events.push(streaming);
        }

        let mut completed = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            "model.completed",
        );
        completed.iteration = Some(iteration.iteration);
        completed.outcome = Some("completed".into());
        completed.payload = Some(json!({
            "iteration": iteration.iteration,
            "run": run.clone(),
        }));
        if Some(iteration.iteration) == iterations.last().map(|item| item.iteration)
            && execution_trace.is_some()
        {
            completed.trace = execution_trace.cloned();
            completed.payload = execution_trace
                .cloned()
                .map(|trace| json!({ "trace": trace }))
                .or_else(|| completed.payload.clone());
        }
        events.push(completed);
    }

    if let Some(trace) = execution_trace.cloned() {
        let mut trace_event = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            "trace.emitted",
        );
        trace_event.trace = Some(trace.clone());
        trace_event.payload = Some(json!({ "trace": trace }));
        events.push(trace_event);
    }

    let mut requested_tool_uses = std::collections::BTreeSet::new();
    for record in capability_events {
        let family = capability_family(record);
        let target_ref = format!("capability-call:{}:{}", run.id, record.tool_use_id);
        let capability_id = record.execution.capability_id.clone();
        let tool_name = record.execution.tool_name.clone();
        let source_kind = record
            .capability
            .as_ref()
            .map(|capability| capability.source_kind.as_str().to_string());
        let execution_kind = record
            .capability
            .as_ref()
            .map(|capability| capability.execution_kind.as_str().to_string());
        let provider_key = record
            .capability
            .as_ref()
            .and_then(|capability| capability.provider_key.clone());
        let detail = record.execution.detail.clone();
        if requested_tool_uses.insert(record.tool_use_id.clone()) {
            let mut requested = base_run_event(
                adapter,
                session_id,
                project_id,
                conversation_id,
                run,
                now,
                &format!("{family}.requested"),
            );
            requested.iteration = Some(record.iteration);
            requested.actor_ref = Some(run.actor_ref.clone());
            requested.tool_use_id = Some(record.tool_use_id.clone());
            requested.target_kind = Some("capability-call".into());
            requested.target_ref = Some(target_ref.clone());
            requested.outcome = Some("requested".into());
            requested.payload = Some(json!({
                "capabilityId": capability_id.clone(),
                "toolName": tool_name.clone(),
                "sourceKind": source_kind.clone(),
                "executionKind": execution_kind.clone(),
                "providerKey": provider_key.clone(),
                "dispatchKind": capability_dispatch_kind_label(record.execution.dispatch_kind),
                "requiredPermission": record.execution.required_permission.as_str(),
                "concurrencyPolicy": capability_concurrency_policy_label(record.execution.concurrency_policy),
                "requiresApproval": record.execution.requires_approval,
                "requiresAuth": record.execution.requires_auth,
            }));
            events.push(requested);
        }

        let event_type = match record.execution.phase {
            tools::CapabilityExecutionPhase::Started => format!("{family}.started"),
            tools::CapabilityExecutionPhase::Completed => format!("{family}.completed"),
            _ => format!("{family}.failed"),
        };
        let mut capability_event = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            &event_type,
        );
        capability_event.iteration = Some(record.iteration);
        capability_event.actor_ref = Some(run.actor_ref.clone());
        capability_event.tool_use_id = Some(record.tool_use_id.clone());
        capability_event.target_kind = Some("capability-call".into());
        capability_event.target_ref = Some(target_ref);
        capability_event.outcome = Some(capability_phase_outcome(record.execution.phase).into());
        if matches!(
            record.execution.phase,
            tools::CapabilityExecutionPhase::BlockedApproval
                | tools::CapabilityExecutionPhase::BlockedAuth
        ) {
            capability_event.approval_layer = Some("capability-call".into());
        }
        capability_event.payload = Some(json!({
            "capabilityId": capability_id,
            "toolName": tool_name,
            "sourceKind": source_kind,
            "executionKind": execution_kind,
            "providerKey": provider_key,
            "dispatchKind": capability_dispatch_kind_label(record.execution.dispatch_kind),
            "requiredPermission": record.execution.required_permission.as_str(),
            "concurrencyPolicy": capability_concurrency_policy_label(record.execution.concurrency_policy),
            "requiresApproval": record.execution.requires_approval,
            "requiresAuth": record.execution.requires_auth,
            "detail": detail,
        }));
        events.push(capability_event);
    }

    for subrun in subruns {
        let mut spawned = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            "subrun.spawned",
        );
        spawned.iteration = Some(run.checkpoint.current_iteration_index);
        spawned.parent_run_id = subrun.parent_run_id.clone().or_else(|| Some(run.id.clone()));
        spawned.workflow_run_id = subrun.workflow_run_id.clone();
        spawned.actor_ref = Some(subrun.actor_ref.clone());
        spawned.tool_use_id = subrun.delegated_by_tool_call_id.clone();
        spawned.outcome = Some("spawned".into());
        spawned.payload = Some(json!({ "subrun": subrun }));
        events.push(spawned);

        let terminal_kind = match subrun.status.as_str() {
            "completed" => Some("subrun.completed"),
            "failed" => Some("subrun.failed"),
            _ => None,
        };
        if let Some(terminal_kind) = terminal_kind {
            let mut terminal = base_run_event(
                adapter,
                session_id,
                project_id,
                conversation_id,
                run,
                now,
                terminal_kind,
            );
            terminal.iteration = Some(run.checkpoint.current_iteration_index);
            terminal.parent_run_id = subrun.parent_run_id.clone().or_else(|| Some(run.id.clone()));
            terminal.workflow_run_id = subrun.workflow_run_id.clone();
            terminal.actor_ref = Some(subrun.actor_ref.clone());
            terminal.tool_use_id = subrun.delegated_by_tool_call_id.clone();
            terminal.outcome = Some(subrun.status.clone());
            terminal.payload = Some(json!({ "subrun": subrun }));
            events.push(terminal);
        }
    }
}

fn append_runtime_loop_planner_events(
    events: &mut Vec<RuntimeEventEnvelope>,
    adapter: &RuntimeAdapter,
    session_id: &str,
    now: u64,
    conversation_id: &str,
    project_id: &str,
    run: &RuntimeRunSnapshot,
    planner_events: &[agent_runtime_core::RuntimeLoopPlannerEvent],
) {
    for record in planner_events {
        let event_type = match record.phase {
            agent_runtime_core::RuntimeLoopPlannerPhase::Started => "planner.started",
            agent_runtime_core::RuntimeLoopPlannerPhase::Completed => "planner.completed",
        };
        let mut event = base_run_event(
            adapter,
            session_id,
            project_id,
            conversation_id,
            run,
            now,
            event_type,
        );
        event.iteration = Some(record.iteration);
        event.outcome = Some(match record.phase {
            agent_runtime_core::RuntimeLoopPlannerPhase::Started => "started".into(),
            agent_runtime_core::RuntimeLoopPlannerPhase::Completed => "completed".into(),
        });
        if let Some(summary) = record.capability_plan_summary.clone() {
            event.capability_plan_summary = Some(summary);
        }
        if let Some(provider_state_summary) = record.provider_state_summary.clone() {
            event.provider_state_summary = Some(provider_state_summary);
        }
        if let Some(capability_state_ref) = record.capability_state_ref.clone() {
            event.capability_state_ref = Some(capability_state_ref);
        }
        event.payload = Some(match record.phase {
            agent_runtime_core::RuntimeLoopPlannerPhase::Started => json!({
                "iteration": record.iteration,
            }),
            agent_runtime_core::RuntimeLoopPlannerPhase::Completed => json!({
                "iteration": record.iteration,
                "capabilityPlanSummary": record.capability_plan_summary.clone(),
                "providerStateSummary": record.provider_state_summary.clone(),
                "capabilityStateRef": record.capability_state_ref.clone(),
            }),
        });
        events.push(event);
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
    planner_events: &[agent_runtime_core::RuntimeLoopPlannerEvent],
    model_iterations: &[agent_runtime_core::RuntimeLoopModelIteration],
    capability_events: &[agent_runtime_core::RuntimeLoopCapabilityEvent],
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

    append_runtime_loop_planner_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        planner_events,
    );

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

    let subruns = subrun_summaries_for_run(adapter, session_id, &run.id)?;
    append_runtime_loop_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        assistant_message.as_ref(),
        execution_trace.as_ref(),
        model_iterations,
        capability_events,
        &subruns,
    );

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
    planner_events: &[agent_runtime_core::RuntimeLoopPlannerEvent],
    model_iterations: &[agent_runtime_core::RuntimeLoopModelIteration],
    capability_events: &[agent_runtime_core::RuntimeLoopCapabilityEvent],
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

    append_runtime_loop_planner_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        planner_events,
    );
    let subruns = subrun_summaries_for_run(adapter, session_id, &run.id)?;
    append_runtime_loop_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        assistant_message.as_ref(),
        execution_trace.as_ref(),
        model_iterations,
        capability_events,
        &subruns,
    );

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
    planner_events: &[agent_runtime_core::RuntimeLoopPlannerEvent],
    model_iterations: &[agent_runtime_core::RuntimeLoopModelIteration],
    capability_events: &[agent_runtime_core::RuntimeLoopCapabilityEvent],
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

    append_runtime_loop_planner_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        planner_events,
    );
    let subruns = subrun_summaries_for_run(adapter, session_id, &run.id)?;
    append_runtime_loop_events(
        &mut events,
        adapter,
        session_id,
        now,
        &conversation_id,
        &project_id,
        &run,
        assistant_message.as_ref(),
        execution_trace.as_ref(),
        model_iterations,
        capability_events,
        &subruns,
    );

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
