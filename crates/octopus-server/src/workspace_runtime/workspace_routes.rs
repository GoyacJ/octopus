use super::*;

pub(crate) async fn workspace(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::WorkspaceSummary>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "workspace.overview.read",
        None,
        Some("workspace"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.workspace_summary().await?))
}

pub(crate) async fn update_workspace_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let workspace = state.services.workspace.workspace_summary().await?;
    if workspace.owner_user_id.as_deref() != Some(session.user_id.as_str()) {
        return Err(ApiError::from(AppError::auth(
            "workspace settings require the workspace owner",
        )));
    }
    Ok(Json(
        state.services.workspace.update_workspace(request).await?,
    ))
}

pub(crate) async fn workspace_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceOverviewSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "workspace.overview.read",
        None,
        Some("workspace"),
        None,
    )
    .await?;

    let workspace = state.services.workspace.workspace_summary().await?;
    let projects = state.services.workspace.list_projects().await?;
    let conversations = list_conversation_records(&state, None).await?;
    let recent_activity = list_activity_records(&state, None).await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let agents = state.services.workspace.list_agents().await?;
    let project_token_usage = state
        .services
        .observation
        .list_project_token_usage()
        .await?;
    let project_token_usage = project_token_usage
        .into_iter()
        .filter_map(|record| {
            let project = projects
                .iter()
                .find(|project| project.id == record.project_id)?;
            Some(ProjectTokenUsageRecord {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
                used_tokens: record.used_tokens,
            })
        })
        .take(8)
        .collect();

    Ok(Json(WorkspaceOverviewSnapshot {
        workspace,
        metrics: vec![
            metric_record("projects", "Projects", projects.len()),
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        projects,
        project_token_usage,
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

pub(crate) async fn projects(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ProjectRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        None,
        Some("project"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.list_projects().await?))
}
