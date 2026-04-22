use super::*;

#[test]
fn validate_create_project_request_requires_and_trims_resource_directory() {
    let validated = validate_create_project_request(CreateProjectRequest {
        name: "  Resource Project  ".into(),
        description: "  Resource import coverage.  ".into(),
        resource_directory: "  data/projects/resource-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .expect("validated request");

    assert_eq!(validated.name, "Resource Project");
    assert_eq!(validated.description, "Resource import coverage.");
    assert_eq!(
        validated.resource_directory,
        "data/projects/resource-project/resources"
    );

    assert!(validate_create_project_request(CreateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        resource_directory: "   ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}

#[test]
fn validate_update_project_request_requires_status_and_resource_directory() {
    let validated = validate_update_project_request(UpdateProjectRequest {
        name: "  Resource Project  ".into(),
        description: "  Updated description.  ".into(),
        status: " archived ".into(),
        resource_directory: "  data/projects/resource-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .expect("validated update");

    assert_eq!(validated.name, "Resource Project");
    assert_eq!(validated.description, "Updated description.");
    assert_eq!(validated.status, "archived");
    assert_eq!(
        validated.resource_directory,
        "data/projects/resource-project/resources"
    );

    assert!(validate_update_project_request(UpdateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        status: "disabled".into(),
        resource_directory: "data/projects/resource-project/resources".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
    assert!(validate_update_project_request(UpdateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        status: "active".into(),
        resource_directory: " ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: None,
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}

#[test]
fn validate_create_project_request_trims_manager_preset_and_leader() {
    let validated = validate_create_project_request(CreateProjectRequest {
        name: "  Leader Project  ".into(),
        description: "  Use live inheritance.  ".into(),
        resource_directory: "  data/projects/leader-project/resources  ".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: Some("  agent-leader  ".into()),
        manager_user_id: Some("  user-manager  ".into()),
        preset_code: Some("  preset-ops  ".into()),
        assignments: None,
    })
    .expect("validated request");

    assert_eq!(validated.leader_agent_id.as_deref(), Some("agent-leader"));
    assert_eq!(validated.manager_user_id.as_deref(), Some("user-manager"));
    assert_eq!(validated.preset_code.as_deref(), Some("preset-ops"));
    assert!(validated.assignments.is_none());

    assert!(validate_create_project_request(CreateProjectRequest {
        name: "Project".into(),
        description: String::new(),
        resource_directory: "data/projects/leader-project/resources".into(),
        owner_user_id: None,
        member_user_ids: None,
        permission_overrides: None,
        linked_workspace_assets: None,
        leader_agent_id: Some("   ".into()),
        manager_user_id: None,
        preset_code: None,
        assignments: None,
    })
    .is_err());
}

#[tokio::test]
async fn project_leader_rejects_excluded_workspace_agent_on_update() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let workspace_agent = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent");
    let _ = save_project_runtime_config_route(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "projectSettings": {
                    "workspaceAssignments": {
                        "agents": {
                            "excludedAgentIds": [workspace_agent.id.clone()],
                        },
                    },
                },
            }),
            configured_model_credentials: Vec::new(),
        }),
    )
    .await
    .expect("save project workspace assignments");

    let project = state
        .services
        .workspace
        .list_projects()
        .await
        .expect("list projects")
        .into_iter()
        .find(|record| record.id == DEFAULT_PROJECT_ID)
        .expect("default project");
    let mut request = update_request_from_project(project);
    request.leader_agent_id = Some(workspace_agent.id.clone());

    let error = update_project(
        State(state.clone()),
        headers,
        Path(DEFAULT_PROJECT_ID.into()),
        Json(request),
    )
    .await
    .expect_err("excluded leader should be rejected");

    assert!(
        error.source.to_string().contains("leader"),
        "unexpected error: {:?}",
        error
    );
}

#[tokio::test]
async fn project_leader_rejects_project_owned_agent_on_update() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let workspace_agent = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent");
    let project_owned_agent = state
        .services
        .workspace
        .create_agent(project_scoped_agent_input(
            &workspace_agent,
            DEFAULT_PROJECT_ID,
        ))
        .await
        .expect("create project agent");
    let project = state
        .services
        .workspace
        .list_projects()
        .await
        .expect("list projects")
        .into_iter()
        .find(|record| record.id == DEFAULT_PROJECT_ID)
        .expect("default project");
    let mut request = update_request_from_project(project);
    request.leader_agent_id = Some(project_owned_agent.id.clone());

    let error = update_project(
        State(state.clone()),
        headers,
        Path(DEFAULT_PROJECT_ID.into()),
        Json(request),
    )
    .await
    .expect_err("project-owned leader should be rejected");

    assert!(
        error.source.to_string().contains("workspace agent"),
        "unexpected error: {:?}",
        error
    );
}

#[tokio::test]
async fn create_runtime_session_rejects_single_shot_generation_model_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config_with_generation_model(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);
    let actor_ref = visible_workspace_agent_actor_ref(&state).await;

    let error = create_runtime_session(
        State(state),
        headers,
        Json(CreateRuntimeSessionInput {
            conversation_id: "conv-generation-only".into(),
            project_id: None,
            title: "Generation Only Session".into(),
            session_kind: None,
            selected_actor_ref: actor_ref,
            selected_configured_model_id: Some("generation-only-model".into()),
            execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
        }),
    )
    .await
    .expect_err("single-shot generation model should be rejected");

    assert!(
        error
            .source
            .to_string()
            .contains("does not expose a runtime-supported surface"),
        "unexpected error: {:?}",
        error
    );
}

#[tokio::test]
async fn runtime_generation_route_executes_single_shot_generation_models() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config_with_generation_model(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let response = crate::routes::build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runtime/generations")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "configuredModelId": "generation-only-model",
                        "content": "Write a haiku about runtime boundaries.",
                        "systemPrompt": "Reply in one line."
                    }))
                    .expect("generation request json"),
                ))
                .expect("generation request"),
        )
        .await
        .expect("generation response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("generation body");
    let payload: Value = serde_json::from_slice(&body).expect("generation payload");
    assert_eq!(payload["configuredModelId"], "generation-only-model");
    assert_eq!(payload["configuredModelName"], "Generation Only Model");
    assert_eq!(payload["requestId"], "mock-request-id");
    assert_eq!(payload["consumedTokens"], 32);
    assert!(
        payload["content"]
            .as_str()
            .expect("generation content")
            .contains("Write a haiku about runtime boundaries."),
        "unexpected generation payload: {payload:?}"
    );
}

#[tokio::test]
async fn runtime_generation_route_rejects_agent_conversation_models() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let response = crate::routes::build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runtime/generations")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "configuredModelId": "quota-model",
                        "content": "Summarize the latest run."
                    }))
                    .expect("generation request json"),
                ))
                .expect("generation request"),
        )
        .await
        .expect("generation response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("generation body");
    let payload: Value = serde_json::from_slice(&body).expect("generation payload");
    assert_eq!(payload["error"]["code"], "INVALID_INPUT");
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("does not expose a runtime-supported surface"),
        "unexpected generation error payload: {payload:?}"
    );
}

#[tokio::test]
async fn project_leader_cannot_be_disabled_by_runtime_settings() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let workspace_agent = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent");
    let project = state
        .services
        .workspace
        .list_projects()
        .await
        .expect("list projects")
        .into_iter()
        .find(|record| record.id == DEFAULT_PROJECT_ID)
        .expect("default project");
    let mut request = update_request_from_project(project);
    request.leader_agent_id = Some(workspace_agent.id.clone());
    let _ = update_project(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(request),
    )
    .await
    .expect("set project leader");

    let error = save_project_runtime_config_route(
        State(state.clone()),
        headers,
        Path(DEFAULT_PROJECT_ID.into()),
        Json(RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "projectSettings": {
                    "agents": {
                        "disabledAgentIds": [workspace_agent.id],
                    },
                },
            }),
            configured_model_credentials: Vec::new(),
        }),
    )
    .await
    .expect_err("disabling the leader should be rejected");

    assert!(
        error.source.to_string().contains("leader"),
        "unexpected error: {:?}",
        error
    );
}

#[tokio::test]
async fn project_scope_uses_live_workspace_inheritance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);

    let workspace_agents = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents");
    let excluded_workspace_agent = workspace_agents
        .iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent")
        .clone();
    let expected_agent_ids = workspace_agents
        .iter()
        .filter(|record| {
            record.status == "active"
                && agent_visible_in_generic_catalog(record)
                && (record.project_id.as_deref() == Some(DEFAULT_PROJECT_ID)
                    || (record.project_id.is_none()
                        && record.id != excluded_workspace_agent.id))
        })
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();

    let workspace_teams = state
        .services
        .workspace
        .list_teams()
        .await
        .expect("list teams");
    let excluded_workspace_team = workspace_teams
        .iter()
        .find(|record| record.project_id.is_none() && record.status == "active")
        .expect("workspace team")
        .clone();
    let expected_team_ids = workspace_teams
        .iter()
        .filter(|record| {
            record.status == "active"
                && (record.project_id.as_deref() == Some(DEFAULT_PROJECT_ID)
                    || (record.project_id.is_none() && record.id != excluded_workspace_team.id))
        })
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();

    let capability_projection = state
        .services
        .workspace
        .get_capability_management_projection()
        .await
        .expect("capability projection");
    let excluded_source_key = capability_projection
        .assets
        .iter()
        .find(|asset| asset.enabled)
        .map(|asset| asset.source_key.clone())
        .expect("enabled tool");
    let expected_tool_source_keys = capability_projection
        .assets
        .iter()
        .filter(|asset| asset.enabled && asset.source_key != excluded_source_key)
        .map(|asset| asset.source_key.clone())
        .collect::<BTreeSet<_>>();

    let _ = save_project_runtime_config_route(
        State(state.clone()),
        headers.clone(),
        Path(DEFAULT_PROJECT_ID.into()),
        Json(RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "projectSettings": {
                    "workspaceAssignments": {
                        "tools": {
                            "excludedSourceKeys": [excluded_source_key.clone()],
                        },
                        "agents": {
                            "excludedAgentIds": [excluded_workspace_agent.id.clone()],
                            "excludedTeamIds": [excluded_workspace_team.id.clone()],
                        },
                    },
                },
            }),
            configured_model_credentials: Vec::new(),
        }),
    )
    .await
    .expect("save project workspace assignments");

    let Json(dashboard) = project_dashboard(
        State(state.clone()),
        headers,
        Path(DEFAULT_PROJECT_ID.into()),
    )
    .await
    .expect("project dashboard");

    assert_eq!(
        dashboard.overview.agent_count,
        expected_agent_ids.len() as u64
    );
    assert_eq!(
        dashboard.overview.team_count,
        expected_team_ids.len() as u64
    );
    assert_eq!(
        dashboard.overview.tool_count,
        expected_tool_source_keys.len() as u64
    );
    assert!(
        dashboard
            .resource_breakdown
            .iter()
            .find(|item| item.id == "tools")
            .and_then(|item| item.helper.as_deref())
            .is_some_and(|description| {
                expected_tool_source_keys
                    .iter()
                    .all(|source_key| description.contains(source_key))
            }),
        "tool breakdown should reflect live workspace tool source keys"
    );
}

