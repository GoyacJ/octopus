use std::collections::{BTreeMap, BTreeSet, HashMap};

use octopus_core::{
    default_asset_trust_metadata, normalize_task_domains, timestamp_now, AppError,
    ImportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundlePreview,
    ImportWorkspaceAgentBundlePreviewInput, ImportWorkspaceAgentBundleResult,
    ImportedAgentPreviewItem, ImportedAvatarPreviewItem, ImportedMcpPreviewItem,
    ImportedSkillPreviewItem, ImportedTeamPreviewItem, WorkspaceDirectoryUploadEntry,
    ASSET_IMPORT_MANIFEST_VERSION, ASSET_MANIFEST_REVISION_V2,
};
use rusqlite::Connection;

use crate::{agent_assets, WorkspacePaths};

use super::{manifest_v2, translation};

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
        let member_agent_ids = team
            .agent_source_ids
            .iter()
            .filter_map(|source_id| agent_id_by_source.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let leader_agent_id = team.leader_name.as_ref().and_then(|leader_name| {
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
        let actual_action = agent_assets::resolve_team_action(
            workspace_id,
            &target,
            &existing_teams,
            team,
            &skill_ids,
            leader_agent_id.as_deref(),
            &member_agent_ids,
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
                leader_agent_id.clone(),
                member_agent_ids.clone(),
            );
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
        let member_agent_ids = parsed_team
            .agent_source_ids
            .iter()
            .filter_map(|source_id| agent_id_by_source.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let leader_agent_id = parsed_team.leader_name.as_ref().and_then(|leader_name| {
            parsed_team.agent_source_ids.iter().find_map(|source_id| {
                if agent_name_by_source.get(source_id) == Some(leader_name) {
                    agent_id_by_source.get(source_id).cloned()
                } else {
                    None
                }
            })
        });
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
            },
            &skill_slugs
                .iter()
                .map(|slug| crate::agent_bundle::shared::managed_skill_id(slug))
                .collect::<Vec<_>>(),
            leader_agent_id.as_deref(),
            &member_agent_ids,
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
