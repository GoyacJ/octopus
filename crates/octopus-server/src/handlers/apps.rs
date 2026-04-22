use super::*;

pub(crate) async fn list_apps(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ClientAppRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.read", None).await?;
    Ok(Json(state.services.app_registry.list_apps().await?))
}

pub(crate) async fn register_app(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(app): Json<ClientAppRecord>,
) -> Result<Json<ClientAppRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.write", None).await?;
    Ok(Json(state.services.app_registry.register_app(app).await?))
}

