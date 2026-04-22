use super::*;

pub(crate) async fn authenticate_session(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    authenticate_session_with_request_id(state, headers, &request_id).await
}

pub(crate) async fn authenticate_session_with_request_id(
    state: &ServerState,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let token = extract_bearer(headers)
        .ok_or_else(|| ApiError::new(AppError::auth("missing bearer token"), request_id))?;
    let session = state
        .services
        .auth
        .lookup_session(&token)
        .await?
        .ok_or_else(|| ApiError::new(AppError::auth("invalid bearer token"), request_id))?;
    if let Some(workspace_id) = workspace_header(headers) {
        if workspace_id != session.workspace_id {
            return Err(ApiError::new(
                AppError::auth("workspace scope mismatch"),
                request_id,
            ));
        }
    }
    Ok(session)
}

pub(crate) async fn ensure_authorized_session(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    ensure_authorized_session_with_request_id(state, headers, capability, project_id, &request_id)
        .await
}

pub(crate) async fn ensure_authorized_session_with_request_id(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let session = authenticate_session_with_request_id(state, headers, request_id).await?;
    authorize_session(state, &session, capability, project_id, request_id).await?;
    Ok(session)
}

pub(crate) async fn authorize_session(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<(), ApiError> {
    authorize_request(
        state,
        session,
        &AuthorizationRequest {
            subject_id: session.user_id.clone(),
            capability: capability.into(),
            project_id: project_id.map(str::to_string),
            resource_type: None,
            resource_id: None,
            resource_subtype: None,
            tags: Vec::new(),
            classification: None,
            owner_subject_type: None,
            owner_subject_id: None,
        },
        request_id,
    )
    .await
}

pub(crate) async fn ensure_authorized_request(
    state: &ServerState,
    headers: &HeaderMap,
    authorization_request: &AuthorizationRequest,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    let session = authenticate_session_with_request_id(state, headers, &request_id).await?;
    authorize_request(state, &session, authorization_request, &request_id).await?;
    Ok(session)
}

pub(crate) async fn authorize_request(
    state: &ServerState,
    session: &SessionRecord,
    authorization_request: &AuthorizationRequest,
    request_id: &str,
) -> Result<(), ApiError> {
    let mut decision = state
        .services
        .authorization
        .authorize_request(session, authorization_request)
        .await?;
    if decision.allowed {
        if let Some(project_id) = authorization_request.project_id.as_deref() {
            if let Some(reason) = evaluate_project_authorization_denial(
                state,
                session,
                authorization_request,
                project_id,
            )
            .await?
            {
                decision.allowed = false;
                decision.reason = Some(reason);
            }
        }
    }
    if is_sensitive_capability(&authorization_request.capability) {
        let resource_type = authorization_request
            .resource_type
            .as_deref()
            .unwrap_or("authorization");
        let resource =
            audit_resource_label(resource_type, authorization_request.resource_id.as_deref());
        let outcome = if decision.allowed {
            "allowed".to_string()
        } else {
            format!(
                "denied:{}",
                decision
                    .reason
                    .clone()
                    .unwrap_or_else(|| "access denied".into())
            )
        };
        append_session_audit(
            state,
            session,
            &authorization_request.capability,
            &resource,
            &outcome,
            authorization_request.project_id.clone(),
        )
        .await?;
    }
    if !decision.allowed {
        return Err(ApiError::new(
            AppError::auth(decision.reason.unwrap_or_else(|| "access denied".into())),
            request_id,
        ));
    }
    Ok(())
}

async fn evaluate_project_authorization_denial(
    state: &ServerState,
    session: &SessionRecord,
    authorization_request: &AuthorizationRequest,
    project_id: &str,
) -> Result<Option<String>, ApiError> {
    let project = state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))?;

    if !project
        .member_user_ids
        .iter()
        .any(|user_id| user_id == &session.user_id)
    {
        return Ok(Some("project membership is required".into()));
    }

    let Some(module) = project_module_for_request(authorization_request) else {
        return Ok(None);
    };
    let workspace = state.services.workspace.workspace_summary().await?;
    if resolve_project_module_permission(&workspace, &project, module) == "deny" {
        return Ok(Some(format!(
            "project module {module} is not available for this project"
        )));
    }
    Ok(None)
}

pub(crate) fn project_module_for_request(
    authorization_request: &AuthorizationRequest,
) -> Option<&'static str> {
    if authorization_request.capability.starts_with("agent.")
        || authorization_request.capability.starts_with("team.")
    {
        return Some("agents");
    }
    if authorization_request.capability.starts_with("resource.") {
        return Some("resources");
    }
    if authorization_request.capability.starts_with("knowledge.") {
        return Some("knowledge");
    }
    if authorization_request.capability.starts_with("task.") {
        return Some("tasks");
    }
    if authorization_request.capability.starts_with("tool.") {
        return Some("tools");
    }

    match authorization_request.resource_type.as_deref() {
        Some("agent") | Some("team") => Some("agents"),
        Some("resource") => Some("resources"),
        Some("knowledge") => Some("knowledge"),
        Some("task") => Some("tasks"),
        Some(resource_type) if resource_type.starts_with("tool.") => Some("tools"),
        _ => None,
    }
}

pub(crate) fn resolve_project_module_permission<'a>(
    workspace: &'a WorkspaceSummary,
    project: &'a ProjectRecord,
    module: &'a str,
) -> &'a str {
    let default_value = match module {
        "agents" => workspace.project_default_permissions.agents.as_str(),
        "resources" => workspace.project_default_permissions.resources.as_str(),
        "tools" => workspace.project_default_permissions.tools.as_str(),
        "knowledge" => workspace.project_default_permissions.knowledge.as_str(),
        "tasks" => workspace.project_default_permissions.tasks.as_str(),
        _ => "allow",
    };
    let override_value = match module {
        "agents" => project.permission_overrides.agents.as_str(),
        "resources" => project.permission_overrides.resources.as_str(),
        "tools" => project.permission_overrides.tools.as_str(),
        "knowledge" => project.permission_overrides.knowledge.as_str(),
        "tasks" => project.permission_overrides.tasks.as_str(),
        _ => "inherit",
    };
    if override_value == "inherit" {
        default_value
    } else {
        override_value
    }
}


pub(crate) fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(ToOwned::to_owned)
}
