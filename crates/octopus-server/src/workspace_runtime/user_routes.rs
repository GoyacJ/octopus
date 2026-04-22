use super::*;

pub(crate) async fn current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        state
            .services
            .workspace
            .current_user_profile(&session.user_id)
            .await?,
    ))
}

pub(crate) async fn update_current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentUserProfileRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
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
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        state
            .services
            .workspace
            .change_current_user_password(&session.user_id, request)
            .await?,
    ))
}

pub(crate) async fn inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::InboxItemRecord>>, ApiError> {
    let session =
        ensure_capability_session(&state, &headers, "inbox.view", None, Some("inbox"), None)
            .await?;
    Ok(Json(visible_inbox_items(
        &session.user_id,
        state.services.inbox.list_inbox().await?,
    )))
}
