use super::*;

pub(crate) async fn list_runtime_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::RuntimeSessionSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        None,
        Some("runtime.session"),
        None,
    )
    .await?;
    Ok(Json(state.services.runtime_session.list_sessions().await?))
}

pub(crate) async fn create_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::CreateRuntimeSessionInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = input
        .project_id
        .as_deref()
        .and_then(normalize_project_scope)
        .map(str::to_string);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.create_session",
            &input.conversation_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let input = octopus_core::CreateRuntimeSessionInput {
        project_id: project_id.clone(),
        ..input
    };
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, project_id.as_deref()).await?;

    let detail = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(input, &session.user_id, Some(&owner_permission_ceiling))
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&detail, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&detail, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn run_runtime_generation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(mut input): Json<RunRuntimeGenerationInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    normalize_runtime_generation_input(&mut input)?;
    let project_id = input
        .project_id
        .as_deref()
        .and_then(normalize_project_scope)
        .map(str::to_string);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.submit_turn",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let result: RuntimeGenerationResult = state
        .services
        .runtime_execution
        .run_generation(
            RunRuntimeGenerationInput {
                project_id,
                ..input
            },
            &session.user_id,
        )
        .await?;
    let payload = runtime_transport_payload(&result, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(super) async fn derive_runtime_owner_permission_ceiling(
    state: &ServerState,
    session: &SessionRecord,
    project_id: Option<&str>,
) -> Result<String, ApiError> {
    let workspace = state.services.workspace.workspace_summary().await?;
    let workspace_owner = workspace.owner_user_id.as_deref();

    let Some(project_id) = project_id else {
        return Ok(if workspace_owner == Some(session.user_id.as_str()) {
            octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS.into()
        } else {
            octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE.into()
        });
    };

    let project = lookup_project(state, project_id).await?;
    if workspace_owner == Some(session.user_id.as_str()) || project.owner_user_id == session.user_id
    {
        return Ok(octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS.into());
    }
    if !project
        .member_user_ids
        .iter()
        .any(|user_id| user_id == &session.user_id)
    {
        return Ok(octopus_core::RUNTIME_PERMISSION_READ_ONLY.into());
    }
    if resolve_project_module_permission(&workspace, &project, "tools") == "deny" {
        return Ok(octopus_core::RUNTIME_PERMISSION_READ_ONLY.into());
    }
    Ok(octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE.into())
}

pub(crate) async fn get_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        Some("runtime.session"),
        Some(&session_id),
    )
    .await?;
    let detail = state
        .services
        .runtime_session
        .get_session(&session_id)
        .await?;
    let payload = runtime_transport_payload(&detail, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn delete_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        Some("runtime.session"),
        Some(&session_id),
    )
    .await?;
    state
        .services
        .runtime_session
        .delete_session(&session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

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

pub(crate) async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;

    let replay_after = query.after.or_else(|| last_event_id(&headers));

    if !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?;
        let payload = runtime_transport_payload(&events, &request_id)?;
        let mut response = Json(payload).into_response();
        insert_request_id(&mut response, &request_id);
        return Ok(response);
    }

    let replay_events = if replay_after.is_some() {
        state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?
    } else {
        Vec::new()
    };
    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream_request_id = request_id.clone();
    let stream = stream! {
        for event in replay_events {
            if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                if let Ok(data) = serde_json::to_string(&payload) {
                    yield Ok::<Event, std::convert::Infallible>(
                        Event::default()
                            .event(event.event_type.clone())
                            .id(event.id.clone())
                            .data(data)
                    );
                }
            }
        }

        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                        if let Ok(data) = serde_json::to_string(&payload) {
                            yield Ok::<Event, std::convert::Infallible>(
                                Event::default()
                                    .event(event.event_type.clone())
                                    .id(event.id.clone())
                                    .data(data)
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

