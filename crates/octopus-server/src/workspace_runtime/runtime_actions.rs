use super::*;

pub(crate) async fn submit_runtime_turn(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(mut input): Json<SubmitRuntimeTurnInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    normalize_runtime_submit_input(&mut input)?;
    let session = ensure_runtime_submit(
        &state,
        &headers,
        Some(&input),
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.submit_turn", &session_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .submit_turn(&session_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, approval_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeApprovalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.approval.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.resolve_approval", &approval_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_approval(&session_id, &approval_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_auth_challenge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, challenge_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeAuthChallengeInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.auth.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.resolve_auth_challenge",
            &challenge_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_auth_challenge(&session_id, &challenge_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn cancel_runtime_subrun(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, subrun_id)): Path<(String, String)>,
    Json(mut input): Json<CancelRuntimeSubrunInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    input.note = input
        .note
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.subrun.cancel",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.cancel_subrun", &subrun_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .cancel_subrun(&session_id, &subrun_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_memory_proposal(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, proposal_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeMemoryProposalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.approval.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.resolve_memory_proposal",
            &proposal_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_memory_proposal(&session_id, &proposal_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}
