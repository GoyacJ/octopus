use super::*;

pub(crate) fn build_cors_layer(transport_security: &str) -> CorsLayer {
    let allow_origin = if transport_security == "loopback" {
        AllowOrigin::predicate(|origin, _| is_allowed_loopback_origin(origin))
    } else {
        AllowOrigin::predicate(|_, _| false)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static(HEADER_LAST_EVENT_ID),
            HeaderName::from_static(HEADER_REQUEST_ID),
            HeaderName::from_static(HEADER_WORKSPACE_ID),
            HeaderName::from_static(HEADER_IDEMPOTENCY_KEY),
        ])
}

fn is_allowed_loopback_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };

    origin == "http://127.0.0.1"
        || origin.starts_with("http://127.0.0.1:")
        || origin == "http://localhost"
        || origin.starts_with("http://localhost:")
        || origin == "http://tauri.localhost"
        || origin.starts_with("http://tauri.localhost:")
        || origin == "http://[::1]"
        || origin.starts_with("http://[::1]:")
        || origin == "https://127.0.0.1"
        || origin.starts_with("https://127.0.0.1:")
        || origin == "https://localhost"
        || origin.starts_with("https://localhost:")
        || origin == "https://tauri.localhost"
        || origin.starts_with("https://tauri.localhost:")
        || origin == "https://[::1]"
        || origin.starts_with("https://[::1]:")
        || origin == "tauri://localhost"
}

pub(crate) fn request_id(headers: &HeaderMap) -> String {
    headers
        .get(HEADER_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("req-{}", timestamp_now()))
}

pub(crate) fn insert_request_id(response: &mut Response, request_id: &str) {
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response
            .headers_mut()
            .insert(header::HeaderName::from_static(HEADER_REQUEST_ID), value);
    }
}

pub(crate) fn idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_IDEMPOTENCY_KEY)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub(crate) fn last_event_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_LAST_EVENT_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub(crate) fn workspace_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_WORKSPACE_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub(crate) fn load_idempotent_response(
    state: &ServerState,
    scope: &str,
    request_id: &str,
) -> Result<Option<Response>, ApiError> {
    let cache = state.idempotency_cache.lock().map_err(|_| {
        ApiError::new(
            AppError::runtime("idempotency cache mutex poisoned"),
            request_id,
        )
    })?;
    let Some(body) = cache.get(scope).cloned() else {
        return Ok(None);
    };
    drop(cache);

    let mut response = Json(body).into_response();
    insert_request_id(&mut response, request_id);
    Ok(Some(response))
}

pub(crate) fn store_idempotent_response<T: serde::Serialize>(
    state: &ServerState,
    scope: &str,
    value: &T,
    request_id: &str,
) -> Result<(), ApiError> {
    let payload = serde_json::to_value(value)
        .map_err(|error| ApiError::new(AppError::Json(error), request_id))?;
    let mut cache = state.idempotency_cache.lock().map_err(|_| {
        ApiError::new(
            AppError::runtime("idempotency cache mutex poisoned"),
            request_id,
        )
    })?;
    cache.insert(scope.to_string(), payload);
    Ok(())
}

pub(crate) fn idempotency_scope(
    session: &SessionRecord,
    operation: &str,
    resource: &str,
    key: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        session.workspace_id,
        session.user_id,
        operation,
        format!("{resource}:{key}")
    )
}

pub(crate) fn accepts_sse(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/event-stream"))
        .unwrap_or(false)
}
