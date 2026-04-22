use super::*;

pub(crate) async fn workspace_id_for_audit(state: &ServerState) -> Result<String, ApiError> {
    Ok(state.services.workspace.workspace_summary().await?.id)
}

pub(crate) fn audit_resource_label(resource_type: &str, resource_id: Option<&str>) -> String {
    resource_id
        .map(|id| format!("{resource_type}:{id}"))
        .unwrap_or_else(|| resource_type.to_string())
}

pub(crate) async fn append_audit_event(
    state: &ServerState,
    workspace_id: &str,
    project_id: Option<String>,
    actor_type: &str,
    actor_id: &str,
    action: &str,
    resource: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    state
        .services
        .observation
        .append_audit(AuditRecord {
            id: format!("audit-{}", Uuid::new_v4()),
            workspace_id: workspace_id.to_string(),
            project_id,
            actor_type: actor_type.to_string(),
            actor_id: actor_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            outcome: outcome.to_string(),
            created_at: timestamp_now(),
        })
        .await?;
    Ok(())
}

pub(crate) async fn append_session_audit(
    state: &ServerState,
    session: &SessionRecord,
    action: &str,
    resource: &str,
    outcome: &str,
    project_id: Option<String>,
) -> Result<(), ApiError> {
    append_audit_event(
        state,
        &session.workspace_id,
        project_id,
        "user",
        &session.user_id,
        action,
        resource,
        outcome,
    )
    .await
}
