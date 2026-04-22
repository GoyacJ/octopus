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

fn visible_inbox_items(
    user_id: &str,
    items: Vec<octopus_core::InboxItemRecord>,
) -> Vec<octopus_core::InboxItemRecord> {
    items
        .into_iter()
        .filter(|item| item.target_user_id == user_id)
        .collect()
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

mod activity_records;
mod agent_routes;
mod catalog_routes;
mod deliverable_routes;
mod pet_routes;
mod project_dashboard;
mod project_inputs;
mod project_routes;
mod project_scope;
mod resource_routes;
mod runtime_actions;
mod runtime_config;
mod runtime_events;
mod runtime_sessions;
mod task_helpers;
mod task_routes;
mod user_routes;
mod workspace_routes;

pub(crate) use activity_records::{
    list_activity_records, list_conversation_records, workspace_activity_from_audit,
};
pub(crate) use agent_routes::{
    copy_project_agent_from_builtin_route, copy_project_team_from_builtin_route,
    copy_workspace_agent_from_builtin_route, copy_workspace_team_from_builtin_route, create_agent,
    create_team, delete_agent, delete_team, export_agent_bundle_route,
    export_project_agent_bundle_route, import_agent_bundle_route,
    import_project_agent_bundle_route, link_project_agent, link_project_team, list_agents,
    list_project_agent_links, list_project_team_links, list_teams,
    preview_import_agent_bundle_route, preview_import_project_agent_bundle_route,
    unlink_project_agent, unlink_project_team, update_agent, update_team,
};
pub(crate) use catalog_routes::{
    copy_workspace_mcp_server_to_managed_route, copy_workspace_skill_to_managed_route, create_tool,
    create_workspace_mcp_server_route, create_workspace_skill_route, delete_tool,
    delete_workspace_mcp_server_route, delete_workspace_skill_route,
    get_workspace_mcp_server_route, get_workspace_skill_file_route, get_workspace_skill_route,
    get_workspace_skill_tree_route, import_workspace_skill_archive_route,
    import_workspace_skill_folder_route, list_tools, update_tool,
    update_workspace_mcp_server_route, update_workspace_skill_file_route,
    update_workspace_skill_route, workspace_capability_asset_disable,
    workspace_capability_management_projection, workspace_catalog_models,
    workspace_provider_credentials,
};
pub(crate) use deliverable_routes::{
    create_deliverable_version, fork_deliverable, get_deliverable_detail,
    get_deliverable_version_content, knowledge, list_deliverable_versions, promote_deliverable,
    workspace_deliverables,
};
pub(crate) use pet_routes::{
    bind_project_pet_conversation, bind_workspace_pet_conversation, project_pet_snapshot,
    save_project_pet_presence, save_workspace_pet_presence, workspace_pet_dashboard,
    workspace_pet_snapshot,
};
pub(crate) use project_dashboard::project_dashboard;
pub(crate) use project_inputs::{validate_create_project_request, validate_update_project_request};
pub(crate) use project_routes::{
    approve_project_deletion_request, create_project, create_project_deletion_request,
    create_project_promotion_request, delete_project, list_project_deletion_requests,
    list_project_promotion_requests, reject_project_deletion_request, update_project,
};
pub(crate) use project_scope::{
    ensure_project_owner, ensure_project_owner_session, load_project_runtime_document,
    lookup_project, resolve_project_granted_scope, validate_create_project_leader,
    validate_project_runtime_leader, validate_updated_project_leader,
};
pub(crate) use resource_routes::{
    create_project_resource, create_project_resource_folder, create_workspace_resource,
    delete_project_resource, delete_workspace_resource, get_resource_content, get_resource_detail,
    import_project_resource, import_workspace_resource, list_resource_children,
    list_workspace_filesystem_directories, list_workspace_promotion_requests, project_deliverables,
    project_knowledge, project_resources, promote_resource, review_project_promotion_request,
    update_project_resource, update_workspace_resource, workspace_knowledge, workspace_resources,
};
pub(crate) use runtime_actions::{
    cancel_runtime_subrun, resolve_runtime_approval, resolve_runtime_auth_challenge,
    resolve_runtime_memory_proposal, submit_runtime_turn,
};
pub(crate) use runtime_config::{
    get_project_runtime_config_route, get_runtime_config, get_user_runtime_config_route,
    probe_runtime_configured_model_route, runtime_bootstrap, save_project_runtime_config_route,
    save_runtime_config_route, save_user_runtime_config_route,
    validate_project_runtime_config_route, validate_runtime_config_route,
    validate_user_runtime_config_route,
};
pub(crate) use runtime_events::runtime_events;
pub(crate) use runtime_sessions::{
    create_runtime_session, delete_runtime_session, derive_runtime_owner_permission_ceiling,
    get_runtime_session, list_runtime_sessions, run_runtime_generation,
};
pub(crate) use task_routes::{
    create_project_task, create_project_task_intervention, get_project_task_detail,
    launch_project_task, list_project_task_runs, list_project_tasks, rerun_project_task,
    update_project_task,
};
pub(crate) use user_routes::{
    change_current_user_password_route, current_user_profile_route, inbox,
    update_current_user_profile_route,
};
pub(crate) use workspace_routes::{
    projects, update_workspace_route, workspace, workspace_overview,
};

#[cfg(test)]
mod tests;
