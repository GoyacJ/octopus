use super::*;

pub(crate) async fn runtime_bootstrap(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::RuntimeBootstrap>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        None,
        Some("runtime.session"),
        None,
    )
    .await?;
    Ok(Json(state.services.runtime_session.bootstrap().await?))
}

pub(crate) async fn get_runtime_config(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.config.workspace.read",
        None,
        Some("runtime.config"),
        Some("workspace"),
    )
    .await?;
    Ok(Json(state.services.runtime_config.get_config().await?))
}

pub(crate) async fn validate_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.config.workspace.manage",
        None,
        Some("runtime.config"),
        Some("workspace"),
    )
    .await?;
    Ok(Json(
        state.services.runtime_config.validate_config(patch).await?,
    ))
}

pub(crate) async fn probe_runtime_configured_model_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<RuntimeConfiguredModelProbeInput>,
) -> Result<Json<RuntimeConfiguredModelProbeResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.config.workspace.manage",
        None,
        Some("runtime.config"),
        Some("workspace"),
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .probe_configured_model(input)
            .await?,
    ))
}

pub(crate) async fn save_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(scope): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.config.workspace.manage",
        None,
        Some("runtime.config"),
        Some(&scope),
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_config(&scope, patch)
            .await?,
    ))
}

pub(crate) async fn get_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.project.read",
        Some(&project_id),
        Some("runtime.config"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_project_config(&project_id, &session.user_id)
            .await?,
    ))
}

pub(crate) async fn validate_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.project.manage",
        Some(&project_id),
        Some("runtime.config"),
        Some(&project_id),
    )
    .await?;
    let project = ensure_project_owner(&state, &session, &project_id).await?;
    validate_project_runtime_leader(&state, &project, &patch).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn save_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.project.manage",
        Some(&project_id),
        Some("runtime.config"),
        Some(&project_id),
    )
    .await?;
    let project = ensure_project_owner(&state, &session, &project_id).await?;
    validate_project_runtime_leader(&state, &project, &patch).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn get_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.user.read",
        None,
        Some("runtime.config"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_user_config(&session.user_id)
            .await?,
    ))
}

pub(crate) async fn validate_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.user.manage",
        None,
        Some("runtime.config"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_user_config(&session.user_id, patch)
            .await?,
    ))
}

pub(crate) async fn save_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "runtime.config.user.manage",
        None,
        Some("runtime.config"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_user_config(&session.user_id, patch)
            .await?,
    ))
}
