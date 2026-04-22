use super::*;

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

