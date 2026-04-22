use super::*;

pub(crate) async fn list_access_menu_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(build_access_menu_definitions(&state).await?))
}

pub(crate) async fn list_access_feature_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FeatureDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    let menus = build_access_menu_definitions(&state).await?;
    Ok(Json(
        build_access_feature_definitions(&state, &menus).await?,
    ))
}

pub(crate) async fn list_access_menu_gate_results(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuGateResult>>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(
        build_current_authorization_snapshot(&state, &session)
            .await?
            .menu_gates,
    ))
}

pub(crate) async fn list_access_menu_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuPolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(
        state.services.access_control.list_menu_policies().await?,
    ))
}

pub(crate) async fn create_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateMenuPolicyRequest>,
) -> Result<Json<MenuPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    let record = state
        .services
        .access_control
        .upsert_menu_policy(
            &request.menu_id,
            MenuPolicyUpsertRequest {
                enabled: request.enabled,
                order: request.order,
                group: request.group,
                visibility: request.visibility,
            },
        )
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.create",
        &audit_resource_label("access.menu-policy", Some(&request.menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
    Json(request): Json<MenuPolicyUpsertRequest>,
) -> Result<Json<MenuPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    let record = state
        .services
        .access_control
        .upsert_menu_policy(&menu_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.update",
        &audit_resource_label("access.menu-policy", Some(&menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    state
        .services
        .access_control
        .delete_menu_policy(&menu_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.delete",
        &audit_resource_label("access.menu-policy", Some(&menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_protected_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProtectedResourceDescriptor>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        build_access_protected_resource_descriptors(&state).await?,
    ))
}

pub(crate) async fn upsert_access_protected_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(request): Json<ProtectedResourceMetadataUpsertRequest>,
) -> Result<Json<ProtectedResourceDescriptor>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let _ = build_access_protected_resource_descriptors(&state)
        .await?
        .into_iter()
        .find(|record| record.resource_type == resource_type && record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("protected resource")))?;
    state
        .services
        .access_control
        .upsert_protected_resource(&resource_type, &resource_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.protected-resources.update",
        &audit_resource_label(&resource_type, Some(&resource_id)),
        "success",
        None,
    )
    .await?;
    let record = build_access_protected_resource_descriptors(&state)
        .await?
        .into_iter()
        .find(|descriptor| {
            descriptor.resource_type == resource_type && descriptor.id == resource_id
        })
        .ok_or_else(|| ApiError::from(AppError::not_found("protected resource")))?;
    Ok(Json(record))
}
