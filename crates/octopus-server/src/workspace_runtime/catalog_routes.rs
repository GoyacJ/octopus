use super::*;

pub(crate) async fn workspace_catalog_models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelCatalogSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.view",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state.services.runtime_registry.catalog_snapshot().await?,
    ))
}

pub(crate) async fn workspace_provider_credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderCredentialRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "provider-credential.view",
        None,
        Some("provider-credential"),
        None,
    )
    .await?;
    Ok(Json(
        state.services.workspace.list_provider_credentials().await?,
    ))
}

pub(crate) async fn workspace_capability_management_projection(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<CapabilityManagementProjection>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.view",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_capability_management_projection()
            .await?,
    ))
}

pub(crate) async fn workspace_capability_asset_disable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<CapabilityAssetDisablePatch>,
) -> Result<Json<CapabilityManagementProjection>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.manage",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .set_capability_asset_disabled(patch)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.delete",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.view",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(&state, &session, "tool.mcp.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.configure",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.delete",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_workspace_mcp_server(&server_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn copy_workspace_mcp_server_to_managed_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.configure",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_mcp_server_to_managed(&server_name)
            .await?,
    ))
}

pub(crate) async fn list_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ToolRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let records = state.services.workspace.list_tools().await?;
    let mut visible = Vec::new();
    for record in records {
        let capability = format!("{}.view", precise_tool_resource_type(&record.kind));
        if authorize_request(
            &state,
            &session,
            &tool_record_authorization_request(&state, &session, &capability, &record).await?,
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

pub(crate) async fn create_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let capability = format!("{}.configure", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(state.services.workspace.create_tool(record).await?))
}

pub(crate) async fn update_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let capability = format!("{}.configure", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .list_tools()
        .await?
        .into_iter()
        .find(|item| item.id == tool_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("tool not found")))?;
    let capability = format!("{}.delete", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    state.services.workspace.delete_tool(&tool_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
