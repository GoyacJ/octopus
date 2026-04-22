use super::task_helpers::{
    build_task_run_record, sync_rejected_task_run_record_from_runtime, sync_task_record_from_run,
    sync_task_run_record_from_runtime, task_detail_from_records, task_intervention_from_record,
    task_prompt_from_record, task_run_summary_from_record, task_summary_from_record,
    trim_optional_task_input, update_task_record_from_run, validate_create_task_request,
    validate_update_task_request,
};
use super::*;

pub(crate) async fn list_project_tasks(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<TaskSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .project_tasks
            .list_tasks(&project_id)
            .await?
            .iter()
            .map(task_summary_from_record)
            .collect(),
    ))
}

pub(crate) async fn create_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<TaskDetail>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.manage",
        Some(&project_id),
        Some("task"),
        None,
    )
    .await?;
    let request = validate_create_task_request(request)?;
    let task = state
        .services
        .project_tasks
        .create_task(&project_id, &session.user_id, request)
        .await?;
    Ok(Json(task_detail_from_records(&task, &[], &[])))
}

pub(crate) async fn get_project_task_detail(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskDetail>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let runs = state
        .services
        .project_tasks
        .list_task_runs(&project_id, &task_id)
        .await?;
    let interventions = state
        .services
        .project_tasks
        .list_task_interventions(&project_id, &task_id)
        .await?;
    Ok(Json(task_detail_from_records(&task, &runs, &interventions)))
}

pub(crate) async fn update_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<UpdateTaskRequest>,
) -> Result<Json<TaskDetail>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.manage",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let request = validate_update_task_request(request)?;
    let task = state
        .services
        .project_tasks
        .update_task(&project_id, &task_id, &session.user_id, request)
        .await?;
    let runs = state
        .services
        .project_tasks
        .list_task_runs(&project_id, &task_id)
        .await?;
    let interventions = state
        .services
        .project_tasks
        .list_task_interventions(&project_id, &task_id)
        .await?;
    Ok(Json(task_detail_from_records(&task, &runs, &interventions)))
}

pub(crate) async fn launch_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<LaunchTaskRequest>,
) -> Result<Json<TaskRunSummary>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.run",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let actor_ref = trim_optional_task_input(request.actor_ref)
        .unwrap_or_else(|| task.default_actor_ref.clone());
    if actor_ref.is_empty() {
        return Err(AppError::invalid_input("task actor is required").into());
    }
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, Some(&project_id)).await?;
    let runtime_session = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(
            CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some(project_id.clone()),
                title: task.title.clone(),
                session_kind: Some("task".into()),
                selected_actor_ref: actor_ref.clone(),
                selected_configured_model_id: None,
                execution_permission_mode: owner_permission_ceiling.clone(),
            },
            &session.user_id,
            Some(&owner_permission_ceiling),
        )
        .await?;
    let runtime_run = state
        .services
        .runtime_execution
        .submit_turn(
            &runtime_session.summary.id,
            SubmitRuntimeTurnInput {
                content: task_prompt_from_record(&task, "manual", None),
                permission_mode: None,
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await?;
    let run = state
        .services
        .project_tasks
        .save_task_run(build_task_run_record(
            &task,
            &runtime_session,
            &runtime_run,
            "manual",
            &actor_ref,
        ))
        .await?;
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(&task, &run, &session.user_id))
        .await?;
    Ok(Json(task_run_summary_from_record(&run)))
}

pub(crate) async fn rerun_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<RerunTaskRequest>,
) -> Result<Json<TaskRunSummary>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.run",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let actor_ref = trim_optional_task_input(request.actor_ref)
        .unwrap_or_else(|| task.default_actor_ref.clone());
    let source_task_run_id = trim_optional_task_input(request.source_task_run_id);
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, Some(&project_id)).await?;
    let runtime_session = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(
            CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some(project_id.clone()),
                title: format!("{} rerun", task.title),
                session_kind: Some("task".into()),
                selected_actor_ref: actor_ref.clone(),
                selected_configured_model_id: None,
                execution_permission_mode: owner_permission_ceiling.clone(),
            },
            &session.user_id,
            Some(&owner_permission_ceiling),
        )
        .await?;
    let runtime_run = state
        .services
        .runtime_execution
        .submit_turn(
            &runtime_session.summary.id,
            SubmitRuntimeTurnInput {
                content: task_prompt_from_record(&task, "rerun", source_task_run_id.as_deref()),
                permission_mode: None,
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await?;
    let run = state
        .services
        .project_tasks
        .save_task_run(build_task_run_record(
            &task,
            &runtime_session,
            &runtime_run,
            "rerun",
            &actor_ref,
        ))
        .await?;
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(&task, &run, &session.user_id))
        .await?;
    Ok(Json(task_run_summary_from_record(&run)))
}

pub(crate) async fn list_project_task_runs(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<TaskRunSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    Ok(Json(
        state
            .services
            .project_tasks
            .list_task_runs(&project_id, &task_id)
            .await?
            .iter()
            .map(task_run_summary_from_record)
            .collect(),
    ))
}

pub(crate) async fn create_project_task_intervention(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<CreateTaskInterventionRequest>,
) -> Result<Json<TaskInterventionRecord>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.intervene",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    if request.r#type.trim().is_empty() {
        return Err(AppError::invalid_input("task intervention type is required").into());
    }
    let intervention_type = request.r#type.trim();
    let explicit_approval_id = matches!(intervention_type, "approve" | "reject")
        .then(|| trim_optional_task_input(request.approval_id.clone()))
        .flatten();
    let mut runtime_synced_run = None;
    if matches!(intervention_type, "approve" | "reject") {
        let task = state
            .services
            .project_tasks
            .get_task(&project_id, &task_id)
            .await?;
        let target_run_id = trim_optional_task_input(request.task_run_id.clone())
            .or_else(|| task.active_task_run_id.clone());
        if let Some(target_run_id) = target_run_id {
            if let Some(target_run) = state
                .services
                .project_tasks
                .list_task_runs(&project_id, &task_id)
                .await?
                .into_iter()
                .find(|run| run.id == target_run_id)
            {
                if let Some(session_id) = target_run.session_id.as_deref() {
                    let runtime_session =
                        match state.services.runtime_session.get_session(session_id).await {
                            Ok(detail) => Some(detail),
                            Err(AppError::NotFound(_)) => None,
                            Err(error) => return Err(error.into()),
                        };
                    if let Some(runtime_session) = runtime_session {
                        if let Some(approval_id) = explicit_approval_id.clone().or_else(|| {
                            runtime_session
                                .pending_approval
                                .as_ref()
                                .map(|approval| approval.id.clone())
                        }) {
                            let previous_run = target_run.clone();
                            let runtime_run = state
                                .services
                                .runtime_execution
                                .resolve_approval(
                                    session_id,
                                    &approval_id,
                                    octopus_core::ResolveRuntimeApprovalInput {
                                        decision: if intervention_type == "approve" {
                                            "approve".into()
                                        } else {
                                            "reject".into()
                                        },
                                    },
                                )
                                .await?;
                            let refreshed_session = state
                                .services
                                .runtime_session
                                .get_session(session_id)
                                .await?;
                            runtime_synced_run = Some((
                                previous_run.clone(),
                                if intervention_type == "approve" {
                                    sync_task_run_record_from_runtime(
                                        &previous_run,
                                        &refreshed_session,
                                        &runtime_run,
                                    )
                                } else {
                                    sync_rejected_task_run_record_from_runtime(
                                        &previous_run,
                                        &refreshed_session,
                                        &runtime_run,
                                    )
                                },
                            ));
                        }
                    }
                }
            }
        }
        if runtime_synced_run.is_none() {
            if let Some(approval_id) = explicit_approval_id.as_deref() {
                return Err(AppError::conflict(format!(
                    "task approval `{approval_id}` could not be resolved in runtime"
                ))
                .into());
            }
        }
    }
    let record = state
        .services
        .project_tasks
        .create_task_intervention(&project_id, &task_id, &session.user_id, request)
        .await?;
    if let Some((previous_run, run)) = runtime_synced_run {
        let task = state
            .services
            .project_tasks
            .get_task(&project_id, &task_id)
            .await?;
        let run = state.services.project_tasks.save_task_run(run).await?;
        state
            .services
            .project_tasks
            .save_task(sync_task_record_from_run(
                &task,
                &previous_run,
                &run,
                &session.user_id,
            ))
            .await?;
    }
    Ok(Json(task_intervention_from_record(&record)))
}
