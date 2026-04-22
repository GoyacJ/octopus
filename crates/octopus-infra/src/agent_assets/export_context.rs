pub(crate) fn build_export_context(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: AssetTargetScope<'_>,
    input: ExportWorkspaceAgentBundleInput,
) -> Result<ExportContext, AppError> {
    let stored_agents = load_agents(connection)?;
    let stored_teams = load_teams(connection)?;
    let builtin_agents = crate::agent_bundle::list_builtin_agent_templates(workspace_id)?;
    let builtin_teams = crate::agent_bundle::list_builtin_team_templates(workspace_id)?;

    let mut agents = Vec::new();
    let mut teams = Vec::new();
    match &target {
        AssetTargetScope::Workspace => {
            teams.extend(
                stored_teams
                    .iter()
                    .filter(|team| {
                        team.project_id.is_none() && input.team_ids.iter().any(|id| id == &team.id)
                    })
                    .cloned(),
            );
            teams.extend(
                builtin_teams
                    .iter()
                    .filter(|team| input.team_ids.iter().any(|id| id == &team.id))
                    .cloned(),
            );
            agents.extend(
                stored_agents
                    .iter()
                    .filter(|agent| {
                        agent.project_id.is_none()
                            && input.agent_ids.iter().any(|id| id == &agent.id)
                    })
                    .cloned(),
            );
            agents.extend(
                builtin_agents
                    .iter()
                    .filter(|agent| input.agent_ids.iter().any(|id| id == &agent.id))
                    .cloned(),
            );
            if input.mode == "batch" && input.team_ids.is_empty() && input.agent_ids.is_empty() {
                teams = stored_teams
                    .iter()
                    .filter(|item| item.project_id.is_none())
                    .cloned()
                    .collect();
                agents = stored_agents
                    .iter()
                    .filter(|item| item.project_id.is_none())
                    .cloned()
                    .collect();
            }
        }
        AssetTargetScope::Project(project_id) => {
            let project_id = project_id.to_string();
            let assigned = load_projects(connection)?
                .into_iter()
                .find(|project| project.id == project_id)
                .and_then(|project| project.assignments)
                .and_then(|assignments| assignments.agents);
            let assigned_agent_ids = assigned
                .as_ref()
                .map(|agents| agents.agent_ids.iter().cloned().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            let assigned_team_ids = assigned
                .as_ref()
                .map(|teams| teams.team_ids.iter().cloned().collect::<BTreeSet<_>>())
                .unwrap_or_default();

            teams.extend(
                stored_teams
                    .iter()
                    .filter(|team| {
                        input.team_ids.iter().any(|id| id == &team.id)
                            && (team.project_id.as_deref() == Some(project_id.as_str())
                                || (team.project_id.is_none()
                                    && assigned_team_ids.contains(&team.id)))
                    })
                    .cloned(),
            );
            teams.extend(
                builtin_teams
                    .iter()
                    .filter(|team| {
                        input.team_ids.iter().any(|id| id == &team.id)
                            && assigned_team_ids.contains(&team.id)
                    })
                    .cloned(),
            );
            agents.extend(
                stored_agents
                    .iter()
                    .filter(|agent| {
                        input.agent_ids.iter().any(|id| id == &agent.id)
                            && (agent.project_id.as_deref() == Some(project_id.as_str())
                                || (agent.project_id.is_none()
                                    && assigned_agent_ids.contains(&agent.id)))
                    })
                    .cloned(),
            );
            agents.extend(
                builtin_agents
                    .iter()
                    .filter(|agent| {
                        input.agent_ids.iter().any(|id| id == &agent.id)
                            && assigned_agent_ids.contains(&agent.id)
                    })
                    .cloned(),
            );
            if input.mode == "batch" && input.team_ids.is_empty() && input.agent_ids.is_empty() {
                teams = stored_teams
                    .iter()
                    .filter(|team| {
                        team.project_id.as_deref() == Some(project_id.as_str())
                            || (team.project_id.is_none() && assigned_team_ids.contains(&team.id))
                    })
                    .cloned()
                    .collect();
                teams.extend(
                    builtin_teams
                        .iter()
                        .filter(|team| assigned_team_ids.contains(&team.id))
                        .cloned(),
                );
                agents = stored_agents
                    .iter()
                    .filter(|agent| {
                        agent.project_id.as_deref() == Some(project_id.as_str())
                            || (agent.project_id.is_none()
                                && assigned_agent_ids.contains(&agent.id))
                    })
                    .cloned()
                    .collect();
                agents.extend(
                    builtin_agents
                        .iter()
                        .filter(|agent| assigned_agent_ids.contains(&agent.id))
                        .cloned(),
                );
            }
        }
    }

    let mut seen_agent_ids = BTreeSet::new();
    agents.retain(|agent| seen_agent_ids.insert(agent.id.clone()));
    let mut seen_team_ids = BTreeSet::new();
    teams.retain(|team| seen_team_ids.insert(team.id.clone()));
    let combined_agents = stored_agents
        .iter()
        .chain(builtin_agents.iter())
        .cloned()
        .collect::<Vec<_>>();
    for team in &teams {
        for agent in team_member_agents(&combined_agents, team) {
            if seen_agent_ids.contains(&agent.id) {
                continue;
            }
            seen_agent_ids.insert(agent.id.clone());
            agents.push(agent.clone());
        }
    }

    let single_team_export_member_ids = teams
        .first()
        .map(|team| {
            team_member_agents(&agents, team)
                .into_iter()
                .map(|agent| agent.id.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let skill_paths = resolve_skill_paths(paths, &agents, &teams)?;
    let builtin_skill_assets = resolve_builtin_skill_assets(&agents, &teams)?;
    let mcp_configs = resolve_mcp_configs(paths, &target, &agents, &teams)?;
    let avatar_payloads = resolve_avatar_payloads(paths, &target, &agents, &teams)?;
    let descriptors = load_scoped_bundle_asset_descriptors(connection, &target)?
        .into_values()
        .collect::<Vec<_>>();
    let root_dir_name = if input.mode == "single"
        && teams.len() == 1
        && agents
            .iter()
            .all(|agent| single_team_export_member_ids.contains(&agent.id))
    {
        sanitize_export_dir_name(&teams[0].name)
    } else if input.mode == "single" && agents.len() == 1 && teams.is_empty() {
        sanitize_export_dir_name(&agents[0].name)
    } else {
        String::from("templates")
    };
    let bundle_manifest = crate::agent_bundle::manifest_v2::build_export_bundle_manifest(
        &agents,
        &teams,
        &descriptors,
    );
    let translation_report = crate::agent_bundle::manifest_v2::build_export_translation_report(
        &agents,
        &teams,
        &descriptors,
        &bundle_manifest,
    );

    Ok(ExportContext {
        root_dir_name,
        agents,
        teams,
        skill_paths,
        builtin_skill_assets,
        mcp_configs,
        avatar_payloads,
        bundle_manifest,
        translation_report,
        issues: Vec::new(),
    })
}

fn resolve_skill_paths(
    paths: &WorkspacePaths,
    agents: &[AgentRecord],
    teams: &[TeamRecord],
) -> Result<HashMap<String, PathBuf>, AppError> {
    let workspace_root = paths.root.clone();
    let mut roots = discover_skill_roots(&workspace_root);
    let project_ids = agents
        .iter()
        .filter_map(|agent| agent.project_id.as_deref())
        .chain(teams.iter().filter_map(|team| team.project_id.as_deref()))
        .collect::<BTreeSet<_>>();
    for project_id in project_ids {
        let project_skill_root = paths.project_skills_root(project_id);
        if project_skill_root.is_dir() {
            roots.push(crate::resources_skills::SkillCatalogRoot {
                source: crate::resources_skills::SkillDefinitionSource::WorkspaceManaged,
                path: project_skill_root,
                origin: crate::resources_skills::SkillSourceOrigin::SkillsDir,
            });
        }
    }
    let catalog_entries = load_skills_from_roots(&roots)?;
    let skill_paths = catalog_entries
        .into_iter()
        .map(|entry| {
            (
                crate::catalog_hash_id("skill", &crate::display_path(&entry.path, &workspace_root)),
                entry.path.parent().unwrap_or(&entry.path).to_path_buf(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut resolved = HashMap::new();
    for skill_id in agents
        .iter()
        .flat_map(|agent| agent.skill_ids.iter())
        .chain(teams.iter().flat_map(|team| team.skill_ids.iter()))
    {
        if let Some(path) = skill_paths.get(skill_id) {
            resolved.insert(skill_id.clone(), path.clone());
        }
    }
    Ok(resolved)
}

fn resolve_builtin_skill_assets(
    agents: &[AgentRecord],
    teams: &[TeamRecord],
) -> Result<HashMap<String, BuiltinSkillAsset>, AppError> {
    let referenced_ids = agents
        .iter()
        .flat_map(|agent| agent.skill_ids.iter())
        .chain(teams.iter().flat_map(|team| team.skill_ids.iter()))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut resolved = HashMap::new();
    for skill_id in referenced_ids {
        if let Some(asset) = find_builtin_skill_asset_by_id(&skill_id)? {
            resolved.insert(skill_id, asset);
        }
    }
    Ok(resolved)
}

fn resolve_mcp_configs(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
    agents: &[AgentRecord],
    teams: &[TeamRecord],
) -> Result<HashMap<String, JsonValue>, AppError> {
    let configs = load_effective_mcp_map(paths, target)?;
    let mut resolved = HashMap::new();
    for server_name in agents
        .iter()
        .flat_map(|agent| agent.mcp_server_names.iter())
        .chain(teams.iter().flat_map(|team| team.mcp_server_names.iter()))
    {
        if let Some(config) = configs.get(server_name) {
            resolved.insert(server_name.clone(), config.clone());
            continue;
        }
        if let Some(asset) = crate::agent_bundle::find_builtin_mcp_asset(server_name)? {
            resolved.insert(server_name.clone(), asset.config);
        }
    }
    Ok(resolved)
}

fn resolve_avatar_payloads(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
    agents: &[AgentRecord],
    teams: &[TeamRecord],
) -> Result<HashMap<String, Option<(String, String, Vec<u8>)>>, AppError> {
    let mut payloads = HashMap::new();
    for agent in agents {
        payloads.insert(
            format!("agent:{}", agent.id),
            export_avatar_payload(
                paths,
                agent.avatar_path.as_deref(),
                "agent",
                &target.avatar_seed_key(&agent.id),
            )?,
        );
    }
    for team in teams {
        payloads.insert(
            format!("team:{}", team.id),
            export_avatar_payload(
                paths,
                team.avatar_path.as_deref(),
                "team",
                &target.avatar_seed_key(&team.id),
            )?,
        );
    }
    Ok(payloads)
}

fn export_avatar_payload(
    paths: &WorkspacePaths,
    avatar_path: Option<&str>,
    owner_kind: &str,
    seed_key: &str,
) -> Result<Option<(String, String, Vec<u8>)>, AppError> {
    if let Some(avatar_path) = avatar_path {
        let absolute_path = paths.root.join(avatar_path);
        if absolute_path.is_file() {
            let file_name = absolute_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("avatar.png")
                .to_string();
            if let Some(content_type) = content_type_for_avatar(&file_name) {
                return Ok(Some((
                    file_name,
                    content_type.to_string(),
                    fs::read(absolute_path)?,
                )));
            }
        }
    }
    let _ = (owner_kind, seed_key);
    Ok(None)
}

