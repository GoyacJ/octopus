use super::*;

pub(super) fn runtime_transport_payload<T: serde::Serialize>(
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
    pub(super) path: Option<String>,
}

pub(super) fn capability_authorization_request(
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

pub(super) fn optional_transport_project_id(project_id: &str) -> Option<String> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

pub(super) fn resolved_fork_target_project_id(
    requested_project_id: Option<&str>,
    source_project_id: &str,
) -> Option<String> {
    requested_project_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| optional_transport_project_id(source_project_id))
}

pub(super) fn deliverable_conversation_record(
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

pub(super) fn precise_tool_resource_type(kind: &str) -> &'static str {
    match kind.trim() {
        "builtin" => "tool.builtin",
        "mcp" => "tool.mcp",
        _ => "tool.skill",
    }
}

pub(super) fn merge_protected_resource_descriptor(
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

pub(super) async fn protected_resource_metadata(
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

pub(super) fn authorization_request_from_descriptor(
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

pub(super) async fn resource_authorization_request(
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

pub(super) fn resource_input_authorization_request(
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

pub(super) fn resource_visibility_allows(session: &SessionRecord, record: &WorkspaceResourceRecord) -> bool {
    match record.visibility.as_str() {
        "private" => record.owner_user_id == session.user_id,
        _ => true,
    }
}

pub(super) fn knowledge_visibility_allows(session: &SessionRecord, record: &KnowledgeRecord) -> bool {
    if record.scope == "personal" {
        return record.owner_user_id.as_deref() == Some(session.user_id.as_str());
    }

    match record.visibility.as_str() {
        "private" => record.owner_user_id.as_deref() == Some(session.user_id.as_str()),
        _ => true,
    }
}

pub(super) fn knowledge_relevant_to_project_context(record: &KnowledgeRecord, project_id: &str) -> bool {
    record.project_id.as_deref() == Some(project_id)
        || matches!(record.scope.as_str(), "workspace" | "personal")
}

pub(super) fn knowledge_entry_record(record: KnowledgeRecord) -> octopus_core::KnowledgeEntryRecord {
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

pub(super) fn agent_visible_in_generic_catalog(record: &AgentRecord) -> bool {
    record.asset_role != "pet"
}

pub(super) async fn ensure_visible_resource(
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

pub(super) async fn knowledge_authorization_request(
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

pub(super) async fn agent_authorization_request(
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

pub(super) fn agent_input_authorization_request(
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

pub(super) async fn ensure_capability_session(
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

pub(super) async fn ensure_project_delete_review_session(
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

pub(super) async fn tool_record_authorization_request(
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

pub(super) async fn skill_authorization_request(
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

pub(super) async fn mcp_server_authorization_request(
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

