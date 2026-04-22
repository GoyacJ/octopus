use super::*;

#[tokio::test]
async fn project_task_routes_reject_and_resume_interventions_update_task_state() {
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

    let Json(rejected) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: None,
            r#type: "reject".into(),
            payload: serde_json::json!({}),
        }),
    ))
    .await
    .expect("reject task intervention");

    assert_eq!(rejected.status, "applied");

    let Json(rejected_detail) = get_project_task_detail(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get rejected task detail");

    assert_eq!(rejected_detail.status, "attention");
    assert_eq!(rejected_detail.view_status, "attention");
    assert_eq!(rejected_detail.attention_reasons, vec!["waiting_input"]);
    assert_eq!(
        rejected_detail.latest_result_summary.as_deref(),
        Some("Approval rejected. Waiting for updated guidance.")
    );
    assert_eq!(
        rejected_detail
            .active_run
            .as_ref()
            .map(|run| run.status.as_str()),
        Some("waiting_input")
    );
    assert_eq!(
        rejected_detail
            .active_run
            .as_ref()
            .map(|run| run.attention_reasons.clone()),
        Some(vec!["waiting_input".into()])
    );

    let Json(resumed) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: None,
            r#type: "resume".into(),
            payload: serde_json::json!({}),
        }),
    ))
    .await
    .expect("resume task intervention");

    assert_eq!(resumed.status, "applied");

    let Json(resumed_detail) = get_project_task_detail(
        State(state.clone()),
        headers,
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get resumed task detail");

    assert_eq!(resumed_detail.status, "running");
    assert_eq!(resumed_detail.view_status, "healthy");
    assert!(resumed_detail.attention_reasons.is_empty());
    assert_eq!(
        resumed_detail.latest_result_summary.as_deref(),
        Some("Updated guidance received. Continuing the active run.")
    );
    assert_eq!(
        resumed_detail
            .active_run
            .as_ref()
            .map(|run| run.status.as_str()),
        Some("running")
    );
    assert_eq!(resumed_detail.intervention_history.len(), 2);
    assert_eq!(resumed_detail.intervention_history[0].r#type, "resume");
    assert_eq!(resumed_detail.intervention_history[0].status, "applied");
    assert_eq!(resumed_detail.intervention_history[1].r#type, "reject");
    assert_eq!(resumed_detail.intervention_history[1].status, "applied");
}

#[tokio::test]
async fn project_task_routes_edit_brief_intervention_updates_task_projection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Prepare release brief".into(),
            goal: "Keep the release brief aligned with final handoff scope.".into(),
            brief: "Focus on release sequencing and deliverable links.".into(),
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
    let seeded_run = seed_task_run(&state, &task, &session.user_id, "running").await;

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: None,
            r#type: "edit_brief".into(),
            payload: serde_json::json!({
                "brief": "Focus on the final release notes and linked deliverables."
            }),
        }),
    ))
    .await
    .expect("edit brief intervention");

    assert_eq!(intervention.status, "accepted");

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get task detail after brief edit");

    assert_eq!(
        detail.brief,
        "Focus on the final release notes and linked deliverables."
    );
    assert_eq!(detail.status, "running");
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .map(|transition| transition.kind.as_str()),
        Some("intervened")
    );
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .map(|transition| transition.summary.as_str()),
        Some("Task intervention recorded: edit_brief.")
    );
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .and_then(|transition| transition.run_id.as_deref()),
        Some(seeded_run.id.as_str())
    );

    let Json(tasks) = list_project_tasks(
        State(state.clone()),
        headers,
        Path(DEFAULT_PROJECT_ID.into()),
    )
    .await
    .expect("list project tasks after brief edit");

    assert_eq!(
        tasks
            .iter()
            .find(|record| record.id == created.id)
            .and_then(|record| record.latest_transition.as_ref())
            .map(|transition| transition.kind.as_str()),
        Some("intervened")
    );
}

#[tokio::test]
async fn project_task_routes_change_actor_intervention_updates_task_and_target_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Prepare release brief".into(),
            goal: "Keep the release brief aligned with final handoff scope.".into(),
            brief: "Focus on release sequencing and deliverable links.".into(),
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
    let seeded_run = seed_task_run(&state, &task, &session.user_id, "running").await;

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: Some(seeded_run.id.clone()),
            approval_id: None,
            r#type: "change_actor".into(),
            payload: serde_json::json!({
                "actorRef": "agent:release-operator"
            }),
        }),
    ))
    .await
    .expect("change actor intervention");

    assert_eq!(intervention.status, "accepted");

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get task detail after actor change");

    assert_eq!(detail.default_actor_ref, "agent:release-operator");
    assert_eq!(
        detail.active_run.as_ref().map(|run| run.actor_ref.as_str()),
        Some("agent:release-operator")
    );
    assert_eq!(
        detail
            .run_history
            .iter()
            .find(|run| run.id == seeded_run.id)
            .map(|run| run.actor_ref.as_str()),
        Some("agent:release-operator")
    );
    assert_eq!(detail.status, "running");
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .map(|transition| transition.kind.as_str()),
        Some("intervened")
    );

    let Json(runs) = list_project_task_runs(
        State(state.clone()),
        headers,
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("list runs after actor change");

    assert_eq!(
        runs.iter()
            .find(|run| run.id == seeded_run.id)
            .map(|run| run.actor_ref.as_str()),
        Some("agent:release-operator")
    );
}

#[tokio::test]
async fn project_task_routes_takeover_intervention_surfaces_attention_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let Json(created) = create_project_task(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(CreateTaskRequest {
            title: "Audit workspace menu".into(),
            goal: "Review navigation labels and routing consistency.".into(),
            brief: "Validate the desktop project menu before release.".into(),
            default_actor_ref: "team:workspace-core".into(),
            schedule_spec: None,
            context_bundle: TaskContextBundle::default(),
        }),
    )
    .await
    .expect("create task");

    let Json(intervention) = Box::pin(create_project_task_intervention(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        Json(CreateTaskInterventionRequest {
            task_run_id: None,
            approval_id: None,
            r#type: "takeover".into(),
            payload: serde_json::json!({}),
        }),
    ))
    .await
    .expect("takeover intervention");

    assert_eq!(intervention.status, "accepted");

    let Json(detail) = get_project_task_detail(
        State(state.clone()),
        headers.clone(),
        Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
    )
    .await
    .expect("get task detail after takeover");

    assert_eq!(detail.status, "ready");
    assert_eq!(detail.view_status, "attention");
    assert_eq!(detail.attention_reasons, vec!["takeover_recommended"]);
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .map(|transition| transition.kind.as_str()),
        Some("intervened")
    );
    assert_eq!(
        detail
            .latest_transition
            .as_ref()
            .map(|transition| transition.summary.as_str()),
        Some("Task intervention recorded: takeover.")
    );
}

#[tokio::test]
async fn project_task_routes_respect_project_task_module_denials() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;

    let project = state
        .services
        .workspace
        .list_projects()
        .await
        .expect("list projects")
        .into_iter()
        .find(|record| record.id == DEFAULT_PROJECT_ID)
        .expect("default project");

    state
        .services
        .workspace
        .update_project(
            DEFAULT_PROJECT_ID,
            UpdateProjectRequest {
                name: project.name,
                description: project.description,
                status: project.status,
                resource_directory: project.resource_directory,
                owner_user_id: Some(project.owner_user_id),
                member_user_ids: Some(project.member_user_ids),
                permission_overrides: Some(ProjectPermissionOverrides {
                    tasks: "deny".into(),
                    ..project.permission_overrides
                }),
                leader_agent_id: project.leader_agent_id,
                manager_user_id: project.manager_user_id,
                preset_code: project.preset_code,
                linked_workspace_assets: None,
                assignments: None,
            },
        )
        .await
        .expect("deny task module");

    let error = list_project_tasks(
        State(state.clone()),
        auth_headers(&session.token),
        Path(DEFAULT_PROJECT_ID.into()),
    )
    .await
    .expect_err("task list should be denied");

    assert!(
        error
            .source
            .to_string()
            .contains("project module tasks is not available"),
        "unexpected error: {:?}",
        error
    );
}
