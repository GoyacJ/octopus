use super::*;

pub(super) async fn build_access_experience_response(
    state: &ServerState,
    session: &SessionRecord,
) -> Result<AccessExperienceResponse, ApiError> {
    let authorization = build_current_authorization_snapshot(state, session).await?;
    let effective_permission_codes = authorization
        .effective_permission_codes
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let section_grants = build_access_section_grants(&effective_permission_codes);
    let snapshot = state
        .services
        .access_control
        .get_experience_snapshot()
        .await?;
    let summary = AccessExperienceSummary {
        experience_level: snapshot.experience_level.clone(),
        member_count: snapshot.member_count,
        has_org_structure: snapshot.has_org_structure,
        has_custom_roles: snapshot.has_custom_roles,
        has_advanced_policies: snapshot.has_advanced_policies,
        has_menu_governance: snapshot.has_menu_governance,
        has_resource_governance: snapshot.has_resource_governance,
        recommended_landing_section: recommended_access_section_for_snapshot(
            &snapshot,
            &section_grants,
        ),
    };

    Ok(AccessExperienceResponse {
        summary,
        section_grants,
        role_templates: build_access_role_templates(),
        role_presets: build_access_role_presets(),
        capability_bundles: build_access_capability_bundles(),
        counts: snapshot.counts,
    })
}

pub(super) async fn build_access_session_payloads(
    state: &ServerState,
    current_session_id: &str,
) -> Result<Vec<AccessSessionRecord>, ApiError> {
    let users = state
        .services
        .access_control
        .list_users()
        .await?
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();
    let mut sessions = state.services.auth.list_sessions().await?;
    sessions.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(sessions
        .into_iter()
        .filter_map(|session| {
            let user = users.get(&session.user_id)?;
            let current = session.id == current_session_id;
            Some(AccessSessionRecord {
                session_id: session.id,
                user_id: session.user_id,
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                client_app_id: session.client_app_id,
                status: session.status,
                created_at: session.created_at,
                expires_at: session.expires_at,
                current,
            })
        })
        .collect())
}

pub(super) fn precise_tool_resource_type(kind: &str) -> &'static str {
    match kind.trim() {
        "builtin" => "tool.builtin",
        "mcp" => "tool.mcp",
        _ => "tool.skill",
    }
}

pub(super) fn merge_protected_resource_descriptor(
    defaults: ProtectedResourceDescriptor,
    metadata_by_key: &HashMap<(String, String), ProtectedResourceDescriptor>,
) -> ProtectedResourceDescriptor {
    let Some(metadata) =
        metadata_by_key.get(&(defaults.resource_type.clone(), defaults.id.clone()))
    else {
        return defaults;
    };
    ProtectedResourceDescriptor {
        id: defaults.id,
        resource_type: defaults.resource_type,
        resource_subtype: metadata
            .resource_subtype
            .clone()
            .or(defaults.resource_subtype),
        name: defaults.name,
        project_id: metadata.project_id.clone().or(defaults.project_id),
        tags: if metadata.tags.is_empty() {
            defaults.tags
        } else {
            metadata.tags.clone()
        },
        classification: if metadata.classification.trim().is_empty() {
            defaults.classification
        } else {
            metadata.classification.clone()
        },
        owner_subject_type: metadata
            .owner_subject_type
            .clone()
            .or(defaults.owner_subject_type),
        owner_subject_id: metadata
            .owner_subject_id
            .clone()
            .or(defaults.owner_subject_id),
    }
}

pub(super) async fn build_access_protected_resource_descriptors(
    state: &ServerState,
) -> Result<Vec<ProtectedResourceDescriptor>, ApiError> {
    let metadata_by_key = state
        .services
        .access_control
        .list_protected_resources()
        .await?
        .into_iter()
        .map(|record| ((record.resource_type.clone(), record.id.clone()), record))
        .collect::<HashMap<_, _>>();
    let agents = state.services.workspace.list_agents().await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let tools = state.services.workspace.list_tools().await?;

    let mut descriptors = Vec::new();
    descriptors.extend(agents.into_iter().map(|agent| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: agent.id,
                resource_type: "agent".into(),
                resource_subtype: Some(agent.scope),
                name: agent.name,
                project_id: agent.project_id,
                tags: agent.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(resources.into_iter().map(|resource| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: resource.id,
                resource_type: "resource".into(),
                resource_subtype: Some(resource.kind),
                name: resource.name,
                project_id: resource.project_id,
                tags: resource.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(knowledge.into_iter().map(|entry| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: entry.id,
                resource_type: "knowledge".into(),
                resource_subtype: Some(entry.kind),
                name: entry.title,
                project_id: entry.project_id,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(tools.into_iter().map(|tool| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: tool.id,
                resource_type: precise_tool_resource_type(&tool.kind).into(),
                resource_subtype: Some(tool.kind),
                name: tool.name,
                project_id: None,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.sort_by(|left, right| {
        left.resource_type
            .cmp(&right.resource_type)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(descriptors)
}

pub(crate) async fn current_authorization(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AuthorizationSnapshot>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        build_current_authorization_snapshot(&state, &session).await?,
    ))
}

pub(crate) async fn get_access_experience(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AccessExperienceResponse>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        build_access_experience_response(&state, &session).await?,
    ))
}

pub(crate) async fn list_access_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessSessionRecord>>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.sessions.read", None).await?;
    Ok(Json(
        build_access_session_payloads(&state, &session.id).await?,
    ))
}

pub(crate) async fn revoke_current_access_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    state.services.auth.revoke_session(&session.id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke-current",
        &audit_resource_label("access.session", Some(&session.id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revoke_access_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.sessions.manage", None).await?;
    state.services.auth.revoke_session(&session_id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke",
        &audit_resource_label("access.session", Some(&session_id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revoke_access_user_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.sessions.manage", None).await?;
    if session.user_id == user_id {
        return Err(ApiError::from(AppError::invalid_input(
            "current user cannot revoke all active sessions through this route",
        )));
    }
    state.services.auth.revoke_user_sessions(&user_id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke-user",
        &audit_resource_label("access.user-sessions", Some(&user_id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
