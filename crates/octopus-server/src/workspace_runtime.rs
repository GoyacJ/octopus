use super::*;
use crate::dto_mapping::{build_permission_center_alerts, metric_record};
use octopus_core::{ExportWorkspaceAgentBundleInput, ExportWorkspaceAgentBundleResult};

pub(crate) async fn workspace(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::WorkspaceSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.workspace_summary().await?))
}

pub(crate) async fn workspace_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceOverviewSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;

    let workspace = state.services.workspace.workspace_summary().await?;
    let projects = state.services.workspace.list_projects().await?;
    let conversations = list_conversation_records(&state, None).await?;
    let recent_activity = list_activity_records(&state, None).await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let agents = state.services.workspace.list_agents().await?;

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
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

pub(crate) async fn projects(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ProjectRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_projects().await?))
}

pub(crate) fn validate_create_project_request(
    request: CreateProjectRequest,
) -> Result<CreateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    Ok(CreateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        assignments: request.assignments,
    })
}

pub(crate) fn validate_update_project_request(
    request: UpdateProjectRequest,
) -> Result<UpdateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    let status = request.status.trim();
    if status != "active" && status != "archived" {
        return Err(AppError::invalid_input("project status must be active or archived").into());
    }

    Ok(UpdateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        status: status.into(),
        assignments: request.assignments,
    })
}

pub(crate) async fn create_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    let request = validate_create_project_request(request)?;
    Ok(Json(
        state.services.workspace.create_project(request).await?,
    ))
}

pub(crate) async fn update_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    let request = validate_update_project_request(request)?;
    Ok(Json(
        state
            .services
            .workspace
            .update_project(&project_id, request)
            .await?,
    ))
}

pub(crate) async fn project_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectDashboardSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;

    let project = lookup_project(&state, &project_id).await?;
    let conversations = list_conversation_records(&state, Some(&project_id)).await?;
    let recent_activity = list_activity_records(&state, Some(&project_id)).await?;
    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let knowledge = state
        .services
        .workspace
        .list_project_knowledge(&project_id)
        .await?;
    let agents = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == Some(project_id.as_str()))
        .collect::<Vec<_>>();

    Ok(Json(ProjectDashboardSnapshot {
        project,
        metrics: vec![
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

pub(crate) async fn workspace_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_workspace_resources().await?,
    ))
}

pub(crate) async fn project_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_resources(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .create_workspace_resource(&workspace_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn update_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .update_workspace_resource(&workspace_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    state
        .services
        .workspace
        .delete_workspace_resource(&workspace_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn create_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let record = state
        .services
        .workspace
        .create_project_resource(&project_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn create_project_resource_folder(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceFolderInput>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let records = state
        .services
        .workspace
        .create_project_resource_folder(&project_id, input)
        .await?;
    Ok(Json(records))
}

pub(crate) async fn update_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let record = state
        .services
        .workspace
        .update_project_resource(&project_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    state
        .services
        .workspace
        .delete_project_resource(&project_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn workspace_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_workspace_knowledge().await?,
    ))
}

pub(crate) async fn project_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_knowledge(&project_id)
            .await?,
    ))
}

pub(crate) async fn workspace_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_pet_snapshot()
            .await?,
    ))
}

pub(crate) async fn project_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_project_pet_snapshot(&project_id)
            .await?,
    ))
}

pub(crate) async fn save_workspace_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_workspace_pet_presence(input)
            .await?,
    ))
}

pub(crate) async fn save_project_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_project_pet_presence(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn bind_workspace_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_workspace_pet_conversation(input)
            .await?,
    ))
}

pub(crate) async fn bind_project_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_project_pet_conversation(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn list_agents(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_agents().await?))
}

pub(crate) async fn create_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
    )
    .await?;
    Ok(Json(state.services.workspace.create_agent(input).await?))
}

pub(crate) async fn preview_import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    Ok(Json(
        state.services.workspace.import_agent_bundle(input).await?,
    ))
}

pub(crate) async fn export_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn export_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
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
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
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
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_agent(&agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_teams(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_teams().await?))
}

pub(crate) async fn create_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
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
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
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
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_team(&team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_project_agent_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectAgentLinkRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
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
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    state
        .services
        .workspace
        .unlink_project_team(&project_id, &team_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn workspace_catalog_models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.runtime_registry.catalog_snapshot().await?,
    ))
}

pub(crate) async fn workspace_provider_credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderCredentialRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_provider_credentials().await?,
    ))
}

pub(crate) async fn workspace_tool_catalog(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceToolCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.get_tool_catalog().await?))
}

pub(crate) async fn workspace_tool_catalog_disable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<WorkspaceToolDisablePatch>,
) -> Result<Json<WorkspaceToolCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .set_tool_catalog_disabled(patch)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill(&skill_id)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_tree_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillTreeDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_tree(&skill_id)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_file(&skill_id, &relative_path)
            .await?,
    ))
}

pub(crate) async fn create_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_skill(input)
            .await?,
    ))
}

pub(crate) async fn import_workspace_skill_archive_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillArchiveInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_archive(input)
            .await?,
    ))
}

pub(crate) async fn import_workspace_skill_folder_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillFolderInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_folder(input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<UpdateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill(&skill_id, input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceSkillFileInput>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill_file(&skill_id, &relative_path, input)
            .await?,
    ))
}

pub(crate) async fn copy_workspace_skill_to_managed_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<CopyWorkspaceSkillToManagedInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_skill_to_managed(&skill_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_workspace_skill(&skill_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn get_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_mcp_server(&server_name)
            .await?,
    ))
}

pub(crate) async fn create_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_mcp_server(input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_mcp_server(&server_name, input)
            .await?,
    ))
}

pub(crate) async fn delete_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_workspace_mcp_server(&server_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ToolRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_tools().await?))
}

pub(crate) async fn create_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_tool(record).await?))
}

pub(crate) async fn update_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_tool(&tool_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_tool(&tool_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_automations(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AutomationRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_automations().await?))
}

pub(crate) async fn create_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        record.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state.services.workspace.create_automation(record).await?,
    ))
}

pub(crate) async fn update_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        record.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_automation(&automation_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_automation(&automation_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn permission_center_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<PermissionCenterOverviewSnapshot>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    let users = state.services.workspace.list_users().await?;
    let roles = state.services.workspace.list_roles().await?;
    let permissions = state.services.workspace.list_permissions().await?;
    let menus = state.services.workspace.list_menus().await?;
    let current_user = users
        .iter()
        .find(|record| record.id == session.user_id)
        .cloned()
        .ok_or_else(|| ApiError::new(AppError::not_found("current user"), request_id(&headers)))?;

    let role_names = roles
        .iter()
        .filter(|record| {
            current_user
                .role_ids
                .iter()
                .any(|role_id| role_id == &record.id)
        })
        .map(|record| record.name.clone())
        .collect::<Vec<_>>();
    let quick_links = menus
        .iter()
        .filter(|record| record.source == "permission-center" && record.status == "active")
        .cloned()
        .collect::<Vec<_>>();

    Ok(Json(PermissionCenterOverviewSnapshot {
        workspace_id: session.workspace_id.clone(),
        current_user,
        role_names,
        metrics: vec![
            metric_record("users", "Users", users.len()),
            metric_record("roles", "Roles", roles.len()),
            metric_record("permissions", "Permissions", permissions.len()),
            metric_record("menus", "Menus", menus.len()),
        ],
        alerts: build_permission_center_alerts(&session, &permissions),
        quick_links,
    }))
}

pub(crate) async fn list_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserRecordSummary>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_users().await?))
}

pub(crate) async fn create_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkspaceUserRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_user(request).await?))
}

pub(crate) async fn update_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateWorkspaceUserRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_user(&user_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    if session.user_id == user_id {
        return Err(ApiError::from(AppError::invalid_input(
            "current user cannot be deleted",
        )));
    }
    state.services.workspace.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn update_current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentUserProfileRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_current_user_profile(&session.user_id, request)
            .await?,
    ))
}

pub(crate) async fn change_current_user_password_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ChangeCurrentUserPasswordRequest>,
) -> Result<Json<ChangeCurrentUserPasswordResponse>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .change_current_user_password(&session.user_id, request)
            .await?,
    ))
}

pub(crate) async fn list_roles(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RoleRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_roles().await?))
}

pub(crate) async fn create_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<RoleRecord>,
) -> Result<Json<RoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_role(record).await?))
}

pub(crate) async fn update_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
    Json(record): Json<RoleRecord>,
) -> Result<Json<RoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_role(&role_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_role(&role_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_permissions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PermissionRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_permissions().await?))
}

pub(crate) async fn create_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<PermissionRecord>,
) -> Result<Json<PermissionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.create_permission(record).await?,
    ))
}

pub(crate) async fn update_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(permission_id): Path<String>,
    Json(record): Json<PermissionRecord>,
) -> Result<Json<PermissionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_permission(&permission_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(permission_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_permission(&permission_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_menus(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_menus().await?))
}

pub(crate) async fn create_menu(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<MenuRecord>,
) -> Result<Json<MenuRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_menu(record).await?))
}

pub(crate) async fn update_menu(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
    Json(record): Json<MenuRecord>,
) -> Result<Json<MenuRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_menu(&menu_id, record)
            .await?,
    ))
}

pub(crate) async fn inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::InboxItemRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.inbox.list_inbox().await?))
}

pub(crate) async fn artifacts(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ArtifactRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.artifact.list_artifacts().await?))
}

pub(crate) async fn knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::KnowledgeEntryRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.knowledge.list_knowledge().await?))
}

pub(crate) async fn audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::AuditRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "audit.read", None).await?;
    Ok(Json(state.services.observation.list_audit_records().await?))
}

pub(crate) async fn runtime_bootstrap(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::RuntimeBootstrap>, ApiError> {
    ensure_authorized_session(&state, &headers, "runtime.read", None).await?;
    Ok(Json(state.services.runtime_session.bootstrap().await?))
}

pub(crate) async fn get_runtime_config(
    State(state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    Ok(Json(state.services.runtime_config.get_config().await?))
}

pub(crate) async fn validate_runtime_config_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    Ok(Json(
        state.services.runtime_config.validate_config(patch).await?,
    ))
}

pub(crate) async fn probe_runtime_configured_model_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Json(input): Json<RuntimeConfiguredModelProbeInput>,
) -> Result<Json<RuntimeConfiguredModelProbeResult>, ApiError> {
    Ok(Json(
        state
            .services
            .runtime_config
            .probe_configured_model(input)
            .await?,
    ))
}

pub(crate) async fn save_runtime_config_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Path(scope): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    Ok(Json(
        state
            .services
            .runtime_config
            .save_config(&scope, patch)
            .await?,
    ))
}

pub(crate) async fn get_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_project_config(&project_id, &session.user_id)
            .await?,
    ))
}

pub(crate) async fn validate_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn save_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn get_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_user_config(&session.user_id)
            .await?,
    ))
}

pub(crate) async fn validate_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_user_config(&session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn save_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_user_config(&session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn lookup_project(
    state: &ServerState,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))
}

pub(crate) async fn list_conversation_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<ConversationRecord>, ApiError> {
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(sessions
        .into_iter()
        .filter(|record| project_id.map(|id| record.project_id == id).unwrap_or(true))
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
        })
        .collect())
}

pub(crate) async fn list_activity_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<WorkspaceActivityRecord>, ApiError> {
    let mut records = state.services.observation.list_audit_records().await?;
    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(records
        .into_iter()
        .filter(|record| {
            project_id
                .map(|id| record.project_id.as_deref() == Some(id))
                .unwrap_or(true)
        })
        .map(|record| WorkspaceActivityRecord {
            id: record.id,
            title: record.action,
            description: format!(
                "{} {} {}",
                record.actor_type, record.actor_id, record.outcome
            ),
            timestamp: record.created_at,
        })
        .collect())
}

pub(crate) async fn list_runtime_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::RuntimeSessionSummary>>, ApiError> {
    ensure_authorized_session(&state, &headers, "runtime.read", None).await?;
    Ok(Json(state.services.runtime_session.list_sessions().await?))
}

pub(crate) async fn create_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::CreateRuntimeSessionInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = normalize_project_scope(&input.project_id);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.read",
        project_id,
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.create_session",
            &input.conversation_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let detail = state
        .services
        .runtime_session
        .create_session(input, &session.user_id)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &detail, &request_id)?;
    }

    let mut response = Json(detail).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn get_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<octopus_core::RuntimeSessionDetail>, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(&state, &headers, "runtime.read", project_id.as_deref()).await?;
    Ok(Json(
        state
            .services
            .runtime_session
            .get_session(&session_id)
            .await?,
    ))
}

pub(crate) async fn delete_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(&state, &headers, "runtime.read", project_id.as_deref()).await?;
    state
        .services
        .runtime_session
        .delete_session(&session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn submit_runtime_turn(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(mut input): Json<SubmitRuntimeTurnInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    normalize_runtime_submit_input(&mut input)?;
    let session = ensure_runtime_submit(
        &state,
        &headers,
        Some(&input),
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.submit_turn", &session_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .submit_turn(&session_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &run, &request_id)?;
    }

    let mut response = Json(run).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, approval_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeApprovalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.resolve_approval",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.resolve_approval", &approval_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_approval(&session_id, &approval_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &run, &request_id)?;
    }

    let mut response = Json(run).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;

    let replay_after = query.after.or_else(|| last_event_id(&headers));

    if !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?;
        let mut response = Json(events).into_response();
        insert_request_id(&mut response, &request_id);
        return Ok(response);
    }

    let replay_events = if replay_after.is_some() {
        state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?
    } else {
        Vec::new()
    };
    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream = stream! {
        for event in replay_events {
            if let Ok(data) = serde_json::to_string(&event) {
                yield Ok::<Event, std::convert::Infallible>(
                    Event::default()
                        .event(event.event_type.clone())
                        .id(event.id.clone())
                        .data(data)
                );
            }
        }

        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok::<Event, std::convert::Infallible>(
                            Event::default()
                                .event(event.event_type.clone())
                                .id(event.id.clone())
                                .data(data)
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}
