use super::*;

pub(super) fn trim_optional_task_input(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn normalize_task_context_bundle_input(mut bundle: TaskContextBundle) -> TaskContextBundle {
    bundle.refs = bundle
        .refs
        .into_iter()
        .map(|mut reference| {
            reference.kind = reference.kind.trim().to_string();
            reference.ref_id = reference.ref_id.trim().to_string();
            reference.title = reference.title.trim().to_string();
            reference.subtitle = reference.subtitle.trim().to_string();
            reference.version_ref = trim_optional_task_input(reference.version_ref);
            reference.pin_mode = reference.pin_mode.trim().to_string();
            reference
        })
        .filter(|reference| {
            !reference.kind.is_empty()
                && !reference.ref_id.is_empty()
                && !reference.title.is_empty()
        })
        .collect();
    bundle.pinned_instructions = bundle.pinned_instructions.trim().to_string();
    let resolution_mode = bundle.resolution_mode.trim();
    bundle.resolution_mode = if resolution_mode.is_empty() {
        "explicit_only".into()
    } else {
        resolution_mode.to_string()
    };
    bundle
}

pub(crate) fn validate_create_task_request(
    request: CreateTaskRequest,
) -> Result<CreateTaskRequest, ApiError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(AppError::invalid_input("task title is required").into());
    }
    let goal = request.goal.trim();
    if goal.is_empty() {
        return Err(AppError::invalid_input("task goal is required").into());
    }
    let brief = request.brief.trim();
    if brief.is_empty() {
        return Err(AppError::invalid_input("task brief is required").into());
    }
    let default_actor_ref = request.default_actor_ref.trim();
    if default_actor_ref.is_empty() {
        return Err(AppError::invalid_input("default actor is required").into());
    }

    Ok(CreateTaskRequest {
        title: title.into(),
        goal: goal.into(),
        brief: brief.into(),
        default_actor_ref: default_actor_ref.into(),
        schedule_spec: trim_optional_task_input(request.schedule_spec),
        context_bundle: normalize_task_context_bundle_input(request.context_bundle),
    })
}

pub(crate) fn validate_update_task_request(
    request: UpdateTaskRequest,
) -> Result<UpdateTaskRequest, ApiError> {
    if let Some(title) = request.title.as_deref() {
        if title.trim().is_empty() {
            return Err(AppError::invalid_input("task title must not be empty").into());
        }
    }
    if let Some(goal) = request.goal.as_deref() {
        if goal.trim().is_empty() {
            return Err(AppError::invalid_input("task goal must not be empty").into());
        }
    }
    if let Some(brief) = request.brief.as_deref() {
        if brief.trim().is_empty() {
            return Err(AppError::invalid_input("task brief must not be empty").into());
        }
    }
    if let Some(default_actor_ref) = request.default_actor_ref.as_deref() {
        if default_actor_ref.trim().is_empty() {
            return Err(AppError::invalid_input("default actor must not be empty").into());
        }
    }

    Ok(UpdateTaskRequest {
        title: trim_optional_task_input(request.title),
        goal: trim_optional_task_input(request.goal),
        brief: trim_optional_task_input(request.brief),
        default_actor_ref: trim_optional_task_input(request.default_actor_ref),
        schedule_spec: request.schedule_spec.map(|value| value.trim().to_string()),
        context_bundle: request
            .context_bundle
            .map(normalize_task_context_bundle_input),
    })
}

pub(super) fn task_summary_from_record(record: &ProjectTaskRecord) -> TaskSummary {
    TaskSummary {
        id: record.id.clone(),
        project_id: record.project_id.clone(),
        title: record.title.clone(),
        goal: record.goal.clone(),
        default_actor_ref: record.default_actor_ref.clone(),
        status: record.status.clone(),
        schedule_spec: record.schedule_spec.clone(),
        next_run_at: record.next_run_at,
        last_run_at: record.last_run_at,
        latest_result_summary: record.latest_result_summary.clone(),
        latest_failure_category: record.latest_failure_category.clone(),
        latest_transition: record.latest_transition.clone(),
        view_status: record.view_status.clone(),
        attention_reasons: record.attention_reasons.clone(),
        attention_updated_at: record.attention_updated_at,
        active_task_run_id: record.active_task_run_id.clone(),
        analytics_summary: record.analytics_summary.clone(),
        updated_at: record.updated_at,
    }
}

pub(super) fn task_run_summary_from_record(record: &ProjectTaskRunRecord) -> TaskRunSummary {
    TaskRunSummary {
        id: record.id.clone(),
        task_id: record.task_id.clone(),
        trigger_type: record.trigger_type.clone(),
        status: record.status.clone(),
        session_id: record.session_id.clone(),
        conversation_id: record.conversation_id.clone(),
        runtime_run_id: record.runtime_run_id.clone(),
        actor_ref: record.actor_ref.clone(),
        started_at: record.started_at,
        completed_at: record.completed_at,
        result_summary: record.result_summary.clone(),
        pending_approval_id: record.pending_approval_id.clone(),
        failure_category: record.failure_category.clone(),
        failure_summary: record.failure_summary.clone(),
        view_status: record.view_status.clone(),
        attention_reasons: record.attention_reasons.clone(),
        attention_updated_at: record.attention_updated_at,
        deliverable_refs: record.deliverable_refs.clone(),
        artifact_refs: record.artifact_refs.clone(),
        latest_transition: record.latest_transition.clone(),
    }
}

pub(super) fn task_intervention_from_record(record: &ProjectTaskInterventionRecord) -> TaskInterventionRecord {
    TaskInterventionRecord {
        id: record.id.clone(),
        task_id: record.task_id.clone(),
        task_run_id: record.task_run_id.clone(),
        r#type: record.r#type.clone(),
        payload: record.payload.clone(),
        created_by: record.created_by.clone(),
        created_at: record.created_at,
        applied_to_session_id: record.applied_to_session_id.clone(),
        status: record.status.clone(),
    }
}

pub(super) fn task_detail_from_records(
    task: &ProjectTaskRecord,
    runs: &[ProjectTaskRunRecord],
    interventions: &[ProjectTaskInterventionRecord],
) -> TaskDetail {
    let run_history = runs
        .iter()
        .map(task_run_summary_from_record)
        .collect::<Vec<_>>();
    let active_run = task
        .active_task_run_id
        .as_deref()
        .and_then(|run_id| run_history.iter().find(|run| run.id == run_id).cloned())
        .or_else(|| run_history.first().cloned());

    TaskDetail {
        id: task.id.clone(),
        project_id: task.project_id.clone(),
        title: task.title.clone(),
        goal: task.goal.clone(),
        brief: task.brief.clone(),
        default_actor_ref: task.default_actor_ref.clone(),
        status: task.status.clone(),
        schedule_spec: task.schedule_spec.clone(),
        next_run_at: task.next_run_at,
        last_run_at: task.last_run_at,
        latest_result_summary: task.latest_result_summary.clone(),
        latest_failure_category: task.latest_failure_category.clone(),
        latest_transition: task.latest_transition.clone(),
        view_status: task.view_status.clone(),
        attention_reasons: task.attention_reasons.clone(),
        attention_updated_at: task.attention_updated_at,
        active_task_run_id: task.active_task_run_id.clone(),
        analytics_summary: task.analytics_summary.clone(),
        context_bundle: task.context_bundle.clone(),
        latest_deliverable_refs: task.latest_deliverable_refs.clone(),
        latest_artifact_refs: task.latest_artifact_refs.clone(),
        run_history,
        intervention_history: interventions
            .iter()
            .map(task_intervention_from_record)
            .collect(),
        active_run,
        created_by: task.created_by.clone(),
        updated_by: task.updated_by.clone(),
        created_at: task.created_at,
        updated_at: task.updated_at,
    }
}

pub(super) fn task_prompt_from_record(
    task: &ProjectTaskRecord,
    trigger_label: &str,
    source_task_run_id: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("Task title: {}", task.title),
        format!("Trigger: {trigger_label}"),
        String::new(),
        "Goal:".into(),
        task.goal.clone(),
        String::new(),
        "Brief:".into(),
        task.brief.clone(),
    ];

    if !task.context_bundle.pinned_instructions.trim().is_empty() {
        lines.extend([
            String::new(),
            "Pinned instructions:".into(),
            task.context_bundle.pinned_instructions.clone(),
        ]);
    }

    if !task.context_bundle.refs.is_empty() {
        lines.push(String::new());
        lines.push("Context refs:".into());
        lines.extend(task.context_bundle.refs.iter().map(|reference| {
            format!(
                "- [{}] {} ({})",
                reference.kind, reference.title, reference.ref_id
            )
        }));
    }

    if let Some(source_task_run_id) = source_task_run_id {
        lines.extend([String::new(), format!("Source run: {source_task_run_id}")]);
    }

    lines.join("\n")
}

pub(super) fn task_run_status_from_runtime(run: &RuntimeRunSnapshot) -> String {
    match run.status.as_str() {
        "queued" | "running" | "waiting_input" | "waiting_approval" | "completed" | "failed"
        | "canceled" | "skipped" => run.status.clone(),
        "auth-required" | "blocked" => "waiting_input".into(),
        _ => "running".into(),
    }
}

pub(super) fn task_run_pending_approval_id(run: &RuntimeRunSnapshot) -> Option<String> {
    (task_run_status_from_runtime(run) == "waiting_approval")
        .then(|| {
            run.approval_target
                .as_ref()
                .map(|approval| approval.id.clone())
        })
        .flatten()
}

pub(super) fn build_task_run_record(
    task: &ProjectTaskRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
    trigger_type: &str,
    actor_ref: &str,
) -> ProjectTaskRunRecord {
    let status = task_run_status_from_runtime(run);
    let completed_at = matches!(
        status.as_str(),
        "completed" | "failed" | "canceled" | "skipped"
    )
    .then_some(run.updated_at);
    let failure_category = (status == "failed").then_some("runtime_error".into());
    let failure_summary = (status == "failed").then_some("Runtime task execution failed.".into());
    let attention_reasons = match status.as_str() {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let latest_transition = Some(TaskStateTransitionSummary {
        kind: match status.as_str() {
            "completed" => "completed".into(),
            "failed" => "failed".into(),
            "waiting_approval" => "waiting_approval".into(),
            _ => "launched".into(),
        },
        summary: match status.as_str() {
            "completed" => "Task run completed in the runtime.".into(),
            "failed" => "Task run failed in the runtime.".into(),
            "waiting_approval" => "Task run is waiting for approval.".into(),
            "waiting_input" => "Task run is waiting for input.".into(),
            _ => "Task run started in the runtime.".into(),
        },
        at: completed_at.unwrap_or(run.started_at),
        run_id: Some(run.id.clone()),
    });

    ProjectTaskRunRecord {
        id: format!("task-run-{}", uuid::Uuid::new_v4()),
        workspace_id: task.workspace_id.clone(),
        project_id: task.project_id.clone(),
        task_id: task.id.clone(),
        trigger_type: trigger_type.into(),
        status: status.clone(),
        session_id: Some(session.summary.id.clone()),
        conversation_id: Some(session.summary.conversation_id.clone()),
        runtime_run_id: Some(run.id.clone()),
        actor_ref: actor_ref.into(),
        started_at: run.started_at,
        completed_at,
        result_summary: (status == "completed")
            .then_some("Task run completed in the runtime.".into()),
        pending_approval_id: task_run_pending_approval_id(run),
        failure_category,
        failure_summary,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(completed_at.unwrap_or(run.started_at))
        },
        deliverable_refs: run.deliverable_refs.clone(),
        artifact_refs: Vec::new(),
        latest_transition,
    }
}

pub(super) fn sync_task_run_record_from_runtime(
    existing: &ProjectTaskRunRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
) -> ProjectTaskRunRecord {
    let status = task_run_status_from_runtime(run);
    let completed_at = matches!(
        status.as_str(),
        "completed" | "failed" | "canceled" | "skipped"
    )
    .then_some(run.updated_at);
    let failure_category = (status == "failed").then_some("runtime_error".into());
    let failure_summary = (status == "failed").then_some("Runtime task execution failed.".into());
    let attention_reasons = match status.as_str() {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let latest_transition = Some(TaskStateTransitionSummary {
        kind: match status.as_str() {
            "completed" => "completed".into(),
            "failed" => "failed".into(),
            "waiting_approval" => "waiting_approval".into(),
            _ => "launched".into(),
        },
        summary: match status.as_str() {
            "completed" => "Task run completed in the runtime.".into(),
            "failed" => "Task run failed in the runtime.".into(),
            "waiting_approval" => "Task run is waiting for approval.".into(),
            "waiting_input" => "Task run is waiting for input.".into(),
            _ => "Task run started in the runtime.".into(),
        },
        at: completed_at.unwrap_or(run.updated_at),
        run_id: Some(run.id.clone()),
    });

    ProjectTaskRunRecord {
        id: existing.id.clone(),
        workspace_id: existing.workspace_id.clone(),
        project_id: existing.project_id.clone(),
        task_id: existing.task_id.clone(),
        trigger_type: existing.trigger_type.clone(),
        status,
        session_id: Some(session.summary.id.clone()),
        conversation_id: Some(session.summary.conversation_id.clone()),
        runtime_run_id: Some(run.id.clone()),
        actor_ref: if run.actor_ref.trim().is_empty() {
            existing.actor_ref.clone()
        } else {
            run.actor_ref.clone()
        },
        started_at: run.started_at,
        completed_at,
        result_summary: (run.status == "completed")
            .then_some("Task run completed in the runtime.".into()),
        pending_approval_id: task_run_pending_approval_id(run),
        failure_category,
        failure_summary,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(completed_at.unwrap_or(run.updated_at))
        },
        deliverable_refs: run.deliverable_refs.clone(),
        artifact_refs: existing.artifact_refs.clone(),
        latest_transition,
    }
}

pub(super) fn sync_rejected_task_run_record_from_runtime(
    existing: &ProjectTaskRunRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
) -> ProjectTaskRunRecord {
    let mut synced = sync_task_run_record_from_runtime(existing, session, run);
    if synced.status == "waiting_input" {
        synced.result_summary = Some("Approval rejected. Waiting for updated guidance.".into());
    }
    synced
}

pub(super) fn task_run_duration_ms(run: &ProjectTaskRunRecord) -> u64 {
    run.completed_at
        .unwrap_or(run.started_at)
        .saturating_sub(run.started_at)
}

pub(super) fn update_task_analytics_from_run(
    analytics: &TaskAnalyticsSummary,
    run: &ProjectTaskRunRecord,
) -> TaskAnalyticsSummary {
    let mut updated = analytics.clone();
    updated.run_count = updated.run_count.saturating_add(1);
    match run.trigger_type.as_str() {
        "manual" => updated.manual_run_count = updated.manual_run_count.saturating_add(1),
        "scheduled" => updated.scheduled_run_count = updated.scheduled_run_count.saturating_add(1),
        "takeover" => updated.takeover_count = updated.takeover_count.saturating_add(1),
        _ => {}
    }
    if run.status == "completed" {
        updated.completion_count = updated.completion_count.saturating_add(1);
        updated.last_successful_run_at = run.completed_at.or(Some(run.started_at));
    }
    if run.status == "failed" {
        updated.failure_count = updated.failure_count.saturating_add(1);
    }
    if run.status == "waiting_approval" {
        updated.approval_required_count = updated.approval_required_count.saturating_add(1);
    }
    let duration_ms = run
        .completed_at
        .unwrap_or(run.started_at)
        .saturating_sub(run.started_at);
    if updated.run_count == 0 {
        updated.average_run_duration_ms = duration_ms;
    } else {
        let previous_total = analytics
            .average_run_duration_ms
            .saturating_mul(analytics.run_count);
        updated.average_run_duration_ms = previous_total
            .saturating_add(duration_ms)
            .saturating_div(updated.run_count.max(1));
    }
    updated
}

pub(super) fn sync_task_analytics_from_run(
    analytics: &TaskAnalyticsSummary,
    previous_run: &ProjectTaskRunRecord,
    run: &ProjectTaskRunRecord,
) -> TaskAnalyticsSummary {
    let mut updated = analytics.clone();
    if previous_run.status != "completed" && run.status == "completed" {
        updated.completion_count = updated.completion_count.saturating_add(1);
        updated.last_successful_run_at = run.completed_at.or(Some(run.started_at));
    }
    if previous_run.status != "failed" && run.status == "failed" {
        updated.failure_count = updated.failure_count.saturating_add(1);
    }
    if previous_run.status != "waiting_approval" && run.status == "waiting_approval" {
        updated.approval_required_count = updated.approval_required_count.saturating_add(1);
    }
    let run_count = updated.run_count.max(1);
    let previous_total = analytics.average_run_duration_ms.saturating_mul(run_count);
    updated.average_run_duration_ms = previous_total
        .saturating_sub(task_run_duration_ms(previous_run))
        .saturating_add(task_run_duration_ms(run))
        .saturating_div(run_count);
    updated
}

pub(super) fn update_task_record_from_run(
    task: &ProjectTaskRecord,
    run: &ProjectTaskRunRecord,
    updated_by: &str,
) -> ProjectTaskRecord {
    let attention_reasons = run.attention_reasons.clone();
    let updated_at = run.completed_at.unwrap_or(run.started_at);
    ProjectTaskRecord {
        status: match run.status.as_str() {
            "completed" => "completed".into(),
            "failed" | "waiting_approval" | "waiting_input" => "attention".into(),
            _ => "running".into(),
        },
        last_run_at: Some(run.started_at),
        active_task_run_id: Some(run.id.clone()),
        latest_result_summary: run.result_summary.clone(),
        latest_failure_category: run.failure_category.clone(),
        latest_transition: run.latest_transition.clone(),
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(updated_at)
        },
        analytics_summary: update_task_analytics_from_run(&task.analytics_summary, run),
        latest_deliverable_refs: run.deliverable_refs.clone(),
        latest_artifact_refs: run.artifact_refs.clone(),
        updated_by: Some(updated_by.into()),
        updated_at,
        ..task.clone()
    }
}

pub(super) fn sync_task_record_from_run(
    task: &ProjectTaskRecord,
    previous_run: &ProjectTaskRunRecord,
    run: &ProjectTaskRunRecord,
    updated_by: &str,
) -> ProjectTaskRecord {
    let attention_reasons = run.attention_reasons.clone();
    let updated_at = run.completed_at.unwrap_or(run.started_at);
    ProjectTaskRecord {
        status: match run.status.as_str() {
            "completed" => "completed".into(),
            "failed" | "waiting_approval" | "waiting_input" => "attention".into(),
            _ => "running".into(),
        },
        last_run_at: Some(run.started_at),
        active_task_run_id: Some(run.id.clone()),
        latest_result_summary: run.result_summary.clone(),
        latest_failure_category: run.failure_category.clone(),
        latest_transition: run.latest_transition.clone(),
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(updated_at)
        },
        analytics_summary: sync_task_analytics_from_run(&task.analytics_summary, previous_run, run),
        latest_deliverable_refs: run.deliverable_refs.clone(),
        latest_artifact_refs: run.artifact_refs.clone(),
        updated_by: Some(updated_by.into()),
        updated_at,
        ..task.clone()
    }
}

