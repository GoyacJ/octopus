use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::{
    agent_assets::{
        BuiltinAgentTemplateSource, BuiltinCatalogSources, BuiltinMcpAsset, BuiltinSkillAsset,
        BuiltinTeamTemplateSource, BundleFile, BUILTIN_SKILL_DISPLAY_ROOT, SKILL_FRONTMATTER_FILE,
    },
    catalog_hash_id,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use include_dir::{Dir, DirEntry};
use octopus_core::{
    capability_policy_from_sources, default_agent_asset_role, default_agent_delegation_policy,
    default_agent_memory_policy, default_agent_shared_capability_policy,
    default_approval_preference, default_artifact_handoff_policy, default_asset_import_metadata,
    default_asset_trust_metadata, default_mailbox_policy, default_output_contract,
    default_permission_envelope, default_shared_memory_policy, default_team_delegation_policy,
    default_team_memory_policy, default_team_shared_capability_policy, normalize_task_domains,
    team_topology_from_refs, workflow_affordance_from_task_domains, AgentRecord, AppError,
    TeamRecord, WorkspaceDirectoryUploadEntry, ASSET_MANIFEST_REVISION_V2,
};

static BUILTIN_BUNDLE_ASSET_DIR: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../templates");

pub(crate) fn embedded_bundle_files(dir: &Dir<'_>) -> Result<Vec<BundleFile>, AppError> {
    let mut files = Vec::new();
    collect_embedded_bundle_files(dir, "", &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

pub(crate) fn extract_builtin_agent_template_files(
    agent_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    let Some(root_dir) = find_builtin_agent_template_root(agent_id)? else {
        return Ok(None);
    };
    Ok(Some(encode_builtin_bundle_entries(&root_dir)?))
}

pub(crate) fn extract_builtin_team_template_files(
    team_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    let Some(root_dir) = find_builtin_team_template_root(team_id)? else {
        return Ok(None);
    };
    Ok(Some(encode_builtin_bundle_entries(&root_dir)?))
}

pub(crate) fn list_builtin_skill_assets() -> Result<Vec<BuiltinSkillAsset>, AppError> {
    let mut unique_skills =
        BTreeMap::<(String, String), (String, String, Vec<String>, Vec<(String, Vec<u8>)>)>::new();
    for source in load_builtin_catalog_sources()?.skill_sources {
        unique_skills
            .entry((source.canonical_slug.clone(), source.content_hash.clone()))
            .or_insert_with(|| {
                (
                    source.name.clone(),
                    source.description.clone(),
                    Vec::new(),
                    source.files.clone(),
                )
            })
            .2
            .push(source.source_id);
    }

    let mut assigned_hash_by_slug = BTreeMap::<String, String>::new();
    let mut assets = Vec::new();
    for ((canonical_slug, content_hash), (name, description, mut source_ids, files)) in
        unique_skills
    {
        let slug = match assigned_hash_by_slug.get(&canonical_slug) {
            Some(existing_hash) if existing_hash != &content_hash => {
                format!("{canonical_slug}-{}", short_hash(&content_hash))
            }
            _ => canonical_slug,
        };
        assigned_hash_by_slug.insert(slug.clone(), content_hash);
        source_ids.sort();
        source_ids.dedup();
        assets.push(BuiltinSkillAsset {
            slug: slug.clone(),
            name,
            description,
            display_path: format!("{BUILTIN_SKILL_DISPLAY_ROOT}/{slug}/{SKILL_FRONTMATTER_FILE}"),
            root_display_path: format!("{BUILTIN_SKILL_DISPLAY_ROOT}/{slug}"),
            files,
        });
    }
    assets.sort_by(|left, right| left.name.cmp(&right.name).then(left.slug.cmp(&right.slug)));
    Ok(assets)
}

pub(crate) fn find_builtin_skill_asset_by_id(
    skill_id: &str,
) -> Result<Option<BuiltinSkillAsset>, AppError> {
    Ok(list_builtin_skill_assets()?
        .into_iter()
        .find(|asset| catalog_hash_id("skill", &asset.display_path) == skill_id))
}

pub(crate) fn list_builtin_agent_templates(
    workspace_id: &str,
) -> Result<Vec<AgentRecord>, AppError> {
    let catalog = load_builtin_catalog_sources()?;
    let skill_id_by_source = build_builtin_skill_id_by_source_id(&catalog.skill_sources);
    let mut records = catalog
        .agent_sources
        .into_iter()
        .map(|source| build_builtin_agent_record(workspace_id, &skill_id_by_source, source))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(records)
}

pub(crate) fn find_builtin_agent_template_record(
    workspace_id: &str,
    agent_id: &str,
) -> Result<Option<AgentRecord>, AppError> {
    let catalog = load_builtin_catalog_sources()?;
    let skill_id_by_source = build_builtin_skill_id_by_source_id(&catalog.skill_sources);
    Ok(catalog
        .agent_sources
        .into_iter()
        .find(|source| catalog_hash_id("builtin-agent", &source.source_id) == agent_id)
        .map(|source| build_builtin_agent_record(workspace_id, &skill_id_by_source, source)))
}

pub(crate) fn list_builtin_team_templates(workspace_id: &str) -> Result<Vec<TeamRecord>, AppError> {
    let catalog = load_builtin_catalog_sources()?;
    let skill_id_by_source = build_builtin_skill_id_by_source_id(&catalog.skill_sources);
    let mut records = catalog
        .team_sources
        .into_iter()
        .map(|source| build_builtin_team_record(workspace_id, &skill_id_by_source, source))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(records)
}

pub(crate) fn find_builtin_team_template_record(
    workspace_id: &str,
    team_id: &str,
) -> Result<Option<TeamRecord>, AppError> {
    let catalog = load_builtin_catalog_sources()?;
    let skill_id_by_source = build_builtin_skill_id_by_source_id(&catalog.skill_sources);
    Ok(catalog
        .team_sources
        .into_iter()
        .find(|source| catalog_hash_id("builtin-team", &source.source_id) == team_id)
        .map(|source| build_builtin_team_record(workspace_id, &skill_id_by_source, source)))
}

pub(crate) fn find_builtin_agent_template_root(agent_id: &str) -> Result<Option<String>, AppError> {
    Ok(load_builtin_catalog_sources()?
        .agent_sources
        .into_iter()
        .find(|source| catalog_hash_id("builtin-agent", &source.source_id) == agent_id)
        .map(|source| source.source_id))
}

pub(crate) fn find_builtin_team_template_root(team_id: &str) -> Result<Option<String>, AppError> {
    Ok(load_builtin_catalog_sources()?
        .team_sources
        .into_iter()
        .find(|source| catalog_hash_id("builtin-team", &source.source_id) == team_id)
        .map(|source| source.source_id))
}

pub(crate) fn list_builtin_mcp_assets() -> Result<Vec<BuiltinMcpAsset>, AppError> {
    Ok(Vec::new())
}

pub(crate) fn find_builtin_mcp_asset(
    server_name: &str,
) -> Result<Option<BuiltinMcpAsset>, AppError> {
    Ok(list_builtin_mcp_assets()?
        .into_iter()
        .find(|asset| asset.server_name == server_name))
}

fn collect_embedded_bundle_files(
    dir: &Dir<'_>,
    prefix: &str,
    files: &mut Vec<BundleFile>,
) -> Result<(), AppError> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => {
                let name = child
                    .path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| AppError::invalid_input("invalid builtin asset directory"))?;
                let next_prefix = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}/{name}")
                };
                collect_embedded_bundle_files(child, &next_prefix, files)?;
            }
            DirEntry::File(file) => {
                let name = file
                    .path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| AppError::invalid_input("invalid builtin asset file name"))?;
                let relative_path = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}/{name}")
                };
                files.push(BundleFile {
                    relative_path,
                    bytes: file.contents().to_vec(),
                });
            }
        }
    }
    Ok(())
}

fn encode_builtin_bundle_entries(
    root_dir: &str,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, AppError> {
    let prefix = format!("{root_dir}/");
    let mut files = embedded_bundle_files(&BUILTIN_BUNDLE_ASSET_DIR)?
        .into_iter()
        .filter(|file| file.relative_path == root_dir || file.relative_path.starts_with(&prefix))
        .map(|file| {
            encode_file(
                &file.relative_path,
                content_type_for_export(&file.relative_path),
                file.bytes,
            )
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if files.is_empty() {
        return Err(AppError::not_found("builtin template bundle"));
    }
    Ok(files)
}

fn content_type_for_export(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

fn load_builtin_catalog_sources() -> Result<BuiltinCatalogSources, AppError> {
    crate::agent_assets::load_builtin_catalog_sources()
}

fn build_builtin_skill_id_by_source_id(
    skill_sources: &[crate::agent_assets::BuiltinSkillCatalogSource],
) -> HashMap<String, String> {
    let mut unique_skills = BTreeMap::<(String, String), Vec<String>>::new();
    for source in skill_sources {
        unique_skills
            .entry((source.canonical_slug.clone(), source.content_hash.clone()))
            .or_default()
            .push(source.source_id.clone());
    }

    let mut assigned_hash_by_slug = BTreeMap::<String, String>::new();
    let mut map = HashMap::new();
    for ((canonical_slug, content_hash), mut source_ids) in unique_skills {
        let slug = match assigned_hash_by_slug.get(&canonical_slug) {
            Some(existing_hash) if existing_hash != &content_hash => {
                format!("{canonical_slug}-{}", short_hash(&content_hash))
            }
            _ => canonical_slug,
        };
        assigned_hash_by_slug.insert(slug.clone(), content_hash);
        source_ids.sort();
        source_ids.dedup();
        let display_path = format!("{BUILTIN_SKILL_DISPLAY_ROOT}/{slug}/{SKILL_FRONTMATTER_FILE}");
        let skill_id = catalog_hash_id("skill", &display_path);
        for source_id in source_ids {
            map.insert(source_id, skill_id.clone());
        }
    }
    map
}

fn build_builtin_agent_record(
    workspace_id: &str,
    skill_id_by_source: &HashMap<String, String>,
    source: BuiltinAgentTemplateSource,
) -> AgentRecord {
    let skill_ids = source
        .skill_source_ids
        .iter()
        .filter_map(|source_id| skill_id_by_source.get(source_id))
        .cloned()
        .collect::<Vec<_>>();
    let task_domains = normalize_task_domains(source.tags.clone());

    AgentRecord {
        id: catalog_hash_id("builtin-agent", &source.source_id),
        workspace_id: workspace_id.to_string(),
        project_id: None,
        scope: "workspace".into(),
        owner_user_id: None,
        asset_role: default_agent_asset_role(),
        name: source.name,
        avatar_path: None,
        avatar: source.avatar_data_url,
        personality: source.personality,
        tags: source.tags,
        prompt: source.prompt,
        builtin_tool_keys: source.builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: source.mcp_server_names.clone(),
        task_domains,
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: crate::agent_assets::model_strategy_from_template(
            source.model.as_deref(),
        ),
        capability_policy: capability_policy_from_sources(
            &source.builtin_tool_keys,
            &skill_ids,
            &source.mcp_server_names,
        ),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_agent_memory_policy(),
        delegation_policy: default_agent_delegation_policy(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_agent_shared_capability_policy(),
        integration_source: Some(octopus_core::WorkspaceLinkIntegrationSource {
            kind: "builtin-template".into(),
            source_id: source.source_id,
        }),
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: source.description,
        status: "active".into(),
        updated_at: 0,
    }
}

fn build_builtin_team_record(
    workspace_id: &str,
    skill_id_by_source: &HashMap<String, String>,
    source: BuiltinTeamTemplateSource,
) -> TeamRecord {
    let skill_ids = source
        .skill_source_ids
        .iter()
        .filter_map(|source_id| skill_id_by_source.get(source_id))
        .cloned()
        .collect::<Vec<_>>();
    let task_domains = normalize_task_domains(source.tags.clone());
    let delegation_policy = default_team_delegation_policy();
    let leader_agent_record_id = source
        .leader_agent_source_id
        .as_ref()
        .map(|source_id| catalog_hash_id("builtin-agent", source_id));
    let member_agent_record_ids = source
        .member_agent_source_ids
        .iter()
        .map(|source_id| catalog_hash_id("builtin-agent", source_id))
        .collect::<Vec<_>>();
    let leader_ref = leader_agent_record_id
        .as_deref()
        .map(crate::canonical_agent_ref)
        .unwrap_or_default();
    let member_refs = crate::canonical_agent_refs(&member_agent_record_ids);

    TeamRecord {
        id: catalog_hash_id("builtin-team", &source.source_id),
        workspace_id: workspace_id.to_string(),
        project_id: None,
        scope: "workspace".into(),
        name: source.name,
        avatar_path: None,
        avatar: source.avatar_data_url,
        personality: source.personality,
        tags: source.tags,
        prompt: source.prompt,
        builtin_tool_keys: source.builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: source.mcp_server_names.clone(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: crate::agent_assets::model_strategy_from_template(
            source.model.as_deref(),
        ),
        capability_policy: capability_policy_from_sources(
            &source.builtin_tool_keys,
            &skill_ids,
            &source.mcp_server_names,
        ),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_team_memory_policy(),
        delegation_policy: delegation_policy.clone(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_team_shared_capability_policy(),
        leader_ref: leader_ref.clone(),
        member_refs: member_refs.clone(),
        team_topology: team_topology_from_refs(Some(leader_ref), member_refs.clone()),
        shared_memory_policy: default_shared_memory_policy(),
        mailbox_policy: default_mailbox_policy(),
        artifact_handoff_policy: default_artifact_handoff_policy(),
        workflow_affordance: workflow_affordance_from_task_domains(&task_domains, true, true),
        worker_concurrency_limit: delegation_policy.max_worker_count,
        integration_source: Some(octopus_core::WorkspaceLinkIntegrationSource {
            kind: "builtin-template".into(),
            source_id: source.source_id,
        }),
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: source.description,
        status: "active".into(),
        updated_at: 0,
    }
}

fn short_hash(value: &str) -> String {
    value
        .rsplit('-')
        .next()
        .unwrap_or(value)
        .chars()
        .take(8)
        .collect()
}

fn encode_file(
    relative_path: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> WorkspaceDirectoryUploadEntry {
    WorkspaceDirectoryUploadEntry {
        relative_path: relative_path.to_string(),
        file_name: Path::new(relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
        content_type: content_type.to_string(),
        data_base64: BASE64_STANDARD.encode(&bytes),
        byte_size: bytes.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_builtin_skill_id_by_source_id, embedded_bundle_files,
        extract_builtin_agent_template_files, extract_builtin_team_template_files,
        find_builtin_agent_template_root, find_builtin_mcp_asset, find_builtin_skill_asset_by_id,
        find_builtin_team_template_root, list_builtin_agent_templates, list_builtin_mcp_assets,
        list_builtin_skill_assets, list_builtin_team_templates, load_builtin_catalog_sources,
        BUILTIN_BUNDLE_ASSET_DIR,
    };
    use crate::catalog_hash_id;

    #[test]
    fn embedded_bundle_files_returns_sorted_relative_paths() {
        let files = embedded_bundle_files(&BUILTIN_BUNDLE_ASSET_DIR).expect("embedded files");

        assert!(!files.is_empty());
        assert!(files
            .windows(2)
            .all(|pair| pair[0].relative_path <= pair[1].relative_path));
        assert!(
            files.iter().any(|file| file.relative_path.ends_with(".md")),
            "expected markdown bundle entries"
        );
    }

    #[test]
    fn extract_builtin_agent_template_files_returns_bundle_entries_for_known_builtin() {
        let builtin_agent = list_builtin_agent_templates("ws-local")
            .expect("builtin agents")
            .into_iter()
            .next()
            .expect("at least one builtin agent");

        let root = find_builtin_agent_template_root(&builtin_agent.id)
            .expect("find builtin agent root")
            .expect("builtin agent root");
        let files = extract_builtin_agent_template_files(&builtin_agent.id)
            .expect("extract builtin agent")
            .expect("builtin agent files");

        assert!(!files.is_empty());
        assert!(files.iter().any(|file| file.relative_path.ends_with(".md")));
        assert!(files.iter().all(|file| file.relative_path == root
            || file.relative_path.starts_with(&(root.clone() + "/"))));
    }

    #[test]
    fn extract_builtin_team_template_files_returns_bundle_entries_for_known_builtin() {
        let builtin_team = list_builtin_team_templates("ws-local")
            .expect("builtin teams")
            .into_iter()
            .next()
            .expect("at least one builtin team");

        let root = find_builtin_team_template_root(&builtin_team.id)
            .expect("find builtin team root")
            .expect("builtin team root");
        let files = extract_builtin_team_template_files(&builtin_team.id)
            .expect("extract builtin team")
            .expect("builtin team files");

        assert!(!files.is_empty());
        assert!(files.iter().any(|file| file.relative_path.ends_with(".md")));
        assert!(files.iter().all(|file| file.relative_path == root
            || file.relative_path.starts_with(&(root.clone() + "/"))));
    }

    #[test]
    fn list_builtin_mcp_assets_returns_empty_when_templates_do_not_embed_mcp_configs() {
        let assets = list_builtin_mcp_assets().expect("builtin mcp assets");
        assert!(assets.is_empty());
        assert!(find_builtin_mcp_asset("finance-data")
            .expect("find builtin mcp")
            .is_none());
    }

    #[test]
    fn find_builtin_skill_asset_by_id_returns_known_asset() {
        let skill = list_builtin_skill_assets()
            .expect("builtin skill assets")
            .into_iter()
            .next()
            .expect("at least one builtin skill");
        let skill_id = catalog_hash_id("skill", &skill.display_path);

        let loaded = find_builtin_skill_asset_by_id(&skill_id)
            .expect("find builtin skill")
            .expect("builtin skill asset");

        assert_eq!(loaded.display_path, skill.display_path);
        assert!(!loaded.files.is_empty());
    }

    #[test]
    fn builtin_skill_id_mapping_covers_all_source_ids() {
        let assets = list_builtin_skill_assets().expect("builtin skill assets");
        let catalog = load_builtin_catalog_sources().expect("builtin catalog sources");
        let mapping = build_builtin_skill_id_by_source_id(&catalog.skill_sources);

        assert!(!mapping.is_empty());
        let asset_ids = assets
            .into_iter()
            .map(|asset| catalog_hash_id("skill", &asset.display_path))
            .collect::<std::collections::BTreeSet<_>>();
        for source in catalog.skill_sources {
            let mapped = mapping
                .get(&source.source_id)
                .expect("missing mapping for builtin skill source");
            assert!(
                asset_ids.contains(mapped),
                "unexpected skill id for source {}",
                source.source_id
            );
        }
    }

    #[test]
    fn load_builtin_catalog_sources_exposes_skill_agent_and_team_sources_together() {
        let catalog = load_builtin_catalog_sources().expect("builtin catalog sources");

        assert!(!catalog.skill_sources.is_empty());
        assert!(!catalog.agent_sources.is_empty());
        assert!(!catalog.team_sources.is_empty());
    }

    #[test]
    fn builtin_team_templates_keep_member_relationships_resolvable_to_builtin_agents() {
        let builtin_agents = list_builtin_agent_templates("ws-local").expect("builtin agents");
        let builtin_teams = list_builtin_team_templates("ws-local").expect("builtin teams");

        assert!(!builtin_teams.is_empty());

        let builtin_agent_refs = builtin_agents
            .iter()
            .map(|agent| crate::canonical_agent_ref(&agent.id))
            .collect::<std::collections::BTreeSet<_>>();

        let team_with_members = builtin_teams
            .iter()
            .find(|team| !team.member_refs.is_empty())
            .expect("expected at least one builtin team with members");

        assert!(
            team_with_members
                .member_refs
                .iter()
                .all(|agent_ref| builtin_agent_refs.contains(agent_ref)),
            "builtin team members should resolve to builtin agent refs",
        );

        assert!(
            builtin_agent_refs.contains(&team_with_members.leader_ref),
            "builtin team leader should resolve to a builtin agent ref",
        );

        assert_eq!(
            team_with_members.member_refs,
            team_with_members
                .member_refs
                .iter()
                .filter(|agent_ref| builtin_agent_refs.contains(*agent_ref))
                .cloned()
                .collect::<Vec<_>>(),
            "builtin team member refs should remain canonical actor refs",
        );
    }
}
