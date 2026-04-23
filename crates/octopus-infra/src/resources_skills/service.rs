use super::*;

impl InfraWorkspaceService {
    pub(crate) fn find_skill_catalog_entry(
        &self,
        skill_id: &str,
    ) -> Result<SkillCatalogEntry, AppError> {
        let workspace_root = self.state.paths.root.clone();
        let projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone();
        load_skills_from_roots(&discover_catalog_skill_roots(&self.state.paths, &projects))?
            .into_iter()
            .find(|skill| {
                catalog_hash_id("skill", &display_path(&skill.path, &workspace_root)) == skill_id
            })
            .ok_or_else(|| AppError::not_found("workspace skill"))
    }

    pub(crate) fn get_workspace_skill_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        if let Some(asset) = find_builtin_skill_asset_by_id(skill_id)? {
            return builtin_skill_document(&asset);
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    pub(crate) fn get_workspace_skill_tree_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        if let Some(asset) = find_builtin_skill_asset_by_id(skill_id)? {
            return builtin_skill_tree_document(&asset);
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_tree_document_from_path(
            &self.state.paths.root,
            skill_id,
            &skill_source_key(&entry.path, &self.state.paths.root),
            &entry.path,
            entry.origin,
        )
    }

    pub(crate) fn get_workspace_skill_file_document(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        if let Some(asset) = find_builtin_skill_asset_by_id(skill_id)? {
            return builtin_skill_file_document(&asset, relative_path);
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let source_key = skill_source_key(&entry.path, &self.state.paths.root);
        let relative = workspace_relative_path(&entry.path, &self.state.paths.root);
        let readonly = !is_workspace_owned_skill(relative.as_deref(), entry.origin);
        let skill_root = skill_root_path(&entry.path, entry.origin)?;
        let path = resolve_skill_file_path(&skill_root, entry.origin, relative_path)?;
        if !path.exists() {
            return Err(AppError::not_found("workspace skill file"));
        }
        skill_file_document_from_path(
            &self.state.paths.root,
            skill_id,
            &source_key,
            &skill_root,
            entry.origin,
            &path,
            readonly,
        )
    }

    pub(crate) fn ensure_workspace_owned_skill_entry(
        &self,
        skill_id: &str,
    ) -> Result<SkillCatalogEntry, AppError> {
        if find_builtin_skill_asset_by_id(skill_id)?.is_some() {
            return Err(AppError::invalid_input(
                "only workspace-owned managed skills can be edited or deleted",
            ));
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let relative = workspace_relative_path(&entry.path, &self.state.paths.root);
        if !is_workspace_owned_skill(relative.as_deref(), entry.origin) {
            return Err(AppError::invalid_input(
                "only workspace-owned managed skills can be edited or deleted",
            ));
        }
        Ok(entry)
    }

    pub(crate) fn import_skill_files_to_managed_root(
        &self,
        slug: &str,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let slug = validate_skill_slug(slug)?;
        let skill_dir = workspace_owned_skill_root(&self.state.paths).join(&slug);
        if skill_dir.exists() {
            return Err(AppError::conflict(format!(
                "workspace skill '{slug}' already exists"
            )));
        }
        if !files
            .iter()
            .any(|(relative_path, _)| relative_path == "SKILL.md")
        {
            return Err(AppError::invalid_input(
                "imported skill must contain SKILL.md at the root",
            ));
        }
        fs::create_dir_all(&skill_dir)?;
        write_skill_tree_files(&skill_dir, &files)?;
        let skill_path = skill_dir.join("SKILL.md");
        rewrite_skill_frontmatter_name(&skill_path, &slug)?;
        skill_document_from_path(
            &self.state.paths.root,
            &skill_path,
            SkillSourceOrigin::SkillsDir,
        )
    }

    pub(crate) fn load_mcp_server_document(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        let document = load_workspace_runtime_document(&self.state.paths)?;
        if let Some(config) = document
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .and_then(|servers| servers.get(server_name))
            .cloned()
        {
            let config = config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
            return Ok(WorkspaceMcpServerDocument {
                server_name: server_name.into(),
                source_key: format!("mcp:{server_name}"),
                display_path: "config/runtime/workspace.json".into(),
                scope: "workspace".into(),
                config: serde_json::Value::Object(config),
            });
        }

        let asset = find_builtin_mcp_asset(server_name)?
            .ok_or_else(|| AppError::not_found("workspace mcp server"))?;
        let config =
            asset.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        Ok(WorkspaceMcpServerDocument {
            server_name: server_name.into(),
            source_key: format!("mcp:{server_name}"),
            display_path: asset.display_path,
            scope: "builtin".into(),
            config: serde_json::Value::Object(config),
        })
    }

    pub(crate) fn save_workspace_runtime_document(
        &self,
        document: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AppError> {
        validate_workspace_runtime_document(&document)?;
        write_workspace_runtime_document(&self.state.paths, &document)
    }

    pub(crate) async fn build_tool_catalog_entries(
        &self,
    ) -> Result<Vec<WorkspaceToolCatalogEntry>, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let workspace_root = self.state.paths.root.clone();
        let asset_state_document = load_workspace_asset_state_document(&self.state.paths)?;
        let projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone();
        let agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .clone();
        let teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .clone();
        let mut consumer_agents = agents.clone();
        consumer_agents.extend(list_builtin_agent_templates(&workspace_id)?);
        let mut consumer_teams = teams.clone();
        consumer_teams.extend(list_builtin_team_templates(&workspace_id)?);
        let project_name_by_id = projects
            .iter()
            .map(|project| (project.id.clone(), project.name.clone()))
            .collect::<HashMap<_, _>>();
        let project_mcp_configs: HashMap<String, BTreeMap<String, serde_json::Value>> = projects
            .iter()
            .map(|project| {
                let document = load_runtime_document(
                    &self
                        .state
                        .paths
                        .runtime_project_config_dir
                        .join(format!("{}.json", project.id)),
                )?;
                let configs = extract_mcp_server_configs(&document)?;
                Ok::<_, AppError>((project.id.clone(), configs))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let project_mcp_servers: HashMap<String, BTreeMap<String, McpServerConfig>> =
            project_mcp_configs
                .iter()
                .map(
                    |(project_id, configs): (&String, &BTreeMap<String, serde_json::Value>)| {
                        let parsed = configs
                            .iter()
                            .filter_map(|(server_name, config): (&String, &serde_json::Value)| {
                                parse_mcp_server_config(config)
                                    .map(|parsed| (server_name.clone(), parsed))
                            })
                            .collect::<BTreeMap<_, _>>();
                        (project_id.clone(), parsed)
                    },
                )
                .collect::<HashMap<_, _>>();
        let project_mcp_source_keys = project_mcp_configs
            .iter()
            .flat_map(
                |(project_id, configs): (&String, &BTreeMap<String, serde_json::Value>)| {
                    configs.keys().map(move |server_name: &String| {
                        (
                            (project_id.clone(), server_name.clone()),
                            mcp_source_key("project", Some(project_id), server_name),
                        )
                    })
                },
            )
            .collect::<HashMap<_, _>>();
        let consumer_maps = build_tool_consumer_maps(
            &consumer_agents,
            &consumer_teams,
            &project_name_by_id,
            &project_mcp_source_keys,
        );
        let mut entries = Vec::new();

        // Shared capability projection mirrors only the current live builtin surface.
        for spec in builtin_tool_catalog().entries() {
            let source_key = format!("builtin:{}", spec.name);
            let capability_id = format!("builtin-{}", spec.name);
            entries.push(WorkspaceToolCatalogEntry {
                id: capability_id.clone(),
                asset_id: Some(capability_id.clone()),
                capability_id: Some(capability_id),
                workspace_id: workspace_id.clone(),
                name: spec.name.into(),
                kind: "builtin".into(),
                source_kind: Some("builtin".into()),
                execution_kind: Some("tool".into()),
                description: spec.description.into(),
                required_permission: normalize_required_permission(spec.required_permission),
                availability: "healthy".into(),
                source_key: source_key.clone(),
                display_path: "runtime builtin registry".into(),
                disabled: workspace_asset_is_disabled(&asset_state_document, &source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: false,
                    can_delete: false,
                },
                builtin_key: Some(spec.name.into()),
                active: None,
                shadowed_by: None,
                source_origin: None,
                workspace_owned: None,
                relative_path: None,
                server_name: None,
                endpoint: None,
                tool_names: None,
                resource_uri: None,
                status_detail: None,
                scope: None,
                owner_scope: Some("builtin".into()),
                owner_id: None,
                owner_label: Some("Builtin".into()),
                consumers: clone_non_empty_consumers(consumer_maps.builtin.get(spec.name)),
            });
        }

        for skill in
            load_skills_from_roots(&discover_catalog_skill_roots(&self.state.paths, &projects))?
        {
            let is_active = skill.shadowed_by.is_none();
            let source_key = skill_source_key(&skill.path, &workspace_root);
            let relative_path = workspace_relative_path(&skill.path, &workspace_root);
            let workspace_owned = is_workspace_owned_skill(relative_path.as_deref(), skill.origin);
            let project_owner_id =
                project_owned_skill_project_id(relative_path.as_deref(), skill.origin);
            let skill_id = catalog_hash_id("skill", &display_path(&skill.path, &workspace_root));
            entries.push(WorkspaceToolCatalogEntry {
                id: skill_id.clone(),
                asset_id: Some(skill_id.clone()),
                capability_id: Some(skill_id.clone()),
                workspace_id: workspace_id.clone(),
                name: skill.name.clone(),
                kind: "skill".into(),
                source_kind: Some("local_skill".into()),
                execution_kind: Some("prompt_skill".into()),
                description: skill.description.unwrap_or_default(),
                required_permission: None,
                availability: if is_active {
                    "healthy".into()
                } else {
                    "configured".into()
                },
                source_key: source_key.clone(),
                display_path: display_path(&skill.path, &workspace_root),
                disabled: workspace_asset_is_disabled(&asset_state_document, &source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: workspace_owned,
                    can_delete: workspace_owned,
                },
                builtin_key: None,
                active: Some(is_active),
                shadowed_by: skill.shadowed_by.clone(),
                source_origin: Some(skill.origin.as_str().into()),
                workspace_owned: Some(workspace_owned),
                relative_path,
                server_name: None,
                endpoint: None,
                tool_names: None,
                resource_uri: None,
                status_detail: None,
                scope: None,
                owner_scope: if workspace_owned {
                    Some("workspace".into())
                } else if project_owner_id.is_some() {
                    Some("project".into())
                } else {
                    None
                },
                owner_id: project_owner_id.clone(),
                owner_label: project_owner_id
                    .as_ref()
                    .and_then(|project_id| project_name_by_id.get(project_id))
                    .map(ToOwned::to_owned),
                consumers: clone_non_empty_consumers(consumer_maps.skills.get(&skill_id)),
            });
        }

        for skill in list_builtin_skill_assets()? {
            let skill_id = catalog_hash_id("skill", &skill.display_path);
            let source_key = builtin_skill_source_key(&skill);
            entries.push(WorkspaceToolCatalogEntry {
                id: skill_id.clone(),
                asset_id: Some(skill_id.clone()),
                capability_id: Some(skill_id.clone()),
                workspace_id: workspace_id.clone(),
                name: skill.name.clone(),
                kind: "skill".into(),
                source_kind: Some("bundled_skill".into()),
                execution_kind: Some("prompt_skill".into()),
                description: skill.description.clone(),
                required_permission: None,
                availability: "healthy".into(),
                source_key: source_key.clone(),
                display_path: skill.display_path.clone(),
                disabled: workspace_asset_is_disabled(&asset_state_document, &source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: false,
                    can_delete: false,
                },
                builtin_key: None,
                active: Some(true),
                shadowed_by: None,
                source_origin: Some(BUILTIN_SKILL_SOURCE_ORIGIN.into()),
                workspace_owned: Some(false),
                relative_path: None,
                server_name: None,
                endpoint: None,
                tool_names: None,
                resource_uri: None,
                status_detail: None,
                scope: None,
                owner_scope: Some("builtin".into()),
                owner_id: None,
                owner_label: Some("Builtin".into()),
                consumers: clone_non_empty_consumers(consumer_maps.skills.get(&skill_id)),
            });
        }

        let workspace_runtime_document = load_workspace_runtime_document(&self.state.paths)?;
        let workspace_mcp_servers = parse_mcp_servers(&workspace_runtime_document);
        let workspace_mcp_capabilities: BTreeMap<String, DiscoveredMcpServerCapabilities> =
            discover_mcp_server_capabilities(&workspace_mcp_servers).await;

        for (server_name, config) in &workspace_mcp_servers {
            let source_key = mcp_source_key("workspace", None, server_name);
            append_mcp_catalog_entries(
                &mut entries,
                &workspace_id,
                &asset_state_document,
                &source_key,
                "config/runtime/workspace.json",
                WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: true,
                    can_delete: true,
                },
                Some("workspace".into()),
                None,
                Some("Workspace".into()),
                mcp_scope_label(),
                server_name,
                &sdk_mcp_endpoint(config),
                clone_non_empty_consumers(consumer_maps.mcps.get(&source_key)),
                workspace_mcp_capabilities.get(server_name),
                "Configured MCP server.",
            );
        }

        let managed_workspace_servers = workspace_mcp_servers
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>();

        let builtin_mcp_assets = list_builtin_mcp_assets()?;
        let builtin_mcp_servers = builtin_mcp_assets
            .iter()
            .filter(|asset| !managed_workspace_servers.contains(&asset.server_name))
            .filter_map(|asset| {
                parse_mcp_server_config(&asset.config)
                    .map(|config| (asset.server_name.clone(), config))
            })
            .collect::<BTreeMap<_, _>>();
        let builtin_mcp_capabilities: BTreeMap<String, DiscoveredMcpServerCapabilities> =
            discover_mcp_server_capabilities(&builtin_mcp_servers).await;

        for asset in builtin_mcp_assets {
            if managed_workspace_servers.contains(&asset.server_name) {
                continue;
            }
            let source_key = format!("mcp:{}", asset.server_name);
            append_mcp_catalog_entries(
                &mut entries,
                &workspace_id,
                &asset_state_document,
                &source_key,
                &asset.display_path,
                WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: false,
                    can_delete: false,
                },
                Some("builtin".into()),
                None,
                Some("Builtin".into()),
                "builtin",
                &asset.server_name,
                &mcp_endpoint_from_document(&asset.config),
                clone_non_empty_consumers(consumer_maps.mcps.get(&source_key)),
                builtin_mcp_capabilities.get(&asset.server_name),
                "Builtin MCP server template.",
            );
        }

        let mut project_mcp_capabilities: HashMap<
            String,
            BTreeMap<String, DiscoveredMcpServerCapabilities>,
        > = HashMap::new();
        for (project_id, servers) in &project_mcp_servers {
            project_mcp_capabilities.insert(
                project_id.clone(),
                discover_mcp_server_capabilities(servers).await,
            );
        }

        for project in &projects {
            let project_configs: BTreeMap<String, serde_json::Value> = project_mcp_configs
                .get(&project.id)
                .cloned()
                .unwrap_or_default();
            for (server_name, config) in project_configs {
                let source_key = mcp_source_key("project", Some(&project.id), &server_name);
                let discovered = project_mcp_capabilities
                    .get(&project.id)
                    .and_then(|capabilities| capabilities.get(&server_name));
                append_mcp_catalog_entries(
                    &mut entries,
                    &workspace_id,
                    &asset_state_document,
                    &source_key,
                    &format!("config/runtime/projects/{}.json", project.id),
                    WorkspaceToolManagementCapabilities {
                        can_disable: true,
                        can_edit: false,
                        can_delete: false,
                    },
                    Some("project".into()),
                    Some(project.id.clone()),
                    Some(project.name.clone()),
                    "project",
                    &server_name,
                    &mcp_endpoint_from_document(&config),
                    clone_non_empty_consumers(consumer_maps.mcps.get(&source_key)),
                    discovered,
                    "Configured MCP server.",
                );
            }
        }

        entries.sort_by(|left, right| {
            left.kind.cmp(&right.kind).then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
        });

        Ok(entries)
    }

    pub(crate) async fn build_capability_management_projection(
        &self,
    ) -> Result<CapabilityManagementProjection, AppError> {
        let entries = self.build_tool_catalog_entries().await?;
        Ok(capability_management_projection(entries))
    }
}
