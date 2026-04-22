use super::*;

pub(crate) async fn workspace_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    let session =
        ensure_capability_session(&state, &headers, "pet.view", None, Some("pet"), None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_pet_snapshot(&session.user_id)
            .await?,
    ))
}

pub(crate) async fn workspace_pet_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<PetDashboardSummary>, ApiError> {
    let session =
        ensure_capability_session(&state, &headers, "pet.view", None, Some("pet"), None).await?;
    let snapshot = state
        .services
        .workspace
        .get_workspace_pet_snapshot(&session.user_id)
        .await?;
    let request_id = request_id(&headers);

    let mut resource_count = 0_u64;
    for record in state.services.workspace.list_workspace_resources().await? {
        if record.project_id.is_some() {
            continue;
        }
        if authorize_request(
            &state,
            &session,
            &resource_authorization_request(&state, &session, "resource.view", &record).await?,
            &request_id,
        )
        .await
        .is_ok()
            && resource_visibility_allows(&session, &record)
        {
            resource_count += 1;
        }
    }

    let mut knowledge_count = 0_u64;
    for record in state.services.workspace.list_workspace_knowledge().await? {
        if record.project_id.is_some() {
            continue;
        }
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
            knowledge_count += 1;
        }
    }

    let reminder_count =
        visible_inbox_items(&session.user_id, state.services.inbox.list_inbox().await?)
            .into_iter()
            .filter(|item| item.status == "pending")
            .count() as u64;
    let has_home_binding = snapshot.binding.is_some();
    let has_home_session = snapshot
        .binding
        .as_ref()
        .and_then(|binding| binding.session_id.as_ref())
        .is_some();
    let last_interaction_at = (snapshot.presence.last_interaction_at > 0)
        .then_some(snapshot.presence.last_interaction_at);

    Ok(Json(PetDashboardSummary {
        pet_id: snapshot.profile.id,
        workspace_id: snapshot.workspace_id,
        owner_user_id: snapshot.owner_user_id,
        species: snapshot.profile.species,
        mood: snapshot.profile.mood,
        active_conversation_count: if has_home_binding { 1 } else { 0 },
        knowledge_count,
        memory_count: if has_home_session { 1 } else { 0 },
        reminder_count,
        resource_count,
        last_interaction_at,
    }))
}

pub(crate) async fn project_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "pet.view",
        Some(&project_id),
        Some("pet"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_project_pet_snapshot(&session.user_id, &project_id)
            .await?,
    ))
}

pub(crate) async fn save_workspace_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    let session =
        ensure_capability_session(&state, &headers, "pet.manage", None, Some("pet"), None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_workspace_pet_presence(&session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn save_project_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "pet.manage",
        Some(&project_id),
        Some("pet"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_project_pet_presence(&session.user_id, &project_id, input)
            .await?,
    ))
}

pub(crate) async fn bind_workspace_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    let session =
        ensure_capability_session(&state, &headers, "pet.manage", None, Some("pet"), None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_workspace_pet_conversation(&session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn bind_project_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "pet.manage",
        Some(&project_id),
        Some("pet"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_project_pet_conversation(&session.user_id, &project_id, input)
            .await?,
    ))
}
