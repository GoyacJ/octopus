use super::*;

pub(crate) async fn list_access_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessUserRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.read", None).await?;
    Ok(Json(state.services.access_control.list_users().await?))
}

pub(crate) async fn list_access_members(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessMemberSummary>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.read", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .list_member_summaries()
            .await?,
    ))
}

pub(crate) async fn create_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<AccessUserUpsertRequest>,
) -> Result<Json<AccessUserRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    Ok(Json(
        state.services.access_control.create_user(request).await?,
    ))
}

pub(crate) async fn update_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<AccessUserUpsertRequest>,
) -> Result<Json<AccessUserRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_user(&user_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    state.services.access_control.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn update_access_user_preset(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<AccessUserPresetUpdateRequest>,
) -> Result<Json<AccessMemberSummary>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    let summary = state
        .services
        .access_control
        .assign_user_preset(&user_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.users.update-preset",
        &audit_resource_label("access.user", Some(&user_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(summary))
}

pub(crate) async fn list_access_org_units(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrgUnitRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(state.services.access_control.list_org_units().await?))
}

pub(crate) async fn create_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<OrgUnitUpsertRequest>,
) -> Result<Json<OrgUnitRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_org_unit(request)
            .await?,
    ))
}

pub(crate) async fn update_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(org_unit_id): Path<String>,
    Json(request): Json<OrgUnitUpsertRequest>,
) -> Result<Json<OrgUnitRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_org_unit(&org_unit_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(org_unit_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_org_unit(&org_unit_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_positions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PositionRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(state.services.access_control.list_positions().await?))
}

pub(crate) async fn create_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<PositionUpsertRequest>,
) -> Result<Json<PositionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_position(request)
            .await?,
    ))
}

pub(crate) async fn update_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(position_id): Path<String>,
    Json(request): Json<PositionUpsertRequest>,
) -> Result<Json<PositionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_position(&position_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(position_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_position(&position_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_user_groups(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserGroupRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(
        state.services.access_control.list_user_groups().await?,
    ))
}

pub(crate) async fn create_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UserGroupUpsertRequest>,
) -> Result<Json<UserGroupRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_user_group(request)
            .await?,
    ))
}

pub(crate) async fn update_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(request): Json<UserGroupUpsertRequest>,
) -> Result<Json<UserGroupRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_user_group(&group_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_user_group(&group_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_user_org_assignments(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOrgAssignmentRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .list_user_org_assignments()
            .await?,
    ))
}

pub(crate) async fn upsert_access_user_org_assignment(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UserOrgAssignmentUpsertRequest>,
) -> Result<Json<UserOrgAssignmentRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .upsert_user_org_assignment(request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user_org_assignment(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((user_id, org_unit_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_user_org_assignment(&user_id, &org_unit_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
