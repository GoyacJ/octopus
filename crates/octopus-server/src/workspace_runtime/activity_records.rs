use super::*;

pub(crate) fn workspace_activity_from_audit(record: &AuditRecord) -> WorkspaceActivityRecord {
    WorkspaceActivityRecord {
        id: record.id.clone(),
        title: record.action.clone(),
        description: format!(
            "{} {} {}",
            record.actor_type, record.actor_id, record.outcome
        ),
        timestamp: record.created_at,
        actor_id: Some(record.actor_id.clone()),
        actor_type: Some(record.actor_type.clone()),
        resource: Some(record.resource.clone()),
        outcome: Some(record.outcome.clone()),
    }
}

pub(crate) async fn list_conversation_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<ConversationRecord>, ApiError> {
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_at));
    Ok(sessions
        .into_iter()
        .filter(|record| project_id.map(|id| record.project_id == id).unwrap_or(true))
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
        })
        .collect())
}

pub(crate) async fn list_activity_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<WorkspaceActivityRecord>, ApiError> {
    let mut records = state.services.observation.list_audit_records().await?;
    records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
    Ok(records
        .into_iter()
        .filter(|record| {
            project_id
                .map(|id| record.project_id.as_deref() == Some(id))
                .unwrap_or(true)
        })
        .map(|record| workspace_activity_from_audit(&record))
        .collect())
}
