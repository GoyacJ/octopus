use super::support::*;
use super::*;

#[tokio::test]
async fn project_task_routes_create_launch_rerun_and_intervene_against_project_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);
    let workspace_agent_ref = visible_workspace_agent_actor_ref(&state).await;

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Prepare launch checklist".into(),
            goal: "Create a launch-ready checklist for the redesign rollout.".into(),
            brief: "Focus on sequencing, dependencies, and handoff notes.".into(),
            default_actor_ref: workspace_agent_ref.clone(),
            schedule_spec: Some("0 9 * * 1-5".into()),
            context_bundle: TaskContextBundle {
                refs: vec![TaskContextRef {
                    kind: "resource".into(),
                    ref_id: "res-brief".into(),
                    title: "Project brief".into(),
                    subtitle: "Source brief".into(),
                    version_ref: None,
                    pin_mode: "snapshot".into(),
                }],
                pinned_instructions: "Keep the output concise.".into(),
                resolution_mode: "explicit_only".into(),
                last_resolved_at: None,
            },
        }),
    )
    .await
    .expect("create task");

    assert_eq!(created.project_id, DEFAULT_PROJECT_ID);
    assert_eq!(created.run_history.len(), 0);

    let Json(tasks) = list_project_tasks(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
    )
    .await
    .expect("list project tasks");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, created.id);

    let Json(launch_run) = Box::pin(launch_project_task(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(LaunchTaskRequest {
            actor_ref: Some(workspace_agent_ref.clone()),
        }),
    ))
    .await
    .expect("launch project task");
    assert_eq!(launch_run.task_id, created.id);
    assert!(launch_run.session_id.is_some());

    let Json(rerun) = Box::pin(rerun_project_task(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(RerunTaskRequest {
            actor_ref: Some(workspace_agent_ref),
            source_task_run_id: Some(launch_run.id.clone()),
        }),
    ))
    .await
    .expect("rerun project task");
    assert_eq!(rerun.task_id, created.id);

    let Json(runs) = list_project_task_runs(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("list project task runs");
    assert_eq!(runs.len(), 2);

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(rerun.id.clone()),
            approval_id: None,
            r#type: "comment".into(),
            payload: serde_json::json!({
                "note": "Please keep the checklist aligned with project handoff rules."
            }),
        }),
    ))
    .await
    .expect("create project task intervention");
    assert_eq!(intervention.task_id, created.id);

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers,
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get project task detail");

    assert_eq!(detail.run_history.len(), 2);
    assert_eq!(detail.intervention_history.len(), 1);
    assert_eq!(
        detail.active_run.as_ref().map(|run| run.id.as_str()),
        Some(rerun.id.as_str())
    );
}

#[tokio::test]
async fn project_task_routes_approve_intervention_updates_waiting_approval_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Review launch approval".into(),
            goal: "Pause the task until an owner approves the plan.".into(),
            brief: "Route the active run through an approval gate.".into(),
            default_actor_ref: "team:workspace-core".into(),
            schedule_spec: None,
            context_bundle: TaskContextBundle::default(),
        }),
    )
    .await
    .expect("create task");

    let task = state
        .services
        .project_tasks
        .get_task(DEFAULT_PROJECT_ID, &created.id)
        .await
        .expect("get created task");
    let seeded_run = seed_task_run(&state, &task, &session.user_id, "waiting_approval").await;

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: None,
            r#type: "approve".into(),
            payload: serde_json::json!({}),
        }),
    ))
    .await
    .expect("approve task intervention");

    assert_eq!(intervention.status, "applied");
    assert_eq!(intervention.r#type, "approve");

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers,
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get project task detail");

    assert_eq!(detail.status, "running");
    assert_eq!(detail.view_status, "healthy");
    assert!(detail.attention_reasons.is_empty());
    assert_eq!(
        detail.latest_result_summary.as_deref(),
        Some("Approval received. Continuing the active run.")
    );
    assert_eq!(
        detail.active_run.as_ref().map(|run| run.status.as_str()),
        Some("running")
    );
    assert_eq!(
        detail
            .active_run
            .as_ref()
            .map(|run| run.view_status.as_str()),
        Some("healthy")
    );
    assert_eq!(
        detail
            .active_run
            .as_ref()
            .map(|run| run.attention_reasons.clone()),
        Some(Vec::new())
    );
    assert_eq!(detail.intervention_history.len(), 1);
    assert_eq!(detail.intervention_history[0].status, "applied");
}

#[tokio::test]
async fn project_task_routes_approve_intervention_resolves_runtime_pending_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);
    insert_approval_required_agent(temp.path());

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Review launch approval".into(),
            goal: "Pause the task until an owner approves the plan.".into(),
            brief: "Route the active run through an approval gate.".into(),
            default_actor_ref: APPROVAL_AGENT_REF.into(),
            schedule_spec: None,
            context_bundle: TaskContextBundle::default(),
        }),
    )
    .await
    .expect("create task");

    let task = state
        .services
        .project_tasks
        .get_task(DEFAULT_PROJECT_ID, &created.id)
        .await
        .expect("get created task");
    let seeded_run = Box::pin(seed_runtime_pending_approval_task_run(
        &state,
        &task,
        &session.user_id,
    ))
    .await;
    let runtime_session_id = seeded_run
        .session_id
        .clone()
        .expect("runtime-backed task run session id");

    let runtime_before = state
        .services
        .runtime_session
        .get_session(&runtime_session_id)
        .await
        .expect("runtime session before intervention");
    assert!(runtime_before.pending_approval.is_some());
    let approval_id = runtime_before
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("runtime pending approval id");

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: Some(approval_id),
            r#type: "approve".into(),
            payload: serde_json::json!({}),
        }),
    ))
    .await
    .expect("approve task intervention");

    assert_eq!(intervention.status, "applied");
    assert_eq!(intervention.r#type, "approve");

    let runtime_after = state
        .services
        .runtime_session
        .get_session(&runtime_session_id)
        .await
        .expect("runtime session after intervention");
    assert!(runtime_after.pending_approval.is_none());

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers,
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get project task detail");

    assert_eq!(detail.status, "completed");
    assert_eq!(detail.view_status, "healthy");
    assert_eq!(
        detail.active_run.as_ref().map(|run| run.status.as_str()),
        Some("completed")
    );
    assert_eq!(
        detail.latest_result_summary.as_deref(),
        Some("Task run completed in the runtime.")
    );
    assert_eq!(detail.analytics_summary.run_count, 1);
    assert_eq!(detail.analytics_summary.manual_run_count, 1);
    assert_eq!(detail.analytics_summary.completion_count, 1);
    assert_eq!(detail.analytics_summary.approval_required_count, 1);
    assert_eq!(detail.intervention_history.len(), 1);
    assert_eq!(detail.intervention_history[0].status, "applied");
}
