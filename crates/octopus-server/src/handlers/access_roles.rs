use super::*;

pub(crate) async fn list_access_roles(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessRoleRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.read", None).await?;
    Ok(Json(state.services.access_control.list_roles().await?))
}

pub(crate) async fn create_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RoleUpsertRequest>,
) -> Result<Json<AccessRoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    Ok(Json(
        state.services.access_control.create_role(request).await?,
    ))
}

pub(crate) async fn update_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
    Json(request): Json<RoleUpsertRequest>,
) -> Result<Json<AccessRoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_role(&role_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    state.services.access_control.delete_role(&role_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_role_bindings(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RoleBindingRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state.services.access_control.list_role_bindings().await?,
    ))
}

pub(crate) async fn create_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RoleBindingUpsertRequest>,
) -> Result<Json<RoleBindingRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_role_binding(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.create",
        &audit_resource_label("access.role-binding", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
    Json(request): Json<RoleBindingUpsertRequest>,
) -> Result<Json<RoleBindingRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_role_binding(&binding_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.update",
        &audit_resource_label("access.role-binding", Some(&binding_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_role_binding(&binding_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.delete",
        &audit_resource_label("access.role-binding", Some(&binding_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_data_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DataPolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state.services.access_control.list_data_policies().await?,
    ))
}

pub(crate) async fn create_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<DataPolicyUpsertRequest>,
) -> Result<Json<DataPolicyRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_data_policy(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.create",
        &audit_resource_label("access.data-policy", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
    Json(request): Json<DataPolicyUpsertRequest>,
) -> Result<Json<DataPolicyRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_data_policy(&policy_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.update",
        &audit_resource_label("access.data-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_data_policy(&policy_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.delete",
        &audit_resource_label("access.data-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_resource_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ResourcePolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .list_resource_policies()
            .await?,
    ))
}

pub(crate) async fn create_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ResourcePolicyUpsertRequest>,
) -> Result<Json<ResourcePolicyRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_resource_policy(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.create",
        &audit_resource_label("access.resource-policy", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
    Json(request): Json<ResourcePolicyUpsertRequest>,
) -> Result<Json<ResourcePolicyRecord>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_resource_policy(&policy_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.update",
        &audit_resource_label("access.resource-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_resource_policy(&policy_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.delete",
        &audit_resource_label("access.resource-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
