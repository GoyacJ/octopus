use super::*;

pub(crate) async fn workspace_deliverables(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "artifact.view",
        None,
        Some("artifact"),
        None,
    )
    .await?;
    Ok(Json(state.services.artifact.list_artifacts().await?))
}

pub(crate) async fn get_deliverable_detail(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(deliverable_id): Path<String>,
) -> Result<Json<DeliverableDetail>, ApiError> {
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    Ok(Json(detail))
}

pub(crate) async fn list_deliverable_versions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(deliverable_id): Path<String>,
) -> Result<Json<Vec<DeliverableVersionSummary>>, ApiError> {
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_session
            .list_deliverable_versions(&deliverable_id)
            .await?,
    ))
}

pub(crate) async fn get_deliverable_version_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((deliverable_id, version)): Path<(String, u32)>,
) -> Result<Json<DeliverableVersionContent>, ApiError> {
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_session
            .get_deliverable_version_content(&deliverable_id, version)
            .await?,
    ))
}

pub(crate) async fn create_deliverable_version(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(deliverable_id): Path<String>,
    Json(input): Json<CreateDeliverableVersionInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "deliverable.create_version",
            &deliverable_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let updated = state
        .services
        .runtime_session
        .create_deliverable_version(&deliverable_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&updated, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&updated, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn promote_deliverable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(deliverable_id): Path<String>,
    Json(input): Json<PromoteDeliverableInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    authorize_request(
        &state,
        &session,
        &capability_authorization_request(
            &session.user_id,
            "knowledge.create",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("knowledge"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
        &request_id,
    )
    .await?;
    let knowledge_title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| detail.title.clone());
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "deliverable.promote", &deliverable_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let promoted = state
        .services
        .runtime_session
        .promote_deliverable(&deliverable_id, input)
        .await?;
    let payload = KnowledgeEntryRecord {
        id: promoted.promotion_knowledge_id.clone().ok_or_else(|| {
            ApiError::from(AppError::runtime(
                "deliverable promotion did not create knowledge",
            ))
        })?,
        workspace_id: promoted.workspace_id.clone(),
        project_id: optional_transport_project_id(&promoted.project_id),
        title: knowledge_title,
        scope: if promoted.project_id.trim().is_empty() {
            "workspace".into()
        } else {
            "project".into()
        },
        status: "active".into(),
        source_type: "artifact".into(),
        source_ref: promoted.id.clone(),
        updated_at: promoted.updated_at,
    };
    if let Some(scope) = idempotency_scope.as_deref() {
        let cached = runtime_transport_payload(&payload, &request_id)?;
        store_idempotent_response(&state, scope, &cached, &request_id)?;
    }

    let response_payload = runtime_transport_payload(&payload, &request_id)?;
    let mut response = Json(response_payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn fork_deliverable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(deliverable_id): Path<String>,
    Json(input): Json<ForkDeliverableInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let detail = state
        .services
        .runtime_session
        .get_deliverable_detail(&deliverable_id)
        .await?;
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            optional_transport_project_id(&detail.project_id).as_deref(),
            Some("artifact"),
            Some(&detail.id),
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let source_project_id = optional_transport_project_id(&detail.project_id);
    let target_project_id =
        resolved_fork_target_project_id(input.project_id.as_deref(), &detail.project_id);
    if target_project_id != source_project_id {
        if let Some(target_project_id) = target_project_id.as_deref() {
            authorize_request(
                &state,
                &session,
                &capability_authorization_request(
                    &session.user_id,
                    "project.view",
                    Some(target_project_id),
                    Some("project"),
                    Some(target_project_id),
                    None,
                    &[],
                    Some("internal"),
                    None,
                    None,
                ),
                &request_id,
            )
            .await?;
        }
    }
    let source_session = state
        .services
        .runtime_session
        .get_session(&detail.session_id)
        .await?;
    let selected_actor_ref = source_session.selected_actor_ref.trim().to_string();
    if selected_actor_ref.is_empty() {
        return Err(ApiError::from(AppError::invalid_input(
            "source deliverable session has no selected actor",
        )));
    }
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "deliverable.fork", &deliverable_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let configured_model_id = source_session
        .session_policy
        .selected_configured_model_id
        .trim()
        .to_string();
    let forked = state
        .services
        .runtime_session
        .create_session(
            CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: target_project_id,
                title: input
                    .title
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(detail.title.as_str())
                    .to_string(),
                session_kind: Some(source_session.summary.session_kind.clone()),
                selected_actor_ref,
                selected_configured_model_id: if configured_model_id.is_empty() {
                    None
                } else {
                    Some(configured_model_id)
                },
                execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
            },
            &session.user_id,
        )
        .await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let payload = deliverable_conversation_record(&workspace_id, &forked);
    if let Some(scope) = idempotency_scope.as_deref() {
        let cached = runtime_transport_payload(&payload, &request_id)?;
        store_idempotent_response(&state, scope, &cached, &request_id)?;
    }

    let response_payload = runtime_transport_payload(&payload, &request_id)?;
    let mut response = Json(response_payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::KnowledgeEntryRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in state.services.workspace.list_workspace_knowledge().await? {
        if authorize_request(
            &state,
            &session,
            &knowledge_authorization_request(&state, &session, "knowledge.view", &record).await?,
            &request_id,
        )
        .await
        .is_ok()
            && knowledge_visibility_allows(&session, &record)
        {
            visible.push(knowledge_entry_record(record));
        }
    }
    Ok(Json(visible))
}
