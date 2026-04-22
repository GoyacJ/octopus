pub(crate) fn preview_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: agent_assets::AssetTargetScope<'_>,
    input: ImportWorkspaceAgentBundlePreviewInput,
) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
    let plan = build_bundle_plan(connection, paths, workspace_id, target, &input.files)?;
    Ok(translation::plan_to_preview(&plan))
}

pub(crate) fn execute_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: agent_assets::AssetTargetScope<'_>,
    input: ImportWorkspaceAgentBundleInput,
) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
    let plan = build_bundle_plan(
        connection,
        paths,
        workspace_id,
        target.clone(),
        &input.files,
    )?;
    let mut issues = plan.issues.clone();
    let now = timestamp_now();
    let source_kind = target.source_kind();

    let mut total_create = 0_u64;
    let mut total_update = 0_u64;
    let mut total_skip = 0_u64;

    let mut failed_skill_slugs = BTreeSet::new();
    let mut skill_results = Vec::new();
    for skill in &plan.skills {
        let mut action = skill.action;
        if matches!(
            action,
            agent_assets::ImportAction::Create | agent_assets::ImportAction::Update
        ) {
            if let Err(error) = agent_assets::write_managed_skill(
                &target.skill_root(paths),
                &skill.slug,
                &skill.files,
            ) {
                action = agent_assets::ImportAction::Failed;
                failed_skill_slugs.insert(skill.slug.clone());
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_SKILL,
                    skill.source_ids.first().cloned(),
                    format!("failed to import skill '{}': {error}", skill.slug),
                ));
            }
        }
        if action != agent_assets::ImportAction::Failed {
            for source_id in &skill.source_ids {
                agent_assets::upsert_skill_import_source(
                    connection,
                    &source_kind,
                    source_id,
                    &skill.content_hash,
                    &skill.slug,
                    now,
                )?;
            }
        }
        translation::increment_action_counts(
            action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
        skill_results.push(ImportedSkillPreviewItem {
            slug: skill.slug.clone(),
            skill_id: skill.skill_id.clone(),
            name: skill.name.clone(),
            action: action.as_str().into(),
            content_hash: skill.content_hash.clone(),
            file_count: skill.file_count as u64,
            source_ids: skill.source_ids.clone(),
            departments: Vec::new(),
            agent_names: skill.consumer_names.clone(),
        });
    }

    for descriptor in &plan.descriptor_assets {
        let mut record = descriptor.record.clone();
        record.updated_at = now;
        record.import_metadata.imported_at = Some(now);
        if matches!(
            descriptor.action,
            agent_assets::ImportAction::Create | agent_assets::ImportAction::Update
        ) {
            if let Err(error) =
                agent_assets::persist_bundle_descriptor(paths, &record, &descriptor.bytes)
            {
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_BUNDLE,
                    Some(record.source_id.clone()),
                    format!(
                        "failed to persist {} descriptor '{}': {error}",
                        record.asset_kind, record.display_name
                    ),
                ));
                continue;
            }
            crate::infra_state::write_bundle_asset_descriptor_record(
                connection,
                &record,
                descriptor.action == agent_assets::ImportAction::Update,
            )?;
        } else if descriptor.action == agent_assets::ImportAction::Skip {
            crate::infra_state::write_bundle_asset_descriptor_record(connection, &record, true)?;
        }
        translation::increment_action_counts(
            descriptor.action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
    }

    let existing_mcp_target = agent_assets::load_target_runtime_document(paths, &target)?;
    let effective_mcp_document =
        agent_assets::plan_mcp_document_updates(existing_mcp_target, &plan.mcps, &mut issues)?;
    agent_assets::write_target_runtime_document(paths, &target, &effective_mcp_document)?;
    agent_assets::apply_imported_asset_state(paths, &target, &plan)?;

    let mut mcp_results = Vec::new();
    for mcp in &plan.mcps {
        translation::increment_action_counts(
            mcp.action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
        mcp_results.push(ImportedMcpPreviewItem {
            server_name: mcp.server_name.clone(),
            action: mcp.action.as_str().into(),
            content_hash: mcp.content_hash.clone(),
            source_ids: mcp.source_ids.clone(),
            consumer_names: mcp.consumer_names.clone(),
            referenced_only: mcp.referenced_only,
        });
    }

    let existing_agents = agent_assets::load_scoped_agents(connection, &target)?;
    let mut agent_results = Vec::new();
    let mut agent_id_by_source = HashMap::new();
    for agent in &plan.agents {
        let usable_skill_slugs = agent
            .skill_slugs
            .iter()
            .filter(|slug| !failed_skill_slugs.contains(*slug))
            .cloned()
            .collect::<Vec<_>>();
        let skill_ids = usable_skill_slugs
            .iter()
            .map(|slug| crate::agent_bundle::shared::managed_skill_id(slug))
            .collect::<Vec<_>>();
        let actual_action = agent_assets::resolve_agent_action(
            workspace_id,
            &target,
            &existing_agents,
            agent,
            &skill_ids,
        )?;
        let mut result_action = actual_action;
        let agent_id = agent.agent_id.clone().unwrap_or_else(|| {
            agent_assets::deterministic_asset_id("agent", &target, &agent.source_id)
        });
        let avatar_path = match agent_assets::persist_avatar(paths, &agent_id, &agent.avatar) {
            Ok(path) => path,
            Err(error) => {
                result_action = agent_assets::ImportAction::Failed;
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_AVATAR,
                    Some(agent.source_id.clone()),
                    format!("failed to persist agent avatar '{}': {error}", agent.name),
                ));
                None
            }
        };

        if result_action != agent_assets::ImportAction::Failed
            && matches!(
                actual_action,
                agent_assets::ImportAction::Create | agent_assets::ImportAction::Update
            )
        {
            let mut record = agent_assets::build_agent_record(
                paths,
                workspace_id,
                &target,
                &agent_id,
                &agent.name,
                avatar_path.clone(),
                &agent.description,
                &agent.personality,
                &agent.prompt,
                &agent.tags,
                &agent.builtin_tool_keys,
                &skill_ids,
                &agent.mcp_server_names,
            );
            record.default_model_strategy =
                agent_assets::model_strategy_from_template(agent.model.as_deref());
            record.trust_metadata = plan
                .bundle_manifest_template
                .as_ref()
                .map(|manifest| manifest.trust_metadata.clone())
                .unwrap_or_else(default_asset_trust_metadata);
            record.dependency_resolution = plan.dependency_resolution.clone();
            record.import_metadata = octopus_core::AssetImportMetadata {
                origin_kind: "bundle-import".into(),
                source_id: Some(agent.source_id.clone()),
                manifest_version: ASSET_IMPORT_MANIFEST_VERSION,
                translation_status: translation::import_action_translation_mode(result_action)
                    .into(),
                imported_at: Some(now),
            };
            if let Err(error) = agent_assets::write_agent_record(
                connection,
                &record,
                actual_action == agent_assets::ImportAction::Update,
            ) {
                result_action = agent_assets::ImportAction::Failed;
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_AGENT,
                    Some(agent.source_id.clone()),
                    format!("failed to import agent '{}': {error}", agent.name),
                ));
            } else {
                agent_assets::upsert_agent_import_source(
                    connection,
                    &source_kind,
                    &agent.source_id,
                    &agent_id,
                    now,
                )?;
            }
        } else if result_action != agent_assets::ImportAction::Failed {
            agent_assets::upsert_agent_import_source(
                connection,
                &source_kind,
                &agent.source_id,
                &agent_id,
                now,
            )?;
        }

        translation::increment_action_counts(
            result_action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
        if result_action != agent_assets::ImportAction::Failed {
            agent_id_by_source.insert(agent.source_id.clone(), agent_id.clone());
        }
        agent_results.push(ImportedAgentPreviewItem {
            source_id: agent.source_id.clone(),
            agent_id: Some(agent_id),
            name: agent.name.clone(),
            department: agent.department.clone(),
            action: result_action.as_str().into(),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            skill_slugs: usable_skill_slugs,
            mcp_server_names: agent.mcp_server_names.clone(),
            task_domains: normalize_task_domains(agent.tags.clone()),
            translation_mode: translation::import_action_translation_mode(result_action).into(),
        });
    }

    let existing_teams = agent_assets::load_scoped_teams(connection, &target)?;
    let mut team_results = Vec::new();
    for team in &plan.teams {
        let skill_ids = team
            .skill_slugs
            .iter()
            .filter(|slug| !failed_skill_slugs.contains(*slug))
            .map(|slug| crate::agent_bundle::shared::managed_skill_id(slug))
            .collect::<Vec<_>>();
        let member_agent_record_ids = team
            .agent_source_ids
            .iter()
            .filter_map(|source_id| agent_id_by_source.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let leader_agent_record_id = team.leader_name.as_ref().and_then(|leader_name| {
            team.agent_source_ids.iter().find_map(|source_id| {
                let agent = plan
                    .agents
                    .iter()
                    .find(|candidate| &candidate.source_id == source_id)?;
                if &agent.name == leader_name {
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
        let actual_action = agent_assets::resolve_team_action(
            workspace_id,
            &target,
            &existing_teams,
            team,
            &skill_ids,
            &leader_ref,
            &member_refs,
        )?;
        let mut result_action = actual_action;
        let team_id = team.team_id.clone().unwrap_or_else(|| {
            agent_assets::deterministic_asset_id("team", &target, &team.source_id)
        });
        let avatar_path = match agent_assets::persist_avatar(paths, &team_id, &team.avatar) {
            Ok(path) => path,
            Err(error) => {
                result_action = agent_assets::ImportAction::Failed;
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_AVATAR,
                    Some(team.source_id.clone()),
                    format!("failed to persist team avatar '{}': {error}", team.name),
                ));
                None
            }
        };

        if result_action != agent_assets::ImportAction::Failed
            && matches!(
                actual_action,
                agent_assets::ImportAction::Create | agent_assets::ImportAction::Update
            )
        {
            let mut record = agent_assets::build_team_record(
                paths,
                workspace_id,
                &target,
                &team_id,
                &team.name,
                avatar_path.clone(),
                &team.description,
                &team.personality,
                &team.prompt,
                &team.tags,
                &team.builtin_tool_keys,
                &skill_ids,
                &team.mcp_server_names,
                leader_ref.clone(),
                member_refs.clone(),
            );
            record.default_model_strategy =
                agent_assets::model_strategy_from_template(team.model.as_deref());
            record.trust_metadata = plan
                .bundle_manifest_template
                .as_ref()
                .map(|manifest| manifest.trust_metadata.clone())
                .unwrap_or_else(default_asset_trust_metadata);
            record.dependency_resolution = plan.dependency_resolution.clone();
            record.import_metadata = octopus_core::AssetImportMetadata {
                origin_kind: "bundle-import".into(),
                source_id: Some(team.source_id.clone()),
                manifest_version: ASSET_IMPORT_MANIFEST_VERSION,
                translation_status: translation::import_action_translation_mode(result_action)
                    .into(),
                imported_at: Some(now),
            };
            if let Err(error) = crate::infra_state::write_team_record(
                connection,
                &record,
                actual_action == agent_assets::ImportAction::Update,
            ) {
                result_action = agent_assets::ImportAction::Failed;
                issues.push(agent_assets::issue(
                    agent_assets::ISSUE_ERROR,
                    agent_assets::SOURCE_SCOPE_TEAM,
                    Some(team.source_id.clone()),
                    format!("failed to import team '{}': {error}", team.name),
                ));
            } else {
                agent_assets::upsert_team_import_source(
                    connection,
                    &source_kind,
                    &team.source_id,
                    &team_id,
                    now,
                )?;
            }
        } else if result_action != agent_assets::ImportAction::Failed {
            agent_assets::upsert_team_import_source(
                connection,
                &source_kind,
                &team.source_id,
                &team_id,
                now,
            )?;
        }

        translation::increment_action_counts(
            result_action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
        team_results.push(ImportedTeamPreviewItem {
            source_id: team.source_id.clone(),
            team_id: Some(team_id),
            name: team.name.clone(),
            action: result_action.as_str().into(),
            leader_name: team.leader_name.clone(),
            member_names: team.member_names.clone(),
            agent_source_ids: team.agent_source_ids.clone(),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: normalize_task_domains(team.tags.clone()),
            translation_mode: translation::import_action_translation_mode(result_action).into(),
        });
    }

    let avatar_results = plan
        .avatars
        .iter()
        .map(|avatar| ImportedAvatarPreviewItem {
            source_id: avatar.source_id.clone(),
            owner_kind: avatar.owner_kind.clone(),
            owner_name: avatar.owner_name.clone(),
            file_name: avatar.file_name.clone(),
            generated: avatar.generated,
        })
        .collect::<Vec<_>>();

    let translation_report = translation::translation_report_from_issues(
        &plan,
        &issues,
        plan.dependency_resolution.clone(),
    );

    Ok(ImportWorkspaceAgentBundleResult {
        departments: plan.departments.clone(),
        bundle_manifest: manifest_v2::bundle_manifest_from_plan(&plan),
        department_count: plan.departments.len() as u64,
        detected_agent_count: plan.detected_agent_count,
        importable_agent_count: plan.agents.len() as u64,
        detected_team_count: plan.detected_team_count,
        importable_team_count: plan.teams.len() as u64,
        create_count: total_create,
        update_count: total_update,
        skip_count: total_skip,
        failure_count: issues
            .iter()
            .filter(|entry| entry.severity == agent_assets::ISSUE_ERROR)
            .count() as u64,
        unique_skill_count: skill_results.len() as u64,
        unique_mcp_count: mcp_results.len() as u64,
        agent_count: agent_results.len() as u64,
        team_count: team_results.len() as u64,
        skill_count: skill_results.len() as u64,
        mcp_count: mcp_results.len() as u64,
        avatar_count: avatar_results.len() as u64,
        filtered_file_count: plan.filtered_file_count,
        agents: agent_results,
        teams: team_results,
        skills: skill_results,
        mcps: mcp_results,
        avatars: avatar_results,
        issues,
        translation_report,
    })
}
