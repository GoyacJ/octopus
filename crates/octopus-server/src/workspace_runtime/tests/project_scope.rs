use super::support::*;
use super::*;

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
                    || (record.project_id.is_none() && record.id != excluded_workspace_agent.id))
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
