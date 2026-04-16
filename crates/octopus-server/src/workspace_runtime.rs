use super::*;
use crate::dto_mapping::metric_record;
use octopus_core::{
    AuditRecord, AuthorizationRequest, CancelRuntimeSubrunInput, CapabilityManagementProjection,
    ConversationRecord, CostLedgerEntry, CreateDeliverableVersionInput,
    CreateProjectPromotionRequestInput, CreateRuntimeSessionInput, DeliverableDetail,
    DeliverableVersionContent, DeliverableVersionSummary, ExportWorkspaceAgentBundleInput,
    ExportWorkspaceAgentBundleResult, ForkDeliverableInput, KnowledgeEntryRecord,
    PetDashboardSummary, ProjectDashboardBreakdownItem, ProjectDashboardConversationInsight,
    ProjectDashboardRankingItem, ProjectDashboardSummary, ProjectDashboardTrendPoint,
    ProjectDashboardUserStat, ProjectPromotionRequest, ProjectTokenUsageRecord,
    PromoteDeliverableInput, ProtectedResourceDescriptor, ResolveRuntimeAuthChallengeInput,
    ResolveRuntimeMemoryProposalInput, ReviewProjectPromotionRequestInput, RuntimeMessage,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn strip_runtime_transport_escape_hatches(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("payload");
            if let Some(serde_json::Value::Object(checkpoint)) = map.get_mut("checkpoint") {
                checkpoint.remove("serializedSession");
                checkpoint.remove("compactionMetadata");
            }
            for child in map.values_mut() {
                strip_runtime_transport_escape_hatches(child);
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                strip_runtime_transport_escape_hatches(child);
            }
        }
        _ => {}
    }
}

fn runtime_transport_payload<T: serde::Serialize>(
    value: &T,
    request_id: &str,
) -> Result<serde_json::Value, ApiError> {
    let mut payload = serde_json::to_value(value)
        .map_err(|error| ApiError::new(AppError::Json(error), request_id))?;
    strip_runtime_transport_escape_hatches(&mut payload);
    Ok(payload)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceDirectoryBrowserQuery {
    path: Option<String>,
}

fn capability_authorization_request(
    subject_id: &str,
    capability: &str,
    project_id: Option<&str>,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    resource_subtype: Option<&str>,
    tags: &[String],
    classification: Option<&str>,
    owner_subject_type: Option<&str>,
    owner_subject_id: Option<&str>,
) -> AuthorizationRequest {
    AuthorizationRequest {
        subject_id: subject_id.into(),
        capability: capability.into(),
        project_id: project_id.map(str::to_string),
        resource_type: resource_type.map(str::to_string),
        resource_id: resource_id.map(str::to_string),
        resource_subtype: resource_subtype.map(str::to_string),
        tags: tags.to_vec(),
        classification: classification.map(str::to_string),
        owner_subject_type: owner_subject_type.map(str::to_string),
        owner_subject_id: owner_subject_id.map(str::to_string),
    }
}

fn optional_transport_project_id(project_id: &str) -> Option<String> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

fn resolved_fork_target_project_id(
    requested_project_id: Option<&str>,
    source_project_id: &str,
) -> Option<String> {
    requested_project_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| optional_transport_project_id(source_project_id))
}

fn deliverable_conversation_record(
    workspace_id: &str,
    detail: &octopus_core::RuntimeSessionDetail,
) -> ConversationRecord {
    ConversationRecord {
        id: detail.summary.conversation_id.clone(),
        workspace_id: workspace_id.to_string(),
        project_id: detail.summary.project_id.clone(),
        session_id: detail.summary.id.clone(),
        title: detail.summary.title.clone(),
        status: detail.summary.status.clone(),
        updated_at: detail.summary.updated_at,
        last_message_preview: detail.summary.last_message_preview.clone(),
    }
}

fn precise_tool_resource_type(kind: &str) -> &'static str {
    match kind.trim() {
        "builtin" => "tool.builtin",
        "mcp" => "tool.mcp",
        _ => "tool.skill",
    }
}

fn merge_protected_resource_descriptor(
    defaults: ProtectedResourceDescriptor,
    metadata: Option<&ProtectedResourceDescriptor>,
) -> ProtectedResourceDescriptor {
    let Some(metadata) = metadata else {
        return defaults;
    };
    ProtectedResourceDescriptor {
        id: defaults.id,
        resource_type: defaults.resource_type,
        resource_subtype: metadata
            .resource_subtype
            .clone()
            .or(defaults.resource_subtype),
        name: defaults.name,
        project_id: metadata.project_id.clone().or(defaults.project_id),
        tags: if metadata.tags.is_empty() {
            defaults.tags
        } else {
            metadata.tags.clone()
        },
        classification: if metadata.classification.trim().is_empty() {
            defaults.classification
        } else {
            metadata.classification.clone()
        },
        owner_subject_type: metadata
            .owner_subject_type
            .clone()
            .or(defaults.owner_subject_type),
        owner_subject_id: metadata
            .owner_subject_id
            .clone()
            .or(defaults.owner_subject_id),
    }
}

async fn protected_resource_metadata(
    state: &ServerState,
    resource_type: &str,
    resource_id: &str,
) -> Result<Option<ProtectedResourceDescriptor>, ApiError> {
    Ok(state
        .services
        .access_control
        .list_protected_resources()
        .await?
        .into_iter()
        .find(|record| record.resource_type == resource_type && record.id == resource_id))
}

fn authorization_request_from_descriptor(
    session: &SessionRecord,
    capability: &str,
    descriptor: ProtectedResourceDescriptor,
) -> AuthorizationRequest {
    capability_authorization_request(
        &session.user_id,
        capability,
        descriptor.project_id.as_deref(),
        Some(&descriptor.resource_type),
        Some(&descriptor.id),
        descriptor.resource_subtype.as_deref(),
        &descriptor.tags,
        Some(&descriptor.classification),
        descriptor.owner_subject_type.as_deref(),
        descriptor.owner_subject_id.as_deref(),
    )
}

async fn resource_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    record: &WorkspaceResourceRecord,
) -> Result<AuthorizationRequest, ApiError> {
    let descriptor = merge_protected_resource_descriptor(
        ProtectedResourceDescriptor {
            id: record.id.clone(),
            resource_type: "resource".into(),
            resource_subtype: Some(record.kind.clone()),
            name: record.name.clone(),
            project_id: record.project_id.clone(),
            tags: record.tags.clone(),
            classification: "internal".into(),
            owner_subject_type: Some("user".into()),
            owner_subject_id: Some(record.owner_user_id.clone()),
        },
        protected_resource_metadata(state, "resource", &record.id)
            .await?
            .as_ref(),
    );
    Ok(authorization_request_from_descriptor(
        session, capability, descriptor,
    ))
}

fn resource_input_authorization_request(
    session: &SessionRecord,
    capability: &str,
    project_id: Option<&str>,
    tags: &[String],
) -> AuthorizationRequest {
    capability_authorization_request(
        &session.user_id,
        capability,
        project_id,
        Some("resource"),
        None,
        None,
        tags,
        Some("internal"),
        None,
        None,
    )
}

fn resource_visibility_allows(session: &SessionRecord, record: &WorkspaceResourceRecord) -> bool {
    match record.visibility.as_str() {
        "private" => record.owner_user_id == session.user_id,
        _ => true,
    }
}

fn knowledge_visibility_allows(session: &SessionRecord, record: &KnowledgeRecord) -> bool {
    if record.scope == "personal" {
        return record.owner_user_id.as_deref() == Some(session.user_id.as_str());
    }

    match record.visibility.as_str() {
        "private" => record.owner_user_id.as_deref() == Some(session.user_id.as_str()),
        _ => true,
    }
}

fn knowledge_relevant_to_project_context(record: &KnowledgeRecord, project_id: &str) -> bool {
    record.project_id.as_deref() == Some(project_id)
        || matches!(record.scope.as_str(), "workspace" | "personal")
}

fn knowledge_entry_record(record: KnowledgeRecord) -> octopus_core::KnowledgeEntryRecord {
    octopus_core::KnowledgeEntryRecord {
        id: record.id,
        workspace_id: record.workspace_id,
        project_id: record.project_id,
        title: record.title,
        scope: record.scope,
        status: record.status,
        source_type: record.source_type,
        source_ref: record.source_ref,
        updated_at: record.updated_at,
    }
}

fn agent_visible_in_generic_catalog(record: &AgentRecord) -> bool {
    record.asset_role != "pet"
}

async fn ensure_visible_resource(
    state: &ServerState,
    headers: &HeaderMap,
    session: &SessionRecord,
    capability: &str,
    record: &WorkspaceResourceRecord,
) -> Result<(), ApiError> {
    authorize_request(
        state,
        session,
        &resource_authorization_request(state, session, capability, record).await?,
        &request_id(headers),
    )
    .await?;
    if resource_visibility_allows(session, record) {
        Ok(())
    } else {
        Err(AppError::not_found("resource not found").into())
    }
}

async fn knowledge_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    record: &KnowledgeRecord,
) -> Result<AuthorizationRequest, ApiError> {
    let descriptor = merge_protected_resource_descriptor(
        ProtectedResourceDescriptor {
            id: record.id.clone(),
            resource_type: "knowledge".into(),
            resource_subtype: Some(record.source_type.clone()),
            name: record.title.clone(),
            project_id: record.project_id.clone(),
            tags: Vec::new(),
            classification: "internal".into(),
            owner_subject_type: record.owner_user_id.as_ref().map(|_| "user".into()),
            owner_subject_id: record.owner_user_id.clone(),
        },
        protected_resource_metadata(state, "knowledge", &record.id)
            .await?
            .as_ref(),
    );
    Ok(authorization_request_from_descriptor(
        session, capability, descriptor,
    ))
}

async fn agent_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    record: &AgentRecord,
) -> Result<AuthorizationRequest, ApiError> {
    let descriptor = merge_protected_resource_descriptor(
        ProtectedResourceDescriptor {
            id: record.id.clone(),
            resource_type: "agent".into(),
            resource_subtype: Some(record.scope.clone()),
            name: record.name.clone(),
            project_id: record.project_id.clone(),
            tags: record.tags.clone(),
            classification: "internal".into(),
            owner_subject_type: record.owner_user_id.as_ref().map(|_| "user".into()),
            owner_subject_id: record.owner_user_id.clone(),
        },
        protected_resource_metadata(state, "agent", &record.id)
            .await?
            .as_ref(),
    );
    Ok(authorization_request_from_descriptor(
        session, capability, descriptor,
    ))
}

fn agent_input_authorization_request(
    session: &SessionRecord,
    capability: &str,
    input: &UpsertAgentInput,
    resource_id: Option<&str>,
) -> AuthorizationRequest {
    capability_authorization_request(
        &session.user_id,
        capability,
        input.project_id.as_deref(),
        Some("agent"),
        resource_id,
        Some(&input.scope),
        &input.tags,
        Some("internal"),
        None,
        None,
    )
}

async fn ensure_capability_session(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
) -> Result<SessionRecord, ApiError> {
    ensure_authorized_request(
        state,
        headers,
        &capability_authorization_request(
            "",
            capability,
            project_id,
            resource_type,
            resource_id,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await
}

async fn tool_record_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    record: &ToolRecord,
) -> Result<AuthorizationRequest, ApiError> {
    let resource_type = precise_tool_resource_type(&record.kind);
    let descriptor = merge_protected_resource_descriptor(
        ProtectedResourceDescriptor {
            id: record.id.clone(),
            resource_type: resource_type.into(),
            resource_subtype: Some(record.kind.clone()),
            name: record.name.clone(),
            project_id: None,
            tags: Vec::new(),
            classification: "internal".into(),
            owner_subject_type: None,
            owner_subject_id: None,
        },
        protected_resource_metadata(state, resource_type, &record.id)
            .await?
            .as_ref(),
    );
    Ok(authorization_request_from_descriptor(
        session, capability, descriptor,
    ))
}

async fn skill_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    skill_id: Option<&str>,
) -> Result<AuthorizationRequest, ApiError> {
    match skill_id {
        Some(skill_id) => {
            let descriptor = merge_protected_resource_descriptor(
                ProtectedResourceDescriptor {
                    id: skill_id.to_string(),
                    resource_type: "tool.skill".into(),
                    resource_subtype: Some("skill".into()),
                    name: skill_id.to_string(),
                    project_id: None,
                    tags: Vec::new(),
                    classification: "internal".into(),
                    owner_subject_type: None,
                    owner_subject_id: None,
                },
                protected_resource_metadata(state, "tool.skill", skill_id)
                    .await?
                    .as_ref(),
            );
            Ok(authorization_request_from_descriptor(
                session, capability, descriptor,
            ))
        }
        None => Ok(capability_authorization_request(
            &session.user_id,
            capability,
            None,
            Some("tool.skill"),
            None,
            Some("skill"),
            &[],
            Some("internal"),
            None,
            None,
        )),
    }
}

async fn mcp_server_authorization_request(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    server_name: Option<&str>,
) -> Result<AuthorizationRequest, ApiError> {
    match server_name {
        Some(server_name) => {
            let descriptor = merge_protected_resource_descriptor(
                ProtectedResourceDescriptor {
                    id: server_name.to_string(),
                    resource_type: "tool.mcp".into(),
                    resource_subtype: Some("mcp".into()),
                    name: server_name.to_string(),
                    project_id: None,
                    tags: Vec::new(),
                    classification: "internal".into(),
                    owner_subject_type: None,
                    owner_subject_id: None,
                },
                protected_resource_metadata(state, "tool.mcp", server_name)
                    .await?
                    .as_ref(),
            );
            Ok(authorization_request_from_descriptor(
                session, capability, descriptor,
            ))
        }
        None => Ok(capability_authorization_request(
            &session.user_id,
            capability,
            None,
            Some("tool.mcp"),
            None,
            Some("mcp"),
            &[],
            Some("internal"),
            None,
            None,
        )),
    }
}

pub(crate) async fn workspace(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::WorkspaceSummary>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "workspace.overview.read",
        None,
        Some("workspace"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.workspace_summary().await?))
}

pub(crate) async fn workspace_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceOverviewSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "workspace.overview.read",
        None,
        Some("workspace"),
        None,
    )
    .await?;

    let workspace = state.services.workspace.workspace_summary().await?;
    let projects = state.services.workspace.list_projects().await?;
    let conversations = list_conversation_records(&state, None).await?;
    let recent_activity = list_activity_records(&state, None).await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let agents = state.services.workspace.list_agents().await?;
    let project_token_usage = state
        .services
        .observation
        .list_project_token_usage()
        .await?;
    let project_token_usage = project_token_usage
        .into_iter()
        .filter_map(|record| {
            let project = projects
                .iter()
                .find(|project| project.id == record.project_id)?;
            Some(ProjectTokenUsageRecord {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
                used_tokens: record.used_tokens,
            })
        })
        .take(8)
        .collect();

    Ok(Json(WorkspaceOverviewSnapshot {
        workspace,
        metrics: vec![
            metric_record("projects", "Projects", projects.len()),
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        projects,
        project_token_usage,
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

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
        linked_workspace_assets: request.linked_workspace_assets,
        assignments: request.assignments,
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
        linked_workspace_assets: request.linked_workspace_assets,
        assignments: request.assignments,
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
    ensure_project_owner_session(&state, &headers, &project_id).await?;
    let request = validate_update_project_request(request)?;
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
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
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
    audit_records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
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
    let all_agents = state.services.workspace.list_agents().await?;
    let project_agent_ids = collect_project_agent_ids(&project);
    let agents = all_agents
        .into_iter()
        .filter(|record| {
            agent_visible_in_generic_catalog(record)
                && (record.project_id.as_deref() == Some(project_id.as_str())
                    || project_agent_ids.contains(&record.id))
        })
        .collect::<Vec<_>>();
    let team_links = state
        .services
        .workspace
        .list_project_team_links(&project_id)
        .await?;
    let project_team_ids = collect_project_team_ids(&project, &team_links);
    let teams = state
        .services
        .workspace
        .list_teams()
        .await?
        .into_iter()
        .filter(|record| {
            record.project_id.as_deref() == Some(project_id.as_str())
                || project_team_ids.contains(&record.id)
        })
        .collect::<Vec<_>>();
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
    let tool_source_keys = project_tool_source_keys(&project);
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
        used_tokens,
    }))
}

pub(crate) async fn workspace_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "resource.view",
            None,
            Some("resource"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in resources {
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
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn project_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "resource.view",
            Some(&project_id),
            Some("resource"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in resources {
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
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn project_deliverables(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "artifact.view",
            Some(&project_id),
            Some("artifact"),
            None,
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
            .workspace
            .list_project_deliverables(&project_id)
            .await?,
    ))
}

pub(crate) async fn create_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            input.project_id.as_deref(),
            &input.tags,
        ),
        &request_id(&headers),
    )
    .await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .create_workspace_resource(&workspace_id, &session.user_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn import_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<WorkspaceResourceImportInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let tags = input.tags.clone().unwrap_or_default();
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(&session, "resource.upload", None, &tags),
        &request_id(&headers),
    )
    .await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_resource(&workspace_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn get_resource_detail(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(record))
}

pub(crate) async fn get_resource_content(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<WorkspaceResourceContentDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_resource_content(&resource_id)
            .await?,
    ))
}

pub(crate) async fn list_resource_children(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceChildrenRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    ensure_visible_resource(&state, &headers, &session, "resource.view", &record).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_resource_children(&resource_id)
            .await?,
    ))
}

pub(crate) async fn promote_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<PromoteWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .get_resource_detail(&resource_id)
        .await?;
    let capability = if input.scope == "workspace" {
        "resource.publish"
    } else {
        "resource.update"
    };
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .promote_resource(&resource_id, input)
            .await?,
    ))
}

pub(crate) async fn list_workspace_filesystem_directories(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<WorkspaceDirectoryBrowserQuery>,
) -> Result<Json<WorkspaceDirectoryBrowserResponse>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        None,
        Some("project"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_directories(query.path.as_deref())
            .await?,
    ))
}

pub(crate) async fn update_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let current = state
        .services
        .workspace
        .list_workspace_resources()
        .await?
        .into_iter()
        .find(|record| record.id == resource_id && record.workspace_id == workspace_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    let tags = input.tags.clone().unwrap_or_else(|| current.tags.clone());
    authorize_request(
        &state,
        &session,
        &capability_authorization_request(
            &session.user_id,
            "resource.update",
            current.project_id.as_deref(),
            Some("resource"),
            Some(&current.id),
            Some(&current.kind),
            &tags,
            Some("internal"),
            None,
            None,
        ),
        &request_id(&headers),
    )
    .await?;
    let record = state
        .services
        .workspace
        .update_workspace_resource(&workspace_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let current = state
        .services
        .workspace
        .list_workspace_resources()
        .await?
        .into_iter()
        .find(|record| record.id == resource_id && record.workspace_id == workspace_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, "resource.delete", &current).await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_workspace_resource(&workspace_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn create_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            Some(&project_id),
            &input.tags,
        ),
        &request_id(&headers),
    )
    .await?;
    let record = state
        .services
        .workspace
        .create_project_resource(&project_id, &session.user_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn create_project_resource_folder(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceFolderInput>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(&session, "resource.upload", Some(&project_id), &[]),
        &request_id(&headers),
    )
    .await?;
    let records = state
        .services
        .workspace
        .create_project_resource_folder(&project_id, &session.user_id, input)
        .await?;
    Ok(Json(records))
}

pub(crate) async fn import_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<WorkspaceResourceImportInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let tags = input.tags.clone().unwrap_or_default();
    authorize_request(
        &state,
        &session,
        &resource_input_authorization_request(
            &session,
            "resource.upload",
            Some(&project_id),
            &tags,
        ),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_project_resource(&project_id, &session.user_id, input)
            .await?,
    ))
}

pub(crate) async fn update_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let current = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?
        .into_iter()
        .find(|record| record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    let tags = input.tags.clone().unwrap_or_else(|| current.tags.clone());
    authorize_request(
        &state,
        &session,
        &capability_authorization_request(
            &session.user_id,
            "resource.update",
            current.project_id.as_deref(),
            Some("resource"),
            Some(&current.id),
            Some(&current.kind),
            &tags,
            Some("internal"),
            None,
            None,
        ),
        &request_id(&headers),
    )
    .await?;
    let record = state
        .services
        .workspace
        .update_project_resource(&project_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let current = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?
        .into_iter()
        .find(|record| record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("resource not found")))?;
    authorize_request(
        &state,
        &session,
        &resource_authorization_request(&state, &session, "resource.delete", &current).await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_project_resource(&project_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn workspace_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "knowledge.view",
            None,
            Some("knowledge"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in knowledge {
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
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn project_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "knowledge.view",
            Some(&project_id),
            Some("knowledge"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let request_id = request_id(&headers);
    let mut visible = Vec::new();
    for record in knowledge {
        if !knowledge_relevant_to_project_context(&record, &project_id) {
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
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

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

    let reminder_count = state.services.inbox.list_inbox().await?.len() as u64;
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

pub(crate) async fn list_agents(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentRecord>>, ApiError> {
    let session = ensure_authorized_request(
        &state,
        &headers,
        &capability_authorization_request(
            "",
            "agent.view",
            None,
            Some("agent"),
            None,
            None,
            &[],
            Some("internal"),
            None,
            None,
        ),
    )
    .await?;
    let agents = state.services.workspace.list_agents().await?;
    let mut visible = Vec::new();
    for record in agents {
        if !agent_visible_in_generic_catalog(&record) {
            continue;
        }
        if authorize_request(
            &state,
            &session,
            &agent_authorization_request(&state, &session, "agent.view", &record).await?,
            &request_id(&headers),
        )
        .await
        .is_ok()
        {
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn create_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &agent_input_authorization_request(&session, "agent.edit", &input, None),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(state.services.workspace.create_agent(input).await?))
}

pub(crate) async fn preview_import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.import", None, Some("agent"), None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .preview_import_agent_bundle(input)
            .await?,
    ))
}

pub(crate) async fn import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundleInput>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.import", None, Some("agent"), None).await?;
    Ok(Json(
        state.services.workspace.import_agent_bundle(input).await?,
    ))
}

pub(crate) async fn copy_workspace_agent_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        None,
        Some("agent"),
        Some(&agent_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_agent_from_builtin(&agent_id)
            .await?,
    ))
}

pub(crate) async fn export_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(&state, &headers, "agent.export", None, Some("agent"), None).await?;
    Ok(Json(
        state.services.workspace.export_agent_bundle(input).await?,
    ))
}

pub(crate) async fn preview_import_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .preview_import_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn import_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ImportWorkspaceAgentBundleInput>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn copy_project_agent_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, agent_id)): Path<(String, String)>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.import",
        Some(&project_id),
        Some("agent"),
        Some(&agent_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_project_agent_from_builtin(&project_id, &agent_id)
            .await?,
    ))
}

pub(crate) async fn export_project_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ExportWorkspaceAgentBundleInput>,
) -> Result<Json<ExportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "agent.export",
        Some(&project_id),
        Some("agent"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .export_project_agent_bundle(&project_id, input)
            .await?,
    ))
}

pub(crate) async fn update_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &agent_input_authorization_request(&session, "agent.edit", &input, Some(&agent_id)),
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_agent(&agent_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let agent = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .find(|record| record.id == agent_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("agent not found")))?;
    authorize_request(
        &state,
        &session,
        &agent_authorization_request(&state, &session, "agent.delete", &agent).await?,
        &request_id(&headers),
    )
    .await?;
    state.services.workspace.delete_agent(&agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_teams(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamRecord>>, ApiError> {
    ensure_capability_session(&state, &headers, "team.view", None, Some("team"), None).await?;
    Ok(Json(state.services.workspace.list_teams().await?))
}

pub(crate) async fn create_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        input.project_id.as_deref(),
        Some("team"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.create_team(input).await?))
}

pub(crate) async fn update_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        input.project_id.as_deref(),
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_team(&team_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.manage",
        None,
        Some("team"),
        Some(&team_id),
    )
    .await?;
    state.services.workspace.delete_team(&team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn copy_workspace_team_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.import",
        None,
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_team_from_builtin(&team_id)
            .await?,
    ))
}

pub(crate) async fn copy_project_team_from_builtin_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, team_id)): Path<(String, String)>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "team.import",
        Some(&project_id),
        Some("team"),
        Some(&team_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_project_team_from_builtin(&project_id, &team_id)
            .await?,
    ))
}

pub(crate) async fn list_project_agent_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectAgentLinkRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_agent_links(&project_id)
            .await?,
    ))
}

pub(crate) async fn link_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectAgentLinkInput>,
) -> Result<Json<ProjectAgentLinkRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_agent(input).await?,
    ))
}

pub(crate) async fn unlink_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, agent_id)): Path<(String, String)>,
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
    state
        .services
        .workspace
        .unlink_project_agent(&project_id, &agent_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_project_team_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectTeamLinkRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_team_links(&project_id)
            .await?,
    ))
}

pub(crate) async fn link_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectTeamLinkInput>,
) -> Result<Json<ProjectTeamLinkRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.manage",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_team(input).await?,
    ))
}

pub(crate) async fn unlink_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, team_id)): Path<(String, String)>,
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
    state
        .services
        .workspace
        .unlink_project_team(&project_id, &team_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn workspace_catalog_models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelCatalogSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.view",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state.services.runtime_registry.catalog_snapshot().await?,
    ))
}

pub(crate) async fn workspace_provider_credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderCredentialRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "provider-credential.view",
        None,
        Some("provider-credential"),
        None,
    )
    .await?;
    Ok(Json(
        state.services.workspace.list_provider_credentials().await?,
    ))
}

pub(crate) async fn workspace_capability_management_projection(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<CapabilityManagementProjection>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.view",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_capability_management_projection()
            .await?,
    ))
}

pub(crate) async fn workspace_capability_asset_disable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<CapabilityAssetDisablePatch>,
) -> Result<Json<CapabilityManagementProjection>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "tool.catalog.manage",
        None,
        Some("tool.catalog"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .set_capability_asset_disabled(patch)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill(&skill_id)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_tree_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillTreeDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_tree(&skill_id)
            .await?,
    ))
}

pub(crate) async fn get_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.view", Some(skill_id.as_str()))
            .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_file(&skill_id, &relative_path)
            .await?,
    ))
}

pub(crate) async fn create_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_skill(input)
            .await?,
    ))
}

pub(crate) async fn import_workspace_skill_archive_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillArchiveInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_archive(input)
            .await?,
    ))
}

pub(crate) async fn import_workspace_skill_folder_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillFolderInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(&state, &session, "tool.skill.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_folder(input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<UpdateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill(&skill_id, input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceSkillFileInput>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill_file(&skill_id, &relative_path, input)
            .await?,
    ))
}

pub(crate) async fn copy_workspace_skill_to_managed_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<CopyWorkspaceSkillToManagedInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.configure",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_skill_to_managed(&skill_id, input)
            .await?,
    ))
}

pub(crate) async fn delete_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &skill_authorization_request(
            &state,
            &session,
            "tool.skill.delete",
            Some(skill_id.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_workspace_skill(&skill_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn get_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.view",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_mcp_server(&server_name)
            .await?,
    ))
}

pub(crate) async fn create_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(&state, &session, "tool.mcp.configure", None).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_mcp_server(input)
            .await?,
    ))
}

pub(crate) async fn update_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.configure",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_mcp_server(&server_name, input)
            .await?,
    ))
}

pub(crate) async fn delete_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.delete",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    state
        .services
        .workspace
        .delete_workspace_mcp_server(&server_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn copy_workspace_mcp_server_to_managed_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    authorize_request(
        &state,
        &session,
        &mcp_server_authorization_request(
            &state,
            &session,
            "tool.mcp.configure",
            Some(server_name.as_str()),
        )
        .await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_mcp_server_to_managed(&server_name)
            .await?,
    ))
}

pub(crate) async fn list_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ToolRecord>>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let records = state.services.workspace.list_tools().await?;
    let mut visible = Vec::new();
    for record in records {
        let capability = format!("{}.view", precise_tool_resource_type(&record.kind));
        if authorize_request(
            &state,
            &session,
            &tool_record_authorization_request(&state, &session, &capability, &record).await?,
            &request_id(&headers),
        )
        .await
        .is_ok()
        {
            visible.push(record);
        }
    }
    Ok(Json(visible))
}

pub(crate) async fn create_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let capability = format!("{}.configure", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(state.services.workspace.create_tool(record).await?))
}

pub(crate) async fn update_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let capability = format!("{}.configure", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_tool(&tool_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let record = state
        .services
        .workspace
        .list_tools()
        .await?
        .into_iter()
        .find(|item| item.id == tool_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("tool not found")))?;
    let capability = format!("{}.delete", precise_tool_resource_type(&record.kind));
    authorize_request(
        &state,
        &session,
        &tool_record_authorization_request(&state, &session, &capability, &record).await?,
        &request_id(&headers),
    )
    .await?;
    state.services.workspace.delete_tool(&tool_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_automations(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AutomationRecord>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "automation.view",
        None,
        Some("automation"),
        None,
    )
    .await?;
    Ok(Json(state.services.workspace.list_automations().await?))
}

pub(crate) async fn create_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "automation.manage",
        record.project_id.as_deref(),
        Some("automation"),
        None,
    )
    .await?;
    Ok(Json(
        state.services.workspace.create_automation(record).await?,
    ))
}

pub(crate) async fn update_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "automation.manage",
        record.project_id.as_deref(),
        Some("automation"),
        Some(&automation_id),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_automation(&automation_id, record)
            .await?,
    ))
}

pub(crate) async fn delete_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "automation.manage",
        None,
        Some("automation"),
        Some(&automation_id),
    )
    .await?;
    state
        .services
        .workspace
        .delete_automation(&automation_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn update_current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentUserProfileRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_current_user_profile(&session.user_id, request)
            .await?,
    ))
}

pub(crate) async fn change_current_user_password_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ChangeCurrentUserPasswordRequest>,
) -> Result<Json<ChangeCurrentUserPasswordResponse>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        state
            .services
            .workspace
            .change_current_user_password(&session.user_id, request)
            .await?,
    ))
}

pub(crate) async fn inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::InboxItemRecord>>, ApiError> {
    ensure_capability_session(&state, &headers, "inbox.view", None, Some("inbox"), None).await?;
    Ok(Json(state.services.inbox.list_inbox().await?))
}

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
    ensure_project_owner(&state, &session, &project_id).await?;
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
    ensure_project_owner(&state, &session, &project_id).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
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

fn project_member_ids(project: &ProjectRecord) -> Vec<String> {
    let mut members = BTreeSet::new();
    members.insert(project.owner_user_id.clone());
    for user_id in &project.member_user_ids {
        if !user_id.trim().is_empty() {
            members.insert(user_id.clone());
        }
    }
    members.into_iter().collect()
}

fn collect_project_agent_ids(project: &ProjectRecord) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    ids.extend(project.linked_workspace_assets.agent_ids.iter().cloned());
    if let Some(assignments) = project
        .assignments
        .as_ref()
        .and_then(|value| value.agents.as_ref())
    {
        ids.extend(assignments.agent_ids.iter().cloned());
    }
    ids
}

fn collect_project_team_ids(
    project: &ProjectRecord,
    links: &[ProjectTeamLinkRecord],
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    ids.extend(links.iter().map(|record| record.team_id.clone()));
    if let Some(assignments) = project
        .assignments
        .as_ref()
        .and_then(|value| value.agents.as_ref())
    {
        ids.extend(assignments.team_ids.iter().cloned());
    }
    ids
}

fn project_tool_source_keys(project: &ProjectRecord) -> Vec<String> {
    let mut source_keys = BTreeSet::new();
    source_keys.extend(
        project
            .linked_workspace_assets
            .tool_source_keys
            .iter()
            .cloned(),
    );
    if let Some(assignments) = project
        .assignments
        .as_ref()
        .and_then(|value| value.tools.as_ref())
    {
        source_keys.extend(assignments.source_keys.iter().cloned());
    }
    source_keys.into_iter().collect()
}

fn workspace_activity_from_audit(record: &AuditRecord) -> WorkspaceActivityRecord {
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

async fn load_project_session_details(
    state: &ServerState,
    sessions: &[octopus_core::RuntimeSessionSummary],
) -> Result<HashMap<String, octopus_core::RuntimeSessionDetail>, ApiError> {
    let mut details = HashMap::new();
    for session in sessions {
        if let Ok(detail) = state
            .services
            .runtime_session
            .get_session(&session.id)
            .await
        {
            details.insert(session.id.clone(), detail);
        }
    }
    Ok(details)
}

fn usage_total_tokens(value: &serde_json::Value) -> Option<u64> {
    let direct = ["total_tokens", "totalTokens", "tokens"]
        .iter()
        .find_map(|key| value.get(key).and_then(serde_json::Value::as_u64));
    if direct.is_some() {
        return direct;
    }

    let input = value
        .get("input_tokens")
        .or_else(|| value.get("inputTokens"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let output = value
        .get("output_tokens")
        .or_else(|| value.get("outputTokens"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    (input > 0 || output > 0).then_some(input + output)
}

fn message_token_count(message: &RuntimeMessage) -> u64 {
    message
        .usage
        .as_ref()
        .and_then(usage_total_tokens)
        .unwrap_or(0)
}

fn tool_call_label(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(raw) => Some(raw.clone()),
        serde_json::Value::Object(_) => ["toolName", "tool_name", "name", "id"]
            .iter()
            .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
            .map(str::to_string),
        _ => None,
    }
}

fn is_mediation_activity(record: &AuditRecord) -> bool {
    let action = record.action.to_ascii_lowercase();
    let resource = record.resource.to_ascii_lowercase();
    action.contains("approval")
        || action.contains("auth")
        || resource.contains("approval")
        || resource.contains("auth")
}

fn build_bucket_timestamps(
    sessions: &[octopus_core::RuntimeSessionSummary],
    cost_entries: &[CostLedgerEntry],
    audit_records: &[AuditRecord],
    bucket_count: usize,
) -> (Vec<ProjectDashboardTrendPoint>, u64, u64) {
    let mut timestamps = sessions
        .iter()
        .map(|record| record.updated_at)
        .collect::<Vec<_>>();
    timestamps.extend(cost_entries.iter().map(|record| record.created_at));
    timestamps.extend(audit_records.iter().map(|record| record.created_at));

    let max_timestamp = timestamps.iter().copied().max().unwrap_or(0);
    let min_timestamp = timestamps.iter().copied().min().unwrap_or(max_timestamp);
    let span = max_timestamp.saturating_sub(min_timestamp);
    let step =
        ((span.max(bucket_count.saturating_sub(1) as u64)) / bucket_count.max(1) as u64).max(1);
    let start = max_timestamp.saturating_sub(step * bucket_count.saturating_sub(1) as u64);

    let buckets = (0..bucket_count)
        .map(|index| {
            let timestamp = start + step * index as u64;
            ProjectDashboardTrendPoint {
                id: format!("bucket-{index}"),
                label: timestamp.to_string(),
                timestamp,
                conversation_count: 0,
                message_count: 0,
                tool_call_count: 0,
                approval_count: 0,
                token_count: 0,
            }
        })
        .collect::<Vec<_>>();

    (buckets, start, step)
}

fn bucket_index(timestamp: u64, start: u64, step: u64, bucket_count: usize) -> usize {
    if bucket_count <= 1 {
        return 0;
    }
    let raw = timestamp.saturating_sub(start) / step.max(1);
    raw.min(bucket_count.saturating_sub(1) as u64) as usize
}

fn build_dashboard_trend(
    sessions: &[octopus_core::RuntimeSessionSummary],
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    cost_entries: &[CostLedgerEntry],
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardTrendPoint> {
    let bucket_count = 7;
    let (mut buckets, start, step) =
        build_bucket_timestamps(sessions, cost_entries, audit_records, bucket_count);

    for session in sessions {
        let index = bucket_index(session.updated_at, start, step, bucket_count);
        buckets[index].conversation_count += 1;
        if let Some(detail) = session_details.get(&session.id) {
            let mut session_tokens = 0_u64;
            for message in &detail.messages {
                let message_index = bucket_index(message.timestamp, start, step, bucket_count);
                let token_count = message_token_count(message);
                let tool_calls = message.tool_calls.as_ref().map_or(0, Vec::len) as u64;
                buckets[message_index].message_count += 1;
                buckets[message_index].tool_call_count += tool_calls;
                buckets[message_index].token_count += token_count;
                session_tokens += token_count;
            }
            if session_tokens == 0 {
                buckets[index].token_count += u64::from(detail.run.consumed_tokens.unwrap_or(0));
            }
            if detail.pending_mediation.is_some() {
                buckets[index].approval_count += 1;
            }
        }
    }

    for record in cost_entries {
        let index = bucket_index(record.created_at, start, step, bucket_count);
        buckets[index].token_count += record.amount.max(0) as u64;
    }

    for record in audit_records {
        if is_mediation_activity(record) {
            let index = bucket_index(record.created_at, start, step, bucket_count);
            buckets[index].approval_count += 1;
        }
    }

    buckets
}

fn build_conversation_insights(
    sessions: &[octopus_core::RuntimeSessionSummary],
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardConversationInsight> {
    let mut items = sessions
        .iter()
        .map(|session| {
            let detail = session_details.get(&session.id);
            let message_count = detail.map_or(0, |value| value.messages.len() as u64);
            let tool_call_count = detail.map_or(0, |value| {
                value
                    .messages
                    .iter()
                    .map(|message| message.tool_calls.as_ref().map_or(0, Vec::len) as u64)
                    .sum()
            });
            let token_count = detail.map_or(0, |value| {
                let total = value.messages.iter().map(message_token_count).sum::<u64>();
                if total > 0 {
                    total
                } else {
                    u64::from(value.run.consumed_tokens.unwrap_or(0))
                }
            });
            let approval_count = detail
                .and_then(|value| value.pending_mediation.as_ref())
                .map(|_| 1_u64)
                .unwrap_or(0)
                + audit_records
                    .iter()
                    .filter(|record| {
                        is_mediation_activity(record)
                            && (record.resource.contains(&session.id)
                                || record.resource.contains(&session.conversation_id))
                    })
                    .count() as u64;
            ProjectDashboardConversationInsight {
                id: session.id.clone(),
                conversation_id: session.conversation_id.clone(),
                title: session.title.clone(),
                status: session.status.clone(),
                updated_at: session.updated_at,
                last_message_preview: session.last_message_preview.clone(),
                message_count,
                tool_call_count,
                approval_count,
                token_count,
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .token_count
            .cmp(&left.token_count)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    items
}

fn build_tool_ranking(
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardRankingItem> {
    let mut counts = BTreeMap::<String, u64>::new();
    for detail in session_details.values() {
        for message in &detail.messages {
            for tool_call in message.tool_calls.clone().unwrap_or_default() {
                if let Some(label) = tool_call_label(&tool_call) {
                    *counts.entry(label).or_default() += 1;
                }
            }
        }
    }

    if counts.is_empty() {
        for record in audit_records {
            if record.resource.trim().is_empty() {
                continue;
            }
            *counts.entry(record.resource.clone()).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|(label, value)| ProjectDashboardRankingItem {
            id: label.to_ascii_lowercase().replace(' ', "-"),
            label,
            value,
            helper: None,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.label.cmp(&right.label))
    });
    rows.into_iter().take(8).collect()
}

fn build_model_breakdown(cost_entries: &[CostLedgerEntry]) -> Vec<ProjectDashboardBreakdownItem> {
    let mut grouped = BTreeMap::<String, u64>::new();
    for record in cost_entries {
        let key = record
            .configured_model_id
            .clone()
            .unwrap_or_else(|| "unassigned".into());
        *grouped.entry(key).or_default() += record.amount.max(0) as u64;
    }

    grouped
        .into_iter()
        .map(|(label, value)| dashboard_breakdown_item(&label, &label, value, None))
        .collect()
}

fn build_user_stats(
    project: &ProjectRecord,
    users: &[AccessUserRecord],
    audit_records: &[AuditRecord],
    trend: &[ProjectDashboardTrendPoint],
) -> Vec<ProjectDashboardUserStat> {
    let member_ids = project_member_ids(project);
    let mut display_names = users
        .iter()
        .map(|record| (record.id.clone(), record.display_name.clone()))
        .collect::<HashMap<_, _>>();
    for user_id in &member_ids {
        display_names
            .entry(user_id.clone())
            .or_insert_with(|| user_id.clone());
    }

    let mut stats = member_ids
        .iter()
        .map(|user_id| {
            (
                user_id.clone(),
                ProjectDashboardUserStat {
                    user_id: user_id.clone(),
                    display_name: display_names
                        .get(user_id)
                        .cloned()
                        .unwrap_or_else(|| user_id.clone()),
                    activity_count: 0,
                    conversation_count: 0,
                    message_count: 0,
                    tool_call_count: 0,
                    approval_count: 0,
                    token_count: 0,
                    activity_trend: vec![0; trend.len()],
                    token_trend: vec![0; trend.len()],
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let start = trend.first().map(|item| item.timestamp).unwrap_or(0);
    let step = if trend.len() > 1 {
        trend[1].timestamp.saturating_sub(trend[0].timestamp).max(1)
    } else {
        1
    };

    for record in audit_records {
        let Some(user_id) = Some(&record.actor_id) else {
            continue;
        };
        let Some(item) = stats.get_mut(user_id) else {
            continue;
        };
        let index = bucket_index(record.created_at, start, step, trend.len().max(1));
        item.activity_count += 1;
        item.activity_trend[index] += 1;
        if is_mediation_activity(record) {
            item.approval_count += 1;
        }
    }

    for (index, bucket) in trend.iter().enumerate() {
        let active_ids = stats
            .iter()
            .filter_map(|(user_id, item)| {
                (item.activity_trend[index] > 0).then_some(user_id.clone())
            })
            .collect::<Vec<_>>();
        let total_activity = active_ids
            .iter()
            .map(|user_id| {
                stats
                    .get(user_id)
                    .map_or(0, |item| item.activity_trend[index])
            })
            .sum::<u64>();

        if active_ids.is_empty() {
            if let Some(owner) = stats.get_mut(&project.owner_user_id) {
                owner.token_count += bucket.token_count;
                owner.token_trend[index] += bucket.token_count;
                owner.message_count += bucket.message_count;
                owner.tool_call_count += bucket.tool_call_count;
            }
            continue;
        }

        let fallback_user_id = active_ids.first().cloned();
        let mut remaining_tokens = bucket.token_count;
        let mut remaining_messages = bucket.message_count;
        let mut remaining_tools = bucket.tool_call_count;
        for user_id in &active_ids {
            let share = stats
                .get(user_id)
                .map_or(0, |item| item.activity_trend[index]);
            let denominator = total_activity.max(1);
            let token_share = bucket.token_count * share / denominator;
            let message_share = bucket.message_count * share / denominator;
            let tool_share = bucket.tool_call_count * share / denominator;
            if let Some(item) = stats.get_mut(user_id) {
                item.token_count += token_share;
                item.token_trend[index] += token_share;
                item.message_count += message_share;
                item.tool_call_count += tool_share;
            }
            remaining_tokens = remaining_tokens.saturating_sub(token_share);
            remaining_messages = remaining_messages.saturating_sub(message_share);
            remaining_tools = remaining_tools.saturating_sub(tool_share);
        }

        if let Some(user_id) = fallback_user_id {
            if let Some(item) = stats.get_mut(&user_id) {
                item.token_count += remaining_tokens;
                item.token_trend[index] += remaining_tokens;
                item.message_count += remaining_messages;
                item.tool_call_count += remaining_tools;
            }
        }
    }

    for item in stats.values_mut() {
        item.conversation_count = u64::from(item.activity_count > 0);
    }

    let mut rows = stats.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .token_count
            .cmp(&left.token_count)
            .then_with(|| right.activity_count.cmp(&left.activity_count))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    rows
}

fn dashboard_breakdown_item(
    id: &str,
    label: &str,
    value: u64,
    helper: Option<String>,
) -> ProjectDashboardBreakdownItem {
    ProjectDashboardBreakdownItem {
        id: id.into(),
        label: label.into(),
        value,
        helper,
    }
}

pub(crate) async fn list_conversation_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<ConversationRecord>, ApiError> {
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
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
    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
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

pub(crate) async fn list_runtime_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::RuntimeSessionSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        None,
        Some("runtime.session"),
        None,
    )
    .await?;
    Ok(Json(state.services.runtime_session.list_sessions().await?))
}

pub(crate) async fn create_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::CreateRuntimeSessionInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = input
        .project_id
        .as_deref()
        .and_then(normalize_project_scope)
        .map(str::to_string);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.create_session",
            &input.conversation_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let input = octopus_core::CreateRuntimeSessionInput {
        project_id: project_id.clone(),
        ..input
    };
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, project_id.as_deref()).await?;

    let detail = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(input, &session.user_id, Some(&owner_permission_ceiling))
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&detail, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&detail, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

async fn derive_runtime_owner_permission_ceiling(
    state: &ServerState,
    session: &SessionRecord,
    project_id: Option<&str>,
) -> Result<String, ApiError> {
    let workspace = state.services.workspace.workspace_summary().await?;
    let workspace_owner = workspace.owner_user_id.as_deref();

    let Some(project_id) = project_id else {
        return Ok(if workspace_owner == Some(session.user_id.as_str()) {
            octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS.into()
        } else {
            octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE.into()
        });
    };

    let project = lookup_project(state, project_id).await?;
    if workspace_owner == Some(session.user_id.as_str()) || project.owner_user_id == session.user_id
    {
        return Ok(octopus_core::RUNTIME_PERMISSION_DANGER_FULL_ACCESS.into());
    }
    if !project
        .member_user_ids
        .iter()
        .any(|user_id| user_id == &session.user_id)
    {
        return Ok(octopus_core::RUNTIME_PERMISSION_READ_ONLY.into());
    }
    if resolve_project_module_permission(&workspace, &project, "tools") == "deny" {
        return Ok(octopus_core::RUNTIME_PERMISSION_READ_ONLY.into());
    }
    Ok(octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE.into())
}

pub(crate) async fn get_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        Some("runtime.session"),
        Some(&session_id),
    )
    .await?;
    let detail = state
        .services
        .runtime_session
        .get_session(&session_id)
        .await?;
    let payload = runtime_transport_payload(&detail, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn delete_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_capability_session(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        Some("runtime.session"),
        Some(&session_id),
    )
    .await?;
    state
        .services
        .runtime_session
        .delete_session(&session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn submit_runtime_turn(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(mut input): Json<SubmitRuntimeTurnInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    normalize_runtime_submit_input(&mut input)?;
    let session = ensure_runtime_submit(
        &state,
        &headers,
        Some(&input),
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.submit_turn", &session_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .submit_turn(&session_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, approval_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeApprovalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.approval.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.resolve_approval", &approval_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_approval(&session_id, &approval_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_auth_challenge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, challenge_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeAuthChallengeInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.auth.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.resolve_auth_challenge",
            &challenge_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_auth_challenge(&session_id, &challenge_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn cancel_runtime_subrun(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, subrun_id)): Path<(String, String)>,
    Json(mut input): Json<CancelRuntimeSubrunInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    input.note = input
        .note
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.subrun.cancel",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.cancel_subrun", &subrun_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .cancel_subrun(&session_id, &subrun_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn resolve_runtime_memory_proposal(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, proposal_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeMemoryProposalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.approval.resolve",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.resolve_memory_proposal",
            &proposal_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_memory_proposal(&session_id, &proposal_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        let payload = runtime_transport_payload(&run, &request_id)?;
        store_idempotent_response(&state, scope, &payload, &request_id)?;
    }

    let payload = runtime_transport_payload(&run, &request_id)?;
    let mut response = Json(payload).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

pub(crate) async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;

    let replay_after = query.after.or_else(|| last_event_id(&headers));

    if !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?;
        let payload = runtime_transport_payload(&events, &request_id)?;
        let mut response = Json(payload).into_response();
        insert_request_id(&mut response, &request_id);
        return Ok(response);
    }

    let replay_events = if replay_after.is_some() {
        state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?
    } else {
        Vec::new()
    };
    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream_request_id = request_id.clone();
    let stream = stream! {
        for event in replay_events {
            if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                if let Ok(data) = serde_json::to_string(&payload) {
                    yield Ok::<Event, std::convert::Infallible>(
                        Event::default()
                            .event(event.event_type.clone())
                            .id(event.id.clone())
                            .data(data)
                    );
                }
            }
        }

        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                        if let Ok(data) = serde_json::to_string(&payload) {
                            yield Ok::<Event, std::convert::Infallible>(
                                Event::default()
                                    .event(event.event_type.clone())
                                    .id(event.id.clone())
                                    .data(data)
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample_session() -> SessionRecord {
        SessionRecord {
            id: "sess-1".into(),
            workspace_id: "ws-local".into(),
            user_id: "user-owner".into(),
            client_app_id: "octopus-desktop".into(),
            token: "token".into(),
            status: "active".into(),
            created_at: 1,
            expires_at: None,
        }
    }

    fn sample_resource(visibility: &str, owner_user_id: &str) -> WorkspaceResourceRecord {
        WorkspaceResourceRecord {
            id: "res-1".into(),
            workspace_id: "ws-local".into(),
            project_id: Some("proj-redesign".into()),
            kind: "file".into(),
            name: "brief.md".into(),
            location: Some("data/projects/proj-redesign/resources/brief.md".into()),
            origin: "source".into(),
            scope: "project".into(),
            visibility: visibility.into(),
            owner_user_id: owner_user_id.into(),
            storage_path: Some("data/projects/proj-redesign/resources/brief.md".into()),
            content_type: Some("text/markdown".into()),
            byte_size: Some(12),
            preview_kind: "markdown".into(),
            status: "healthy".into(),
            updated_at: 1,
            tags: Vec::new(),
            source_artifact_id: None,
        }
    }

    fn sample_knowledge(
        scope: &str,
        visibility: &str,
        owner_user_id: Option<&str>,
    ) -> KnowledgeRecord {
        KnowledgeRecord {
            id: "kn-1".into(),
            workspace_id: "ws-local".into(),
            project_id: if scope == "project" {
                Some("proj-redesign".into())
            } else {
                None
            },
            title: "Knowledge brief".into(),
            summary: "Knowledge summary".into(),
            kind: "shared".into(),
            status: "reviewed".into(),
            source_type: "artifact".into(),
            source_ref: "artifact-1".into(),
            updated_at: 1,
            scope: scope.into(),
            visibility: visibility.into(),
            owner_user_id: owner_user_id.map(str::to_string),
        }
    }

    fn sample_agent(asset_role: &str, owner_user_id: Option<&str>) -> AgentRecord {
        AgentRecord {
            id: format!("agent-{asset_role}"),
            workspace_id: "ws-local".into(),
            project_id: None,
            scope: if asset_role == "pet" {
                "personal".into()
            } else {
                "workspace".into()
            },
            owner_user_id: owner_user_id.map(str::to_string),
            asset_role: asset_role.into(),
            name: format!("{asset_role} agent"),
            avatar_path: None,
            avatar: None,
            personality: "Helpful".into(),
            tags: Vec::new(),
            prompt: "Assist the workspace.".into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: Vec::new(),
            mcp_server_names: Vec::new(),
            task_domains: Vec::new(),
            manifest_revision: "asset-manifest/v2".into(),
            default_model_strategy: octopus_core::default_model_strategy(),
            capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
            permission_envelope: octopus_core::default_permission_envelope(),
            memory_policy: octopus_core::default_agent_memory_policy(),
            delegation_policy: octopus_core::default_agent_delegation_policy(),
            approval_preference: octopus_core::default_approval_preference(),
            output_contract: octopus_core::default_output_contract(),
            shared_capability_policy: octopus_core::default_agent_shared_capability_policy(),
            integration_source: None,
            trust_metadata: octopus_core::default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: octopus_core::default_asset_import_metadata(),
            description: "Test agent".into(),
            status: "active".into(),
            updated_at: 1,
        }
    }

    fn sample_runtime_run_snapshot() -> octopus_core::RuntimeRunSnapshot {
        octopus_core::RuntimeRunSnapshot {
            id: "run-1".into(),
            session_id: "session-1".into(),
            conversation_id: "conversation-1".into(),
            status: "running".into(),
            current_step: "workflow_step".into(),
            started_at: 10,
            updated_at: 20,
            selected_memory: Vec::new(),
            freshness_summary: None,
            pending_memory_proposal: None,
            memory_state_ref: "memory-state-1".into(),
            configured_model_id: Some("quota-model".into()),
            configured_model_name: Some("Quota Model".into()),
            model_id: Some("provider-model".into()),
            consumed_tokens: Some(42),
            next_action: Some("await_workflow".into()),
            config_snapshot_id: "config-1".into(),
            effective_config_hash: "hash-1".into(),
            started_from_scope_set: vec!["workspace".into()],
            run_kind: "primary".into(),
            parent_run_id: None,
            actor_ref: "team:workspace-core".into(),
            delegated_by_tool_call_id: Some("tool-call-1".into()),
            workflow_run: Some("workflow-1".into()),
            workflow_run_detail: Some(octopus_core::RuntimeWorkflowRunDetail {
                workflow_run_id: "workflow-1".into(),
                status: "background_running".into(),
                current_step_id: Some("step-1".into()),
                current_step_label: Some("Worker review".into()),
                total_steps: 3,
                completed_steps: 1,
                background_capable: true,
                steps: vec![octopus_core::RuntimeWorkflowStepSummary {
                    step_id: "step-1".into(),
                    node_kind: "worker".into(),
                    label: "Worker review".into(),
                    actor_ref: "agent:workspace-worker".into(),
                    run_id: Some("subrun-1".into()),
                    parent_run_id: Some("run-1".into()),
                    delegated_by_tool_call_id: Some("tool-call-1".into()),
                    mailbox_ref: Some("mailbox-1".into()),
                    handoff_ref: Some("handoff-1".into()),
                    status: "running".into(),
                    started_at: 12,
                    updated_at: 20,
                }],
                blocking: None,
            }),
            mailbox_ref: Some("mailbox-1".into()),
            handoff_ref: Some("handoff-1".into()),
            background_state: Some("background_running".into()),
            worker_dispatch: Some(octopus_core::RuntimeWorkerDispatchSummary {
                total_subruns: 1,
                active_subruns: 1,
                completed_subruns: 0,
                failed_subruns: 0,
            }),
            approval_state: "not-required".into(),
            approval_target: None,
            auth_target: None,
            usage_summary: octopus_core::RuntimeUsageSummary::default(),
            artifact_refs: vec!["runtime-artifact-run-1".into()],
            trace_context: octopus_core::RuntimeTraceContext::default(),
            checkpoint: octopus_core::RuntimeRunCheckpoint::default(),
            capability_plan_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            pending_mediation: None,
            capability_state_ref: Some("capability-state-1".into()),
            last_execution_outcome: None,
            last_mediation_outcome: None,
            resolved_target: None,
            requested_actor_kind: Some("team".into()),
            requested_actor_id: Some("team:workspace-core".into()),
            resolved_actor_kind: Some("team".into()),
            resolved_actor_id: Some("team:workspace-core".into()),
            resolved_actor_label: Some("Workspace Core".into()),
        }
    }

    fn sample_runtime_session_detail() -> octopus_core::RuntimeSessionDetail {
        let run = sample_runtime_run_snapshot();
        let workflow = octopus_core::RuntimeWorkflowSummary {
            workflow_run_id: "workflow-1".into(),
            label: "Team workflow".into(),
            status: "background_running".into(),
            total_steps: 3,
            completed_steps: 1,
            current_step_id: Some("step-1".into()),
            current_step_label: Some("Worker review".into()),
            background_capable: true,
            updated_at: 20,
        };
        let mailbox = octopus_core::RuntimeMailboxSummary {
            mailbox_ref: "mailbox-1".into(),
            channel: "leader-hub".into(),
            status: "pending".into(),
            pending_count: 1,
            total_messages: 1,
            updated_at: 20,
        };
        let background = octopus_core::RuntimeBackgroundRunSummary {
            run_id: run.id.clone(),
            workflow_run_id: Some("workflow-1".into()),
            status: "background_running".into(),
            background_capable: true,
            continuation_state: "running".into(),
            blocking: None,
            updated_at: 20,
        };

        octopus_core::RuntimeSessionDetail {
            summary: octopus_core::RuntimeSessionSummary {
                id: "session-1".into(),
                conversation_id: "conversation-1".into(),
                project_id: "project-1".into(),
                title: "Phase 4".into(),
                session_kind: "project".into(),
                status: "running".into(),
                updated_at: 20,
                last_message_preview: Some("Workflow in progress".into()),
                config_snapshot_id: "config-1".into(),
                effective_config_hash: "hash-1".into(),
                started_from_scope_set: vec!["workspace".into()],
                selected_actor_ref: "team:workspace-core".into(),
                manifest_revision: "manifest-1".into(),
                session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
                active_run_id: run.id.clone(),
                subrun_count: 1,
                workflow: Some(workflow.clone()),
                pending_mailbox: Some(mailbox.clone()),
                background_run: Some(background.clone()),
                memory_summary: octopus_core::RuntimeMemorySummary::default(),
                memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
                pending_memory_proposal_count: 0,
                memory_state_ref: "memory-state-1".into(),
                capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
                provider_state_summary: Vec::new(),
                auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
                pending_mediation: None,
                policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
                capability_state_ref: Some("capability-state-1".into()),
                last_execution_outcome: None,
            },
            selected_actor_ref: "team:workspace-core".into(),
            manifest_revision: "manifest-1".into(),
            session_policy: octopus_core::RuntimeSessionPolicySnapshot::default(),
            active_run_id: run.id.clone(),
            subrun_count: 1,
            workflow: Some(workflow),
            pending_mailbox: Some(mailbox),
            background_run: Some(background),
            memory_summary: octopus_core::RuntimeMemorySummary::default(),
            memory_selection_summary: octopus_core::RuntimeMemorySelectionSummary::default(),
            pending_memory_proposal_count: 0,
            memory_state_ref: "memory-state-1".into(),
            capability_summary: octopus_core::RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            auth_state_summary: octopus_core::RuntimeAuthStateSummary::default(),
            pending_mediation: None,
            policy_decision_summary: octopus_core::RuntimePolicyDecisionSummary::default(),
            capability_state_ref: Some("capability-state-1".into()),
            last_execution_outcome: None,
            run,
            subruns: vec![octopus_core::RuntimeSubrunSummary {
                run_id: "subrun-1".into(),
                parent_run_id: Some("run-1".into()),
                actor_ref: "agent:worker".into(),
                label: "Worker".into(),
                status: "running".into(),
                run_kind: "subrun".into(),
                delegated_by_tool_call_id: Some("tool-call-1".into()),
                workflow_run_id: Some("workflow-1".into()),
                mailbox_ref: Some("mailbox-1".into()),
                handoff_ref: Some("handoff-1".into()),
                started_at: 11,
                updated_at: 20,
            }],
            handoffs: vec![octopus_core::RuntimeHandoffSummary {
                handoff_ref: "handoff-1".into(),
                mailbox_ref: "mailbox-1".into(),
                sender_actor_ref: "team:workspace-core".into(),
                receiver_actor_ref: "agent:worker".into(),
                state: "pending".into(),
                artifact_refs: vec!["runtime-artifact-run-1".into()],
                updated_at: 20,
            }],
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        }
    }

    #[test]
    fn validate_create_project_request_requires_and_trims_resource_directory() {
        let validated = validate_create_project_request(CreateProjectRequest {
            name: "  Resource Project  ".into(),
            description: "  Resource import coverage.  ".into(),
            resource_directory: "  data/projects/resource-project/resources  ".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            assignments: None,
        })
        .expect("validated request");

        assert_eq!(validated.name, "Resource Project");
        assert_eq!(validated.description, "Resource import coverage.");
        assert_eq!(
            validated.resource_directory,
            "data/projects/resource-project/resources"
        );

        assert!(validate_create_project_request(CreateProjectRequest {
            name: "Project".into(),
            description: String::new(),
            resource_directory: "   ".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            assignments: None,
        })
        .is_err());
    }

    #[test]
    fn validate_update_project_request_requires_status_and_resource_directory() {
        let validated = validate_update_project_request(UpdateProjectRequest {
            name: "  Resource Project  ".into(),
            description: "  Updated description.  ".into(),
            status: " archived ".into(),
            resource_directory: "  data/projects/resource-project/resources  ".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            assignments: None,
        })
        .expect("validated update");

        assert_eq!(validated.name, "Resource Project");
        assert_eq!(validated.description, "Updated description.");
        assert_eq!(validated.status, "archived");
        assert_eq!(
            validated.resource_directory,
            "data/projects/resource-project/resources"
        );

        assert!(validate_update_project_request(UpdateProjectRequest {
            name: "Project".into(),
            description: String::new(),
            status: "disabled".into(),
            resource_directory: "data/projects/resource-project/resources".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            assignments: None,
        })
        .is_err());
        assert!(validate_update_project_request(UpdateProjectRequest {
            name: "Project".into(),
            description: String::new(),
            status: "active".into(),
            resource_directory: " ".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            assignments: None,
        })
        .is_err());
    }

    #[test]
    fn generic_agent_catalog_filter_excludes_pet_records() {
        let visible = vec![
            sample_agent("default", None),
            sample_agent("pet", Some("user-owner")),
        ]
        .into_iter()
        .filter(agent_visible_in_generic_catalog)
        .collect::<Vec<_>>();

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].asset_role, "default");
    }

    fn sample_runtime_event() -> octopus_core::RuntimeEventEnvelope {
        octopus_core::RuntimeEventEnvelope {
            id: "evt-1".into(),
            event_type: "runtime.run.updated".into(),
            workspace_id: "ws-local".into(),
            project_id: Some("project-1".into()),
            session_id: "session-1".into(),
            conversation_id: "conversation-1".into(),
            run_id: Some("run-1".into()),
            emitted_at: 20,
            sequence: 1,
            run: Some(sample_runtime_run_snapshot()),
            capability_plan_summary: Some(octopus_core::RuntimeCapabilityPlanSummary::default()),
            provider_state_summary: Some(Vec::new()),
            ..Default::default()
        }
    }

    #[test]
    fn runtime_session_detail_transport_preserves_phase_four_fields_without_escape_hatches() {
        let json =
            runtime_transport_payload(&sample_runtime_session_detail(), "req-test").expect("json");

        assert_eq!(
            json.pointer("/workflow/workflowRunId")
                .and_then(Value::as_str),
            Some("workflow-1")
        );
        assert_eq!(
            json.pointer("/pendingMailbox/channel")
                .and_then(Value::as_str),
            Some("leader-hub")
        );
        assert_eq!(
            json.pointer("/backgroundRun/status")
                .and_then(Value::as_str),
            Some("background_running")
        );
        assert_eq!(
            json.pointer("/subruns/0/workflowRunId")
                .and_then(Value::as_str),
            Some("workflow-1")
        );
        assert_eq!(
            json.pointer("/handoffs/0/artifactRefs/0")
                .and_then(Value::as_str),
            Some("runtime-artifact-run-1")
        );
        assert_eq!(
            json.pointer("/run/workflowRunDetail/currentStepId")
                .and_then(Value::as_str),
            Some("step-1")
        );
        assert!(json.pointer("/run/checkpoint/serializedSession").is_none());
        assert!(json.pointer("/run/checkpoint/compactionMetadata").is_none());
    }

    #[test]
    fn runtime_run_transport_preserves_phase_four_fields_without_escape_hatches() {
        let json = runtime_transport_payload(&sample_runtime_run_snapshot(), "req-test")
            .expect("runtime run json");

        assert_eq!(
            json.pointer("/workflowRun").and_then(Value::as_str),
            Some("workflow-1")
        );
        assert_eq!(
            json.pointer("/workflowRunDetail/status")
                .and_then(Value::as_str),
            Some("background_running")
        );
        assert_eq!(
            json.pointer("/mailboxRef").and_then(Value::as_str),
            Some("mailbox-1")
        );
        assert_eq!(
            json.pointer("/backgroundState").and_then(Value::as_str),
            Some("background_running")
        );
        assert_eq!(
            json.pointer("/workerDispatch/totalSubruns")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            json.pointer("/artifactRefs/0").and_then(Value::as_str),
            Some("runtime-artifact-run-1")
        );
        assert!(json.pointer("/checkpoint/serializedSession").is_none());
        assert!(json.pointer("/checkpoint/compactionMetadata").is_none());
    }

    #[test]
    fn runtime_event_transport_drops_payload_escape_hatch() {
        let json = runtime_transport_payload(&sample_runtime_event(), "req-test").expect("json");

        assert!(json.pointer("/payload").is_none());
        assert_eq!(
            json.pointer("/run/workflowRun").and_then(Value::as_str),
            Some("workflow-1")
        );
    }

    #[test]
    fn resource_visibility_allows_private_resources_only_for_the_owner() {
        let session = sample_session();

        assert!(resource_visibility_allows(
            &session,
            &sample_resource("public", "another-user")
        ));
        assert!(resource_visibility_allows(
            &session,
            &sample_resource("private", "user-owner")
        ));
        assert!(!resource_visibility_allows(
            &session,
            &sample_resource("private", "another-user")
        ));
    }

    #[test]
    fn knowledge_visibility_allows_personal_records_only_for_the_owner() {
        let session = sample_session();

        assert!(knowledge_visibility_allows(
            &session,
            &sample_knowledge("workspace", "public", None)
        ));
        assert!(knowledge_visibility_allows(
            &session,
            &sample_knowledge("personal", "private", Some("user-owner"))
        ));
        assert!(!knowledge_visibility_allows(
            &session,
            &sample_knowledge("personal", "private", Some("another-user"))
        ));
    }

    #[test]
    fn resolved_fork_target_project_id_preserves_workspace_scope() {
        assert_eq!(resolved_fork_target_project_id(None, ""), None);
        assert_eq!(resolved_fork_target_project_id(Some("   "), ""), None);
        assert_eq!(
            resolved_fork_target_project_id(Some(" project-2 "), ""),
            Some("project-2".into())
        );
    }
}
