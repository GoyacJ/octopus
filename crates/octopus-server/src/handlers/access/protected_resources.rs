use super::*;

fn precise_tool_resource_type(kind: &str) -> &'static str {
    match kind.trim() {
        "builtin" => "tool.builtin",
        "mcp" => "tool.mcp",
        _ => "tool.skill",
    }
}

fn merge_protected_resource_descriptor(
    defaults: ProtectedResourceDescriptor,
    metadata_by_key: &HashMap<(String, String), ProtectedResourceDescriptor>,
) -> ProtectedResourceDescriptor {
    let Some(metadata) =
        metadata_by_key.get(&(defaults.resource_type.clone(), defaults.id.clone()))
    else {
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

pub(super) async fn build_access_protected_resource_descriptors(
    state: &ServerState,
) -> Result<Vec<ProtectedResourceDescriptor>, ApiError> {
    let metadata_by_key = state
        .services
        .access_control
        .list_protected_resources()
        .await?
        .into_iter()
        .map(|record| ((record.resource_type.clone(), record.id.clone()), record))
        .collect::<HashMap<_, _>>();
    let agents = state.services.workspace.list_agents().await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let tools = state.services.workspace.list_tools().await?;

    let mut descriptors = Vec::new();
    descriptors.extend(agents.into_iter().map(|agent| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: agent.id,
                resource_type: "agent".into(),
                resource_subtype: Some(agent.scope),
                name: agent.name,
                project_id: agent.project_id,
                tags: agent.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(resources.into_iter().map(|resource| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: resource.id,
                resource_type: "resource".into(),
                resource_subtype: Some(resource.kind),
                name: resource.name,
                project_id: resource.project_id,
                tags: resource.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(knowledge.into_iter().map(|entry| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: entry.id,
                resource_type: "knowledge".into(),
                resource_subtype: Some(entry.kind),
                name: entry.title,
                project_id: entry.project_id,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(tools.into_iter().map(|tool| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: tool.id,
                resource_type: precise_tool_resource_type(&tool.kind).into(),
                resource_subtype: Some(tool.kind),
                name: tool.name,
                project_id: None,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.sort_by(|left, right| {
        left.resource_type
            .cmp(&right.resource_type)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(descriptors)
}
