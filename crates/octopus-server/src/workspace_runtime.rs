use super::*;
use crate::dto_mapping::metric_record;
use octopus_core::{
    AuditRecord, AuthorizationRequest, CancelRuntimeSubrunInput, CapabilityManagementProjection,
    ConversationRecord, CostLedgerEntry, CreateDeliverableVersionInput,
    CreateProjectDeletionRequestInput, CreateProjectPromotionRequestInput,
    CreateRuntimeSessionInput, CreateTaskInterventionRequest, CreateTaskRequest, DeliverableDetail,
    DeliverableVersionContent, DeliverableVersionSummary, ExportWorkspaceAgentBundleInput,
    ExportWorkspaceAgentBundleResult, ForkDeliverableInput, KnowledgeEntryRecord,
    LaunchTaskRequest, PetDashboardSummary, ProjectDashboardBreakdownItem,
    ProjectDashboardConversationInsight, ProjectDashboardRankingItem, ProjectDashboardSnapshot,
    ProjectDashboardSummary, ProjectDashboardTrendPoint, ProjectDashboardUserStat,
    ProjectDeletionRequest, ProjectPromotionRequest, ProjectTaskInterventionRecord,
    ProjectTaskRecord, ProjectTaskRunRecord, ProjectTokenUsageRecord, PromoteDeliverableInput,
    ProtectedResourceDescriptor, RerunTaskRequest, ResolveRuntimeAuthChallengeInput,
    ResolveRuntimeMemoryProposalInput, ReviewProjectDeletionRequestInput,
    ReviewProjectPromotionRequestInput, RunRuntimeGenerationInput, RuntimeGenerationResult,
    RuntimeMessage, RuntimeRunSnapshot, TaskAnalyticsSummary, TaskContextBundle, TaskDetail,
    TaskInterventionRecord, TaskRunSummary, TaskStateTransitionSummary, TaskSummary,
    UpdateTaskRequest, UpdateWorkspaceRequest,
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

fn normalize_project_string_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !normalized.iter().any(|item| item == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn visible_inbox_items(
    user_id: &str,
    items: Vec<octopus_core::InboxItemRecord>,
) -> Vec<octopus_core::InboxItemRecord> {
    items
        .into_iter()
        .filter(|item| item.target_user_id == user_id)
        .collect()
}

fn normalize_project_assignments(
    assignments: Option<octopus_core::ProjectWorkspaceAssignments>,
) -> Option<octopus_core::ProjectWorkspaceAssignments> {
    assignments.map(|mut assignments| {
        if let Some(models) = assignments.models.as_mut() {
            models.configured_model_ids =
                normalize_project_string_list(std::mem::take(&mut models.configured_model_ids));
            models.default_configured_model_id =
                models.default_configured_model_id.trim().to_string();
        }
        if let Some(tools) = assignments.tools.as_mut() {
            tools.source_keys =
                normalize_project_string_list(std::mem::take(&mut tools.source_keys));
            tools.excluded_source_keys =
                normalize_project_string_list(std::mem::take(&mut tools.excluded_source_keys));
        }
        if let Some(agents) = assignments.agents.as_mut() {
            agents.agent_ids = normalize_project_string_list(std::mem::take(&mut agents.agent_ids));
            agents.team_ids = normalize_project_string_list(std::mem::take(&mut agents.team_ids));
            agents.excluded_agent_ids =
                normalize_project_string_list(std::mem::take(&mut agents.excluded_agent_ids));
            agents.excluded_team_ids =
                normalize_project_string_list(std::mem::take(&mut agents.excluded_team_ids));
        }
        assignments
    })
}

fn project_workspace_assignments(
    document: &serde_json::Value,
) -> Option<octopus_core::ProjectWorkspaceAssignments> {
    let assignments = document
        .get("projectSettings")
        .and_then(|settings| settings.get("workspaceAssignments"))?;
    let assignments = serde_json::from_value(assignments.clone()).ok()?;
    normalize_project_assignments(Some(assignments))
}

#[derive(Debug, Default)]
struct ProjectGrantedScope {
    workspace_active_agent_ids: BTreeSet<String>,
    agents: Vec<octopus_core::AgentRecord>,
    teams: Vec<octopus_core::TeamRecord>,
    tool_source_keys: Vec<String>,
}

#[derive(Debug, Default)]
struct ProjectRuntimeDisables {
    agent_ids: BTreeSet<String>,
}

fn project_tool_assignments(
    assignments: Option<&octopus_core::ProjectWorkspaceAssignments>,
) -> Option<&octopus_core::ProjectToolAssignments> {
    assignments.and_then(|assignments| assignments.tools.as_ref())
}

fn project_agent_assignments(
    assignments: Option<&octopus_core::ProjectWorkspaceAssignments>,
) -> Option<&octopus_core::ProjectAgentAssignments> {
    assignments.and_then(|assignments| assignments.agents.as_ref())
}

async fn resolve_project_granted_scope(
    state: &ServerState,
    project: &ProjectRecord,
    runtime_document: &serde_json::Value,
) -> Result<ProjectGrantedScope, ApiError> {
    let assignments = project_workspace_assignments(runtime_document);
    let excluded_agent_ids = project_agent_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_agent_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let excluded_team_ids = project_agent_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_team_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let excluded_tool_source_keys = project_tool_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_source_keys
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let mut workspace_active_agent_ids = BTreeSet::new();
    let mut seen_agent_ids = BTreeSet::new();
    let mut agents = Vec::new();
    for record in state.services.workspace.list_agents().await? {
        if record.status != "active" || !agent_visible_in_generic_catalog(&record) {
            continue;
        }
        let is_project_owned = record.project_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = record.project_id.is_none();
        if is_workspace_inherited {
            workspace_active_agent_ids.insert(record.id.clone());
        }
        if (is_project_owned
            || (is_workspace_inherited && !excluded_agent_ids.contains(&record.id)))
            && seen_agent_ids.insert(record.id.clone())
        {
            agents.push(record);
        }
    }

    let mut seen_team_ids = BTreeSet::new();
    let mut teams = Vec::new();
    for record in state.services.workspace.list_teams().await? {
        if record.status != "active" {
            continue;
        }
        let is_project_owned = record.project_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = record.project_id.is_none();
        if (is_project_owned || (is_workspace_inherited && !excluded_team_ids.contains(&record.id)))
            && seen_team_ids.insert(record.id.clone())
        {
            teams.push(record);
        }
    }

    let mut tool_source_keys = BTreeSet::new();
    for asset in state
        .services
        .workspace
        .get_capability_management_projection()
        .await?
        .assets
    {
        if !asset.enabled {
            continue;
        }
        let is_project_owned = asset.owner_scope.as_deref() == Some("project")
            && asset.owner_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = asset.owner_scope.as_deref() != Some("project");
        if is_project_owned
            || (is_workspace_inherited && !excluded_tool_source_keys.contains(&asset.source_key))
        {
            tool_source_keys.insert(asset.source_key);
        }
    }

    Ok(ProjectGrantedScope {
        workspace_active_agent_ids,
        agents,
        teams,
        tool_source_keys: tool_source_keys.into_iter().collect(),
    })
}

fn merge_runtime_config_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match patch {
        serde_json::Value::Object(patch_map) => {
            if !target.is_object() {
                *target = serde_json::Value::Object(serde_json::Map::new());
            }
            let target_map = target
                .as_object_mut()
                .expect("target should be an object after initialization");
            for (key, value) in patch_map {
                if value.is_null() {
                    target_map.remove(key);
                    continue;
                }
                if let Some(existing) = target_map.get_mut(key) {
                    merge_runtime_config_patch(existing, value);
                } else {
                    target_map.insert(key.clone(), value.clone());
                }
            }
        }
        _ => *target = patch.clone(),
    }
}

async fn load_project_runtime_document(
    state: &ServerState,
    project: &ProjectRecord,
    patch: Option<&serde_json::Value>,
) -> Result<serde_json::Value, ApiError> {
    let config = state
        .services
        .runtime_config
        .get_project_config(&project.id, &project.owner_user_id)
        .await?;
    let mut document = config
        .sources
        .into_iter()
        .find(|source| source.scope == "project")
        .and_then(|source| source.document)
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    if let Some(patch) = patch {
        merge_runtime_config_patch(&mut document, patch);
    }
    Ok(document)
}

fn normalize_runtime_string_set(value: Option<&serde_json::Value>) -> Option<BTreeSet<String>> {
    let values = value.and_then(serde_json::Value::as_array)?;
    Some(
        values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn resolve_project_runtime_disables(
    document: &serde_json::Value,
    scope: &ProjectGrantedScope,
) -> ProjectRuntimeDisables {
    let project_settings = document
        .get("projectSettings")
        .and_then(serde_json::Value::as_object);
    let agents_object = project_settings
        .and_then(|settings| settings.get("agents"))
        .and_then(serde_json::Value::as_object);

    let granted_agent_ids = scope
        .agents
        .iter()
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();

    let disabled_agent_ids = normalize_runtime_string_set(
        agents_object.and_then(|settings| settings.get("disabledAgentIds")),
    )
    .map(|values| {
        values
            .into_iter()
            .filter(|value| granted_agent_ids.contains(value))
            .collect()
    })
    .or_else(|| {
        normalize_runtime_string_set(
            agents_object.and_then(|settings| settings.get("enabledAgentIds")),
        )
        .map(|enabled| {
            granted_agent_ids
                .iter()
                .filter(|value| !enabled.contains(*value))
                .cloned()
                .collect()
        })
    })
    .unwrap_or_default();

    ProjectRuntimeDisables {
        agent_ids: disabled_agent_ids,
    }
}

fn validate_project_leader_against_scope(
    leader_agent_id: Option<&str>,
    scope: &ProjectGrantedScope,
    runtime_disables: &ProjectRuntimeDisables,
) -> Result<(), ApiError> {
    let Some(leader_agent_id) = leader_agent_id else {
        return Ok(());
    };
    let granted_agent_ids = scope
        .agents
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    if !scope.workspace_active_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must reference an active workspace agent",
        )
        .into());
    }
    if !granted_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must remain in the effective project agent scope",
        )
        .into());
    }
    if runtime_disables.agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input("project leader must remain runtime enabled").into());
    }
    Ok(())
}

async fn validate_create_project_leader(
    state: &ServerState,
    request: &CreateProjectRequest,
) -> Result<(), ApiError> {
    let Some(leader_agent_id) = request.leader_agent_id.as_deref() else {
        return Ok(());
    };
    let workspace_active_agent_ids = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .filter(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .map(|record| record.id)
        .collect::<BTreeSet<_>>();
    if !workspace_active_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must reference a granted active workspace agent",
        )
        .into());
    }
    Ok(())
}

async fn validate_updated_project_leader(
    state: &ServerState,
    project: &ProjectRecord,
    request: &UpdateProjectRequest,
) -> Result<(), ApiError> {
    let runtime_document = load_project_runtime_document(state, project, None).await?;
    let scope = resolve_project_granted_scope(state, project, &runtime_document).await?;
    let runtime_disables = resolve_project_runtime_disables(&runtime_document, &scope);
    let leader_agent_id = request
        .leader_agent_id
        .as_deref()
        .or(project.leader_agent_id.as_deref());
    validate_project_leader_against_scope(leader_agent_id, &scope, &runtime_disables)
}

async fn validate_project_runtime_leader(
    state: &ServerState,
    project: &ProjectRecord,
    patch: &RuntimeConfigPatch,
) -> Result<(), ApiError> {
    let runtime_document =
        load_project_runtime_document(state, project, Some(&patch.patch)).await?;
    let scope = resolve_project_granted_scope(state, project, &runtime_document).await?;
    let runtime_disables = resolve_project_runtime_disables(&runtime_document, &scope);
    validate_project_leader_against_scope(
        project.leader_agent_id.as_deref(),
        &scope,
        &runtime_disables,
    )
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

async fn ensure_project_delete_review_session(
    state: &ServerState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    let session = authenticate_session_with_request_id(state, headers, &request_id).await?;
    let decision = state
        .services
        .authorization
        .authorize_request(
            &session,
            &capability_authorization_request(
                &session.user_id,
                "project.manage",
                Some(project_id),
                Some("project"),
                Some(project_id),
                None,
                &[],
                Some("internal"),
                None,
                None,
            ),
        )
        .await?;
    if !decision.allowed {
        return Err(ApiError::new(
            AppError::auth(decision.reason.unwrap_or_else(|| "access denied".into())),
            request_id,
        ));
    }
    Ok(session)
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

pub(crate) async fn update_workspace_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    let workspace = state.services.workspace.workspace_summary().await?;
    if workspace.owner_user_id.as_deref() != Some(session.user_id.as_str()) {
        return Err(ApiError::from(AppError::auth(
            "workspace settings require the workspace owner",
        )));
    }
    Ok(Json(
        state.services.workspace.update_workspace(request).await?,
    ))
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

fn trim_optional_task_input(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_task_context_bundle_input(mut bundle: TaskContextBundle) -> TaskContextBundle {
    bundle.refs = bundle
        .refs
        .into_iter()
        .map(|mut reference| {
            reference.kind = reference.kind.trim().to_string();
            reference.ref_id = reference.ref_id.trim().to_string();
            reference.title = reference.title.trim().to_string();
            reference.subtitle = reference.subtitle.trim().to_string();
            reference.version_ref = trim_optional_task_input(reference.version_ref);
            reference.pin_mode = reference.pin_mode.trim().to_string();
            reference
        })
        .filter(|reference| {
            !reference.kind.is_empty()
                && !reference.ref_id.is_empty()
                && !reference.title.is_empty()
        })
        .collect();
    bundle.pinned_instructions = bundle.pinned_instructions.trim().to_string();
    let resolution_mode = bundle.resolution_mode.trim();
    bundle.resolution_mode = if resolution_mode.is_empty() {
        "explicit_only".into()
    } else {
        resolution_mode.to_string()
    };
    bundle
}

pub(crate) fn validate_create_task_request(
    request: CreateTaskRequest,
) -> Result<CreateTaskRequest, ApiError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(AppError::invalid_input("task title is required").into());
    }
    let goal = request.goal.trim();
    if goal.is_empty() {
        return Err(AppError::invalid_input("task goal is required").into());
    }
    let brief = request.brief.trim();
    if brief.is_empty() {
        return Err(AppError::invalid_input("task brief is required").into());
    }
    let default_actor_ref = request.default_actor_ref.trim();
    if default_actor_ref.is_empty() {
        return Err(AppError::invalid_input("default actor is required").into());
    }

    Ok(CreateTaskRequest {
        title: title.into(),
        goal: goal.into(),
        brief: brief.into(),
        default_actor_ref: default_actor_ref.into(),
        schedule_spec: trim_optional_task_input(request.schedule_spec),
        context_bundle: normalize_task_context_bundle_input(request.context_bundle),
    })
}

pub(crate) fn validate_update_task_request(
    request: UpdateTaskRequest,
) -> Result<UpdateTaskRequest, ApiError> {
    if let Some(title) = request.title.as_deref() {
        if title.trim().is_empty() {
            return Err(AppError::invalid_input("task title must not be empty").into());
        }
    }
    if let Some(goal) = request.goal.as_deref() {
        if goal.trim().is_empty() {
            return Err(AppError::invalid_input("task goal must not be empty").into());
        }
    }
    if let Some(brief) = request.brief.as_deref() {
        if brief.trim().is_empty() {
            return Err(AppError::invalid_input("task brief must not be empty").into());
        }
    }
    if let Some(default_actor_ref) = request.default_actor_ref.as_deref() {
        if default_actor_ref.trim().is_empty() {
            return Err(AppError::invalid_input("default actor must not be empty").into());
        }
    }

    Ok(UpdateTaskRequest {
        title: trim_optional_task_input(request.title),
        goal: trim_optional_task_input(request.goal),
        brief: trim_optional_task_input(request.brief),
        default_actor_ref: trim_optional_task_input(request.default_actor_ref),
        schedule_spec: request.schedule_spec.map(|value| value.trim().to_string()),
        context_bundle: request
            .context_bundle
            .map(normalize_task_context_bundle_input),
    })
}

fn task_summary_from_record(record: &ProjectTaskRecord) -> TaskSummary {
    TaskSummary {
        id: record.id.clone(),
        project_id: record.project_id.clone(),
        title: record.title.clone(),
        goal: record.goal.clone(),
        default_actor_ref: record.default_actor_ref.clone(),
        status: record.status.clone(),
        schedule_spec: record.schedule_spec.clone(),
        next_run_at: record.next_run_at,
        last_run_at: record.last_run_at,
        latest_result_summary: record.latest_result_summary.clone(),
        latest_failure_category: record.latest_failure_category.clone(),
        latest_transition: record.latest_transition.clone(),
        view_status: record.view_status.clone(),
        attention_reasons: record.attention_reasons.clone(),
        attention_updated_at: record.attention_updated_at,
        active_task_run_id: record.active_task_run_id.clone(),
        analytics_summary: record.analytics_summary.clone(),
        updated_at: record.updated_at,
    }
}

fn task_run_summary_from_record(record: &ProjectTaskRunRecord) -> TaskRunSummary {
    TaskRunSummary {
        id: record.id.clone(),
        task_id: record.task_id.clone(),
        trigger_type: record.trigger_type.clone(),
        status: record.status.clone(),
        session_id: record.session_id.clone(),
        conversation_id: record.conversation_id.clone(),
        runtime_run_id: record.runtime_run_id.clone(),
        actor_ref: record.actor_ref.clone(),
        started_at: record.started_at,
        completed_at: record.completed_at,
        result_summary: record.result_summary.clone(),
        pending_approval_id: record.pending_approval_id.clone(),
        failure_category: record.failure_category.clone(),
        failure_summary: record.failure_summary.clone(),
        view_status: record.view_status.clone(),
        attention_reasons: record.attention_reasons.clone(),
        attention_updated_at: record.attention_updated_at,
        deliverable_refs: record.deliverable_refs.clone(),
        artifact_refs: record.artifact_refs.clone(),
        latest_transition: record.latest_transition.clone(),
    }
}

fn task_intervention_from_record(record: &ProjectTaskInterventionRecord) -> TaskInterventionRecord {
    TaskInterventionRecord {
        id: record.id.clone(),
        task_id: record.task_id.clone(),
        task_run_id: record.task_run_id.clone(),
        r#type: record.r#type.clone(),
        payload: record.payload.clone(),
        created_by: record.created_by.clone(),
        created_at: record.created_at,
        applied_to_session_id: record.applied_to_session_id.clone(),
        status: record.status.clone(),
    }
}

fn task_detail_from_records(
    task: &ProjectTaskRecord,
    runs: &[ProjectTaskRunRecord],
    interventions: &[ProjectTaskInterventionRecord],
) -> TaskDetail {
    let run_history = runs
        .iter()
        .map(task_run_summary_from_record)
        .collect::<Vec<_>>();
    let active_run = task
        .active_task_run_id
        .as_deref()
        .and_then(|run_id| run_history.iter().find(|run| run.id == run_id).cloned())
        .or_else(|| run_history.first().cloned());

    TaskDetail {
        id: task.id.clone(),
        project_id: task.project_id.clone(),
        title: task.title.clone(),
        goal: task.goal.clone(),
        brief: task.brief.clone(),
        default_actor_ref: task.default_actor_ref.clone(),
        status: task.status.clone(),
        schedule_spec: task.schedule_spec.clone(),
        next_run_at: task.next_run_at,
        last_run_at: task.last_run_at,
        latest_result_summary: task.latest_result_summary.clone(),
        latest_failure_category: task.latest_failure_category.clone(),
        latest_transition: task.latest_transition.clone(),
        view_status: task.view_status.clone(),
        attention_reasons: task.attention_reasons.clone(),
        attention_updated_at: task.attention_updated_at,
        active_task_run_id: task.active_task_run_id.clone(),
        analytics_summary: task.analytics_summary.clone(),
        context_bundle: task.context_bundle.clone(),
        latest_deliverable_refs: task.latest_deliverable_refs.clone(),
        latest_artifact_refs: task.latest_artifact_refs.clone(),
        run_history,
        intervention_history: interventions
            .iter()
            .map(task_intervention_from_record)
            .collect(),
        active_run,
        created_by: task.created_by.clone(),
        updated_by: task.updated_by.clone(),
        created_at: task.created_at,
        updated_at: task.updated_at,
    }
}

fn task_prompt_from_record(
    task: &ProjectTaskRecord,
    trigger_label: &str,
    source_task_run_id: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("Task title: {}", task.title),
        format!("Trigger: {trigger_label}"),
        String::new(),
        "Goal:".into(),
        task.goal.clone(),
        String::new(),
        "Brief:".into(),
        task.brief.clone(),
    ];

    if !task.context_bundle.pinned_instructions.trim().is_empty() {
        lines.extend([
            String::new(),
            "Pinned instructions:".into(),
            task.context_bundle.pinned_instructions.clone(),
        ]);
    }

    if !task.context_bundle.refs.is_empty() {
        lines.push(String::new());
        lines.push("Context refs:".into());
        lines.extend(task.context_bundle.refs.iter().map(|reference| {
            format!(
                "- [{}] {} ({})",
                reference.kind, reference.title, reference.ref_id
            )
        }));
    }

    if let Some(source_task_run_id) = source_task_run_id {
        lines.extend([String::new(), format!("Source run: {source_task_run_id}")]);
    }

    lines.join("\n")
}

fn task_run_status_from_runtime(run: &RuntimeRunSnapshot) -> String {
    match run.status.as_str() {
        "queued" | "running" | "waiting_input" | "waiting_approval" | "completed" | "failed"
        | "canceled" | "skipped" => run.status.clone(),
        "auth-required" | "blocked" => "waiting_input".into(),
        _ => "running".into(),
    }
}

fn task_run_pending_approval_id(run: &RuntimeRunSnapshot) -> Option<String> {
    (task_run_status_from_runtime(run) == "waiting_approval")
        .then(|| {
            run.approval_target
                .as_ref()
                .map(|approval| approval.id.clone())
        })
        .flatten()
}

fn build_task_run_record(
    task: &ProjectTaskRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
    trigger_type: &str,
    actor_ref: &str,
) -> ProjectTaskRunRecord {
    let status = task_run_status_from_runtime(run);
    let completed_at = matches!(
        status.as_str(),
        "completed" | "failed" | "canceled" | "skipped"
    )
    .then_some(run.updated_at);
    let failure_category = (status == "failed").then_some("runtime_error".into());
    let failure_summary = (status == "failed").then_some("Runtime task execution failed.".into());
    let attention_reasons = match status.as_str() {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let latest_transition = Some(TaskStateTransitionSummary {
        kind: match status.as_str() {
            "completed" => "completed".into(),
            "failed" => "failed".into(),
            "waiting_approval" => "waiting_approval".into(),
            _ => "launched".into(),
        },
        summary: match status.as_str() {
            "completed" => "Task run completed in the runtime.".into(),
            "failed" => "Task run failed in the runtime.".into(),
            "waiting_approval" => "Task run is waiting for approval.".into(),
            "waiting_input" => "Task run is waiting for input.".into(),
            _ => "Task run started in the runtime.".into(),
        },
        at: completed_at.unwrap_or(run.started_at),
        run_id: Some(run.id.clone()),
    });

    ProjectTaskRunRecord {
        id: format!("task-run-{}", uuid::Uuid::new_v4()),
        workspace_id: task.workspace_id.clone(),
        project_id: task.project_id.clone(),
        task_id: task.id.clone(),
        trigger_type: trigger_type.into(),
        status: status.clone(),
        session_id: Some(session.summary.id.clone()),
        conversation_id: Some(session.summary.conversation_id.clone()),
        runtime_run_id: Some(run.id.clone()),
        actor_ref: actor_ref.into(),
        started_at: run.started_at,
        completed_at,
        result_summary: (status == "completed")
            .then_some("Task run completed in the runtime.".into()),
        pending_approval_id: task_run_pending_approval_id(run),
        failure_category,
        failure_summary,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(completed_at.unwrap_or(run.started_at))
        },
        deliverable_refs: run.deliverable_refs.clone(),
        artifact_refs: Vec::new(),
        latest_transition,
    }
}

fn sync_task_run_record_from_runtime(
    existing: &ProjectTaskRunRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
) -> ProjectTaskRunRecord {
    let status = task_run_status_from_runtime(run);
    let completed_at = matches!(
        status.as_str(),
        "completed" | "failed" | "canceled" | "skipped"
    )
    .then_some(run.updated_at);
    let failure_category = (status == "failed").then_some("runtime_error".into());
    let failure_summary = (status == "failed").then_some("Runtime task execution failed.".into());
    let attention_reasons = match status.as_str() {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let latest_transition = Some(TaskStateTransitionSummary {
        kind: match status.as_str() {
            "completed" => "completed".into(),
            "failed" => "failed".into(),
            "waiting_approval" => "waiting_approval".into(),
            _ => "launched".into(),
        },
        summary: match status.as_str() {
            "completed" => "Task run completed in the runtime.".into(),
            "failed" => "Task run failed in the runtime.".into(),
            "waiting_approval" => "Task run is waiting for approval.".into(),
            "waiting_input" => "Task run is waiting for input.".into(),
            _ => "Task run started in the runtime.".into(),
        },
        at: completed_at.unwrap_or(run.updated_at),
        run_id: Some(run.id.clone()),
    });

    ProjectTaskRunRecord {
        id: existing.id.clone(),
        workspace_id: existing.workspace_id.clone(),
        project_id: existing.project_id.clone(),
        task_id: existing.task_id.clone(),
        trigger_type: existing.trigger_type.clone(),
        status,
        session_id: Some(session.summary.id.clone()),
        conversation_id: Some(session.summary.conversation_id.clone()),
        runtime_run_id: Some(run.id.clone()),
        actor_ref: if run.actor_ref.trim().is_empty() {
            existing.actor_ref.clone()
        } else {
            run.actor_ref.clone()
        },
        started_at: run.started_at,
        completed_at,
        result_summary: (run.status == "completed")
            .then_some("Task run completed in the runtime.".into()),
        pending_approval_id: task_run_pending_approval_id(run),
        failure_category,
        failure_summary,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(completed_at.unwrap_or(run.updated_at))
        },
        deliverable_refs: run.deliverable_refs.clone(),
        artifact_refs: existing.artifact_refs.clone(),
        latest_transition,
    }
}

fn sync_rejected_task_run_record_from_runtime(
    existing: &ProjectTaskRunRecord,
    session: &octopus_core::RuntimeSessionDetail,
    run: &RuntimeRunSnapshot,
) -> ProjectTaskRunRecord {
    let mut synced = sync_task_run_record_from_runtime(existing, session, run);
    if synced.status == "waiting_input" {
        synced.result_summary = Some("Approval rejected. Waiting for updated guidance.".into());
    }
    synced
}

fn task_run_duration_ms(run: &ProjectTaskRunRecord) -> u64 {
    run.completed_at
        .unwrap_or(run.started_at)
        .saturating_sub(run.started_at)
}

fn update_task_analytics_from_run(
    analytics: &TaskAnalyticsSummary,
    run: &ProjectTaskRunRecord,
) -> TaskAnalyticsSummary {
    let mut updated = analytics.clone();
    updated.run_count = updated.run_count.saturating_add(1);
    match run.trigger_type.as_str() {
        "manual" => updated.manual_run_count = updated.manual_run_count.saturating_add(1),
        "scheduled" => updated.scheduled_run_count = updated.scheduled_run_count.saturating_add(1),
        "takeover" => updated.takeover_count = updated.takeover_count.saturating_add(1),
        _ => {}
    }
    if run.status == "completed" {
        updated.completion_count = updated.completion_count.saturating_add(1);
        updated.last_successful_run_at = run.completed_at.or(Some(run.started_at));
    }
    if run.status == "failed" {
        updated.failure_count = updated.failure_count.saturating_add(1);
    }
    if run.status == "waiting_approval" {
        updated.approval_required_count = updated.approval_required_count.saturating_add(1);
    }
    let duration_ms = run
        .completed_at
        .unwrap_or(run.started_at)
        .saturating_sub(run.started_at);
    if updated.run_count == 0 {
        updated.average_run_duration_ms = duration_ms;
    } else {
        let previous_total = analytics
            .average_run_duration_ms
            .saturating_mul(analytics.run_count);
        updated.average_run_duration_ms = previous_total
            .saturating_add(duration_ms)
            .saturating_div(updated.run_count.max(1));
    }
    updated
}

fn sync_task_analytics_from_run(
    analytics: &TaskAnalyticsSummary,
    previous_run: &ProjectTaskRunRecord,
    run: &ProjectTaskRunRecord,
) -> TaskAnalyticsSummary {
    let mut updated = analytics.clone();
    if previous_run.status != "completed" && run.status == "completed" {
        updated.completion_count = updated.completion_count.saturating_add(1);
        updated.last_successful_run_at = run.completed_at.or(Some(run.started_at));
    }
    if previous_run.status != "failed" && run.status == "failed" {
        updated.failure_count = updated.failure_count.saturating_add(1);
    }
    if previous_run.status != "waiting_approval" && run.status == "waiting_approval" {
        updated.approval_required_count = updated.approval_required_count.saturating_add(1);
    }
    let run_count = updated.run_count.max(1);
    let previous_total = analytics.average_run_duration_ms.saturating_mul(run_count);
    updated.average_run_duration_ms = previous_total
        .saturating_sub(task_run_duration_ms(previous_run))
        .saturating_add(task_run_duration_ms(run))
        .saturating_div(run_count);
    updated
}

fn update_task_record_from_run(
    task: &ProjectTaskRecord,
    run: &ProjectTaskRunRecord,
    updated_by: &str,
) -> ProjectTaskRecord {
    let attention_reasons = run.attention_reasons.clone();
    let updated_at = run.completed_at.unwrap_or(run.started_at);
    ProjectTaskRecord {
        status: match run.status.as_str() {
            "completed" => "completed".into(),
            "failed" | "waiting_approval" | "waiting_input" => "attention".into(),
            _ => "running".into(),
        },
        last_run_at: Some(run.started_at),
        active_task_run_id: Some(run.id.clone()),
        latest_result_summary: run.result_summary.clone(),
        latest_failure_category: run.failure_category.clone(),
        latest_transition: run.latest_transition.clone(),
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(updated_at)
        },
        analytics_summary: update_task_analytics_from_run(&task.analytics_summary, run),
        latest_deliverable_refs: run.deliverable_refs.clone(),
        latest_artifact_refs: run.artifact_refs.clone(),
        updated_by: Some(updated_by.into()),
        updated_at,
        ..task.clone()
    }
}

fn sync_task_record_from_run(
    task: &ProjectTaskRecord,
    previous_run: &ProjectTaskRunRecord,
    run: &ProjectTaskRunRecord,
    updated_by: &str,
) -> ProjectTaskRecord {
    let attention_reasons = run.attention_reasons.clone();
    let updated_at = run.completed_at.unwrap_or(run.started_at);
    ProjectTaskRecord {
        status: match run.status.as_str() {
            "completed" => "completed".into(),
            "failed" | "waiting_approval" | "waiting_input" => "attention".into(),
            _ => "running".into(),
        },
        last_run_at: Some(run.started_at),
        active_task_run_id: Some(run.id.clone()),
        latest_result_summary: run.result_summary.clone(),
        latest_failure_category: run.failure_category.clone(),
        latest_transition: run.latest_transition.clone(),
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_reasons: attention_reasons.clone(),
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(updated_at)
        },
        analytics_summary: sync_task_analytics_from_run(&task.analytics_summary, previous_run, run),
        latest_deliverable_refs: run.deliverable_refs.clone(),
        latest_artifact_refs: run.artifact_refs.clone(),
        updated_by: Some(updated_by.into()),
        updated_at,
        ..task.clone()
    }
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

pub(crate) async fn list_project_tasks(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<TaskSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        None,
    )
    .await?;
    Ok(Json(
        state
            .services
            .project_tasks
            .list_tasks(&project_id)
            .await?
            .iter()
            .map(task_summary_from_record)
            .collect(),
    ))
}

pub(crate) async fn create_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<TaskDetail>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.manage",
        Some(&project_id),
        Some("task"),
        None,
    )
    .await?;
    let request = validate_create_task_request(request)?;
    let task = state
        .services
        .project_tasks
        .create_task(&project_id, &session.user_id, request)
        .await?;
    Ok(Json(task_detail_from_records(&task, &[], &[])))
}

pub(crate) async fn get_project_task_detail(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskDetail>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let runs = state
        .services
        .project_tasks
        .list_task_runs(&project_id, &task_id)
        .await?;
    let interventions = state
        .services
        .project_tasks
        .list_task_interventions(&project_id, &task_id)
        .await?;
    Ok(Json(task_detail_from_records(&task, &runs, &interventions)))
}

pub(crate) async fn update_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<UpdateTaskRequest>,
) -> Result<Json<TaskDetail>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.manage",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let request = validate_update_task_request(request)?;
    let task = state
        .services
        .project_tasks
        .update_task(&project_id, &task_id, &session.user_id, request)
        .await?;
    let runs = state
        .services
        .project_tasks
        .list_task_runs(&project_id, &task_id)
        .await?;
    let interventions = state
        .services
        .project_tasks
        .list_task_interventions(&project_id, &task_id)
        .await?;
    Ok(Json(task_detail_from_records(&task, &runs, &interventions)))
}

pub(crate) async fn launch_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<LaunchTaskRequest>,
) -> Result<Json<TaskRunSummary>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.run",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let actor_ref = trim_optional_task_input(request.actor_ref)
        .unwrap_or_else(|| task.default_actor_ref.clone());
    if actor_ref.is_empty() {
        return Err(AppError::invalid_input("task actor is required").into());
    }
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, Some(&project_id)).await?;
    let runtime_session = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(
            CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some(project_id.clone()),
                title: task.title.clone(),
                session_kind: Some("task".into()),
                selected_actor_ref: actor_ref.clone(),
                selected_configured_model_id: None,
                execution_permission_mode: owner_permission_ceiling.clone(),
            },
            &session.user_id,
            Some(&owner_permission_ceiling),
        )
        .await?;
    let runtime_run = state
        .services
        .runtime_execution
        .submit_turn(
            &runtime_session.summary.id,
            SubmitRuntimeTurnInput {
                content: task_prompt_from_record(&task, "manual", None),
                permission_mode: None,
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await?;
    let run = state
        .services
        .project_tasks
        .save_task_run(build_task_run_record(
            &task,
            &runtime_session,
            &runtime_run,
            "manual",
            &actor_ref,
        ))
        .await?;
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(&task, &run, &session.user_id))
        .await?;
    Ok(Json(task_run_summary_from_record(&run)))
}

pub(crate) async fn rerun_project_task(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<RerunTaskRequest>,
) -> Result<Json<TaskRunSummary>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.run",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    let task = state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    let actor_ref = trim_optional_task_input(request.actor_ref)
        .unwrap_or_else(|| task.default_actor_ref.clone());
    let source_task_run_id = trim_optional_task_input(request.source_task_run_id);
    let owner_permission_ceiling =
        derive_runtime_owner_permission_ceiling(&state, &session, Some(&project_id)).await?;
    let runtime_session = state
        .services
        .runtime_session
        .create_session_with_owner_ceiling(
            CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some(project_id.clone()),
                title: format!("{} rerun", task.title),
                session_kind: Some("task".into()),
                selected_actor_ref: actor_ref.clone(),
                selected_configured_model_id: None,
                execution_permission_mode: owner_permission_ceiling.clone(),
            },
            &session.user_id,
            Some(&owner_permission_ceiling),
        )
        .await?;
    let runtime_run = state
        .services
        .runtime_execution
        .submit_turn(
            &runtime_session.summary.id,
            SubmitRuntimeTurnInput {
                content: task_prompt_from_record(&task, "rerun", source_task_run_id.as_deref()),
                permission_mode: None,
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await?;
    let run = state
        .services
        .project_tasks
        .save_task_run(build_task_run_record(
            &task,
            &runtime_session,
            &runtime_run,
            "rerun",
            &actor_ref,
        ))
        .await?;
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(&task, &run, &session.user_id))
        .await?;
    Ok(Json(task_run_summary_from_record(&run)))
}

pub(crate) async fn list_project_task_runs(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<TaskRunSummary>>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "task.view",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    state
        .services
        .project_tasks
        .get_task(&project_id, &task_id)
        .await?;
    Ok(Json(
        state
            .services
            .project_tasks
            .list_task_runs(&project_id, &task_id)
            .await?
            .iter()
            .map(task_run_summary_from_record)
            .collect(),
    ))
}

pub(crate) async fn create_project_task_intervention(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, task_id)): Path<(String, String)>,
    Json(request): Json<CreateTaskInterventionRequest>,
) -> Result<Json<TaskInterventionRecord>, ApiError> {
    let session = ensure_capability_session(
        &state,
        &headers,
        "task.intervene",
        Some(&project_id),
        Some("task"),
        Some(&task_id),
    )
    .await?;
    if request.r#type.trim().is_empty() {
        return Err(AppError::invalid_input("task intervention type is required").into());
    }
    let intervention_type = request.r#type.trim();
    let explicit_approval_id = matches!(intervention_type, "approve" | "reject")
        .then(|| trim_optional_task_input(request.approval_id.clone()))
        .flatten();
    let mut runtime_synced_run = None;
    if matches!(intervention_type, "approve" | "reject") {
        let task = state
            .services
            .project_tasks
            .get_task(&project_id, &task_id)
            .await?;
        let target_run_id = trim_optional_task_input(request.task_run_id.clone())
            .or_else(|| task.active_task_run_id.clone());
        if let Some(target_run_id) = target_run_id {
            if let Some(target_run) = state
                .services
                .project_tasks
                .list_task_runs(&project_id, &task_id)
                .await?
                .into_iter()
                .find(|run| run.id == target_run_id)
            {
                if let Some(session_id) = target_run.session_id.as_deref() {
                    let runtime_session =
                        match state.services.runtime_session.get_session(session_id).await {
                            Ok(detail) => Some(detail),
                            Err(AppError::NotFound(_)) => None,
                            Err(error) => return Err(error.into()),
                        };
                    if let Some(runtime_session) = runtime_session {
                        if let Some(approval_id) = explicit_approval_id.clone().or_else(|| {
                            runtime_session
                                .pending_approval
                                .as_ref()
                                .map(|approval| approval.id.clone())
                        }) {
                            let previous_run = target_run.clone();
                            let runtime_run = state
                                .services
                                .runtime_execution
                                .resolve_approval(
                                    session_id,
                                    &approval_id,
                                    octopus_core::ResolveRuntimeApprovalInput {
                                        decision: if intervention_type == "approve" {
                                            "approve".into()
                                        } else {
                                            "reject".into()
                                        },
                                    },
                                )
                                .await?;
                            let refreshed_session = state
                                .services
                                .runtime_session
                                .get_session(session_id)
                                .await?;
                            runtime_synced_run = Some((
                                previous_run.clone(),
                                if intervention_type == "approve" {
                                    sync_task_run_record_from_runtime(
                                        &previous_run,
                                        &refreshed_session,
                                        &runtime_run,
                                    )
                                } else {
                                    sync_rejected_task_run_record_from_runtime(
                                        &previous_run,
                                        &refreshed_session,
                                        &runtime_run,
                                    )
                                },
                            ));
                        }
                    }
                }
            }
        }
        if runtime_synced_run.is_none() {
            if let Some(approval_id) = explicit_approval_id.as_deref() {
                return Err(AppError::conflict(format!(
                    "task approval `{approval_id}` could not be resolved in runtime"
                ))
                .into());
            }
        }
    }
    let record = state
        .services
        .project_tasks
        .create_task_intervention(&project_id, &task_id, &session.user_id, request)
        .await?;
    if let Some((previous_run, run)) = runtime_synced_run {
        let task = state
            .services
            .project_tasks
            .get_task(&project_id, &task_id)
            .await?;
        let run = state.services.project_tasks.save_task_run(run).await?;
        state
            .services
            .project_tasks
            .save_task(sync_task_record_from_run(
                &task,
                &previous_run,
                &run,
                &session.user_id,
            ))
            .await?;
    }
    Ok(Json(task_intervention_from_record(&record)))
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

pub(crate) async fn current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        state
            .services
            .workspace
            .current_user_profile(&session.user_id)
            .await?,
    ))
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
    let session =
        ensure_capability_session(&state, &headers, "inbox.view", None, Some("inbox"), None)
            .await?;
    Ok(Json(visible_inbox_items(
        &session.user_id,
        state.services.inbox.list_inbox().await?,
    )))
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

pub(crate) async fn run_runtime_generation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(mut input): Json<RunRuntimeGenerationInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    normalize_runtime_generation_input(&mut input)?;
    let project_id = input
        .project_id
        .as_deref()
        .and_then(normalize_project_scope)
        .map(str::to_string);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.submit_turn",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let result: RuntimeGenerationResult = state
        .services
        .runtime_execution
        .run_generation(
            RunRuntimeGenerationInput {
                project_id,
                ..input
            },
            &session.user_id,
        )
        .await?;
    let payload = runtime_transport_payload(&result, &request_id)?;
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
    use std::{
        collections::HashMap,
        fs,
        path::Path,
        sync::{Arc, Mutex},
    };

    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request, StatusCode},
    };
    use octopus_core::{
        default_connection_stubs, default_host_state, default_preferences, AccessUserUpsertRequest,
        CreateProjectDeletionRequestInput, CreateProjectRequest, CreateRuntimeSessionInput,
        CreateTaskInterventionRequest, CreateTaskRequest, DataPolicyUpsertRequest,
        DesktopBackendConnection, LaunchTaskRequest, LoginRequest, ProjectDeletionRequest,
        ProjectPermissionOverrides, RegisterBootstrapAdminRequest, RerunTaskRequest,
        ReviewProjectDeletionRequestInput, RoleBindingUpsertRequest, RoleUpsertRequest,
        SubmitRuntimeTurnInput, TaskContextBundle, TaskContextRef, UpdateWorkspaceRequest,
        WorkspaceSummary, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
    };
    use octopus_infra::build_infra_bundle;
    use octopus_platform::PlatformServices;
    use octopus_runtime_adapter::{MockRuntimeModelDriver, RuntimeAdapter};
    use rusqlite::{params, Connection};
    use serde_json::{json, Value};
    use tower::ServiceExt;

    const APPROVAL_AGENT_ID: &str = "agent-task-runtime-approval";
    const APPROVAL_AGENT_REF: &str = "agent:agent-task-runtime-approval";
    const CHAINED_APPROVAL_TEAM_REF: &str = "team:team-spawn-workflow-approval";

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

    fn avatar_payload() -> octopus_core::AvatarUploadPayload {
        octopus_core::AvatarUploadPayload {
            file_name: "avatar.png".into(),
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            byte_size: 8,
        }
    }

    fn update_request_from_project(project: ProjectRecord) -> UpdateProjectRequest {
        UpdateProjectRequest {
            name: project.name,
            description: project.description,
            status: project.status,
            resource_directory: project.resource_directory,
            owner_user_id: Some(project.owner_user_id),
            member_user_ids: Some(project.member_user_ids),
            permission_overrides: Some(project.permission_overrides),
            leader_agent_id: project.leader_agent_id,
            manager_user_id: project.manager_user_id,
            preset_code: project.preset_code,
            linked_workspace_assets: None,
            assignments: None,
        }
    }

    fn project_scoped_agent_input(
        record: &octopus_core::AgentRecord,
        project_id: &str,
    ) -> octopus_core::UpsertAgentInput {
        octopus_core::UpsertAgentInput {
            workspace_id: record.workspace_id.clone(),
            project_id: Some(project_id.into()),
            scope: "project".into(),
            name: format!("{} Project Copy", record.name),
            avatar: None,
            remove_avatar: None,
            personality: record.personality.clone(),
            tags: record.tags.clone(),
            prompt: record.prompt.clone(),
            builtin_tool_keys: record.builtin_tool_keys.clone(),
            skill_ids: record.skill_ids.clone(),
            mcp_server_names: record.mcp_server_names.clone(),
            task_domains: record.task_domains.clone(),
            default_model_strategy: Some(record.default_model_strategy.clone()),
            capability_policy: Some(record.capability_policy.clone()),
            permission_envelope: Some(record.permission_envelope.clone()),
            memory_policy: Some(record.memory_policy.clone()),
            delegation_policy: Some(record.delegation_policy.clone()),
            approval_preference: Some(record.approval_preference.clone()),
            output_contract: Some(record.output_contract.clone()),
            shared_capability_policy: Some(record.shared_capability_policy.clone()),
            description: record.description.clone(),
            status: "active".into(),
        }
    }

    fn auth_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).expect("bearer header"),
        );
        headers.insert(
            "x-workspace-id",
            HeaderValue::from_static(DEFAULT_WORKSPACE_ID),
        );
        headers
    }

    async fn visible_workspace_agent_actor_ref(state: &ServerState) -> String {
        let agent = state
            .services
            .workspace
            .list_agents()
            .await
            .expect("list agents")
            .into_iter()
            .find(|record| {
                record.project_id.is_none()
                    && record.status == "active"
                    && agent_visible_in_generic_catalog(record)
            })
            .expect("workspace agent");
        format!("agent:{}", agent.id)
    }

    fn test_server_state(root: &Path) -> ServerState {
        let infra = build_infra_bundle(root).expect("infra bundle");
        let runtime = Arc::new(RuntimeAdapter::new_with_executor(
            DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
        ));
        let services = PlatformServices {
            workspace: infra.workspace.clone(),
            project_tasks: infra.workspace.clone(),
            access_control: infra.access_control.clone(),
            auth: infra.auth.clone(),
            app_registry: infra.app_registry.clone(),
            authorization: infra.authorization.clone(),
            runtime_session: runtime.clone(),
            runtime_execution: runtime.clone(),
            runtime_config: runtime.clone(),
            runtime_registry: runtime,
            artifact: infra.artifact.clone(),
            inbox: infra.inbox.clone(),
            knowledge: infra.knowledge.clone(),
            observation: infra.observation.clone(),
        };

        ServerState {
            services,
            host_auth_token: "host-test-token".into(),
            transport_security: "loopback".into(),
            idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
            auth_rate_limits: Arc::new(Mutex::new(HashMap::new())),
            host_state: default_host_state("0.1.0-test".into(), true),
            host_connections: default_connection_stubs(),
            host_preferences_path: root.join("config").join("shell-preferences.json"),
            host_workspace_connections_path: root
                .join("config")
                .join("shell-workspace-connections.json"),
            host_default_preferences: default_preferences(DEFAULT_WORKSPACE_ID, DEFAULT_PROJECT_ID),
            backend_connection: DesktopBackendConnection {
                base_url: Some("http://127.0.0.1:43127".into()),
                auth_token: Some("desktop-test-token".into()),
                state: "ready".into(),
                transport: "http".into(),
            },
        }
    }

    async fn bootstrap_owner(state: &ServerState) -> SessionRecord {
        state
            .services
            .auth
            .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
                mapped_directory: None,
            })
            .await
            .expect("bootstrap admin")
            .session
    }

    async fn create_user_session(
        state: &ServerState,
        username: &str,
        display_name: &str,
    ) -> SessionRecord {
        state
            .services
            .access_control
            .create_user(AccessUserUpsertRequest {
                username: username.into(),
                display_name: display_name.into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            })
            .await
            .expect("create user");
        state
            .services
            .auth
            .login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: username.into(),
                password: "password123".into(),
                workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
            })
            .await
            .expect("login user")
            .session
    }

    async fn seed_task_run(
        state: &ServerState,
        task: &ProjectTaskRecord,
        user_id: &str,
        status: &str,
    ) -> ProjectTaskRunRecord {
        let started_at = timestamp_now();
        let attention_reasons = match status {
            "waiting_approval" => vec!["needs_approval".into()],
            "waiting_input" => vec!["waiting_input".into()],
            "failed" => vec!["failed".into()],
            _ => Vec::new(),
        };
        let run = ProjectTaskRunRecord {
            id: format!("task-run-{}-{status}", task.id),
            workspace_id: task.workspace_id.clone(),
            project_id: task.project_id.clone(),
            task_id: task.id.clone(),
            trigger_type: "manual".into(),
            status: status.into(),
            session_id: Some(format!("runtime-session-{}-{status}", task.id)),
            conversation_id: Some(format!("conversation-{}-{status}", task.id)),
            runtime_run_id: Some(format!("runtime-run-{}-{status}", task.id)),
            actor_ref: task.default_actor_ref.clone(),
            started_at,
            completed_at: None,
            result_summary: None,
            pending_approval_id: (status == "waiting_approval")
                .then_some(format!("approval-task-run-{}-{status}", task.id)),
            failure_category: None,
            failure_summary: None,
            view_status: if attention_reasons.is_empty() {
                "healthy".into()
            } else {
                "attention".into()
            },
            attention_updated_at: if attention_reasons.is_empty() {
                None
            } else {
                Some(started_at)
            },
            attention_reasons,
            deliverable_refs: Vec::new(),
            artifact_refs: Vec::new(),
            latest_transition: Some(TaskStateTransitionSummary {
                kind: status.into(),
                summary: match status {
                    "waiting_approval" => "Task run is waiting for approval.".into(),
                    "waiting_input" => "Task run is waiting for input.".into(),
                    "failed" => "Task run failed in the runtime.".into(),
                    "completed" => "Task run completed in the runtime.".into(),
                    _ => "Task run started in the runtime.".into(),
                },
                at: started_at,
                run_id: Some(format!("runtime-run-{}-{status}", task.id)),
            }),
        };
        state
            .services
            .project_tasks
            .save_task_run(run.clone())
            .await
            .expect("save seeded task run");
        state
            .services
            .project_tasks
            .save_task(update_task_record_from_run(task, &run, user_id))
            .await
            .expect("save seeded task projection");
        run
    }

    fn write_runtime_workspace_config(root: &Path) {
        std::env::set_var(
            "OCTOPUS_TEST_ANTHROPIC_API_KEY",
            "test-octopus-server-anthropic-key",
        );
        let path = root.join("config").join("runtime").join("workspace.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("runtime config dir");
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "configuredModels": {
                    "opus": {
                        "configuredModelId": "opus",
                        "name": "Opus",
                        "providerId": "anthropic",
                        "modelId": "claude-opus-4-6",
                        "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    },
                    "quota-model": {
                        "configuredModelId": "quota-model",
                        "name": "Quota Model",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }))
            .expect("runtime config json"),
        )
        .expect("write runtime config");
    }

    fn write_runtime_workspace_config_with_generation_model(root: &Path) {
        std::env::set_var(
            "OCTOPUS_TEST_GOOGLE_API_KEY",
            "test-octopus-server-google-key",
        );
        let path = root.join("config").join("runtime").join("workspace.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("runtime config dir");
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "configuredModels": {
                    "generation-only-model": {
                        "configuredModelId": "generation-only-model",
                        "name": "Generation Only Model",
                        "providerId": "google",
                        "modelId": "gemini-2.5-flash",
                        "credentialRef": "env:OCTOPUS_TEST_GOOGLE_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }))
            .expect("runtime config json"),
        )
        .expect("write runtime config");
    }

    #[tokio::test]
    async fn workspace_summary_route_returns_persisted_mapped_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let mapped_root = temp.path().to_string_lossy().to_string();

        let session = state
            .services
            .auth
            .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
                mapped_directory: Some(mapped_root.clone()),
            })
            .await
            .expect("bootstrap admin")
            .session;

        let app = crate::routes::build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace")
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("workspace summary response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("workspace summary body");
        let workspace: WorkspaceSummary =
            serde_json::from_slice(&body).expect("workspace summary json");
        assert_eq!(
            workspace.mapped_directory.as_deref(),
            Some(mapped_root.as_str())
        );
        assert_eq!(
            workspace.mapped_directory_default.as_deref(),
            Some(mapped_root.as_str())
        );
    }

    #[tokio::test]
    async fn workspace_summary_patch_route_updates_workspace_settings() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let current_root = temp.path().to_string_lossy().to_string();
        let app = crate::routes::build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/api/v1/workspace")
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&UpdateWorkspaceRequest {
                            name: Some("Workspace Rebuilt".into()),
                            avatar: None,
                            remove_avatar: Some(true),
                            mapped_directory: Some(current_root.clone()),
                        })
                        .expect("workspace update json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("workspace update response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("workspace update body");
        let workspace: WorkspaceSummary =
            serde_json::from_slice(&body).expect("workspace update json");
        assert_eq!(workspace.name, "Workspace Rebuilt");
        assert_eq!(
            workspace.mapped_directory.as_deref(),
            Some(current_root.as_str())
        );
    }

    #[tokio::test]
    async fn workspace_summary_patch_route_moves_workspace_root_and_preserves_shell_root_pointer() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let mapped_root = temp
            .path()
            .parent()
            .expect("temp parent")
            .join(format!("octopus-mapped-root-{}", uuid::Uuid::new_v4()));
        let mapped_root_string = mapped_root.to_string_lossy().to_string();
        let shell_root_string = temp.path().to_string_lossy().to_string();
        let app = crate::routes::build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/api/v1/workspace")
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&UpdateWorkspaceRequest {
                            name: Some("Workspace Moved".into()),
                            avatar: None,
                            remove_avatar: Some(false),
                            mapped_directory: Some(mapped_root_string.clone()),
                        })
                        .expect("workspace update json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("workspace update response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(mapped_root.join("data").join("main.db").exists());
        assert!(!temp.path().join("data").join("main.db").exists());

        let shell_pointer = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
            .expect("shell pointer workspace config");
        assert!(shell_pointer.contains(mapped_root_string.as_str()));

        let reloaded = test_server_state(&mapped_root);
        let workspace = reloaded
            .services
            .workspace
            .workspace_summary()
            .await
            .expect("reloaded workspace summary");
        assert_eq!(workspace.name, "Workspace Moved");
        assert_eq!(
            workspace.mapped_directory.as_deref(),
            Some(mapped_root_string.as_str())
        );
        assert_eq!(
            workspace.mapped_directory_default.as_deref(),
            Some(shell_root_string.as_str())
        );
    }

    #[tokio::test]
    async fn personal_center_profile_route_returns_stored_avatar_summary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let app = crate::routes::build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace/personal-center/profile")
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("profile response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("profile body");
        let profile: octopus_core::UserRecordSummary =
            serde_json::from_slice(&body).expect("profile json");
        assert_eq!(profile.id, session.user_id);
        assert_eq!(
            profile.avatar.as_deref(),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );
    }

    #[tokio::test]
    async fn project_delete_request_routes_create_and_list_archived_project_requests() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        let app = crate::routes::build_router(state.clone());

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Delete Governed Project".into(),
                description: "Deletion request route coverage.".into(),
                resource_directory: "data/projects/delete-governed-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Retired project".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("create deletion request body");
        let created: ProjectDeletionRequest =
            serde_json::from_slice(&body).expect("project deletion request json");
        assert_eq!(created.project_id, project.id);
        assert_eq!(created.requested_by_user_id, session.user_id);
        assert_eq!(created.status, "pending");
        assert_eq!(created.reason.as_deref(), Some("Retired project"));
        let inbox_items = state.services.inbox.list_inbox().await.expect("list inbox");
        let inbox_item = inbox_items
            .iter()
            .find(|item| {
                item.project_id.as_deref() == Some(project.id.as_str())
                    && item.item_type == "project-deletion-request"
                    && item.target_user_id == session.user_id
            })
            .expect("project deletion request inbox item");
        assert_eq!(
            inbox_item.route_to.as_deref(),
            Some(
                format!(
                    "/workspaces/{}/projects/{}/settings",
                    DEFAULT_WORKSPACE_ID, project.id
                )
                .as_str()
            )
        );
        assert_eq!(inbox_item.action_label.as_deref(), Some("Review approval"));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("list deletion requests response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("list deletion requests body");
        let listed: Vec<ProjectDeletionRequest> =
            serde_json::from_slice(&body).expect("project deletion request list json");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].status, "pending");
        assert_eq!(headers.get("x-workspace-id").is_some(), true);
    }

    #[tokio::test]
    async fn project_delete_request_approve_route_records_reviewer_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let app = crate::routes::build_router(state.clone());

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Approve Delete Project".into(),
                description: "Deletion approval route coverage.".into(),
                resource_directory: "data/projects/approve-delete-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Sunset flow".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("create deletion request body");
        let created: ProjectDeletionRequest =
            serde_json::from_slice(&create_body).expect("project deletion request json");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/projects/{}/deletion-requests/{}/approve",
                        project.id, created.id
                    ))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                            review_comment: Some("Approved for cleanup".into()),
                        })
                        .expect("approve deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("approve deletion request response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("approve deletion request body");
        let approved: ProjectDeletionRequest =
            serde_json::from_slice(&body).expect("approved deletion request json");
        assert_eq!(approved.status, "approved");
        assert_eq!(
            approved.reviewed_by_user_id.as_deref(),
            Some(session.user_id.as_str())
        );
        assert_eq!(
            approved.review_comment.as_deref(),
            Some("Approved for cleanup")
        );
        assert!(approved.reviewed_at.is_some());
    }

    #[tokio::test]
    async fn project_delete_request_reject_route_records_reviewer_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let app = crate::routes::build_router(state.clone());

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Reject Delete Project".into(),
                description: "Deletion rejection route coverage.".into(),
                resource_directory: "data/projects/reject-delete-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Rejected path".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("create deletion request body");
        let created: ProjectDeletionRequest =
            serde_json::from_slice(&create_body).expect("project deletion request json");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/projects/{}/deletion-requests/{}/reject",
                        project.id, created.id
                    ))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                            review_comment: Some("Need to retain project history".into()),
                        })
                        .expect("reject deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("reject deletion request response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reject deletion request body");
        let rejected: ProjectDeletionRequest =
            serde_json::from_slice(&body).expect("rejected deletion request json");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(
            rejected.reviewed_by_user_id.as_deref(),
            Some(session.user_id.as_str())
        );
        assert_eq!(
            rejected.review_comment.as_deref(),
            Some("Need to retain project history")
        );
        assert!(rejected.reviewed_at.is_some());
    }

    #[tokio::test]
    async fn project_delete_request_delete_route_requires_archived_approved_project() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let app = crate::routes::build_router(state.clone());

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Delete Project Route".into(),
                description: "Deletion route guard coverage.".into(),
                resource_directory: "data/projects/delete-project-route/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/v1/projects/{}", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("delete active project response");
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/v1/projects/{}", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("delete archived project response");
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Final cleanup".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("create deletion request body");
        let created: ProjectDeletionRequest =
            serde_json::from_slice(&create_body).expect("project deletion request json");

        let approve_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/projects/{}/deletion-requests/{}/approve",
                        project.id, created.id
                    ))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                            review_comment: Some("Ready to delete".into()),
                        })
                        .expect("approve deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("approve deletion request response");
        assert_eq!(approve_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/v1/projects/{}", project.id))
                    .header("authorization", format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("delete approved project response");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let projects = state
            .services
            .workspace
            .list_projects()
            .await
            .expect("list projects");
        assert!(!projects.iter().any(|record| record.id == project.id));
    }

    #[tokio::test]
    async fn project_delete_request_approve_route_allows_project_scoped_admin_reviewers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let owner_session = bootstrap_owner(&state).await;
        let approver_session = create_user_session(&state, "project-admin", "Project Admin").await;
        let app = crate::routes::build_router(state.clone());

        let project_admin_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.project-delete-reviewer".into(),
                name: "Project Delete Reviewer".into(),
                description: "Can approve project deletion for selected projects.".into(),
                status: "active".into(),
                permission_codes: vec!["project.manage".into()],
            })
            .await
            .expect("create project admin role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: project_admin_role.id,
                subject_type: "user".into(),
                subject_id: approver_session.user_id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind project admin role");

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Scoped Admin Delete Project".into(),
                description: "Deletion approval by scoped admin.".into(),
                resource_directory: "data/projects/scoped-admin-delete-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "project delete reviewer scope".into(),
                subject_type: "user".into(),
                subject_id: approver_session.user_id.clone(),
                resource_type: "project".into(),
                scope_type: "selected-projects".into(),
                project_ids: vec![project.id.clone()],
                tags: Vec::new(),
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create scoped data policy");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", owner_session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Scoped admin should review".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        assert_eq!(create_response.status(), StatusCode::OK);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("create deletion request body");
        let created: ProjectDeletionRequest =
            serde_json::from_slice(&create_body).expect("project deletion request json");

        let approve_response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/projects/{}/deletion-requests/{}/approve",
                        project.id, created.id
                    ))
                    .header(
                        "authorization",
                        format!("Bearer {}", approver_session.token),
                    )
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ReviewProjectDeletionRequestInput {
                            review_comment: Some("Scoped admin approved".into()),
                        })
                        .expect("approve deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("approve deletion request response");

        assert_eq!(approve_response.status(), StatusCode::OK);
        let approve_body = to_bytes(approve_response.into_body(), usize::MAX)
            .await
            .expect("approve deletion request body");
        let approved: ProjectDeletionRequest =
            serde_json::from_slice(&approve_body).expect("approved deletion request json");
        assert_eq!(
            approved.reviewed_by_user_id.as_deref(),
            Some(approver_session.user_id.as_str())
        );
        assert_eq!(approved.status, "approved");
    }

    #[tokio::test]
    async fn project_delete_request_list_route_allows_project_scoped_admin_reviewers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let owner_session = bootstrap_owner(&state).await;
        let reviewer_session =
            create_user_session(&state, "project-reviewer", "Project Reviewer").await;
        let app = crate::routes::build_router(state.clone());

        let reviewer_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.project-delete-list-reviewer".into(),
                name: "Project Delete List Reviewer".into(),
                description: "Can review scoped project deletions.".into(),
                status: "active".into(),
                permission_codes: vec!["project.manage".into()],
            })
            .await
            .expect("create reviewer role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: reviewer_role.id,
                subject_type: "user".into(),
                subject_id: reviewer_session.user_id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind reviewer role");

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Scoped Reviewer Delete Project".into(),
                description: "Deletion list review by scoped reviewer.".into(),
                resource_directory: "data/projects/scoped-reviewer-delete-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "project delete list reviewer scope".into(),
                subject_type: "user".into(),
                subject_id: reviewer_session.user_id.clone(),
                resource_type: "project".into(),
                scope_type: "selected-projects".into(),
                project_ids: vec![project.id.clone()],
                tags: Vec::new(),
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create reviewer policy");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", owner_session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Scoped reviewer should list".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        assert_eq!(create_response.status(), StatusCode::OK);

        let list_response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header(
                        "authorization",
                        format!("Bearer {}", reviewer_session.token),
                    )
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("list deletion requests response");

        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .expect("list deletion requests body");
        let listed: Vec<ProjectDeletionRequest> =
            serde_json::from_slice(&list_body).expect("project deletion request list json");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].project_id, project.id);
        assert_eq!(listed[0].status, "pending");
    }

    #[tokio::test]
    async fn inbox_route_returns_only_current_users_project_delete_items() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let owner_session = bootstrap_owner(&state).await;
        let approver_session =
            create_user_session(&state, "inbox-approver", "Inbox Approver").await;
        let app = crate::routes::build_router(state.clone());

        let project = state
            .services
            .workspace
            .create_project(CreateProjectRequest {
                name: "Inbox Scoped Delete Project".into(),
                description: "Targeted inbox route coverage.".into(),
                resource_directory: "data/projects/inbox-scoped-delete-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            })
            .await
            .expect("created project");
        let mut archive_request = update_request_from_project(project.clone());
        archive_request.status = "archived".into();
        state
            .services
            .workspace
            .update_project(&project.id, archive_request)
            .await
            .expect("archived project");
        let inbox_reviewer_role = state
            .services
            .access_control
            .create_role(RoleUpsertRequest {
                code: "custom.project-delete-inbox-reviewer".into(),
                name: "Project Delete Inbox Reviewer".into(),
                description: "Can review scoped project deletions and read inbox.".into(),
                status: "active".into(),
                permission_codes: vec!["project.manage".into(), "inbox.view".into()],
            })
            .await
            .expect("create inbox reviewer role");
        state
            .services
            .access_control
            .create_role_binding(RoleBindingUpsertRequest {
                role_id: inbox_reviewer_role.id,
                subject_type: "user".into(),
                subject_id: approver_session.user_id.clone(),
                effect: "allow".into(),
            })
            .await
            .expect("bind inbox reviewer role");
        state
            .services
            .access_control
            .create_data_policy(DataPolicyUpsertRequest {
                name: "inbox reviewer scope".into(),
                subject_type: "user".into(),
                subject_id: approver_session.user_id.clone(),
                resource_type: "project".into(),
                scope_type: "selected-projects".into(),
                project_ids: vec![project.id.clone()],
                tags: Vec::new(),
                classifications: Vec::new(),
                effect: "allow".into(),
            })
            .await
            .expect("create inbox reviewer policy");

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/projects/{}/deletion-requests", project.id))
                    .header("authorization", format!("Bearer {}", owner_session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateProjectDeletionRequestInput {
                            reason: Some("Need targeted inbox".into()),
                        })
                        .expect("create deletion request json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create deletion request response");
        assert_eq!(create_response.status(), StatusCode::OK);

        let owner_inbox_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/inbox")
                    .header("authorization", format!("Bearer {}", owner_session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("owner inbox response");
        assert_eq!(owner_inbox_response.status(), StatusCode::OK);
        let owner_inbox_body = to_bytes(owner_inbox_response.into_body(), usize::MAX)
            .await
            .expect("owner inbox body");
        let owner_items: Vec<octopus_core::InboxItemRecord> =
            serde_json::from_slice(&owner_inbox_body).expect("owner inbox json");
        assert_eq!(owner_items.len(), 1);
        assert_eq!(owner_items[0].target_user_id, owner_session.user_id);

        let approver_inbox_response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/inbox")
                    .header(
                        "authorization",
                        format!("Bearer {}", approver_session.token),
                    )
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("approver inbox response");
        assert_eq!(approver_inbox_response.status(), StatusCode::OK);
        let approver_inbox_body = to_bytes(approver_inbox_response.into_body(), usize::MAX)
            .await
            .expect("approver inbox body");
        let approver_items: Vec<octopus_core::InboxItemRecord> =
            serde_json::from_slice(&approver_inbox_body).expect("approver inbox json");
        assert_eq!(approver_items.len(), 1);
        assert_eq!(approver_items[0].target_user_id, approver_session.user_id);
    }

    fn insert_approval_required_agent(root: &Path) {
        let connection = Connection::open(root.join("data").join("main.db")).expect("db");
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                params![
                    APPROVAL_AGENT_ID,
                    DEFAULT_WORKSPACE_ID,
                    DEFAULT_PROJECT_ID,
                    "project",
                    "Task Runtime Approval Agent",
                    Option::<String>::None,
                    "Approver",
                    serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                    "Require approval before model execution starts.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Agent for task runtime approval route tests.",
                    serde_json::to_string(&json!({})).expect("default model strategy"),
                    serde_json::to_string(&json!({})).expect("capability policy"),
                    serde_json::to_string(&json!({})).expect("permission envelope"),
                    serde_json::to_string(&json!({})).expect("memory policy"),
                    serde_json::to_string(&json!({})).expect("delegation policy"),
                    serde_json::to_string(&json!({
                        "toolExecution": "require-approval",
                        "memoryWrite": "require-approval",
                        "mcpAuth": "require-approval",
                        "teamSpawn": "require-approval",
                        "workflowEscalation": "require-approval"
                    }))
                    .expect("approval preference"),
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert approval-required agent");
    }

    fn insert_chained_approval_team(root: &Path) {
        let connection = Connection::open(root.join("data").join("main.db")).expect("db");
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-team-spawn-workflow-leader",
                    DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Spawn Workflow Leader",
                    Option::<String>::None,
                    "Coordinator",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Lead the team.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Leader for chained workflow approval tests.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert chained leader");
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-team-spawn-workflow-worker",
                    DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Spawn Workflow Worker",
                    Option::<String>::None,
                    "Executor",
                    serde_json::to_string(&vec!["delivery"]).expect("tags"),
                    "Do the delegated work.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Worker for chained workflow approval tests.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert chained worker");
        connection
            .execute(
                "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, approval_preference_json, leader_ref, member_refs, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    "team-spawn-workflow-approval",
                    DEFAULT_WORKSPACE_ID,
                    Option::<String>::None,
                    "workspace",
                    "Spawn Workflow Approval Team",
                    Option::<String>::None,
                    "Approval aware team",
                    serde_json::to_string(&vec!["coordination"]).expect("tags"),
                    "Delegate after approval, then continue workflow after approval.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    serde_json::to_string(&json!({
                        "toolExecution": "auto",
                        "memoryWrite": "require-approval",
                        "mcpAuth": "require-approval",
                        "teamSpawn": "require-approval",
                        "workflowEscalation": "require-approval"
                    }))
                    .expect("approval preference"),
                    "agent:agent-team-spawn-workflow-leader",
                    serde_json::to_string(&vec![
                        "agent:agent-team-spawn-workflow-leader",
                        "agent:agent-team-spawn-workflow-worker"
                    ])
                    .expect("member refs"),
                    "Team for chained workflow approval tests.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert chained approval team");
    }

    async fn seed_runtime_pending_approval_task_run(
        state: &ServerState,
        task: &ProjectTaskRecord,
        user_id: &str,
    ) -> ProjectTaskRunRecord {
        let runtime_session = state
            .services
            .runtime_session
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: format!("conversation-{}-approval", task.id),
                    project_id: Some(task.project_id.clone()),
                    title: format!("{} runtime approval", task.title),
                    session_kind: Some("task".into()),
                    selected_actor_ref: task.default_actor_ref.clone(),
                    selected_configured_model_id: Some("quota-model".into()),
                    execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
                },
                user_id,
            )
            .await
            .expect("create runtime session");
        let runtime_run = state
            .services
            .runtime_execution
            .submit_turn(
                &runtime_session.summary.id,
                SubmitRuntimeTurnInput {
                    content: task_prompt_from_record(task, "manual", None),
                    permission_mode: None,
                    recall_mode: None,
                    ignored_memory_ids: Vec::new(),
                    memory_intent: None,
                },
            )
            .await
            .expect("submit task turn");
        assert_eq!(runtime_run.status, "waiting_approval");
        let run = state
            .services
            .project_tasks
            .save_task_run(build_task_run_record(
                task,
                &runtime_session,
                &runtime_run,
                "manual",
                &task.default_actor_ref,
            ))
            .await
            .expect("save runtime-backed task run");
        state
            .services
            .project_tasks
            .save_task(update_task_record_from_run(task, &run, user_id))
            .await
            .expect("save runtime-backed task projection");
        run
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
            deliverable_refs: Vec::new(),
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
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
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
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
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
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
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
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
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
            leader_agent_id: None,
            manager_user_id: None,
            preset_code: None,
            assignments: None,
        })
        .is_err());
    }

    #[test]
    fn validate_create_project_request_trims_manager_preset_and_leader() {
        let validated = validate_create_project_request(CreateProjectRequest {
            name: "  Leader Project  ".into(),
            description: "  Use live inheritance.  ".into(),
            resource_directory: "  data/projects/leader-project/resources  ".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            leader_agent_id: Some("  agent-leader  ".into()),
            manager_user_id: Some("  user-manager  ".into()),
            preset_code: Some("  preset-ops  ".into()),
            assignments: None,
        })
        .expect("validated request");

        assert_eq!(validated.leader_agent_id.as_deref(), Some("agent-leader"));
        assert_eq!(validated.manager_user_id.as_deref(), Some("user-manager"));
        assert_eq!(validated.preset_code.as_deref(), Some("preset-ops"));
        assert!(validated.assignments.is_none());

        assert!(validate_create_project_request(CreateProjectRequest {
            name: "Project".into(),
            description: String::new(),
            resource_directory: "data/projects/leader-project/resources".into(),
            owner_user_id: None,
            member_user_ids: None,
            permission_overrides: None,
            linked_workspace_assets: None,
            leader_agent_id: Some("   ".into()),
            manager_user_id: None,
            preset_code: None,
            assignments: None,
        })
        .is_err());
    }

    #[tokio::test]
    async fn project_leader_rejects_excluded_workspace_agent_on_update() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let workspace_agent = state
            .services
            .workspace
            .list_agents()
            .await
            .expect("list agents")
            .into_iter()
            .find(|record| {
                record.project_id.is_none()
                    && record.status == "active"
                    && agent_visible_in_generic_catalog(record)
            })
            .expect("workspace agent");
        let _ = save_project_runtime_config_route(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "workspaceAssignments": {
                            "agents": {
                                "excludedAgentIds": [workspace_agent.id.clone()],
                            },
                        },
                    },
                }),
                configured_model_credentials: Vec::new(),
            }),
        )
        .await
        .expect("save project workspace assignments");

        let project = state
            .services
            .workspace
            .list_projects()
            .await
            .expect("list projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");
        let mut request = update_request_from_project(project);
        request.leader_agent_id = Some(workspace_agent.id.clone());

        let error = update_project(
            State(state.clone()),
            headers,
            Path(DEFAULT_PROJECT_ID.into()),
            Json(request),
        )
        .await
        .expect_err("excluded leader should be rejected");

        assert!(
            error.source.to_string().contains("leader"),
            "unexpected error: {:?}",
            error
        );
    }

    #[tokio::test]
    async fn project_leader_rejects_project_owned_agent_on_update() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let workspace_agent = state
            .services
            .workspace
            .list_agents()
            .await
            .expect("list agents")
            .into_iter()
            .find(|record| {
                record.project_id.is_none()
                    && record.status == "active"
                    && agent_visible_in_generic_catalog(record)
            })
            .expect("workspace agent");
        let project_owned_agent = state
            .services
            .workspace
            .create_agent(project_scoped_agent_input(
                &workspace_agent,
                DEFAULT_PROJECT_ID,
            ))
            .await
            .expect("create project agent");
        let project = state
            .services
            .workspace
            .list_projects()
            .await
            .expect("list projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");
        let mut request = update_request_from_project(project);
        request.leader_agent_id = Some(project_owned_agent.id.clone());

        let error = update_project(
            State(state.clone()),
            headers,
            Path(DEFAULT_PROJECT_ID.into()),
            Json(request),
        )
        .await
        .expect_err("project-owned leader should be rejected");

        assert!(
            error.source.to_string().contains("workspace agent"),
            "unexpected error: {:?}",
            error
        );
    }

    #[tokio::test]
    async fn create_runtime_session_rejects_single_shot_generation_model_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config_with_generation_model(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        let actor_ref = visible_workspace_agent_actor_ref(&state).await;

        let error = create_runtime_session(
            State(state),
            headers,
            Json(CreateRuntimeSessionInput {
                conversation_id: "conv-generation-only".into(),
                project_id: None,
                title: "Generation Only Session".into(),
                session_kind: None,
                selected_actor_ref: actor_ref,
                selected_configured_model_id: Some("generation-only-model".into()),
                execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
            }),
        )
        .await
        .expect_err("single-shot generation model should be rejected");

        assert!(
            error
                .source
                .to_string()
                .contains("does not expose a runtime-supported surface"),
            "unexpected error: {:?}",
            error
        );
    }

    #[tokio::test]
    async fn runtime_generation_route_executes_single_shot_generation_models() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config_with_generation_model(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let response = crate::routes::build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/runtime/generations")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "configuredModelId": "generation-only-model",
                            "content": "Write a haiku about runtime boundaries.",
                            "systemPrompt": "Reply in one line."
                        }))
                        .expect("generation request json"),
                    ))
                    .expect("generation request"),
            )
            .await
            .expect("generation response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("generation body");
        let payload: Value = serde_json::from_slice(&body).expect("generation payload");
        assert_eq!(payload["configuredModelId"], "generation-only-model");
        assert_eq!(payload["configuredModelName"], "Generation Only Model");
        assert_eq!(payload["requestId"], "mock-request-id");
        assert_eq!(payload["consumedTokens"], 32);
        assert!(
            payload["content"]
                .as_str()
                .expect("generation content")
                .contains("Write a haiku about runtime boundaries."),
            "unexpected generation payload: {payload:?}"
        );
    }

    #[tokio::test]
    async fn runtime_generation_route_rejects_agent_conversation_models() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let response = crate::routes::build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/runtime/generations")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "configuredModelId": "quota-model",
                            "content": "Summarize the latest run."
                        }))
                        .expect("generation request json"),
                    ))
                    .expect("generation request"),
            )
            .await
            .expect("generation response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("generation body");
        let payload: Value = serde_json::from_slice(&body).expect("generation payload");
        assert_eq!(payload["error"]["code"], "INVALID_INPUT");
        assert!(
            payload["error"]["message"]
                .as_str()
                .expect("error message")
                .contains("does not expose a runtime-supported surface"),
            "unexpected generation error payload: {payload:?}"
        );
    }

    #[tokio::test]
    async fn project_leader_cannot_be_disabled_by_runtime_settings() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let workspace_agent = state
            .services
            .workspace
            .list_agents()
            .await
            .expect("list agents")
            .into_iter()
            .find(|record| {
                record.project_id.is_none()
                    && record.status == "active"
                    && agent_visible_in_generic_catalog(record)
            })
            .expect("workspace agent");
        let project = state
            .services
            .workspace
            .list_projects()
            .await
            .expect("list projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");
        let mut request = update_request_from_project(project);
        request.leader_agent_id = Some(workspace_agent.id.clone());
        let _ = update_project(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(request),
        )
        .await
        .expect("set project leader");

        let error = save_project_runtime_config_route(
            State(state.clone()),
            headers,
            Path(DEFAULT_PROJECT_ID.into()),
            Json(RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "agents": {
                            "disabledAgentIds": [workspace_agent.id],
                        },
                    },
                }),
                configured_model_credentials: Vec::new(),
            }),
        )
        .await
        .expect_err("disabling the leader should be rejected");

        assert!(
            error.source.to_string().contains("leader"),
            "unexpected error: {:?}",
            error
        );
    }

    #[tokio::test]
    async fn project_scope_uses_live_workspace_inheritance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let workspace_agents = state
            .services
            .workspace
            .list_agents()
            .await
            .expect("list agents");
        let excluded_workspace_agent = workspace_agents
            .iter()
            .find(|record| {
                record.project_id.is_none()
                    && record.status == "active"
                    && agent_visible_in_generic_catalog(record)
            })
            .expect("workspace agent")
            .clone();
        let expected_agent_ids = workspace_agents
            .iter()
            .filter(|record| {
                record.status == "active"
                    && agent_visible_in_generic_catalog(record)
                    && (record.project_id.as_deref() == Some(DEFAULT_PROJECT_ID)
                        || (record.project_id.is_none()
                            && record.id != excluded_workspace_agent.id))
            })
            .map(|record| record.id.clone())
            .collect::<BTreeSet<_>>();

        let workspace_teams = state
            .services
            .workspace
            .list_teams()
            .await
            .expect("list teams");
        let excluded_workspace_team = workspace_teams
            .iter()
            .find(|record| record.project_id.is_none() && record.status == "active")
            .expect("workspace team")
            .clone();
        let expected_team_ids = workspace_teams
            .iter()
            .filter(|record| {
                record.status == "active"
                    && (record.project_id.as_deref() == Some(DEFAULT_PROJECT_ID)
                        || (record.project_id.is_none() && record.id != excluded_workspace_team.id))
            })
            .map(|record| record.id.clone())
            .collect::<BTreeSet<_>>();

        let capability_projection = state
            .services
            .workspace
            .get_capability_management_projection()
            .await
            .expect("capability projection");
        let excluded_source_key = capability_projection
            .assets
            .iter()
            .find(|asset| asset.enabled)
            .map(|asset| asset.source_key.clone())
            .expect("enabled tool");
        let expected_tool_source_keys = capability_projection
            .assets
            .iter()
            .filter(|asset| asset.enabled && asset.source_key != excluded_source_key)
            .map(|asset| asset.source_key.clone())
            .collect::<BTreeSet<_>>();

        let _ = save_project_runtime_config_route(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "workspaceAssignments": {
                            "tools": {
                                "excludedSourceKeys": [excluded_source_key.clone()],
                            },
                            "agents": {
                                "excludedAgentIds": [excluded_workspace_agent.id.clone()],
                                "excludedTeamIds": [excluded_workspace_team.id.clone()],
                            },
                        },
                    },
                }),
                configured_model_credentials: Vec::new(),
            }),
        )
        .await
        .expect("save project workspace assignments");

        let Json(dashboard) = project_dashboard(
            State(state.clone()),
            headers,
            Path(DEFAULT_PROJECT_ID.into()),
        )
        .await
        .expect("project dashboard");

        assert_eq!(
            dashboard.overview.agent_count,
            expected_agent_ids.len() as u64
        );
        assert_eq!(
            dashboard.overview.team_count,
            expected_team_ids.len() as u64
        );
        assert_eq!(
            dashboard.overview.tool_count,
            expected_tool_source_keys.len() as u64
        );
        assert!(
            dashboard
                .resource_breakdown
                .iter()
                .find(|item| item.id == "tools")
                .and_then(|item| item.helper.as_deref())
                .is_some_and(|description| {
                    expected_tool_source_keys
                        .iter()
                        .all(|source_key| description.contains(source_key))
                }),
            "tool breakdown should reflect live workspace tool source keys"
        );
    }

    #[tokio::test]
    async fn project_task_routes_create_launch_rerun_and_intervene_against_project_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        let workspace_agent_ref = visible_workspace_agent_actor_ref(&state).await;

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Prepare launch checklist".into(),
                goal: "Create a launch-ready checklist for the redesign rollout.".into(),
                brief: "Focus on sequencing, dependencies, and handoff notes.".into(),
                default_actor_ref: workspace_agent_ref.clone(),
                schedule_spec: Some("0 9 * * 1-5".into()),
                context_bundle: TaskContextBundle {
                    refs: vec![TaskContextRef {
                        kind: "resource".into(),
                        ref_id: "res-brief".into(),
                        title: "Project brief".into(),
                        subtitle: "Source brief".into(),
                        version_ref: None,
                        pin_mode: "snapshot".into(),
                    }],
                    pinned_instructions: "Keep the output concise.".into(),
                    resolution_mode: "explicit_only".into(),
                    last_resolved_at: None,
                },
            }),
        )
        .await
        .expect("create task");

        assert_eq!(created.project_id, DEFAULT_PROJECT_ID);
        assert_eq!(created.run_history.len(), 0);

        let Json(tasks) = list_project_tasks(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
        )
        .await
        .expect("list project tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, created.id);

        let Json(launch_run) = Box::pin(launch_project_task(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(LaunchTaskRequest {
                actor_ref: Some(workspace_agent_ref.clone()),
            }),
        ))
        .await
        .expect("launch project task");
        assert_eq!(launch_run.task_id, created.id);
        assert!(launch_run.session_id.is_some());

        let Json(rerun) = Box::pin(rerun_project_task(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(RerunTaskRequest {
                actor_ref: Some(workspace_agent_ref),
                source_task_run_id: Some(launch_run.id.clone()),
            }),
        ))
        .await
        .expect("rerun project task");
        assert_eq!(rerun.task_id, created.id);

        let Json(runs) = list_project_task_runs(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("list project task runs");
        assert_eq!(runs.len(), 2);

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(rerun.id.clone()),
                approval_id: None,
                r#type: "comment".into(),
                payload: serde_json::json!({
                    "note": "Please keep the checklist aligned with project handoff rules."
                }),
            }),
        ))
        .await
        .expect("create project task intervention");
        assert_eq!(intervention.task_id, created.id);

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get project task detail");

        assert_eq!(detail.run_history.len(), 2);
        assert_eq!(detail.intervention_history.len(), 1);
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.id.as_str()),
            Some(rerun.id.as_str())
        );
    }

    #[tokio::test]
    async fn project_task_routes_approve_intervention_updates_waiting_approval_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review launch approval".into(),
                goal: "Pause the task until an owner approves the plan.".into(),
                brief: "Route the active run through an approval gate.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = seed_task_run(&state, &task, &session.user_id, "waiting_approval").await;

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: None,
                r#type: "approve".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("approve task intervention");

        assert_eq!(intervention.status, "applied");
        assert_eq!(intervention.r#type, "approve");

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get project task detail");

        assert_eq!(detail.status, "running");
        assert_eq!(detail.view_status, "healthy");
        assert!(detail.attention_reasons.is_empty());
        assert_eq!(
            detail.latest_result_summary.as_deref(),
            Some("Approval received. Continuing the active run.")
        );
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.status.as_str()),
            Some("running")
        );
        assert_eq!(
            detail
                .active_run
                .as_ref()
                .map(|run| run.view_status.as_str()),
            Some("healthy")
        );
        assert_eq!(
            detail
                .active_run
                .as_ref()
                .map(|run| run.attention_reasons.clone()),
            Some(Vec::new())
        );
        assert_eq!(detail.intervention_history.len(), 1);
        assert_eq!(detail.intervention_history[0].status, "applied");
    }

    #[tokio::test]
    async fn project_task_routes_approve_intervention_resolves_runtime_pending_approval() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        insert_approval_required_agent(temp.path());

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review launch approval".into(),
                goal: "Pause the task until an owner approves the plan.".into(),
                brief: "Route the active run through an approval gate.".into(),
                default_actor_ref: APPROVAL_AGENT_REF.into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = Box::pin(seed_runtime_pending_approval_task_run(
            &state,
            &task,
            &session.user_id,
        ))
        .await;
        let runtime_session_id = seeded_run
            .session_id
            .clone()
            .expect("runtime-backed task run session id");

        let runtime_before = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session before intervention");
        assert!(runtime_before.pending_approval.is_some());
        let approval_id = runtime_before
            .pending_approval
            .as_ref()
            .map(|approval| approval.id.clone())
            .expect("runtime pending approval id");

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: Some(approval_id),
                r#type: "approve".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("approve task intervention");

        assert_eq!(intervention.status, "applied");
        assert_eq!(intervention.r#type, "approve");

        let runtime_after = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session after intervention");
        assert!(runtime_after.pending_approval.is_none());

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get project task detail");

        assert_eq!(detail.status, "completed");
        assert_eq!(detail.view_status, "healthy");
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.status.as_str()),
            Some("completed")
        );
        assert_eq!(
            detail.latest_result_summary.as_deref(),
            Some("Task run completed in the runtime.")
        );
        assert_eq!(detail.analytics_summary.run_count, 1);
        assert_eq!(detail.analytics_summary.manual_run_count, 1);
        assert_eq!(detail.analytics_summary.completion_count, 1);
        assert_eq!(detail.analytics_summary.approval_required_count, 1);
        assert_eq!(detail.intervention_history.len(), 1);
        assert_eq!(detail.intervention_history[0].status, "applied");
    }

    #[tokio::test]
    async fn project_task_routes_approve_intervention_keeps_waiting_when_runtime_chains_to_next_approval(
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        insert_chained_approval_team(temp.path());

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review chained approvals".into(),
                goal: "Keep the task blocked until the second approval is resolved.".into(),
                brief:
                    "Approve the team spawn first, then wait for workflow continuation approval."
                        .into(),
                default_actor_ref: CHAINED_APPROVAL_TEAM_REF.into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = Box::pin(seed_runtime_pending_approval_task_run(
            &state,
            &task,
            &session.user_id,
        ))
        .await;
        let runtime_session_id = seeded_run
            .session_id
            .clone()
            .expect("runtime-backed task run session id");

        let runtime_before = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session before intervention");
        let first_approval_id = runtime_before
            .pending_approval
            .as_ref()
            .map(|approval| approval.id.clone())
            .expect("initial runtime pending approval id");

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: Some(first_approval_id.clone()),
                r#type: "approve".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("approve task intervention");

        assert_eq!(intervention.status, "applied");
        assert_eq!(intervention.r#type, "approve");

        let runtime_after = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session after intervention");
        let next_approval = runtime_after
            .pending_approval
            .clone()
            .expect("chained runtime pending approval");
        assert_ne!(next_approval.id, first_approval_id);
        assert_eq!(
            next_approval.target_kind.as_deref(),
            Some("workflow-continuation")
        );

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get project task detail");

        assert_eq!(detail.status, "attention");
        assert_eq!(detail.view_status, "attention");
        assert_eq!(detail.attention_reasons, vec!["needs_approval"]);
        assert_eq!(detail.latest_result_summary, None);
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.status.as_str()),
            Some("waiting_approval")
        );
        assert_eq!(
            detail
                .active_run
                .as_ref()
                .and_then(|run| run.pending_approval_id.clone()),
            Some(next_approval.id.clone())
        );
        assert_eq!(detail.analytics_summary.run_count, 1);
        assert_eq!(detail.analytics_summary.manual_run_count, 1);
        assert_eq!(detail.analytics_summary.completion_count, 0);
        assert_eq!(detail.analytics_summary.approval_required_count, 1);
        assert_eq!(detail.intervention_history.len(), 1);
        assert_eq!(detail.intervention_history[0].status, "applied");
    }

    #[tokio::test]
    async fn project_task_routes_reject_intervention_resolves_runtime_pending_approval() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_runtime_workspace_config(temp.path());
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);
        insert_approval_required_agent(temp.path());

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review launch approval".into(),
                goal: "Pause the task until an owner approves the plan.".into(),
                brief: "Route the active run through an approval gate.".into(),
                default_actor_ref: APPROVAL_AGENT_REF.into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = Box::pin(seed_runtime_pending_approval_task_run(
            &state,
            &task,
            &session.user_id,
        ))
        .await;
        let runtime_session_id = seeded_run
            .session_id
            .clone()
            .expect("runtime-backed task run session id");

        let runtime_before = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session before intervention");
        let first_approval_id = runtime_before
            .pending_approval
            .as_ref()
            .map(|approval| approval.id.clone())
            .expect("runtime pending approval id");

        let Json(rejected) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: Some(first_approval_id.clone()),
                r#type: "reject".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("reject task intervention");

        assert_eq!(rejected.status, "applied");
        assert_eq!(rejected.r#type, "reject");

        let runtime_after_reject = state
            .services
            .runtime_session
            .get_session(&runtime_session_id)
            .await
            .expect("runtime session after reject");
        assert!(runtime_after_reject.pending_approval.is_none());

        let Json(rejected_detail) = get_project_task_detail(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get rejected task detail");

        assert_eq!(rejected_detail.status, "attention");
        assert_eq!(rejected_detail.view_status, "attention");
        assert_eq!(rejected_detail.attention_reasons, vec!["waiting_input"]);
        assert_eq!(
            rejected_detail
                .active_run
                .as_ref()
                .map(|run| run.status.as_str()),
            Some("waiting_input")
        );
        assert_eq!(
            rejected_detail
                .active_run
                .as_ref()
                .and_then(|run| run.pending_approval_id.clone()),
            None
        );
        assert_eq!(
            rejected_detail.latest_result_summary.as_deref(),
            Some("Approval rejected. Waiting for updated guidance.")
        );
        assert_eq!(rejected_detail.intervention_history.len(), 1);
        assert_eq!(rejected_detail.intervention_history[0].r#type, "reject");
        assert_eq!(rejected_detail.intervention_history[0].status, "applied");
    }

    #[tokio::test]
    async fn project_task_routes_approve_with_explicit_approval_id_does_not_fall_back_to_projection_only(
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review launch approval".into(),
                goal: "Pause the task until an owner approves the plan.".into(),
                brief: "Route the active run through an approval gate.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = seed_task_run(&state, &task, &session.user_id, "waiting_approval").await;
        let approval_id = seeded_run
            .pending_approval_id
            .clone()
            .expect("seeded pending approval id");

        let error = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: Some(approval_id.clone()),
                r#type: "approve".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect_err("explicit task approval should require runtime resolution");

        assert!(
            error.source.to_string().contains(&format!(
                "task approval `{approval_id}` could not be resolved in runtime"
            )),
            "unexpected error: {:?}",
            error
        );

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get project task detail");

        assert_eq!(detail.status, "attention");
        assert_eq!(detail.view_status, "attention");
        assert_eq!(detail.attention_reasons, vec!["needs_approval"]);
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.status.as_str()),
            Some("waiting_approval")
        );
        assert_eq!(detail.intervention_history.len(), 0);
    }

    #[tokio::test]
    async fn project_task_routes_reject_and_resume_interventions_update_task_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Review launch approval".into(),
                goal: "Pause the task until an owner approves the plan.".into(),
                brief: "Route the active run through an approval gate.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = seed_task_run(&state, &task, &session.user_id, "waiting_approval").await;

        let Json(rejected) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: None,
                r#type: "reject".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("reject task intervention");

        assert_eq!(rejected.status, "applied");

        let Json(rejected_detail) = get_project_task_detail(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get rejected task detail");

        assert_eq!(rejected_detail.status, "attention");
        assert_eq!(rejected_detail.view_status, "attention");
        assert_eq!(rejected_detail.attention_reasons, vec!["waiting_input"]);
        assert_eq!(
            rejected_detail.latest_result_summary.as_deref(),
            Some("Approval rejected. Waiting for updated guidance.")
        );
        assert_eq!(
            rejected_detail
                .active_run
                .as_ref()
                .map(|run| run.status.as_str()),
            Some("waiting_input")
        );
        assert_eq!(
            rejected_detail
                .active_run
                .as_ref()
                .map(|run| run.attention_reasons.clone()),
            Some(vec!["waiting_input".into()])
        );

        let Json(resumed) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: None,
                r#type: "resume".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("resume task intervention");

        assert_eq!(resumed.status, "applied");

        let Json(resumed_detail) = get_project_task_detail(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get resumed task detail");

        assert_eq!(resumed_detail.status, "running");
        assert_eq!(resumed_detail.view_status, "healthy");
        assert!(resumed_detail.attention_reasons.is_empty());
        assert_eq!(
            resumed_detail.latest_result_summary.as_deref(),
            Some("Updated guidance received. Continuing the active run.")
        );
        assert_eq!(
            resumed_detail
                .active_run
                .as_ref()
                .map(|run| run.status.as_str()),
            Some("running")
        );
        assert_eq!(resumed_detail.intervention_history.len(), 2);
        assert_eq!(resumed_detail.intervention_history[0].r#type, "resume");
        assert_eq!(resumed_detail.intervention_history[0].status, "applied");
        assert_eq!(resumed_detail.intervention_history[1].r#type, "reject");
        assert_eq!(resumed_detail.intervention_history[1].status, "applied");
    }

    #[tokio::test]
    async fn project_task_routes_edit_brief_intervention_updates_task_projection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Prepare release brief".into(),
                goal: "Keep the release brief aligned with final handoff scope.".into(),
                brief: "Focus on release sequencing and deliverable links.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = seed_task_run(&state, &task, &session.user_id, "running").await;

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: None,
                r#type: "edit_brief".into(),
                payload: serde_json::json!({
                    "brief": "Focus on the final release notes and linked deliverables."
                }),
            }),
        ))
        .await
        .expect("edit brief intervention");

        assert_eq!(intervention.status, "accepted");

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get task detail after brief edit");

        assert_eq!(
            detail.brief,
            "Focus on the final release notes and linked deliverables."
        );
        assert_eq!(detail.status, "running");
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .map(|transition| transition.kind.as_str()),
            Some("intervened")
        );
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .map(|transition| transition.summary.as_str()),
            Some("Task intervention recorded: edit_brief.")
        );
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .and_then(|transition| transition.run_id.as_deref()),
            Some(seeded_run.id.as_str())
        );

        let Json(tasks) = list_project_tasks(
            State(state.clone()),
            headers,
            Path(DEFAULT_PROJECT_ID.into()),
        )
        .await
        .expect("list project tasks after brief edit");

        assert_eq!(
            tasks
                .iter()
                .find(|record| record.id == created.id)
                .and_then(|record| record.latest_transition.as_ref())
                .map(|transition| transition.kind.as_str()),
            Some("intervened")
        );
    }

    #[tokio::test]
    async fn project_task_routes_change_actor_intervention_updates_task_and_target_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Prepare release brief".into(),
                goal: "Keep the release brief aligned with final handoff scope.".into(),
                brief: "Focus on release sequencing and deliverable links.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let task = state
            .services
            .project_tasks
            .get_task(DEFAULT_PROJECT_ID, &created.id)
            .await
            .expect("get created task");
        let seeded_run = seed_task_run(&state, &task, &session.user_id, "running").await;

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: Some(seeded_run.id.clone()),
                approval_id: None,
                r#type: "change_actor".into(),
                payload: serde_json::json!({
                    "actorRef": "agent:release-operator"
                }),
            }),
        ))
        .await
        .expect("change actor intervention");

        assert_eq!(intervention.status, "accepted");

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get task detail after actor change");

        assert_eq!(detail.default_actor_ref, "agent:release-operator");
        assert_eq!(
            detail.active_run.as_ref().map(|run| run.actor_ref.as_str()),
            Some("agent:release-operator")
        );
        assert_eq!(
            detail
                .run_history
                .iter()
                .find(|run| run.id == seeded_run.id)
                .map(|run| run.actor_ref.as_str()),
            Some("agent:release-operator")
        );
        assert_eq!(detail.status, "running");
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .map(|transition| transition.kind.as_str()),
            Some("intervened")
        );

        let Json(runs) = list_project_task_runs(
            State(state.clone()),
            headers,
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("list runs after actor change");

        assert_eq!(
            runs.iter()
                .find(|run| run.id == seeded_run.id)
                .map(|run| run.actor_ref.as_str()),
            Some("agent:release-operator")
        );
    }

    #[tokio::test]
    async fn project_task_routes_takeover_intervention_surfaces_attention_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;
        let headers = auth_headers(&session.token);

        let Json(created) = create_project_task(
            State(state.clone()),
            headers.clone(),
            Path(DEFAULT_PROJECT_ID.into()),
            Json(CreateTaskRequest {
                title: "Audit workspace menu".into(),
                goal: "Review navigation labels and routing consistency.".into(),
                brief: "Validate the desktop project menu before release.".into(),
                default_actor_ref: "team:workspace-core".into(),
                schedule_spec: None,
                context_bundle: TaskContextBundle::default(),
            }),
        )
        .await
        .expect("create task");

        let Json(intervention) = Box::pin(create_project_task_intervention(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
            Json(CreateTaskInterventionRequest {
                task_run_id: None,
                approval_id: None,
                r#type: "takeover".into(),
                payload: serde_json::json!({}),
            }),
        ))
        .await
        .expect("takeover intervention");

        assert_eq!(intervention.status, "accepted");

        let Json(detail) = get_project_task_detail(
            State(state.clone()),
            headers.clone(),
            Path((DEFAULT_PROJECT_ID.into(), created.id.clone())),
        )
        .await
        .expect("get task detail after takeover");

        assert_eq!(detail.status, "ready");
        assert_eq!(detail.view_status, "attention");
        assert_eq!(detail.attention_reasons, vec!["takeover_recommended"]);
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .map(|transition| transition.kind.as_str()),
            Some("intervened")
        );
        assert_eq!(
            detail
                .latest_transition
                .as_ref()
                .map(|transition| transition.summary.as_str()),
            Some("Task intervention recorded: takeover.")
        );
    }

    #[tokio::test]
    async fn project_task_routes_respect_project_task_module_denials() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = test_server_state(temp.path());
        let session = bootstrap_owner(&state).await;

        let project = state
            .services
            .workspace
            .list_projects()
            .await
            .expect("list projects")
            .into_iter()
            .find(|record| record.id == DEFAULT_PROJECT_ID)
            .expect("default project");

        state
            .services
            .workspace
            .update_project(
                DEFAULT_PROJECT_ID,
                UpdateProjectRequest {
                    name: project.name,
                    description: project.description,
                    status: project.status,
                    resource_directory: project.resource_directory,
                    owner_user_id: Some(project.owner_user_id),
                    member_user_ids: Some(project.member_user_ids),
                    permission_overrides: Some(ProjectPermissionOverrides {
                        tasks: "deny".into(),
                        ..project.permission_overrides
                    }),
                    leader_agent_id: project.leader_agent_id,
                    manager_user_id: project.manager_user_id,
                    preset_code: project.preset_code,
                    linked_workspace_assets: None,
                    assignments: None,
                },
            )
            .await
            .expect("deny task module");

        let error = list_project_tasks(
            State(state.clone()),
            auth_headers(&session.token),
            Path(DEFAULT_PROJECT_ID.into()),
        )
        .await
        .expect_err("task list should be denied");

        assert!(
            error
                .source
                .to_string()
                .contains("project module tasks is not available"),
            "unexpected error: {:?}",
            error
        );
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
