use super::*;

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

pub(crate) async fn list_access_audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AccessAuditQuery>,
) -> Result<Json<AccessAuditListResponse>, ApiError> {
    const PAGE_SIZE: usize = 50;
    ensure_authorized_session(&state, &headers, "audit.read", None).await?;
    let mut items = state.services.observation.list_audit_records().await?;
    items.sort_by_key(|item| std::cmp::Reverse(item.created_at));
    if let Some(actor_id) = query.actor_id.as_deref() {
        items.retain(|record| record.actor_id == actor_id);
    }
    if let Some(action) = query.action.as_deref() {
        items.retain(|record| record.action == action);
    }
    if let Some(resource_type) = query.resource_type.as_deref() {
        items.retain(|record| {
            record.resource == resource_type
                || record
                    .resource
                    .strip_prefix(resource_type)
                    .is_some_and(|suffix| suffix.starts_with(':'))
        });
    }
    if let Some(outcome) = query.outcome.as_deref() {
        items.retain(|record| record.outcome == outcome);
    }
    if let Some(from) = query.from {
        items.retain(|record| record.created_at >= from);
    }
    if let Some(to) = query.to {
        items.retain(|record| record.created_at <= to);
    }
    if let Some(cursor) = query.cursor.as_deref() {
        items.retain(|record| record.created_at.to_string().as_str() < cursor);
    }
    let next_cursor = items
        .get(PAGE_SIZE)
        .map(|record| record.created_at.to_string());
    items.truncate(PAGE_SIZE);
    Ok(Json(AccessAuditListResponse { items, next_cursor }))
}
