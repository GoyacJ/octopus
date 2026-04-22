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

pub(crate) async fn list_access_permission_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PermissionDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(default_permission_definitions()))
}

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
