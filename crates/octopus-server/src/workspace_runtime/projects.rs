use super::*;

pub(crate) async fn projects(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ProjectRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        None,
        Some("project"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.list_projects().await?))
}

pub(crate) fn validate_create_project_request(
    request: CreateProjectRequest,
) -> Result<CreateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }
    let resource_directory = request.resource_directory.trim();
    if resource_directory.is_empty() {
        return Err(AppError::invalid_input("project resource directory is required").into());
    }
    let leader_agent_id = match request.leader_agent_id {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err(
                    AppError::invalid_input("project leader agent id cannot be empty").into(),
                );
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(CreateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        resource_directory: resource_directory.into(),
        owner_user_id: request
            .owner_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        member_user_ids: request.member_user_ids.map(|values| {
            values
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        }),
        permission_overrides: request.permission_overrides,
        linked_workspace_assets: None,
        leader_agent_id,
        manager_user_id: request
            .manager_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        preset_code: request
            .preset_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        assignments: None,
    })
}

pub(crate) fn validate_update_project_request(
    request: UpdateProjectRequest,
) -> Result<UpdateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    let status = request.status.trim();
    if status != "active" && status != "archived" {
        return Err(AppError::invalid_input("project status must be active or archived").into());
    }
    let resource_directory = request.resource_directory.trim();
    if resource_directory.is_empty() {
        return Err(AppError::invalid_input("project resource directory is required").into());
    }
    let leader_agent_id = match request.leader_agent_id {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err(
                    AppError::invalid_input("project leader agent id cannot be empty").into(),
                );
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(UpdateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        status: status.into(),
        resource_directory: resource_directory.into(),
        owner_user_id: request
            .owner_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        member_user_ids: request.member_user_ids.map(|values| {
            values
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        }),
        permission_overrides: request.permission_overrides,
        linked_workspace_assets: None,
        leader_agent_id,
        manager_user_id: request
            .manager_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        preset_code: request
            .preset_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        assignments: None,
    })
}

pub(crate) async fn create_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        None,
        Some("project"),
        None,
    )
    .await?;
    let request = validate_create_project_request(request)?;
    validate_create_project_leader(&state, &request).await?;
    Ok(Json(
        state.services.workspace.create_project(request).await?,
    ))
}

pub(crate) async fn update_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    let project = ensure_project_owner_session(&state, &headers, &project_id).await?;
    let request = validate_update_project_request(request)?;
    validate_updated_project_leader(&state, &project, &request).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_project(&project_id, request)
            .await?,
    ))
}

pub(crate) async fn list_project_promotion_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectPromotionRequest>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner_session(&state, &headers, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_promotion_requests(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_project_promotion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateProjectPromotionRequestInput>,
) -> Result<Json<ProjectPromotionRequest>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_project_promotion_request(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn list_project_deletion_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectDeletionRequest>>, ApiError> {
    ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_deletion_requests(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_project_deletion_request(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn approve_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(input): Json<ReviewProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    let reviewed = state
        .services
        .workspace
        .review_project_deletion_request(&request_id, &session.user_id, true, input)
        .await?;
    if reviewed.project_id != project_id {
        return Err(ApiError::from(AppError::not_found(
            "project deletion request not found",
        )));
    }
    Ok(Json(reviewed))
}

pub(crate) async fn reject_project_deletion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(input): Json<ReviewProjectDeletionRequestInput>,
) -> Result<Json<ProjectDeletionRequest>, ApiError> {
    let session = ensure_project_delete_review_session(&state, &headers, &project_id).await?;
    let reviewed = state
        .services
        .workspace
        .review_project_deletion_request(&request_id, &session.user_id, false, input)
        .await?;
    if reviewed.project_id != project_id {
        return Err(ApiError::from(AppError::not_found(
            "project deletion request not found",
        )));
    }
    Ok(Json(reviewed))
}

pub(crate) async fn delete_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    ensure_project_owner_session(&state, &headers, &project_id).await?;
    state.services.workspace.delete_project(&project_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn project_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectDashboardSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;

    let project = lookup_project(&state, &project_id).await?;
    let runtime_document = load_project_runtime_document(&state, &project, None).await?;
    let project_scope = resolve_project_granted_scope(&state, &project, &runtime_document).await?;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_at));
    sessions.retain(|record| record.project_id == project_id);
    let conversations = sessions
        .iter()
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: project.workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id.clone(),
            title: record.title.clone(),
            status: record.status.clone(),
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview.clone(),
        })
        .collect::<Vec<_>>();

    let mut audit_records = state.services.observation.list_audit_records().await?;
    audit_records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
    audit_records.retain(|record| record.project_id.as_deref() == Some(project_id.as_str()));
    let recent_activity = audit_records
        .iter()
        .take(8)
        .map(workspace_activity_from_audit)
        .collect::<Vec<_>>();

    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let knowledge = state
        .services
        .workspace
        .list_project_knowledge(&project_id)
        .await?;
    let agents = project_scope.agents.clone();
    let teams = project_scope.teams.clone();
    let cost_entries = state
        .services
        .observation
        .list_cost_entries()
        .await?
        .into_iter()
        .filter(|record| {
            record.project_id.as_deref() == Some(project_id.as_str())
                && record.metric == "tokens"
                && record.amount > 0
        })
        .collect::<Vec<_>>();
    let session_details = load_project_session_details(&state, &sessions).await?;
    let tool_source_keys = project_scope.tool_source_keys.clone();
    let tool_ranking = build_tool_ranking(&session_details, &audit_records);
    let model_breakdown = build_model_breakdown(&cost_entries);
    let trend = build_dashboard_trend(&sessions, &session_details, &cost_entries, &audit_records);
    let users = state.services.access_control.list_users().await?;
    let user_stats = build_user_stats(&project, &users, &audit_records, &trend);
    let conversation_insights =
        build_conversation_insights(&sessions, &session_details, &audit_records);
    let used_tokens = state
        .services
        .observation
        .project_used_tokens(&project_id)
        .await?;
    let task_records = state.services.project_tasks.list_tasks(&project_id).await?;
    let recent_tasks = task_records
        .iter()
        .take(8)
        .map(task_summary_from_record)
        .collect::<Vec<_>>();
    let total_tokens =
        used_tokens.max(cost_entries.iter().map(|record| record.amount as u64).sum());
    let approval_count = session_details
        .values()
        .filter(|detail| detail.pending_mediation.is_some())
        .count() as u64
        + audit_records
            .iter()
            .filter(|record| is_mediation_activity(record))
            .count() as u64;
    let overview = ProjectDashboardSummary {
        member_count: project_member_ids(&project).len() as u64,
        active_user_count: user_stats
            .iter()
            .filter(|item| item.activity_count > 0)
            .count() as u64,
        agent_count: agents.len() as u64,
        team_count: teams.len() as u64,
        conversation_count: conversations.len() as u64,
        message_count: session_details
            .values()
            .map(|detail| detail.messages.len() as u64)
            .sum(),
        tool_call_count: tool_ranking.iter().map(|item| item.value).sum(),
        approval_count,
        resource_count: resources.len() as u64,
        knowledge_count: knowledge.len() as u64,
        tool_count: tool_source_keys.len() as u64,
        token_record_count: cost_entries.len() as u64,
        total_tokens,
        activity_count: audit_records.len() as u64,
        task_count: task_records.len() as u64,
        active_task_count: task_records
            .iter()
            .filter(|record| record.status == "running")
            .count() as u64,
        attention_task_count: task_records
            .iter()
            .filter(|record| record.view_status == "attention")
            .count() as u64,
        scheduled_task_count: task_records
            .iter()
            .filter(|record| record.schedule_spec.is_some())
            .count() as u64,
    };
    let resource_breakdown = vec![
        dashboard_breakdown_item("resources", "resources", resources.len() as u64, None),
        dashboard_breakdown_item("knowledge", "knowledge", knowledge.len() as u64, None),
        dashboard_breakdown_item("agents", "agents", agents.len() as u64, None),
        dashboard_breakdown_item("teams", "teams", teams.len() as u64, None),
        dashboard_breakdown_item(
            "tools",
            "tools",
            tool_source_keys.len() as u64,
            Some(tool_source_keys.join(", ")),
        ),
        dashboard_breakdown_item("sessions", "sessions", conversations.len() as u64, None),
    ];

    Ok(Json(ProjectDashboardSnapshot {
        project,
        metrics: vec![
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        overview,
        trend,
        user_stats,
        conversation_insights,
        tool_ranking,
        resource_breakdown,
        model_breakdown,
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity,
        recent_tasks,
        used_tokens,
    }))
}

pub(crate) async fn list_workspace_promotion_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectPromotionRequest>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "resource.publish",
        None,
        Some("resource"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_workspace_promotion_requests()
            .await?,
    ))
}

pub(crate) async fn review_project_promotion_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(input): Json<ReviewProjectPromotionRequestInput>,
) -> Result<Json<ProjectPromotionRequest>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "resource.publish",
        None,
        Some("resource"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .review_project_promotion_request(&request_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn lookup_project(
    state: &ServerState,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))
}

pub(crate) async fn ensure_project_owner(
    state: &ServerState,
    session: &SessionRecord,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    let project = lookup_project(state, project_id).await?;
    if project.owner_user_id != session.user_id {
        return Err(ApiError::from(AppError::auth(
            "project owner access is required",
        )));
    }
    Ok(project)
}

pub(crate) async fn ensure_project_owner_session(
    state: &ServerState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    let session = authenticate_session(state, headers).await?;
    ensure_project_owner(state, &session, project_id).await
}
