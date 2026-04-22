use super::*;

pub(crate) async fn workspace_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "resource.view",
            None,
            Some("resource"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in resources {
        if authorize_request(
            &state,
            &session,
            &resource_authorization_request(&state, &session, "resource.view", &record).await?,
            &request_id,
        )
        .await
        .is_ok()
            && resource_visibility_allows(&session, &record)
        {
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn project_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "resource.view",
            Some(&project_id),
            Some("resource"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in resources {
        if authorize_request(
            &state,
            &session,
            &resource_authorization_request(&state, &session, "resource.view", &record).await?,
            &request_id,
        )
        .await
        .is_ok()
            && resource_visibility_allows(&session, &record)
        {
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn project_deliverables(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            Some(&project_id),
            Some("artifact"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_deliverables(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            input.project_id.as_deref(),
            &input.tags,
        ),
        &request_id(&headers),
    )
    .await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .create_workspace_resource(&workspace_id, &session.user_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn import_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<WorkspaceResourceImportInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let tags = input.tags.clone().unwrap_or_default();
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(&session, "resource.upload", None, &tags),
        &request_id(&headers),
    )
    .await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_resource(&workspace_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn get_resource_detail(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(record))
}

pub(crate) async fn get_resource_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<WorkspaceResourceContentDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_resource_content(&resource_id)
            .await?,
    ))
}

pub(crate) async fn list_resource_children(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceChildrenRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_resource_children(&resource_id)
            .await?,
    ))
}

pub(crate) async fn promote_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<PromoteWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    let capability = if input.scope == "workspace" {
        "resource.publish"
    } else {
        "resource.update"
    };
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .promote_resource(&resource_id, input)
            .await?,
    ))
}

pub(crate) async fn list_workspace_filesystem_directories(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<WorkspaceDirectoryBrowserQuery>,
) -> Result<Json<WorkspaceDirectoryBrowserResponse>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        None,
        Some("project"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_directories(query.path.as_deref())
            .await?,
    ))
}

pub(crate) async fn update_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let current = state
        .services
        .workspace
        .list_workspace_resources()
        .await?
        .into_iter()
        .find(|record| record.id == resource_id && record.workspace_id == workspace_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    let tags = input.tags.clone().unwrap_or_else(|| current.tags.clone());
    authorize_request(
        &state,
        &session,
        &capability_authorization_request(
            &session.user_id,
            "resource.update",
            current.project_id.as_deref(),
            Some("resource"),
            Some(&current.id),
            Some(&current.kind),
            &tags,
            Some("internal"),
            None,
            None,
        ),
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let current = state
        .services
        .workspace
        .list_workspace_resources()
        .await?
        .into_iter()
        .find(|record| record.id == resource_id && record.workspace_id == workspace_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, "resource.delete", &current).await?,
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            Some(&project_id),
            &input.tags,
        ),
        &request_id(&headers),
    )
    .await?;
    let record = state
        .services
        .workspace
        .create_project_resource(&project_id, &session.user_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn create_project_resource_folder(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceFolderInput>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(&session, "resource.upload", Some(&project_id), &[]),
        &request_id(&headers),
    )
    .await?;
    let records = state
        .services
        .workspace
        .create_project_resource_folder(&project_id, &session.user_id, input)
        .await?;
    Ok(Json(records))
}

pub(crate) async fn import_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<WorkspaceResourceImportInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let tags = input.tags.clone().unwrap_or_default();
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            Some(&project_id),
            &tags,
        ),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_project_resource(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn update_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let current = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?
        .into_iter()
        .find(|record| record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    let tags = input.tags.clone().unwrap_or_else(|| current.tags.clone());
    authorize_request(
        &state,
        &session,
        &capability_authorization_request(
            &session.user_id,
            "resource.update",
            current.project_id.as_deref(),
            Some("resource"),
            Some(&current.id),
            Some(&current.kind),
            &tags,
            Some("internal"),
            None,
            None,
        ),
        &request_id(&headers),
    )
    .await?;
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
    let session = authenticate_session(&state, &headers).await?;
    let current = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?
        .into_iter()
        .find(|record| record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, "resource.delete", &current).await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_project_resource(&project_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

