use super::*;

pub(crate) async fn ensure_runtime_submit(
    state: &ServerState,
    headers: &HeaderMap,
    input: Option<&SubmitRuntimeTurnInput>,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let session = ensure_authorized_session_with_request_id(
        state,
        headers,
        "runtime.submit_turn",
        project_id,
        request_id,
    )
    .await?;
    if let Some(input) = input {
        if let Some(permission_mode) = input.permission_mode.as_deref() {
            if permission_mode.trim().is_empty() {
                return Err(ApiError::new(
                    AppError::invalid_input("permission mode must not be empty"),
                    request_id,
                ));
            }
        }
    }
    Ok(session)
}

pub(crate) fn normalize_runtime_submit_input(
    input: &mut SubmitRuntimeTurnInput,
) -> Result<(), ApiError> {
    input.permission_mode = input
        .permission_mode
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(permission_mode) = input.permission_mode.as_deref() {
        let normalized =
            normalize_runtime_permission_mode_label(permission_mode).ok_or_else(|| {
                ApiError::from(AppError::invalid_input(format!(
                    "unsupported permission mode: {permission_mode}"
                )))
            })?;
        input.permission_mode = Some(normalized.to_string());
    }
    input.recall_mode = input
        .recall_mode
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(recall_mode) = input.recall_mode.as_deref() {
        if !matches!(recall_mode, "default" | "skip") {
            return Err(ApiError::from(AppError::invalid_input(format!(
                "unsupported recall mode: {recall_mode}"
            ))));
        }
    }
    input.ignored_memory_ids = input
        .ignored_memory_ids
        .drain(..)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    input.memory_intent = input
        .memory_intent
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(())
}

pub(crate) fn normalize_runtime_generation_input(
    input: &mut RunRuntimeGenerationInput,
) -> Result<(), ApiError> {
    input.project_id = input
        .project_id
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    input.content = input.content.trim().to_string();
    if input.content.is_empty() {
        return Err(ApiError::from(AppError::invalid_input(
            "generation content must not be empty",
        )));
    }
    input.system_prompt = input
        .system_prompt
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(())
}

pub(crate) async fn runtime_project_scope(
    state: &ServerState,
    session_id: &str,
) -> Result<Option<String>, ApiError> {
    let detail = state
        .services
        .runtime_session
        .get_session(session_id)
        .await?;
    Ok(normalize_project_scope(&detail.summary.project_id).map(ToOwned::to_owned))
}

pub(crate) fn normalize_project_scope(project_id: &str) -> Option<&str> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id)
    }
}
