use super::*;

pub(crate) fn mcp_scope_label() -> &'static str {
    "workspace"
}

pub(crate) fn mcp_source_key(scope: &str, owner_id: Option<&str>, server_name: &str) -> String {
    match (scope, owner_id) {
        ("project", Some(project_id)) => format!("mcp:project:{project_id}:{server_name}"),
        _ => format!("mcp:{server_name}"),
    }
}

pub(crate) fn extract_mcp_server_configs(
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<BTreeMap<String, serde_json::Value>, AppError> {
    let mut servers = BTreeMap::new();
    for (server_name, value) in document
        .get("mcpServers")
        .and_then(|value| value.as_object())
        .into_iter()
        .flat_map(|servers| servers.iter())
    {
        servers.insert(server_name.clone(), value.clone());
    }
    Ok(servers)
}

pub(crate) fn mcp_endpoint_from_document(config: &serde_json::Value) -> String {
    parse_mcp_server_config(config)
        .map(|config| sdk_mcp_endpoint(&config))
        .unwrap_or_default()
}

fn tool_consumer_summary(
    kind: &str,
    id: &str,
    name: &str,
    scope: &str,
    owner_id: Option<&str>,
    owner_label: Option<&str>,
) -> WorkspaceToolConsumerSummary {
    WorkspaceToolConsumerSummary {
        kind: kind.into(),
        id: id.into(),
        name: name.into(),
        scope: scope.into(),
        owner_id: owner_id.map(ToOwned::to_owned),
        owner_label: owner_label.map(ToOwned::to_owned),
    }
}

pub(crate) struct ToolConsumerMaps {
    pub(crate) builtin: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
    pub(crate) skills: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
    pub(crate) mcps: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
}

fn push_consumer(
    target: &mut HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
    key: String,
    consumer: &WorkspaceToolConsumerSummary,
) {
    let entries = target.entry(key).or_default();
    if !entries
        .iter()
        .any(|existing| existing.kind == consumer.kind && existing.id == consumer.id)
    {
        entries.push(consumer.clone());
    }
}

pub(crate) fn clone_non_empty_consumers(
    value: Option<&Vec<WorkspaceToolConsumerSummary>>,
) -> Option<Vec<WorkspaceToolConsumerSummary>> {
    match value {
        Some(items) if !items.is_empty() => Some(items.clone()),
        _ => None,
    }
}

fn sort_consumers(entries: &mut HashMap<String, Vec<WorkspaceToolConsumerSummary>>) {
    for consumers in entries.values_mut() {
        consumers.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

pub(crate) fn build_tool_consumer_maps(
    agents: &[AgentRecord],
    teams: &[TeamRecord],
    project_name_by_id: &HashMap<String, String>,
    project_mcp_source_keys: &HashMap<(String, String), String>,
) -> ToolConsumerMaps {
    let mut builtin = HashMap::<String, Vec<WorkspaceToolConsumerSummary>>::new();
    let mut skills = HashMap::<String, Vec<WorkspaceToolConsumerSummary>>::new();
    let mut mcps = HashMap::<String, Vec<WorkspaceToolConsumerSummary>>::new();

    for agent in agents {
        let project_owner_label = agent
            .project_id
            .as_ref()
            .and_then(|project_id| project_name_by_id.get(project_id))
            .map(String::as_str);
        let consumer = tool_consumer_summary(
            "agent",
            &agent.id,
            &agent.name,
            &agent.scope,
            agent.project_id.as_deref(),
            project_owner_label,
        );
        for builtin_key in &agent.builtin_tool_keys {
            push_consumer(&mut builtin, builtin_key.clone(), &consumer);
        }
        for skill_id in &agent.skill_ids {
            push_consumer(&mut skills, skill_id.clone(), &consumer);
        }
        for server_name in &agent.mcp_server_names {
            let key = agent
                .project_id
                .as_ref()
                .and_then(|project_id| {
                    project_mcp_source_keys
                        .get(&(project_id.clone(), server_name.clone()))
                        .cloned()
                })
                .unwrap_or_else(|| mcp_source_key("workspace", None, server_name));
            push_consumer(&mut mcps, key, &consumer);
        }
    }

    for team in teams {
        let project_owner_label = team
            .project_id
            .as_ref()
            .and_then(|project_id| project_name_by_id.get(project_id))
            .map(String::as_str);
        let consumer = tool_consumer_summary(
            "team",
            &team.id,
            &team.name,
            &team.scope,
            team.project_id.as_deref(),
            project_owner_label,
        );
        for builtin_key in &team.builtin_tool_keys {
            push_consumer(&mut builtin, builtin_key.clone(), &consumer);
        }
        for skill_id in &team.skill_ids {
            push_consumer(&mut skills, skill_id.clone(), &consumer);
        }
        for server_name in &team.mcp_server_names {
            let key = team
                .project_id
                .as_ref()
                .and_then(|project_id| {
                    project_mcp_source_keys
                        .get(&(project_id.clone(), server_name.clone()))
                        .cloned()
                })
                .unwrap_or_else(|| mcp_source_key("workspace", None, server_name));
            push_consumer(&mut mcps, key, &consumer);
        }
    }

    sort_consumers(&mut builtin);
    sort_consumers(&mut skills);
    sort_consumers(&mut mcps);

    ToolConsumerMaps {
        builtin,
        skills,
        mcps,
    }
}

fn mcp_resource_capability_id(server_name: &str, uri: &str) -> String {
    qualified_mcp_resource_name(server_name, uri)
}

pub(crate) async fn discover_mcp_server_capabilities(
    servers: &BTreeMap<String, McpServerConfig>,
) -> BTreeMap<String, DiscoveredMcpServerCapabilities> {
    discover_mcp_server_capabilities_best_effort(servers).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_mcp_catalog_entries(
    entries: &mut Vec<WorkspaceToolCatalogEntry>,
    workspace_id: &str,
    asset_state_document: &WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    display_path: &str,
    management: WorkspaceToolManagementCapabilities,
    owner_scope: Option<String>,
    owner_id: Option<String>,
    owner_label: Option<String>,
    scope: &str,
    server_name: &str,
    endpoint: &str,
    consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
    discovered: Option<&DiscoveredMcpServerCapabilities>,
    fallback_description: &str,
) {
    let asset_id = catalog_hash_id("mcp-asset", source_key);
    let disabled = workspace_asset_is_disabled(asset_state_document, source_key);
    let discovered = discovered.cloned().unwrap_or_default().finalize();

    let mut push_entry = |id: String,
                          capability_id: String,
                          name: String,
                          source_kind: &str,
                          execution_kind: &str,
                          description: String,
                          tool_names: Vec<String>,
                          resource_uri: Option<String>| {
        entries.push(WorkspaceToolCatalogEntry {
            id,
            asset_id: Some(asset_id.clone()),
            capability_id: Some(capability_id),
            workspace_id: workspace_id.to_string(),
            name,
            kind: "mcp".into(),
            source_kind: Some(source_kind.into()),
            execution_kind: Some(execution_kind.into()),
            description,
            required_permission: None,
            availability: discovered.availability.clone(),
            source_key: source_key.to_string(),
            display_path: display_path.to_string(),
            disabled,
            management: management.clone(),
            builtin_key: None,
            active: None,
            shadowed_by: None,
            source_origin: None,
            workspace_owned: None,
            relative_path: None,
            server_name: Some(server_name.to_string()),
            endpoint: Some(endpoint.to_string()),
            tool_names: Some(tool_names),
            resource_uri,
            status_detail: discovered.status_detail.clone(),
            scope: Some(scope.to_string()),
            owner_scope: owner_scope.clone(),
            owner_id: owner_id.clone(),
            owner_label: owner_label.clone(),
            consumers: consumers.clone(),
        });
    };

    if discovered.tools.is_empty()
        && discovered.prompts.is_empty()
        && discovered.resources.is_empty()
    {
        let capability_id = format!("mcp_server__{}__{}", scope, server_name);
        push_entry(
            capability_id.clone(),
            capability_id,
            server_name.to_string(),
            "mcp_tool",
            "tool",
            fallback_description.to_string(),
            Vec::new(),
            None,
        );
        return;
    }

    for tool in &discovered.tools {
        push_entry(
            tool.qualified_name.clone(),
            tool.qualified_name.clone(),
            tool.raw_name.clone(),
            "mcp_tool",
            "tool",
            tool.tool
                .description
                .clone()
                .unwrap_or_else(|| format!("Invoke MCP tool `{}`.", tool.raw_name)),
            vec![tool.raw_name.clone()],
            None,
        );
    }

    for prompt in &discovered.prompts {
        push_entry(
            prompt.qualified_name.clone(),
            prompt.qualified_name.clone(),
            prompt.raw_name.clone(),
            "mcp_prompt",
            "prompt_skill",
            prompt
                .prompt
                .description
                .clone()
                .unwrap_or_else(|| format!("Execute MCP prompt `{}`.", prompt.raw_name)),
            Vec::new(),
            None,
        );
    }

    for resource in &discovered.resources {
        let capability_id = mcp_resource_capability_id(server_name, &resource.uri);
        push_entry(
            capability_id.clone(),
            capability_id,
            resource
                .name
                .clone()
                .unwrap_or_else(|| resource.uri.clone()),
            "mcp_resource",
            "resource",
            resource
                .description
                .clone()
                .or_else(|| resource.name.clone())
                .unwrap_or_else(|| {
                    format!(
                        "Read MCP resource `{}` from server `{server_name}`.",
                        resource.uri
                    )
                }),
            Vec::new(),
            Some(resource.uri.clone()),
        );
    }
}
