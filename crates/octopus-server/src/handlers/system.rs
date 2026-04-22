use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterprisePrincipalPayload {
    user_id: String,
    username: String,
    display_name: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseSessionSummaryPayload {
    session_id: String,
    token: String,
    workspace_id: String,
    client_app_id: String,
    status: String,
    created_at: u64,
    expires_at: Option<u64>,
    principal: EnterprisePrincipalPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemAuthStatusPayload {
    workspace: WorkspaceSummary,
    bootstrap_admin_required: bool,
    owner_ready: bool,
    session: Option<EnterpriseSessionSummaryPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseAuthSuccessPayload {
    session: EnterpriseSessionSummaryPayload,
    workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseLoginRequestPayload {
    client_app_id: String,
    username: String,
    password: String,
    workspace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterBootstrapAdminRequestPayload {
    client_app_id: String,
    username: String,
    display_name: String,
    password: String,
    confirm_password: String,
    avatar: AvatarUploadPayload,
    workspace_id: Option<String>,
    mapped_directory: Option<String>,
}

pub(crate) async fn healthcheck(
    State(state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    Ok(Json(build_healthcheck_status(&state)))
}

pub(crate) async fn system_bootstrap(
    State(state): State<ServerState>,
) -> Result<Json<octopus_core::SystemBootstrapStatus>, ApiError> {
    let mut payload = state.services.workspace.system_bootstrap().await?;
    payload.transport_security = state.transport_security.clone();
    Ok(Json(payload))
}

async fn build_enterprise_session_summary(
    state: &ServerState,
    session: &SessionRecord,
) -> Result<EnterpriseSessionSummaryPayload, ApiError> {
    let users = state.services.access_control.list_users().await?;
    let current_user = users
        .iter()
        .find(|user| user.id == session.user_id)
        .cloned()
        .ok_or_else(|| ApiError::from(AppError::not_found("session user")))?;
    let principal = EnterprisePrincipalPayload {
        user_id: current_user.id.clone(),
        username: current_user.username.clone(),
        display_name: current_user.display_name.clone(),
        status: current_user.status.clone(),
    };

    Ok(EnterpriseSessionSummaryPayload {
        session_id: session.id.clone(),
        token: session.token.clone(),
        workspace_id: session.workspace_id.clone(),
        client_app_id: session.client_app_id.clone(),
        status: session.status.clone(),
        created_at: session.created_at,
        expires_at: session.expires_at,
        principal,
    })
}


async fn audit_auth_event(
    state: &ServerState,
    actor_id: &str,
    action: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    let workspace_id = workspace_id_for_audit(state).await?;
    append_audit_event(
        state,
        &workspace_id,
        None,
        "auth",
        actor_id,
        action,
        "system-auth",
        outcome,
    )
    .await
}

async fn ensure_auth_attempt_allowed(
    state: &ServerState,
    workspace_id: &str,
    username: &str,
    headers: &HeaderMap,
) -> Result<String, ApiError> {
    let attempt_key = auth_rate_limit_key(workspace_id, username, headers);
    if let Some(locked_until) = check_auth_rate_limit(state, &attempt_key)? {
        let outcome = format!("locked-until:{locked_until}");
        audit_auth_event(state, username, "system.auth.locked", &outcome).await?;
        return Err(ApiError::from(AppError::auth(
            "authentication temporarily locked due to too many failed attempts",
        )));
    }
    Ok(attempt_key)
}

async fn record_auth_failure_event(
    state: &ServerState,
    attempt_key: &str,
    username: &str,
    action: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    let lock_until = record_auth_failure(state, attempt_key)?;
    audit_auth_event(state, username, action, outcome).await?;
    if let Some(locked_until) = lock_until {
        audit_auth_event(
            state,
            username,
            "system.auth.locked",
            &format!("locked-until:{locked_until}"),
        )
        .await?;
    }
    Ok(())
}


pub(crate) async fn system_auth_status(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<SystemAuthStatusPayload>, ApiError> {
    let bootstrap = state.services.workspace.system_bootstrap().await?;
    let session = match extract_bearer(&headers) {
        Some(token) => state.services.auth.lookup_session(&token).await?,
        None => None,
    };

    let session = match session {
        Some(session) => Some(build_enterprise_session_summary(&state, &session).await?),
        None => None,
    };

    Ok(Json(SystemAuthStatusPayload {
        workspace: bootstrap.workspace,
        bootstrap_admin_required: !bootstrap.owner_ready,
        owner_ready: bootstrap.owner_ready,
        session,
    }))
}

pub(crate) async fn system_auth_login(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseLoginRequestPayload>,
) -> Result<Json<EnterpriseAuthSuccessPayload>, ApiError> {
    let workspace_id = workspace_id_for_audit(&state).await?;
    let attempt_key =
        ensure_auth_attempt_allowed(&state, &workspace_id, &request.username, &headers).await?;
    let username = request.username.clone();
    let response = match state
        .services
        .auth
        .login(LoginRequest {
            client_app_id: request.client_app_id,
            username: request.username,
            password: request.password,
            workspace_id: request.workspace_id,
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            record_auth_failure_event(
                &state,
                &attempt_key,
                &username,
                "system.auth.login.failure",
                &error.to_string(),
            )
            .await?;
            return Err(ApiError::from(error));
        }
    };
    let recovered = clear_auth_failures(&state, &attempt_key)?;
    if recovered {
        audit_auth_event(
            &state,
            &response.session.user_id,
            "system.auth.recovered",
            "cleared",
        )
        .await?;
    }
    audit_auth_event(
        &state,
        &response.session.user_id,
        "system.auth.login.success",
        "success",
    )
    .await?;
    let session = build_enterprise_session_summary(&state, &response.session).await?;
    Ok(Json(EnterpriseAuthSuccessPayload {
        session,
        workspace: response.workspace,
    }))
}

pub(crate) async fn system_auth_bootstrap_admin(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RegisterBootstrapAdminRequestPayload>,
) -> Result<Json<EnterpriseAuthSuccessPayload>, ApiError> {
    let workspace_id = workspace_id_for_audit(&state).await?;
    let attempt_key =
        ensure_auth_attempt_allowed(&state, &workspace_id, &request.username, &headers).await?;
    let username = request.username.clone();
    let response = match state
        .services
        .auth
        .register_bootstrap_admin(RegisterBootstrapAdminRequest {
            client_app_id: request.client_app_id,
            username: request.username,
            display_name: request.display_name,
            password: request.password,
            confirm_password: request.confirm_password,
            avatar: request.avatar,
            workspace_id: request.workspace_id,
            mapped_directory: request.mapped_directory,
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            record_auth_failure_event(
                &state,
                &attempt_key,
                &username,
                "system.auth.bootstrap-admin.failure",
                &error.to_string(),
            )
            .await?;
            return Err(ApiError::from(error));
        }
    };
    let recovered = clear_auth_failures(&state, &attempt_key)?;
    if recovered {
        audit_auth_event(
            &state,
            &response.session.user_id,
            "system.auth.recovered",
            "cleared",
        )
        .await?;
    }
    audit_auth_event(
        &state,
        &response.session.user_id,
        "system.auth.bootstrap-admin.success",
        "success",
    )
    .await?;
    let session = build_enterprise_session_summary(&state, &response.session).await?;
    Ok(Json(EnterpriseAuthSuccessPayload {
        session,
        workspace: response.workspace,
    }))
}

pub(crate) async fn system_auth_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<EnterpriseSessionSummaryPayload>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        build_enterprise_session_summary(&state, &session).await?,
    ))
}
