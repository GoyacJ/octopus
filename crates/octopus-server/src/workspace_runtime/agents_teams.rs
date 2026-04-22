use super::*;

pub(crate) async fn list_agents(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "agent.view",
            None,
            Some("agent"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let agents = state.services.workspace.list_agents().await?;
    let mut visible = Vec::new();
    for record in agents {
        if !agent_visible_in_generic_catalog(&record) {
            continue;
        }
        if authorize_request(
            &state,
            &session,
            &agent_authorization_request(&state, &session, "agent.view", &record).await?,
            &request_id(&headers),
        )
        .await
        .is_ok()
        {
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn create_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &agent_input_authorization_request(&session, "agent.edit", &input, None),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(state.services.workspace.create_agent(input).await?))
}

pub(crate) async fn preview_import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.import", None, Some("agent"), None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .preview_import_agent_bundle(input)
            .await?,
    ))
}

pub(crate) async fn import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundleInput>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.import", None, Some("agent"), None).await?;
    Ok(Json(
        state.services.workspace.import_agent_bundle(input).await?,
    ))
}

pub(crate) async fn copy_workspace_agent_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        None,
        Some("agent"),
        Some(&agent_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_agent_from_builtin(&agent_id)
            .await?,
    ))
}

pub(crate) async fn export_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.export", None, Some("agent"), None).await?;
    Ok(Json(
        state.services.workspace.export_agent_bundle(input).await?,
    ))
}

pub(crate) async fn preview_import_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .preview_import_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn import_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ImportWorkspaceAgentBundleInput>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn copy_project_agent_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, agent_id)): Path<(String, String)>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        Some(&agent_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_project_agent_from_builtin(&project_id, &agent_id)
            .await?,
    ))
}

pub(crate) async fn export_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.export",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .export_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn update_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &agent_input_authorization_request(&session, "agent.edit", &input, Some(&agent_id)),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_agent(&agent_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let agent = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .find(|record| record.id == agent_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("agent not found")))?;
    authorize_request(
        &state,
        &session,
        &agent_authorization_request(&state, &session, "agent.delete", &agent).await?,
        &request_id(&headers),
    )
    .await?;
    state.services.workspace.delete_agent(&agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_teams(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamRecord>>, ApiError> {
    ensure_capability_session(&state, &headers, "team.view", None, Some("team"), None).await?;
    Ok(Json(state.services.workspace.list_teams().await?))
}

pub(crate) async fn create_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        input.project_id.as_deref(),
        Some("team"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.create_team(input).await?))
}

pub(crate) async fn update_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        input.project_id.as_deref(),
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_team(&team_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        None,
        Some("team"),
        Some(&team_id),
    )
    .await?;
    state.services.workspace.delete_team(&team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn copy_workspace_team_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.import",
        None,
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_team_from_builtin(&team_id)
            .await?,
    ))
}

pub(crate) async fn copy_project_team_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, team_id)): Path<(String, String)>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.import",
        Some(&project_id),
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_project_team_from_builtin(&project_id, &team_id)
            .await?,
    ))
}

pub(crate) async fn list_project_agent_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectAgentLinkRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_agent_links(&project_id)
            .await?,
    ))
}

pub(crate) async fn link_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectAgentLinkInput>,
) -> Result<Json<ProjectAgentLinkRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_agent(input).await?,
    ))
}

pub(crate) async fn unlink_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    state
        .services
        .workspace
        .unlink_project_agent(&project_id, &agent_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_project_team_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectTeamLinkRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_team_links(&project_id)
            .await?,
    ))
}

pub(crate) async fn link_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectTeamLinkInput>,
) -> Result<Json<ProjectTeamLinkRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_team(input).await?,
    ))
}

pub(crate) async fn unlink_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, team_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    state
        .services
        .workspace
        .unlink_project_team(&project_id, &team_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
