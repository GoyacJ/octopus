use super::*;

pub(crate) async fn create_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        None,
        Some("project"),
        None,
    )
    .await?;
    let request = validate_create_project_request(request)?;
    validate_create_project_leader(&state, &request).await?;
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
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    let project = ensure_project_owner_session(&state, &headers, &project_id).await?;
    let request = validate_update_project_request(request)?;
    validate_updated_project_leader(&state, &project, &request).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_project(&project_id, request)
            .await?,
    ))
}

pub(crate) async fn list_project_promotion_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectPromotionRequest>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner_session(&state, &headers, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_promotion_requests(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_project_promotion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateProjectPromotionRequestInput>,
) -> Result<Json<ProjectPromotionRequest>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_project_promotion_request(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn list_project_deletion_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectDeletionRequest>>, ApiError> {
    ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_deletion_requests(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_project_deletion_request(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn approve_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(input): Json<ReviewProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    let reviewed = state
        .services
        .workspace
        .review_project_deletion_request(&request_id, &session.user_id, true, input)
        .await?;
    if reviewed.project_id != project_id {
        return Err(ApiError::from(AppError::not_found(
            "project deletion request not found",
        )));
    }
    Ok(Json(reviewed))
}

pub(crate) async fn reject_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(input): Json<ReviewProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    let reviewed = state
        .services
        .workspace
        .review_project_deletion_request(&request_id, &session.user_id, false, input)
        .await?;
    if reviewed.project_id != project_id {
        return Err(ApiError::from(AppError::not_found(
            "project deletion request not found",
        )));
    }
    Ok(Json(reviewed))
}

pub(crate) async fn delete_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
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
    ensure_project_owner_session(&state, &headers, &project_id).await?;
    state.services.workspace.delete_project(&project_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
