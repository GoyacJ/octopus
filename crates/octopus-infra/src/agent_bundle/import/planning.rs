pub(crate) fn build_bundle_plan(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: agent_assets::AssetTargetScope<'_>,
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<agent_assets::BundlePlan, AppError> {
    let (bundle_files, filtered_file_count, mut issues) =
        agent_assets::normalize_bundle_files(files)?;
    let bundle_files = agent_assets::strip_optional_bundle_root(bundle_files);
    let parsed = agent_assets::parse_bundle_files(&bundle_files, &target, &mut issues)?;
    let existing_skill_sources =
        agent_assets::load_existing_skill_import_sources(connection, &target.source_kind())?;
    let existing_team_sources =
        agent_assets::load_existing_team_import_sources(connection, &target.source_kind())?;
    let existing_agent_sources =
        agent_assets::load_existing_agent_import_sources(connection, &target.source_kind())?;
    let existing_managed_skills =
        agent_assets::load_existing_managed_skills(&target.skill_root(paths))?;
    let existing_target_mcp = agent_assets::load_target_mcp_map(paths, &target)?;
    let existing_effective_mcp = agent_assets::load_effective_mcp_map(paths, &target)?;
    let existing_agents = agent_assets::load_scoped_agents(connection, &target)?;
    let existing_teams = agent_assets::load_scoped_teams(connection, &target)?;
    let existing_descriptors =
        agent_assets::load_scoped_bundle_asset_descriptors(connection, &target)?;

    let mut unique_skills = BTreeMap::<(String, String), agent_assets::PlannedSkill>::new();
    for source in &parsed.skills {
        let entry = unique_skills
            .entry((source.canonical_slug.clone(), source.content_hash.clone()))
            .or_insert_with(|| agent_assets::PlannedSkill {
                slug: String::new(),
                skill_id: String::new(),
                name: source.name.clone(),
                action: agent_assets::ImportAction::Create,
                content_hash: source.content_hash.clone(),
                file_count: source.files.len(),
                source_ids: Vec::new(),
                consumer_names: Vec::new(),
                files: source.files.clone(),
            });
        entry.source_ids.push(source.source_id.clone());
        if !entry
            .consumer_names
            .iter()
            .any(|name| name == &source.owner_name)
        {
            entry.consumer_names.push(source.owner_name.clone());
        }
    }

    let mut planned_skills = Vec::new();
    for ((canonical_slug, content_hash), mut planned) in unique_skills {
        let mut mapped_slugs = planned
            .source_ids
            .iter()
            .filter_map(|source_id| existing_skill_sources.get(source_id))
            .filter_map(|mapping| existing_managed_skills.get(&mapping.skill_slug))
            .map(|skill| skill.slug.clone())
            .collect::<BTreeSet<_>>();

        let (slug, action) = if let Some(mapped_slug) = mapped_slugs.pop_first() {
            let existing = existing_managed_skills.get(&mapped_slug);
            if existing.is_some_and(|item| item.content_hash == content_hash) {
                (mapped_slug, agent_assets::ImportAction::Skip)
            } else {
                (mapped_slug, agent_assets::ImportAction::Update)
            }
        } else if let Some(existing) = existing_managed_skills.get(&canonical_slug) {
            if existing.content_hash == content_hash {
                (canonical_slug.clone(), agent_assets::ImportAction::Skip)
            } else {
                let candidate = format!(
                    "{}-{}",
                    canonical_slug,
                    crate::agent_bundle::shared::short_hash(&content_hash)
                );
                let action = if existing_managed_skills
                    .get(&candidate)
                    .is_some_and(|item| item.content_hash == content_hash)
                {
                    agent_assets::ImportAction::Skip
                } else {
                    agent_assets::ImportAction::Create
                };
                (candidate, action)
            }
        } else {
            (canonical_slug.clone(), agent_assets::ImportAction::Create)
        };

        planned.slug = slug.clone();
        planned.skill_id = crate::agent_bundle::shared::managed_skill_id(&slug);
        planned.action = action;
        planned_skills.push(planned);
    }
    planned_skills.sort_by(|left, right| left.slug.cmp(&right.slug));

    let skill_slug_by_source_id = planned_skills
        .iter()
        .flat_map(|skill| {
            skill
                .source_ids
                .iter()
                .cloned()
                .map(move |source_id| (source_id, skill.slug.clone()))
        })
        .collect::<HashMap<_, _>>();

    let mut unique_mcps =
        BTreeMap::<(String, Option<String>, bool), agent_assets::PlannedMcp>::new();
    for source in &parsed.mcps {
        let key = (
            source.server_name.clone(),
            source.content_hash.clone(),
            source.referenced_only,
        );
        let entry = unique_mcps
            .entry(key)
            .or_insert_with(|| agent_assets::PlannedMcp {
                server_name: source.server_name.clone(),
                action: agent_assets::ImportAction::Create,
                content_hash: source.content_hash.clone(),
                source_ids: Vec::new(),
                consumer_names: Vec::new(),
                config: source.config.clone(),
                referenced_only: source.referenced_only,
                resolved: false,
            });
        entry.source_ids.push(source.source_id.clone());
        if !entry
            .consumer_names
            .iter()
            .any(|name| name == &source.owner_name)
        {
            entry.consumer_names.push(source.owner_name.clone());
        }
    }

    let mut planned_mcps = Vec::new();
    let mut resolved_mcp_name_by_source_id = HashMap::new();
    for ((_server_name, content_hash, referenced_only), mut planned) in unique_mcps {
        if referenced_only {
            if existing_effective_mcp.contains_key(&planned.server_name) {
                planned.action = agent_assets::ImportAction::Skip;
                planned.resolved = true;
            } else {
                planned.action = agent_assets::ImportAction::Failed;
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_WARNING,
                    agent_assets::SOURCE_SCOPE_MCP,
                    planned.source_ids.first().cloned(),
                    format!(
                        "bundle references MCP '{}' without mcps/ directory payload; kept as reference only but no matching server exists",
                        planned.server_name
                    ),
                ));
            }
        } else if let Some(config) = planned.config.as_ref() {
            if let Some(existing) = existing_target_mcp.get(&planned.server_name) {
                if existing == config {
                    planned.action = agent_assets::ImportAction::Skip;
                } else {
                    planned.server_name = format!(
                        "{}-{}",
                        planned.server_name,
                        crate::agent_bundle::shared::short_hash(
                            content_hash.as_deref().unwrap_or_default()
                        )
                    );
                    planned.action = if existing_target_mcp
                        .get(&planned.server_name)
                        .is_some_and(|candidate| candidate == config)
                    {
                        agent_assets::ImportAction::Skip
                    } else {
                        agent_assets::ImportAction::Create
                    };
                }
            } else {
                planned.action = agent_assets::ImportAction::Create;
            }
            planned.resolved = true;
        }

        if planned.resolved {
            for source_id in &planned.source_ids {
                resolved_mcp_name_by_source_id
                    .insert(source_id.clone(), planned.server_name.clone());
            }
        }
        planned_mcps.push(planned);
    }
    planned_mcps.sort_by(|left, right| left.server_name.cmp(&right.server_name));

    let mut planned_agents = Vec::new();
    for parsed_agent in &parsed.agents {
        let skill_slugs = parsed_agent
            .skill_source_ids
            .iter()
            .filter_map(|source_id| skill_slug_by_source_id.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let mcp_server_names = parsed_agent
            .mcp_source_ids
            .iter()
            .filter_map(|source_id| resolved_mcp_name_by_source_id.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let agent_id = existing_agent_sources
            .get(&parsed_agent.source_id)
            .map(|mapping| mapping.agent_id.clone())
            .or_else(|| {
                let deterministic =
                    agent_assets::deterministic_asset_id("agent", &target, &parsed_agent.source_id);
                existing_agents
                    .contains_key(&deterministic)
                    .then_some(deterministic)
            });
        let action = agent_assets::resolve_agent_action(
            workspace_id,
            &target,
            &existing_agents,
            &agent_assets::PlannedAgent {
                source_id: parsed_agent.source_id.clone(),
                agent_id: agent_id.clone(),
                name: parsed_agent.name.clone(),
                department: parsed_agent.team_name.clone().unwrap_or_default(),
                action: agent_assets::ImportAction::Create,
                description: parsed_agent.description.clone(),
                personality: parsed_agent.personality.clone(),
                prompt: parsed_agent.prompt.clone(),
                tags: parsed_agent.tags.clone(),
                builtin_tool_keys: parsed_agent.builtin_tool_keys.clone(),
                skill_slugs: skill_slugs.clone(),
                mcp_server_names: mcp_server_names.clone(),
                avatar: parsed_agent.avatar.clone(),
                model: parsed_agent.model.clone(),
            },
            &skill_slugs
                .iter()
                .map(|slug| crate::agent_bundle::shared::managed_skill_id(slug))
                .collect::<Vec<_>>(),
        )?;
        planned_agents.push(agent_assets::PlannedAgent {
            source_id: parsed_agent.source_id.clone(),
            agent_id,
            name: parsed_agent.name.clone(),
            department: parsed_agent.team_name.clone().unwrap_or_default(),
            action,
            description: parsed_agent.description.clone(),
            personality: parsed_agent.personality.clone(),
            prompt: parsed_agent.prompt.clone(),
            tags: parsed_agent.tags.clone(),
            builtin_tool_keys: parsed_agent.builtin_tool_keys.clone(),
            skill_slugs,
            mcp_server_names,
            avatar: parsed_agent.avatar.clone(),
            model: parsed_agent.model.clone(),
        });
    }
    planned_agents.sort_by(|left, right| left.source_id.cmp(&right.source_id));

    let agent_name_by_source = planned_agents
        .iter()
        .map(|agent| (agent.source_id.clone(), agent.name.clone()))
        .collect::<HashMap<_, _>>();
    let agent_id_by_source = planned_agents
        .iter()
        .map(|agent| {
            (
                agent.source_id.clone(),
                agent.agent_id.clone().unwrap_or_else(|| {
                    agent_assets::deterministic_asset_id("agent", &target, &agent.source_id)
                }),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut planned_teams = Vec::new();
    for parsed_team in &parsed.teams {
        let skill_slugs = parsed_team
            .skill_source_ids
            .iter()
            .filter_map(|source_id| skill_slug_by_source_id.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let mcp_server_names = parsed_team
            .mcp_source_ids
            .iter()
            .filter_map(|source_id| resolved_mcp_name_by_source_id.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let member_names = if parsed_team.member_names.is_empty() {
            parsed_team
                .agent_source_ids
                .iter()
                .filter_map(|source_id| agent_name_by_source.get(source_id))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            parsed_team.member_names.clone()
        };
        let member_agent_record_ids = parsed_team
            .agent_source_ids
            .iter()
            .filter_map(|source_id| agent_id_by_source.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let leader_agent_record_id = parsed_team.leader_name.as_ref().and_then(|leader_name| {
            parsed_team.agent_source_ids.iter().find_map(|source_id| {
                if agent_name_by_source.get(source_id) == Some(leader_name) {
                    agent_id_by_source.get(source_id).cloned()
                } else {
                    None
                }
            })
        });
        let leader_ref = leader_agent_record_id
            .as_deref()
            .map(crate::canonical_agent_ref)
            .unwrap_or_default();
        let member_refs = crate::canonical_agent_refs(&member_agent_record_ids);
        let team_id = existing_team_sources
            .get(&parsed_team.source_id)
            .map(|mapping| mapping.team_id.clone())
            .or_else(|| {
                let deterministic =
                    agent_assets::deterministic_asset_id("team", &target, &parsed_team.source_id);
                existing_teams
                    .contains_key(&deterministic)
                    .then_some(deterministic)
            });
        let action = agent_assets::resolve_team_action(
            workspace_id,
            &target,
            &existing_teams,
            &agent_assets::PlannedTeam {
                source_id: parsed_team.source_id.clone(),
                team_id: team_id.clone(),
                name: parsed_team.name.clone(),
                action: agent_assets::ImportAction::Create,
                description: parsed_team.description.clone(),
                personality: parsed_team.personality.clone(),
                prompt: parsed_team.prompt.clone(),
                tags: parsed_team.tags.clone(),
                builtin_tool_keys: parsed_team.builtin_tool_keys.clone(),
                skill_slugs: skill_slugs.clone(),
                mcp_server_names: mcp_server_names.clone(),
                leader_name: parsed_team.leader_name.clone(),
                member_names: member_names.clone(),
                agent_source_ids: parsed_team.agent_source_ids.clone(),
                avatar: parsed_team.avatar.clone(),
                model: parsed_team.model.clone(),
            },
            &skill_slugs
                .iter()
                .map(|slug| crate::agent_bundle::shared::managed_skill_id(slug))
                .collect::<Vec<_>>(),
            &leader_ref,
            &member_refs,
        )?;
        planned_teams.push(agent_assets::PlannedTeam {
            source_id: parsed_team.source_id.clone(),
            team_id,
            name: parsed_team.name.clone(),
            action,
            description: parsed_team.description.clone(),
            personality: parsed_team.personality.clone(),
            prompt: parsed_team.prompt.clone(),
            tags: parsed_team.tags.clone(),
            builtin_tool_keys: parsed_team.builtin_tool_keys.clone(),
            skill_slugs,
            mcp_server_names,
            leader_name: parsed_team.leader_name.clone(),
            member_names,
            agent_source_ids: parsed_team.agent_source_ids.clone(),
            avatar: parsed_team.avatar.clone(),
            model: parsed_team.model.clone(),
        });
    }
    planned_teams.sort_by(|left, right| left.source_id.cmp(&right.source_id));

    let dependency_resolution =
        agent_assets::dependency_resolution_from_manifest(parsed.bundle_manifest.as_ref(), &issues);
    let mut planned_descriptors = Vec::new();
    for descriptor in &parsed.descriptor_assets {
        let descriptor_id = agent_assets::deterministic_descriptor_id(
            &target,
            &descriptor.asset_kind,
            &descriptor.source_id,
        );
        let record = octopus_core::BundleAssetDescriptorRecord {
            id: descriptor_id,
            workspace_id: workspace_id.to_string(),
            project_id: target.project_id().map(ToOwned::to_owned),
            scope: target.scope_label().into(),
            asset_kind: descriptor.asset_kind.clone(),
            source_id: descriptor.source_id.clone(),
            display_name: descriptor.display_name.clone(),
            source_path: descriptor.source_path.clone(),
            storage_path: agent_assets::descriptor_storage_path(&target, descriptor),
            content_hash: agent_assets::hash_bytes(&descriptor.bytes),
            byte_size: descriptor.bytes.len() as u64,
            manifest_revision: descriptor.manifest_revision.clone(),
            task_domains: normalize_task_domains(descriptor.task_domains.clone()),
            translation_mode: descriptor.translation_mode.clone(),
            trust_metadata: parsed
                .bundle_manifest
                .as_ref()
                .map(|manifest| manifest.trust_metadata.clone())
                .unwrap_or_else(default_asset_trust_metadata),
            dependency_resolution: dependency_resolution.clone(),
            import_metadata: octopus_core::AssetImportMetadata {
                origin_kind: "bundle-import".into(),
                source_id: Some(descriptor.source_id.clone()),
                manifest_version: ASSET_IMPORT_MANIFEST_VERSION,
                translation_status: descriptor.translation_mode.clone(),
                imported_at: None,
            },
            updated_at: 0,
        };
        let action = if let Some(existing) = existing_descriptors.get(&record.id) {
            if agent_assets::descriptor_record_matches(existing, &record) {
                agent_assets::ImportAction::Skip
            } else {
                agent_assets::ImportAction::Update
            }
        } else {
            agent_assets::ImportAction::Create
        };
        planned_descriptors.push(agent_assets::PlannedBundleDescriptor {
            action,
            record,
            bytes: descriptor.bytes.clone(),
        });
    }
    planned_descriptors.sort_by(|left, right| {
        left.record
            .asset_kind
            .cmp(&right.record.asset_kind)
            .then(left.record.source_id.cmp(&right.record.source_id))
    });

    Ok(agent_assets::BundlePlan {
        bundle_manifest_template: parsed.bundle_manifest,
        descriptor_assets: planned_descriptors,
        dependency_resolution,
        departments: parsed
            .teams
            .iter()
            .map(|team| team.name.clone())
            .collect::<Vec<_>>(),
        detected_agent_count: parsed.agents.len() as u64,
        detected_team_count: parsed.teams.len() as u64,
        filtered_file_count,
        issues,
        skills: planned_skills,
        mcps: planned_mcps,
        agents: planned_agents,
        teams: planned_teams,
        avatars: parsed.avatars,
        asset_state: parsed.asset_state,
    })
}
