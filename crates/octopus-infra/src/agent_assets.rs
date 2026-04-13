use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{
    capability_policy_from_sources, default_agent_delegation_policy, default_agent_memory_policy,
    default_agent_shared_capability_policy, default_approval_preference,
    default_artifact_handoff_policy, default_asset_import_metadata, default_asset_trust_metadata,
    default_mailbox_policy, default_model_strategy, default_output_contract,
    default_permission_envelope, default_shared_memory_policy, default_team_delegation_policy,
    default_team_memory_policy, default_team_shared_capability_policy, normalize_task_domains,
    team_topology_from_refs, timestamp_now, workflow_affordance_from_task_domains, AgentRecord,
    AppError, AssetBundleManifestV2, AssetDependency, AssetDependencyResolution,
    AssetTranslationReport, BundleAssetDescriptorRecord, ExportWorkspaceAgentBundleInput,
    ImportIssue, TeamRecord, WorkspaceDirectoryUploadEntry, ASSET_MANIFEST_REVISION_V2,
};
use rusqlite::{params, Connection};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use sha2::{Digest, Sha256};

use crate::{
    infra_state::{agent_avatar, load_agents, load_bundle_asset_descriptor_records, load_teams},
    resources_skills::{
        discover_skill_roots, load_skills_from_roots, load_workspace_asset_state_document,
        save_workspace_asset_state_document, set_workspace_asset_enabled,
        set_workspace_asset_trusted, skill_source_key, validate_skill_file_relative_path,
        validate_skill_slug, write_workspace_runtime_document, WorkspaceCapabilityAssetMetadata,
    },
    WorkspacePaths,
};

pub(crate) const ISSUE_WARNING: &str = "warning";
pub(crate) const ISSUE_ERROR: &str = "error";
pub(crate) const SOURCE_SCOPE_BUNDLE: &str = "bundle";
pub(crate) const SOURCE_SCOPE_AGENT: &str = "agent";
pub(crate) const SOURCE_SCOPE_TEAM: &str = "team";
pub(crate) const SOURCE_SCOPE_SKILL: &str = "skill";
pub(crate) const SOURCE_SCOPE_MCP: &str = "mcp";
pub(crate) const SOURCE_SCOPE_AVATAR: &str = "avatar";
pub(crate) const SKILL_FRONTMATTER_FILE: &str = "SKILL.md";
pub(crate) const BUNDLE_ASSET_STATE_PATH: &str = ".octopus/asset-state.json";
const RESERVED_DIRS: &[&str] = &["skills", "mcps", ".octopus"];
const FILTERED_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    ".cache",
    ".turbo",
    "dist",
    "build",
    "coverage",
    "__pycache__",
    ".venv",
    "venv",
];

static DEFAULT_EMPLOYEE_AVATARS: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../packages/assets/header/employee");
static DEFAULT_LEADER_AVATARS: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../packages/assets/header/leader");
static BUILTIN_BUNDLE_ASSET_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/seed/builtin-assets/bundle");

pub(crate) const BUILTIN_SKILL_DISPLAY_ROOT: &str = "builtin-assets/skills";

#[derive(Debug, Clone)]
pub(crate) enum AssetTargetScope<'a> {
    Workspace,
    Project(&'a str),
}

impl AssetTargetScope<'_> {
    pub(crate) fn scope_label(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Project(_) => "project",
        }
    }

    pub(crate) fn project_id(&self) -> Option<&str> {
        match self {
            Self::Workspace => None,
            Self::Project(project_id) => Some(project_id),
        }
    }

    pub(crate) fn source_kind(&self) -> String {
        match self {
            Self::Workspace => String::from("user_import:workspace"),
            Self::Project(project_id) => format!("user_import:project:{project_id}"),
        }
    }

    pub(crate) fn skill_root(&self, paths: &WorkspacePaths) -> PathBuf {
        match self {
            Self::Workspace => paths.managed_skills_dir.clone(),
            Self::Project(project_id) => paths.project_skills_root(project_id),
        }
    }

    pub(crate) fn runtime_document_path(&self, paths: &WorkspacePaths) -> PathBuf {
        match self {
            Self::Workspace => paths.runtime_config_dir.join("workspace.json"),
            Self::Project(project_id) => paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
        }
    }

    pub(crate) fn avatar_seed_key(&self, source_id: &str) -> String {
        format!("{}:{source_id}", self.scope_label())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BundleFile {
    pub(crate) relative_path: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinSkillAsset {
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) display_path: String,
    pub(crate) root_display_path: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinSkillCatalogSource {
    pub(crate) source_id: String,
    pub(crate) name: String,
    pub(crate) canonical_slug: String,
    pub(crate) content_hash: String,
    pub(crate) description: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinAgentTemplateSource {
    pub(crate) source_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) avatar_data_url: Option<String>,
    pub(crate) personality: String,
    pub(crate) tags: Vec<String>,
    pub(crate) prompt: String,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_source_ids: Vec<String>,
    pub(crate) mcp_server_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinTeamTemplateSource {
    pub(crate) source_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) avatar_data_url: Option<String>,
    pub(crate) personality: String,
    pub(crate) tags: Vec<String>,
    pub(crate) prompt: String,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_source_ids: Vec<String>,
    pub(crate) mcp_server_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinCatalogSources {
    pub(crate) skill_sources: Vec<BuiltinSkillCatalogSource>,
    pub(crate) agent_sources: Vec<BuiltinAgentTemplateSource>,
    pub(crate) team_sources: Vec<BuiltinTeamTemplateSource>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinMcpAsset {
    pub(crate) server_name: String,
    pub(crate) display_path: String,
    pub(crate) config: JsonValue,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAssetAvatar {
    pub(crate) source_id: String,
    pub(crate) owner_kind: String,
    pub(crate) owner_name: String,
    pub(crate) file_name: String,
    pub(crate) content_type: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) generated: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAgent {
    pub(crate) source_id: String,
    pub(crate) team_name: Option<String>,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) personality: String,
    pub(crate) prompt: String,
    pub(crate) tags: Vec<String>,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_source_ids: Vec<String>,
    pub(crate) mcp_source_ids: Vec<String>,
    pub(crate) avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedTeam {
    pub(crate) source_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) personality: String,
    pub(crate) prompt: String,
    pub(crate) tags: Vec<String>,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_source_ids: Vec<String>,
    pub(crate) mcp_source_ids: Vec<String>,
    pub(crate) leader_name: Option<String>,
    pub(crate) member_names: Vec<String>,
    pub(crate) agent_source_ids: Vec<String>,
    pub(crate) avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedSkillSource {
    pub(crate) source_id: String,
    pub(crate) owner_name: String,
    pub(crate) name: String,
    pub(crate) canonical_slug: String,
    pub(crate) content_hash: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedMcpSource {
    pub(crate) source_id: String,
    pub(crate) owner_name: String,
    pub(crate) server_name: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) config: Option<JsonValue>,
    pub(crate) referenced_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportAction {
    Create,
    Update,
    Skip,
    Failed,
}

impl ImportAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Skip => "skip",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedSkill {
    pub(crate) slug: String,
    pub(crate) skill_id: String,
    pub(crate) name: String,
    pub(crate) action: ImportAction,
    pub(crate) content_hash: String,
    pub(crate) file_count: usize,
    pub(crate) source_ids: Vec<String>,
    pub(crate) consumer_names: Vec<String>,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedMcp {
    pub(crate) server_name: String,
    pub(crate) action: ImportAction,
    pub(crate) content_hash: Option<String>,
    pub(crate) source_ids: Vec<String>,
    pub(crate) consumer_names: Vec<String>,
    pub(crate) config: Option<JsonValue>,
    pub(crate) referenced_only: bool,
    pub(crate) resolved: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedAgent {
    pub(crate) source_id: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) name: String,
    pub(crate) department: String,
    pub(crate) action: ImportAction,
    pub(crate) description: String,
    pub(crate) personality: String,
    pub(crate) prompt: String,
    pub(crate) tags: Vec<String>,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_slugs: Vec<String>,
    pub(crate) mcp_server_names: Vec<String>,
    pub(crate) avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedTeam {
    pub(crate) source_id: String,
    pub(crate) team_id: Option<String>,
    pub(crate) name: String,
    pub(crate) action: ImportAction,
    pub(crate) description: String,
    pub(crate) personality: String,
    pub(crate) prompt: String,
    pub(crate) tags: Vec<String>,
    pub(crate) builtin_tool_keys: Vec<String>,
    pub(crate) skill_slugs: Vec<String>,
    pub(crate) mcp_server_names: Vec<String>,
    pub(crate) leader_name: Option<String>,
    pub(crate) member_names: Vec<String>,
    pub(crate) agent_source_ids: Vec<String>,
    pub(crate) avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedBundleDescriptor {
    pub(crate) action: ImportAction,
    pub(crate) record: BundleAssetDescriptorRecord,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct BundlePlan {
    pub(crate) bundle_manifest_template: Option<AssetBundleManifestV2>,
    pub(crate) descriptor_assets: Vec<PlannedBundleDescriptor>,
    pub(crate) dependency_resolution: Vec<AssetDependencyResolution>,
    pub(crate) departments: Vec<String>,
    pub(crate) detected_agent_count: u64,
    pub(crate) detected_team_count: u64,
    pub(crate) filtered_file_count: u64,
    pub(crate) issues: Vec<ImportIssue>,
    pub(crate) skills: Vec<PlannedSkill>,
    pub(crate) mcps: Vec<PlannedMcp>,
    pub(crate) agents: Vec<PlannedAgent>,
    pub(crate) teams: Vec<PlannedTeam>,
    pub(crate) avatars: Vec<ParsedAssetAvatar>,
    pub(crate) asset_state: BundleAssetStateDocument,
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingAgentImportSource {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingTeamImportSource {
    pub(crate) team_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingSkillImportSource {
    pub(crate) skill_slug: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingManagedSkill {
    pub(crate) slug: String,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportContext {
    pub(crate) root_dir_name: String,
    pub(crate) agents: Vec<AgentRecord>,
    pub(crate) teams: Vec<TeamRecord>,
    pub(crate) descriptors: Vec<BundleAssetDescriptorRecord>,
    pub(crate) skill_paths: HashMap<String, PathBuf>,
    pub(crate) builtin_skill_assets: HashMap<String, BuiltinSkillAsset>,
    pub(crate) mcp_configs: HashMap<String, JsonValue>,
    pub(crate) avatar_payloads: HashMap<String, (String, String, Vec<u8>)>,
    pub(crate) asset_state: BundleAssetStateDocument,
    pub(crate) bundle_manifest: AssetBundleManifestV2,
    pub(crate) translation_report: AssetTranslationReport,
    pub(crate) issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedBundle {
    pub(crate) bundle_manifest: Option<AssetBundleManifestV2>,
    pub(crate) descriptor_assets: Vec<ParsedBundleDescriptor>,
    pub(crate) agents: Vec<ParsedAgent>,
    pub(crate) teams: Vec<ParsedTeam>,
    pub(crate) skills: Vec<ParsedSkillSource>,
    pub(crate) mcps: Vec<ParsedMcpSource>,
    pub(crate) avatars: Vec<ParsedAssetAvatar>,
    pub(crate) asset_state: BundleAssetStateDocument,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedBundleDescriptor {
    pub(crate) asset_kind: String,
    pub(crate) source_id: String,
    pub(crate) display_name: String,
    pub(crate) source_path: String,
    pub(crate) manifest_revision: String,
    pub(crate) task_domains: Vec<String>,
    pub(crate) translation_mode: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BundleAssetStateDocument {
    #[serde(default)]
    pub(crate) skills: BTreeMap<String, WorkspaceCapabilityAssetMetadata>,
    #[serde(default)]
    pub(crate) mcps: BTreeMap<String, WorkspaceCapabilityAssetMetadata>,
}

impl BundleAssetStateDocument {
    pub(crate) fn is_empty(&self) -> bool {
        self.skills.is_empty() && self.mcps.is_empty()
    }
}

pub(crate) fn parse_bundle_files(
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedBundle, AppError> {
    let asset_state = parse_bundle_asset_state(files, issues);
    let bundle_manifest = parse_bundle_manifest(files, issues);
    let content_files = files
        .iter()
        .filter(|file| !file.relative_path.starts_with(".octopus/"))
        .cloned()
        .collect::<Vec<_>>();
    let descriptor_assets =
        parse_bundle_descriptors(bundle_manifest.as_ref(), &content_files, issues);
    let grouped = group_top_level(&content_files);
    let builtin_tool_keys = builtin_tool_keys();

    let mut agents = Vec::new();
    let mut teams = Vec::new();
    let mut skills = Vec::new();
    let mut mcps = Vec::new();
    let mut avatars = Vec::new();

    for (root_name, root_files) in grouped {
        let root_md = format!("{root_name}/{root_name}.md");
        let Some(team_or_agent_file) = root_files.iter().find(|file| file.relative_path == root_md)
        else {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_BUNDLE,
                Some(root_name.clone()),
                format!("skipped '{root_name}': missing required '{root_md}'"),
            ));
            continue;
        };

        let member_dirs = immediate_child_dirs(&root_files, &root_name)
            .into_iter()
            .filter(|item| !RESERVED_DIRS.iter().any(|reserved| reserved == item))
            .filter(|child| {
                let expected = format!("{root_name}/{child}/{child}.md");
                root_files.iter().any(|file| file.relative_path == expected)
            })
            .collect::<Vec<_>>();

        if member_dirs.is_empty() {
            let parsed_agent = parse_agent_dir(
                &root_name,
                None,
                team_or_agent_file,
                &root_files,
                target,
                &builtin_tool_keys,
                issues,
            )?;
            skills.extend(parsed_agent.1);
            mcps.extend(parsed_agent.2);
            avatars.push(parsed_agent.0.avatar.clone());
            agents.push(parsed_agent.0);
            continue;
        }

        let team_source = String::from_utf8_lossy(&team_or_agent_file.bytes).to_string();
        let (frontmatter, body) = parse_frontmatter(&team_source)?;
        let team_name = yaml_string(&frontmatter, "name").unwrap_or_else(|| root_name.clone());
        let team_description = yaml_string(&frontmatter, "description")
            .or_else(|| first_non_empty_paragraph(&body))
            .unwrap_or_else(|| team_name.clone());
        let team_prompt = body.trim().to_string();
        let team_personality =
            first_non_empty_paragraph(&body).unwrap_or_else(|| team_name.clone());
        let team_tags = split_tags(yaml_string(&frontmatter, "tag"));
        let team_builtin_tools =
            resolve_builtin_tool_keys(yaml_string_list(&frontmatter, "tools"), &builtin_tool_keys);
        let team_avatar = resolve_avatar(
            "team",
            &root_name,
            &team_name,
            &root_name,
            yaml_string(&frontmatter, "avatar"),
            &root_files,
            target,
            issues,
        )?;
        let team_skills =
            parse_skill_sources(&root_name, &root_name, &team_name, &root_files, issues)?;
        let team_mcps = parse_mcp_sources(
            &root_name,
            &root_name,
            &team_name,
            yaml_string_list(&frontmatter, "mcps"),
            &root_files,
            issues,
        )?;
        let mut team_agent_source_ids = Vec::new();
        let mut parsed_member_names = Vec::new();
        let mut member_skill_sources = Vec::new();
        let mut member_mcp_sources = Vec::new();
        for member_dir in member_dirs.iter().cloned() {
            let member_md = format!("{root_name}/{member_dir}/{member_dir}.md");
            let Some(member_file) = root_files
                .iter()
                .find(|file| file.relative_path == member_md)
            else {
                continue;
            };
            let (agent, mut agent_skills, mut agent_mcps) = parse_agent_dir(
                &format!("{root_name}/{member_dir}"),
                Some(team_name.clone()),
                member_file,
                &root_files,
                target,
                &builtin_tool_keys,
                issues,
            )?;
            team_agent_source_ids.push(agent.source_id.clone());
            parsed_member_names.push(agent.name.clone());
            avatars.push(agent.avatar.clone());
            agents.push(agent);
            member_skill_sources.append(&mut agent_skills);
            member_mcp_sources.append(&mut agent_mcps);
        }
        skills.extend(team_skills.clone());
        skills.extend(member_skill_sources);
        mcps.extend(team_mcps.clone());
        mcps.extend(member_mcp_sources);
        avatars.push(team_avatar.clone());

        teams.push(ParsedTeam {
            source_id: root_name.clone(),
            name: team_name.clone(),
            description: team_description,
            personality: team_personality,
            prompt: team_prompt,
            tags: team_tags,
            builtin_tool_keys: team_builtin_tools,
            skill_source_ids: team_skills.into_iter().map(|item| item.source_id).collect(),
            mcp_source_ids: team_mcps.into_iter().map(|item| item.source_id).collect(),
            leader_name: yaml_string(&frontmatter, "leader"),
            member_names: {
                let members = split_csv(yaml_string(&frontmatter, "member"));
                if members.is_empty() {
                    parsed_member_names
                } else {
                    members
                }
            },
            agent_source_ids: team_agent_source_ids,
            avatar: team_avatar,
        });
    }

    if agents.is_empty() && teams.is_empty() {
        issues.push(issue(
            ISSUE_ERROR,
            SOURCE_SCOPE_BUNDLE,
            None,
            String::from("no compatible agent assets were found in the selected bundle"),
        ));
    }

    Ok(ParsedBundle {
        bundle_manifest,
        descriptor_assets,
        agents,
        teams,
        skills,
        mcps,
        avatars,
        asset_state,
    })
}

fn parse_bundle_asset_state(
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> BundleAssetStateDocument {
    let Some(file) = files.iter().find(|file| {
        file.relative_path == BUNDLE_ASSET_STATE_PATH
            || file
                .relative_path
                .ends_with(&format!("/{BUNDLE_ASSET_STATE_PATH}"))
    }) else {
        return BundleAssetStateDocument::default();
    };

    match serde_json::from_slice::<BundleAssetStateDocument>(&file.bytes) {
        Ok(document) => document,
        Err(error) => {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_BUNDLE,
                None,
                format!("ignored invalid '{BUNDLE_ASSET_STATE_PATH}': {error}"),
            ));
            BundleAssetStateDocument::default()
        }
    }
}

fn parse_bundle_manifest(
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Option<AssetBundleManifestV2> {
    let file = files.iter().find(|file| {
        file.relative_path == ".octopus/manifest.json"
            || file.relative_path.ends_with("/.octopus/manifest.json")
    })?;

    match serde_json::from_slice::<AssetBundleManifestV2>(&file.bytes) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_BUNDLE,
                Some(file.relative_path.clone()),
                format!("ignored invalid '.octopus/manifest.json': {error}"),
            ));
            None
        }
    }
}

fn parse_bundle_descriptors(
    bundle_manifest: Option<&AssetBundleManifestV2>,
    content_files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Vec<ParsedBundleDescriptor> {
    let Some(bundle_manifest) = bundle_manifest else {
        return Vec::new();
    };

    let mut descriptors = Vec::new();
    for entry in &bundle_manifest.assets {
        if !matches!(entry.asset_kind.as_str(), "plugin" | "workflow-template") {
            continue;
        }
        let Some(file) = content_files
            .iter()
            .find(|file| file.relative_path == entry.source_path)
        else {
            issues.push(issue(
                ISSUE_ERROR,
                SOURCE_SCOPE_BUNDLE,
                Some(entry.source_id.clone()),
                format!(
                    "bundle manifest entry '{}' is missing source file '{}'",
                    entry.display_name, entry.source_path
                ),
            ));
            continue;
        };
        descriptors.push(ParsedBundleDescriptor {
            asset_kind: entry.asset_kind.clone(),
            source_id: entry.source_id.clone(),
            display_name: entry.display_name.clone(),
            source_path: entry.source_path.clone(),
            manifest_revision: entry.manifest_revision.clone(),
            task_domains: entry.task_domains.clone(),
            translation_mode: entry.translation_mode.clone(),
            bytes: file.bytes.clone(),
        });
    }

    descriptors
}

pub(crate) fn load_builtin_catalog_sources() -> Result<BuiltinCatalogSources, AppError> {
    let parsed = parse_builtin_bundle()?;
    let mcp_name_by_source = parsed
        .mcps
        .iter()
        .map(|mcp| (mcp.source_id.clone(), mcp.server_name.clone()))
        .collect::<HashMap<_, _>>();

    Ok(BuiltinCatalogSources {
        skill_sources: parsed
            .skills
            .into_iter()
            .map(|source| {
                let description = source
                    .files
                    .iter()
                    .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
                    .and_then(|(_, bytes)| String::from_utf8(bytes.clone()).ok())
                    .and_then(|content| parse_frontmatter(&content).ok())
                    .and_then(|(frontmatter, _)| yaml_string(&frontmatter, "description"))
                    .unwrap_or_default();
                BuiltinSkillCatalogSource {
                    source_id: source.source_id,
                    name: source.name,
                    canonical_slug: source.canonical_slug,
                    content_hash: source.content_hash,
                    description,
                    files: source.files,
                }
            })
            .collect(),
        agent_sources: {
            let mut records = parsed
                .agents
                .into_iter()
                .filter(|agent| agent.team_name.is_none())
                .map(|agent| BuiltinAgentTemplateSource {
                    source_id: agent.source_id,
                    name: agent.name.clone(),
                    avatar_data_url: avatar_data_url(&agent.avatar),
                    personality: agent.personality.clone(),
                    tags: agent.tags.clone(),
                    prompt: agent.prompt.clone(),
                    builtin_tool_keys: agent.builtin_tool_keys.clone(),
                    skill_source_ids: agent.skill_source_ids,
                    mcp_server_names: agent
                        .mcp_source_ids
                        .iter()
                        .filter_map(|source_id| mcp_name_by_source.get(source_id))
                        .cloned()
                        .collect(),
                    description: agent.description,
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then(left.source_id.cmp(&right.source_id))
            });
            records
        },
        team_sources: {
            let mut records = parsed
                .teams
                .into_iter()
                .map(|team| BuiltinTeamTemplateSource {
                    source_id: team.source_id,
                    name: team.name.clone(),
                    avatar_data_url: avatar_data_url(&team.avatar),
                    personality: team.personality.clone(),
                    tags: team.tags.clone(),
                    prompt: team.prompt.clone(),
                    builtin_tool_keys: team.builtin_tool_keys.clone(),
                    skill_source_ids: team.skill_source_ids,
                    mcp_server_names: team
                        .mcp_source_ids
                        .iter()
                        .filter_map(|source_id| mcp_name_by_source.get(source_id))
                        .cloned()
                        .collect(),
                    description: team.description,
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then(left.source_id.cmp(&right.source_id))
            });
            records
        },
    })
}

pub(crate) fn find_builtin_skill_asset_by_id(
    skill_id: &str,
) -> Result<Option<BuiltinSkillAsset>, AppError> {
    crate::agent_bundle::find_builtin_skill_asset_by_id(skill_id)
}

fn parse_builtin_bundle() -> Result<ParsedBundle, AppError> {
    let bundle_files =
        crate::agent_bundle::builtin::embedded_bundle_files(&BUILTIN_BUNDLE_ASSET_DIR)?;
    let mut issues = Vec::new();
    let parsed = parse_bundle_files(&bundle_files, &AssetTargetScope::Workspace, &mut issues)?;
    if let Some(issue) = issues.iter().find(|issue| issue.severity == ISSUE_ERROR) {
        return Err(AppError::invalid_input(format!(
            "invalid builtin asset bundle: {}",
            issue.message
        )));
    }
    Ok(parsed)
}

fn avatar_data_url(avatar: &ParsedAssetAvatar) -> Option<String> {
    Some(format!(
        "data:{};base64,{}",
        avatar.content_type,
        BASE64_STANDARD.encode(&avatar.bytes)
    ))
}

fn parse_agent_dir(
    source_id: &str,
    team_name: Option<String>,
    agent_file: &BundleFile,
    root_files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<(ParsedAgent, Vec<ParsedSkillSource>, Vec<ParsedMcpSource>), AppError> {
    let markdown = String::from_utf8_lossy(&agent_file.bytes).to_string();
    let (frontmatter, body) = parse_frontmatter(&markdown)?;
    let dir_name = source_id.rsplit('/').next().unwrap_or(source_id);
    let name = yaml_string(&frontmatter, "name").unwrap_or_else(|| dir_name.to_string());
    let description = yaml_string(&frontmatter, "description")
        .or_else(|| first_non_empty_paragraph(&body))
        .unwrap_or_else(|| name.clone());
    let personality = yaml_string(&frontmatter, "character")
        .or_else(|| first_paragraph_after_heading(&body, "角色定义"))
        .or_else(|| first_paragraph_after_heading(&body, "Role Definition"))
        .unwrap_or_else(|| name.clone());
    let prompt = body.trim().to_string();
    let tags = split_tags(yaml_string(&frontmatter, "tag"));
    let builtin_tool_keys =
        resolve_builtin_tool_keys(yaml_string_list(&frontmatter, "tools"), builtin_tool_keys);
    let skills = parse_skill_sources(source_id, source_id, &name, root_files, issues)?;
    let mcps = parse_mcp_sources(
        source_id,
        source_id,
        &name,
        yaml_string_list(&frontmatter, "mcps"),
        root_files,
        issues,
    )?;
    let avatar = resolve_avatar(
        "agent",
        source_id,
        &name,
        source_id,
        yaml_string(&frontmatter, "avatar"),
        root_files,
        target,
        issues,
    )?;
    Ok((
        ParsedAgent {
            source_id: source_id.to_string(),
            team_name,
            name,
            description,
            personality,
            prompt,
            tags,
            builtin_tool_keys,
            skill_source_ids: skills.iter().map(|item| item.source_id.clone()).collect(),
            mcp_source_ids: mcps.iter().map(|item| item.source_id.clone()).collect(),
            avatar,
        },
        skills,
        mcps,
    ))
}

fn parse_skill_sources(
    owner_dir: &str,
    owner_source_id: &str,
    owner_name: &str,
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Result<Vec<ParsedSkillSource>, AppError> {
    let prefix = format!("{owner_dir}/skills/");
    let mut grouped = BTreeMap::<String, Vec<(String, Vec<u8>)>>::new();
    for file in files {
        if !file.relative_path.starts_with(&prefix) {
            continue;
        }
        let suffix = &file.relative_path[prefix.len()..];
        let segments = suffix.split('/').collect::<Vec<_>>();
        if segments.len() < 2 {
            continue;
        }
        grouped
            .entry(segments[0].to_string())
            .or_default()
            .push((segments[1..].join("/"), file.bytes.clone()));
    }

    let mut parsed = Vec::new();
    for (skill_dir, mut skill_files) in grouped {
        skill_files.sort_by(|left, right| left.0.cmp(&right.0));
        let Some(skill_frontmatter_file) = skill_files
            .iter()
            .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
        else {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_SKILL,
                Some(format!("{owner_source_id}/skills/{skill_dir}")),
                format!("skipped skill '{skill_dir}': missing required '{SKILL_FRONTMATTER_FILE}'"),
            ));
            continue;
        };
        let markdown = String::from_utf8_lossy(&skill_frontmatter_file.1).to_string();
        let (frontmatter, _) = parse_frontmatter(&markdown)?;
        let skill_name = yaml_string(&frontmatter, "name").unwrap_or_else(|| skill_dir.clone());
        let canonical_slug = validate_skill_slug(&slugify_skill_name(&skill_name, "skill"))?;
        let source_id = format!("{owner_source_id}/skills/{skill_dir}");
        parsed.push(ParsedSkillSource {
            source_id,
            owner_name: owner_name.to_string(),
            name: skill_name,
            canonical_slug,
            content_hash: hash_bundle_files(&skill_files),
            files: skill_files,
        });
    }
    Ok(parsed)
}

fn parse_mcp_sources(
    owner_dir: &str,
    owner_source_id: &str,
    owner_name: &str,
    frontmatter_mcps: Vec<String>,
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Result<Vec<ParsedMcpSource>, AppError> {
    let prefix = format!("{owner_dir}/mcps/");
    let mut parsed = Vec::new();
    let mut seen_names = BTreeSet::new();
    for file in files {
        if !file.relative_path.starts_with(&prefix) {
            continue;
        }
        let suffix = &file.relative_path[prefix.len()..];
        if suffix.contains('/') || !suffix.ends_with(".json") {
            continue;
        }
        let server_name = suffix.trim_end_matches(".json").to_string();
        let config = match serde_json::from_slice::<JsonValue>(&file.bytes) {
            Ok(config) if config.is_object() => config,
            Ok(_) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_MCP,
                    Some(file.relative_path.clone()),
                    String::from("skipped MCP file because it is not a JSON object"),
                ));
                continue;
            }
            Err(error) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_MCP,
                    Some(file.relative_path.clone()),
                    format!("skipped MCP file because JSON is invalid: {error}"),
                ));
                continue;
            }
        };
        let source_id = format!("{owner_source_id}/mcps/{server_name}");
        seen_names.insert(server_name.clone());
        parsed.push(ParsedMcpSource {
            source_id,
            owner_name: owner_name.to_string(),
            server_name,
            content_hash: Some(hash_json_value(&config)?),
            config: Some(config),
            referenced_only: false,
        });
    }

    for server_name in frontmatter_mcps {
        if seen_names.contains(&server_name) {
            continue;
        }
        parsed.push(ParsedMcpSource {
            source_id: format!("{owner_source_id}/mcps/{server_name}"),
            owner_name: owner_name.to_string(),
            server_name,
            content_hash: None,
            config: None,
            referenced_only: true,
        });
    }

    Ok(parsed)
}

fn resolve_avatar(
    owner_kind: &str,
    source_id: &str,
    owner_name: &str,
    owner_dir: &str,
    avatar_field: Option<String>,
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedAssetAvatar, AppError> {
    let mut candidates = Vec::new();
    if let Some(avatar_name) = avatar_field
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let path = format!("{owner_dir}/{}", avatar_name.trim());
        if let Some(file) = files.iter().find(|file| file.relative_path == path) {
            candidates.push((avatar_name.trim().to_string(), file.bytes.clone()));
        }
    }

    if candidates.is_empty() {
        let prefix = format!("{owner_dir}/");
        for file in files {
            if !file.relative_path.starts_with(&prefix) {
                continue;
            }
            let suffix = &file.relative_path[prefix.len()..];
            if suffix.contains('/') || !is_supported_avatar_file(suffix) {
                continue;
            }
            candidates.push((suffix.to_string(), file.bytes.clone()));
        }
    }

    for (file_name, bytes) in candidates {
        if let Some(content_type) = content_type_for_avatar(&file_name) {
            return Ok(ParsedAssetAvatar {
                source_id: source_id.to_string(),
                owner_kind: owner_kind.to_string(),
                owner_name: owner_name.to_string(),
                file_name,
                content_type: content_type.to_string(),
                bytes,
                generated: false,
            });
        }
    }

    issues.push(issue(
        ISSUE_WARNING,
        SOURCE_SCOPE_AVATAR,
        Some(source_id.to_string()),
        format!(
            "avatar for {} '{}' is missing or unsupported, generated a deterministic default avatar",
            owner_kind, owner_name
        ),
    ));
    let default_avatar = default_avatar_payload(owner_kind, &target.avatar_seed_key(source_id))?;
    Ok(ParsedAssetAvatar {
        source_id: source_id.to_string(),
        owner_kind: owner_kind.to_string(),
        owner_name: owner_name.to_string(),
        file_name: default_avatar.0,
        content_type: default_avatar.1,
        bytes: default_avatar.2,
        generated: true,
    })
}

fn default_avatar_payload(
    owner_kind: &str,
    seed_key: &str,
) -> Result<(String, String, Vec<u8>), AppError> {
    let dir = if owner_kind == "team" {
        &DEFAULT_LEADER_AVATARS
    } else {
        &DEFAULT_EMPLOYEE_AVATARS
    };
    let mut entries = dir
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            DirEntry::File(file) => Some((
                file.path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_string(),
                file.contents().to_vec(),
            )),
            DirEntry::Dir(_) => None,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let index = deterministic_index(seed_key, entries.len());
    let (file_name, bytes) = entries
        .get(index)
        .cloned()
        .ok_or_else(|| AppError::invalid_input("default avatar assets are not available"))?;
    let content_type = content_type_for_avatar(&file_name)
        .ok_or_else(|| AppError::invalid_input("default avatar has unsupported type"))?;
    Ok((file_name, content_type.to_string(), bytes))
}

fn deterministic_index(seed_key: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let digest = Sha256::digest(seed_key.as_bytes());
    let mut value = 0_u64;
    for byte in digest.iter().take(8) {
        value = (value << 8) + u64::from(*byte);
    }
    (value as usize) % len
}

pub(crate) fn normalize_bundle_files(
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<(Vec<BundleFile>, u64, Vec<ImportIssue>), AppError> {
    if files.is_empty() {
        return Err(AppError::invalid_input("agent bundle files are required"));
    }

    let mut normalized = Vec::new();
    let mut filtered = 0_u64;
    let mut issues = Vec::new();
    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        if path_contains_filtered_directory(&relative_path) {
            filtered += 1;
            continue;
        }
        let bytes = match BASE64_STANDARD.decode(&file.data_base64) {
            Ok(bytes) => bytes,
            Err(error) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_BUNDLE,
                    Some(relative_path),
                    format!("skipped file with invalid base64 payload: {error}"),
                ));
                continue;
            }
        };
        normalized.push(BundleFile {
            relative_path,
            bytes,
        });
    }
    Ok((normalized, filtered, issues))
}

pub(crate) fn strip_optional_bundle_root(files: Vec<BundleFile>) -> Vec<BundleFile> {
    let mut first_segments = files
        .iter()
        .filter_map(|file| file.relative_path.split('/').next().map(ToOwned::to_owned))
        .collect::<BTreeSet<_>>();
    if first_segments.len() != 1 {
        return files;
    }
    let root = first_segments.pop_first().unwrap_or_default();
    let root_md = format!("{root}/{root}.md");
    if files.iter().any(|file| file.relative_path == root_md) {
        return files;
    }

    files
        .into_iter()
        .filter_map(|file| {
            file.relative_path
                .strip_prefix(&format!("{root}/"))
                .map(|relative_path| BundleFile {
                    relative_path: relative_path.to_string(),
                    bytes: file.bytes,
                })
        })
        .collect()
}

fn group_top_level(files: &[BundleFile]) -> BTreeMap<String, Vec<BundleFile>> {
    let mut grouped = BTreeMap::<String, Vec<BundleFile>>::new();
    for file in files {
        if let Some(segment) = file.relative_path.split('/').next() {
            grouped
                .entry(segment.to_string())
                .or_default()
                .push(file.clone());
        }
    }
    grouped
}

fn immediate_child_dirs(files: &[BundleFile], root: &str) -> BTreeSet<String> {
    let prefix = format!("{root}/");
    files
        .iter()
        .filter_map(|file| file.relative_path.strip_prefix(&prefix))
        .filter_map(|suffix| suffix.split('/').next())
        .map(ToOwned::to_owned)
        .collect()
}

fn path_contains_filtered_directory(relative_path: &str) -> bool {
    relative_path.split('/').any(|segment| {
        FILTERED_DIR_NAMES
            .iter()
            .any(|candidate| candidate == &segment)
    })
}

fn parse_frontmatter(
    contents: &str,
) -> Result<(BTreeMap<String, serde_yaml::Value>, String), AppError> {
    let normalized = contents.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let Some(first) = lines.first() else {
        return Ok((BTreeMap::new(), String::new()));
    };
    if !is_frontmatter_delimiter(first) {
        return Ok((BTreeMap::new(), normalized));
    }

    let mut frontmatter_lines = Vec::new();
    let mut body_index = 1_usize;
    while body_index < lines.len() {
        let line = lines[body_index];
        if is_frontmatter_delimiter(line) {
            body_index += 1;
            break;
        }
        frontmatter_lines.push(line);
        body_index += 1;
    }

    let frontmatter = if frontmatter_lines.is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(&frontmatter_lines.join("\n"))
            .map_err(|error| AppError::invalid_input(format!("invalid frontmatter yaml: {error}")))?
    };
    Ok((frontmatter, lines[body_index..].join("\n")))
}

fn is_frontmatter_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.len() >= 3 && trimmed.chars().all(|value| value == '-')
}

fn yaml_string(frontmatter: &BTreeMap<String, serde_yaml::Value>, key: &str) -> Option<String> {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            serde_yaml::Value::String(value) => Some(value.trim().to_string()),
            serde_yaml::Value::Number(value) => Some(value.to_string()),
            serde_yaml::Value::Bool(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
}

fn yaml_string_list(frontmatter: &BTreeMap<String, serde_yaml::Value>, key: &str) -> Vec<String> {
    match frontmatter.get(key) {
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                serde_yaml::Value::String(value) => Some(value.trim().to_string()),
                serde_yaml::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .filter(|value| !value.is_empty())
            .collect(),
        Some(serde_yaml::Value::String(value)) => split_csv(Some(value.clone())),
        _ => Vec::new(),
    }
}

fn split_csv(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(['、', '，', ',', ';'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn split_tags(value: Option<String>) -> Vec<String> {
    split_csv(value)
}

fn first_non_empty_paragraph(body: &str) -> Option<String> {
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed.to_string());
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn first_paragraph_after_heading(body: &str, heading: &str) -> Option<String> {
    let mut heading_found = false;
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !heading_found {
            let candidate = trimmed.trim_start_matches('#').trim();
            if trimmed.starts_with('#') && candidate == heading {
                heading_found = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            break;
        }
        paragraph.push(trimmed.to_string());
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn resolve_builtin_tool_keys(values: Vec<String>, builtin_tool_keys: &[String]) -> Vec<String> {
    if values.iter().any(|value| value.eq_ignore_ascii_case("ALL")) {
        return builtin_tool_keys.to_vec();
    }
    let builtin_set = builtin_tool_keys.iter().collect::<BTreeSet<_>>();
    values
        .into_iter()
        .filter(|value| builtin_set.contains(&value))
        .collect()
}

fn builtin_tool_keys() -> Vec<String> {
    tools::mvp_tool_specs()
        .iter()
        .map(|spec| spec.name.to_string())
        .collect()
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256-{:x}", hasher.finalize())
}

fn hash_bundle_files(files: &[(String, Vec<u8>)]) -> String {
    let mut hasher = Sha256::new();
    for (relative_path, bytes) in files {
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    format!("sha256-{:x}", hasher.finalize())
}

fn hash_json_value(value: &JsonValue) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(value)?))
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256-{:x}", hasher.finalize())
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

fn slugify_skill_name(value: &str, fallback_prefix: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
            continue;
        }
        if matches!(character, '-' | '_' | '.' | ' ') && !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        format!("{fallback_prefix}-{}", short_hash(&hash_text(value)))
    } else {
        slug
    }
}

#[cfg(test)]
fn managed_skill_id(target: &AssetTargetScope<'_>, slug: &str) -> String {
    let display_path = match target {
        AssetTargetScope::Workspace => format!("data/skills/{slug}/{SKILL_FRONTMATTER_FILE}"),
        AssetTargetScope::Project(project_id) => {
            format!("data/projects/{project_id}/skills/{slug}/{SKILL_FRONTMATTER_FILE}")
        }
    };
    crate::catalog_hash_id("skill", &display_path)
}

pub(crate) fn apply_imported_asset_state(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
    plan: &BundlePlan,
) -> Result<(), AppError> {
    if plan.asset_state.is_empty() {
        return Ok(());
    }

    let mut document = load_workspace_asset_state_document(paths)?;
    let mut changed = false;

    for skill in &plan.skills {
        if skill.action == ImportAction::Failed {
            continue;
        }
        let Some(metadata) = plan.asset_state.skills.get(&skill.slug) else {
            continue;
        };
        let source_key = skill_source_key(
            &target
                .skill_root(paths)
                .join(&skill.slug)
                .join(SKILL_FRONTMATTER_FILE),
            &paths.root,
        );
        apply_asset_metadata(&mut document, &source_key, metadata);
        changed = true;
    }

    for mcp in &plan.mcps {
        if mcp.action == ImportAction::Failed || (!mcp.resolved && mcp.action != ImportAction::Skip)
        {
            continue;
        }
        let Some(metadata) = plan.asset_state.mcps.get(&mcp.server_name) else {
            continue;
        };
        let source_key = bundle_mcp_source_key(target, &mcp.server_name);
        apply_asset_metadata(&mut document, &source_key, metadata);
        changed = true;
    }

    if changed {
        save_workspace_asset_state_document(paths, &document)?;
    }

    Ok(())
}

fn apply_asset_metadata(
    document: &mut crate::resources_skills::WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    metadata: &WorkspaceCapabilityAssetMetadata,
) {
    if let Some(enabled) = metadata.enabled {
        set_workspace_asset_enabled(document, source_key, enabled);
    }
    if let Some(trusted) = metadata.trusted {
        set_workspace_asset_trusted(document, source_key, trusted);
    }
}

pub(crate) fn asset_metadata_has_values(metadata: &WorkspaceCapabilityAssetMetadata) -> bool {
    metadata.enabled.is_some() || metadata.trusted.is_some()
}

pub(crate) fn bundle_mcp_source_key(target: &AssetTargetScope<'_>, server_name: &str) -> String {
    match target {
        AssetTargetScope::Project(project_id) => format!("mcp:project:{project_id}:{server_name}"),
        AssetTargetScope::Workspace => format!("mcp:{server_name}"),
    }
}

pub(crate) fn deterministic_asset_id(
    prefix: &str,
    target: &AssetTargetScope<'_>,
    source_id: &str,
) -> String {
    format!(
        "{prefix}-{}",
        short_hash(&hash_text(&format!("{}:{source_id}", target.scope_label())))
    )
}

pub(crate) fn deterministic_descriptor_id(
    target: &AssetTargetScope<'_>,
    asset_kind: &str,
    source_id: &str,
) -> String {
    let prefix = match asset_kind {
        "plugin" => "plugin-asset",
        "workflow-template" => "workflow-template-asset",
        other => other,
    };
    deterministic_asset_id(prefix, target, source_id)
}

pub(crate) fn descriptor_storage_path(
    target: &AssetTargetScope<'_>,
    descriptor: &ParsedBundleDescriptor,
) -> String {
    let scope_segment = match target {
        AssetTargetScope::Workspace => String::from("workspace"),
        AssetTargetScope::Project(project_id) => format!("project/{project_id}"),
    };
    format!(
        "data/artifacts/bundle-assets/{scope_segment}/{}/{}/{}",
        descriptor.asset_kind,
        short_hash(&hash_text(&descriptor.source_id)),
        descriptor.source_path
    )
}

pub(crate) fn basename_from_source_id(source_id: &str) -> &str {
    source_id.rsplit('/').next().unwrap_or(source_id)
}

pub(crate) fn dependency_resolution_from_manifest(
    manifest: Option<&AssetBundleManifestV2>,
    issues: &[ImportIssue],
) -> Vec<AssetDependencyResolution> {
    let Some(manifest) = manifest else {
        return Vec::new();
    };
    let unresolved_dependency_refs = issues
        .iter()
        .filter_map(|issue| issue.dependency_ref.clone())
        .collect::<BTreeSet<_>>();
    manifest
        .dependencies
        .iter()
        .map(|dependency| AssetDependencyResolution {
            kind: dependency.kind.clone(),
            r#ref: dependency.r#ref.clone(),
            required: dependency.required,
            resolution_state: if unresolved_dependency_refs.contains(&dependency.r#ref) {
                "missing".into()
            } else {
                "resolved".into()
            },
            resolved_ref: if unresolved_dependency_refs.contains(&dependency.r#ref) {
                None
            } else {
                Some(dependency.r#ref.clone())
            },
        })
        .collect()
}

pub(crate) fn dependencies_from_resolution(
    dependency_resolution: &[AssetDependencyResolution],
) -> Vec<AssetDependency> {
    dependency_resolution
        .iter()
        .map(|dependency| AssetDependency {
            kind: dependency.kind.clone(),
            r#ref: dependency.r#ref.clone(),
            version_range: "*".into(),
            required: dependency.required,
        })
        .collect()
}

pub(crate) fn descriptor_record_matches(
    existing: &BundleAssetDescriptorRecord,
    candidate: &BundleAssetDescriptorRecord,
) -> bool {
    existing.workspace_id == candidate.workspace_id
        && existing.project_id == candidate.project_id
        && existing.scope == candidate.scope
        && existing.asset_kind == candidate.asset_kind
        && existing.source_id == candidate.source_id
        && existing.display_name == candidate.display_name
        && existing.source_path == candidate.source_path
        && existing.storage_path == candidate.storage_path
        && existing.content_hash == candidate.content_hash
        && existing.byte_size == candidate.byte_size
        && existing.manifest_revision == candidate.manifest_revision
        && existing.task_domains == candidate.task_domains
        && existing.translation_mode == candidate.translation_mode
        && existing.trust_metadata == candidate.trust_metadata
        && existing.dependency_resolution == candidate.dependency_resolution
        && existing.import_metadata.origin_kind == candidate.import_metadata.origin_kind
        && existing.import_metadata.source_id == candidate.import_metadata.source_id
        && existing.import_metadata.manifest_version == candidate.import_metadata.manifest_version
        && existing.import_metadata.translation_status
            == candidate.import_metadata.translation_status
}

pub(crate) fn issue(
    severity: &str,
    scope: &str,
    source_id: Option<String>,
    message: String,
) -> ImportIssue {
    ImportIssue {
        severity: severity.into(),
        scope: scope.into(),
        code: format!("{scope}-diagnostic"),
        stage: "translate".into(),
        source_id,
        source_path: None,
        dependency_ref: None,
        asset_kind: None,
        message,
        suggestion: None,
        details: None,
    }
}

pub(crate) fn build_agent_record(
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    agent_id: &str,
    name: &str,
    avatar_path: Option<String>,
    description: &str,
    personality: &str,
    prompt: &str,
    tags: &[String],
    builtin_tool_keys: &[String],
    skill_ids: &[String],
    mcp_server_names: &[String],
) -> AgentRecord {
    let builtin_tool_keys = builtin_tool_keys.to_vec();
    let skill_ids = skill_ids.to_vec();
    let mcp_server_names = mcp_server_names.to_vec();
    let task_domains = normalize_task_domains(tags.to_vec());
    AgentRecord {
        id: agent_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: target.project_id().map(ToOwned::to_owned),
        scope: target.scope_label().into(),
        name: name.trim().to_string(),
        avatar_path: avatar_path.clone(),
        avatar: agent_avatar(paths, avatar_path.as_deref()),
        personality: personality.trim().to_string(),
        tags: tags.to_vec(),
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: mcp_server_names.clone(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(
            &builtin_tool_keys,
            &skill_ids,
            &mcp_server_names,
        ),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_agent_memory_policy(),
        delegation_policy: default_agent_delegation_policy(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_agent_shared_capability_policy(),
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

pub(crate) fn build_team_record(
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    team_id: &str,
    name: &str,
    avatar_path: Option<String>,
    description: &str,
    personality: &str,
    prompt: &str,
    tags: &[String],
    builtin_tool_keys: &[String],
    skill_ids: &[String],
    mcp_server_names: &[String],
    leader_agent_id: Option<String>,
    member_agent_ids: Vec<String>,
) -> TeamRecord {
    let builtin_tool_keys = builtin_tool_keys.to_vec();
    let skill_ids = skill_ids.to_vec();
    let mcp_server_names = mcp_server_names.to_vec();
    let task_domains = normalize_task_domains(tags.to_vec());
    let delegation_policy = default_team_delegation_policy();
    let leader_ref = leader_agent_id.clone().unwrap_or_default();
    TeamRecord {
        id: team_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: target.project_id().map(ToOwned::to_owned),
        scope: target.scope_label().into(),
        name: name.trim().to_string(),
        avatar_path: avatar_path.clone(),
        avatar: agent_avatar(paths, avatar_path.as_deref()),
        personality: personality.trim().to_string(),
        tags: tags.to_vec(),
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: mcp_server_names.clone(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(
            &builtin_tool_keys,
            &skill_ids,
            &mcp_server_names,
        ),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_team_memory_policy(),
        delegation_policy: delegation_policy.clone(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_team_shared_capability_policy(),
        leader_agent_id,
        member_agent_ids: member_agent_ids.clone(),
        leader_ref: leader_ref.clone(),
        member_refs: member_agent_ids.clone(),
        team_topology: team_topology_from_refs(Some(leader_ref), member_agent_ids.clone()),
        shared_memory_policy: default_shared_memory_policy(),
        mailbox_policy: default_mailbox_policy(),
        artifact_handoff_policy: default_artifact_handoff_policy(),
        workflow_affordance: workflow_affordance_from_task_domains(&task_domains, true, true),
        worker_concurrency_limit: delegation_policy.max_worker_count,
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

fn compute_agent_hash(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    record: &PlannedAgent,
    skill_ids: &[String],
) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": workspace_id,
        "projectId": target.project_id(),
        "scope": target.scope_label(),
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "status": "active",
    }))?))
}

fn compute_existing_agent_hash(record: &AgentRecord) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": record.workspace_id,
        "projectId": record.project_id,
        "scope": record.scope,
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": record.skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "status": record.status,
    }))?))
}

fn compute_team_hash(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    record: &PlannedTeam,
    skill_ids: &[String],
    leader_agent_id: Option<&str>,
    member_agent_ids: &[String],
) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": workspace_id,
        "projectId": target.project_id(),
        "scope": target.scope_label(),
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "leaderAgentId": leader_agent_id,
        "memberAgentIds": member_agent_ids,
        "status": "active",
    }))?))
}

fn compute_existing_team_hash(record: &TeamRecord) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": record.workspace_id,
        "projectId": record.project_id,
        "scope": record.scope,
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": record.skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "leaderAgentId": record.leader_agent_id,
        "memberAgentIds": record.member_agent_ids,
        "status": record.status,
    }))?))
}

pub(crate) fn resolve_agent_action(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    existing_agents: &HashMap<String, AgentRecord>,
    planned: &PlannedAgent,
    skill_ids: &[String],
) -> Result<ImportAction, AppError> {
    let agent_id = planned
        .agent_id
        .clone()
        .unwrap_or_else(|| deterministic_asset_id("agent", target, &planned.source_id));
    let desired_hash = compute_agent_hash(workspace_id, target, planned, skill_ids)?;
    let Some(existing) = existing_agents.get(&agent_id) else {
        return Ok(ImportAction::Create);
    };
    if compute_existing_agent_hash(existing)? == desired_hash {
        Ok(ImportAction::Skip)
    } else {
        Ok(ImportAction::Update)
    }
}

pub(crate) fn resolve_team_action(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    existing_teams: &HashMap<String, TeamRecord>,
    planned: &PlannedTeam,
    skill_ids: &[String],
    leader_agent_id: Option<&str>,
    member_agent_ids: &[String],
) -> Result<ImportAction, AppError> {
    let team_id = planned
        .team_id
        .clone()
        .unwrap_or_else(|| deterministic_asset_id("team", target, &planned.source_id));
    let desired_hash = compute_team_hash(
        workspace_id,
        target,
        planned,
        skill_ids,
        leader_agent_id,
        member_agent_ids,
    )?;
    let Some(existing) = existing_teams.get(&team_id) else {
        return Ok(ImportAction::Create);
    };
    if compute_existing_team_hash(existing)? == desired_hash {
        Ok(ImportAction::Skip)
    } else {
        Ok(ImportAction::Update)
    }
}

pub(crate) fn load_scoped_agents(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, AgentRecord>, AppError> {
    Ok(load_agents(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

pub(crate) fn load_scoped_teams(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, TeamRecord>, AppError> {
    Ok(load_teams(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

pub(crate) fn load_existing_managed_skills(
    skills_root: &Path,
) -> Result<HashMap<String, ExistingManagedSkill>, AppError> {
    let mut skills = HashMap::new();
    if !skills_root.is_dir() {
        return Ok(skills);
    }
    for entry in fs::read_dir(skills_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let skill_file = path.join(SKILL_FRONTMATTER_FILE);
        if !skill_file.is_file() {
            continue;
        }
        let files = read_directory_files(&path)?;
        skills.insert(
            slug.clone(),
            ExistingManagedSkill {
                slug,
                content_hash: hash_bundle_files(&files),
            },
        );
    }
    Ok(skills)
}

fn read_directory_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let mut files = Vec::new();
    read_directory_files_recursive(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn read_directory_files_recursive(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            read_directory_files_recursive(root, &path, files)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| AppError::invalid_input(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        files.push((relative, fs::read(path)?));
    }
    Ok(())
}

pub(crate) fn write_managed_skill(
    skills_root: &Path,
    slug: &str,
    files: &[(String, Vec<u8>)],
) -> Result<(), AppError> {
    let slug = validate_skill_slug(slug)?;
    let target_dir = skills_root.join(&slug);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;
    for (relative_path, bytes) in files {
        let safe_relative_path = validate_skill_file_relative_path(relative_path)?;
        let target_path = target_dir.join(&safe_relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
    }
    Ok(())
}

pub(crate) fn write_agent_record(
    connection: &Connection,
    record: &AgentRecord,
    replace: bool,
) -> Result<(), AppError> {
    crate::infra_state::write_agent_record(connection, record, replace)
}

pub(crate) fn load_target_runtime_document(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
) -> Result<JsonMap<String, JsonValue>, AppError> {
    read_runtime_document(&target.runtime_document_path(paths))
}

pub(crate) fn load_target_mcp_map(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
) -> Result<BTreeMap<String, JsonValue>, AppError> {
    extract_mcp_map(&load_target_runtime_document(paths, target)?)
}

pub(crate) fn load_effective_mcp_map(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
) -> Result<BTreeMap<String, JsonValue>, AppError> {
    let mut merged = extract_mcp_map(&read_runtime_document(
        &paths.runtime_config_dir.join("workspace.json"),
    )?)?;
    if let AssetTargetScope::Project(project_id) = target {
        for (name, config) in extract_mcp_map(&read_runtime_document(
            &paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
        )?)? {
            merged.insert(name, config);
        }
    }
    Ok(merged)
}

fn read_runtime_document(path: &Path) -> Result<JsonMap<String, JsonValue>, AppError> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if raw.trim().is_empty() {
                return Ok(JsonMap::new());
            }
            let parsed = serde_json::from_str::<JsonValue>(&raw)?;
            parsed
                .as_object()
                .cloned()
                .ok_or_else(|| AppError::invalid_input("runtime config must be a JSON object"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(JsonMap::new()),
        Err(error) => Err(error.into()),
    }
}

fn extract_mcp_map(
    document: &JsonMap<String, JsonValue>,
) -> Result<BTreeMap<String, JsonValue>, AppError> {
    Ok(document
        .get("mcpServers")
        .and_then(|value| value.as_object())
        .map(|servers| {
            servers
                .iter()
                .map(|(name, config)| (name.clone(), config.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default())
}

pub(crate) fn plan_mcp_document_updates(
    mut document: JsonMap<String, JsonValue>,
    mcps: &[PlannedMcp],
    issues: &mut Vec<ImportIssue>,
) -> Result<JsonMap<String, JsonValue>, AppError> {
    let servers = document
        .entry("mcpServers")
        .or_insert_with(|| JsonValue::Object(JsonMap::new()))
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input("mcpServers must be a JSON object"))?;

    for mcp in mcps {
        if !mcp.resolved || mcp.referenced_only {
            continue;
        }
        let Some(config) = mcp.config.as_ref() else {
            continue;
        };
        if !config.is_object() {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_MCP,
                mcp.source_ids.first().cloned(),
                format!(
                    "skipped MCP '{}' because config is not an object",
                    mcp.server_name
                ),
            ));
            continue;
        }
        servers.insert(mcp.server_name.clone(), config.clone());
    }

    Ok(document)
}

pub(crate) fn write_target_runtime_document(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
    document: &JsonMap<String, JsonValue>,
) -> Result<(), AppError> {
    let path = target.runtime_document_path(paths);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if matches!(target, AssetTargetScope::Workspace) {
        write_workspace_runtime_document(paths, document)?;
        return Ok(());
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&JsonValue::Object(document.clone()))?,
    )?;
    Ok(())
}

pub(crate) fn persist_avatar(
    paths: &WorkspacePaths,
    entity_id: &str,
    avatar: &ParsedAssetAvatar,
) -> Result<Option<String>, AppError> {
    let extension = match avatar.content_type.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/jpeg" | "image/jpg" => "jpg",
        other => {
            return Err(AppError::invalid_input(format!(
                "unsupported avatar content type: {other}"
            )))
        }
    };
    let relative_path = format!("data/blobs/avatars/{entity_id}.{extension}");
    let absolute_path = paths.root.join(&relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute_path, &avatar.bytes)?;
    Ok(Some(relative_path))
}

pub(crate) fn persist_bundle_descriptor(
    paths: &WorkspacePaths,
    record: &BundleAssetDescriptorRecord,
    bytes: &[u8],
) -> Result<(), AppError> {
    let absolute_path = paths.root.join(&record.storage_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute_path, bytes)?;
    Ok(())
}

pub(crate) fn upsert_skill_import_source(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    content_hash: &str,
    skill_slug: &str,
    last_imported_at: u64,
) -> Result<(), AppError> {
    connection.execute(
        "INSERT OR REPLACE INTO skill_import_sources
         (source_kind, source_id, source_path, content_hash, skill_slug, department, last_imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![source_kind, source_id, source_id, content_hash, skill_slug, "", last_imported_at as i64],
    ).map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn upsert_agent_import_source(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    agent_id: &str,
    last_imported_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT OR REPLACE INTO agent_import_sources
         (source_kind, source_id, source_path, content_hash, agent_id, department, last_imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source_kind,
                source_id,
                source_id,
                "",
                agent_id,
                "",
                last_imported_at as i64
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn upsert_team_import_source(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    team_id: &str,
    last_imported_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT OR REPLACE INTO team_import_sources
         (source_kind, source_id, source_path, content_hash, team_id, department, last_imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source_kind,
                source_id,
                source_id,
                "",
                team_id,
                "",
                last_imported_at as i64
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn load_existing_skill_import_sources(
    connection: &Connection,
    source_kind: &str,
) -> Result<HashMap<String, ExistingSkillImportSource>, AppError> {
    let mut stmt = connection
        .prepare("SELECT source_id, skill_slug FROM skill_import_sources WHERE source_kind = ?1")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_kind], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ExistingSkillImportSource {
                    skill_slug: row.get(1)?,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    let mut mappings = HashMap::new();
    for row in rows {
        let (source_id, mapping) = row.map_err(|error| AppError::database(error.to_string()))?;
        mappings.insert(source_id, mapping);
    }
    Ok(mappings)
}

pub(crate) fn load_existing_agent_import_sources(
    connection: &Connection,
    source_kind: &str,
) -> Result<HashMap<String, ExistingAgentImportSource>, AppError> {
    let mut stmt = connection
        .prepare("SELECT source_id, agent_id FROM agent_import_sources WHERE source_kind = ?1")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_kind], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ExistingAgentImportSource {
                    agent_id: row.get(1)?,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    let mut mappings = HashMap::new();
    for row in rows {
        let (source_id, mapping) = row.map_err(|error| AppError::database(error.to_string()))?;
        mappings.insert(source_id, mapping);
    }
    Ok(mappings)
}

pub(crate) fn load_existing_team_import_sources(
    connection: &Connection,
    source_kind: &str,
) -> Result<HashMap<String, ExistingTeamImportSource>, AppError> {
    let mut stmt = connection
        .prepare("SELECT source_id, team_id FROM team_import_sources WHERE source_kind = ?1")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_kind], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ExistingTeamImportSource {
                    team_id: row.get(1)?,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    let mut mappings = HashMap::new();
    for row in rows {
        let (source_id, mapping) = row.map_err(|error| AppError::database(error.to_string()))?;
        mappings.insert(source_id, mapping);
    }
    Ok(mappings)
}

pub(crate) fn load_scoped_bundle_asset_descriptors(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, BundleAssetDescriptorRecord>, AppError> {
    let descriptors = load_bundle_asset_descriptor_records(connection)?;
    Ok(descriptors
        .into_iter()
        .filter(|descriptor| {
            descriptor.scope == target.scope_label()
                && descriptor.project_id.as_deref() == target.project_id()
        })
        .map(|descriptor| (descriptor.id.clone(), descriptor))
        .collect())
}

fn is_supported_avatar_file(file_name: &str) -> bool {
    content_type_for_avatar(file_name).is_some()
}

fn content_type_for_avatar(file_name: &str) -> Option<&'static str> {
    match Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        _ => None,
    }
}

pub(crate) fn content_type_for_export(path: &str) -> &'static str {
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

pub(crate) fn encode_file(
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

fn render_agent_markdown(
    agent: &AgentRecord,
    avatar_file_name: &str,
    exported_skill_slugs: &[String],
) -> String {
    let tools = if agent.builtin_tool_keys == builtin_tool_keys() {
        vec![String::from("ALL")]
    } else {
        agent.builtin_tool_keys.clone()
    };
    [
        "---".into(),
        format!("name: {}", yaml_inline_string(&agent.name)),
        format!("description: {}", yaml_inline_string(&agent.description)),
        format!("character: {}", yaml_inline_string(&agent.personality)),
        format!("avatar: {}", yaml_inline_string(avatar_file_name)),
        format!("tag: {}", yaml_inline_string(&agent.tags.join("、"))),
        format!(
            "tools: {}",
            serde_json::to_string(&tools).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "skills: {}",
            serde_json::to_string(exported_skill_slugs).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "mcps: {}",
            serde_json::to_string(&agent.mcp_server_names).unwrap_or_else(|_| "[]".into())
        ),
        "model: \"\"".into(),
        "-----------".into(),
        String::new(),
        agent.prompt.clone(),
    ]
    .join("\n")
}

fn render_team_markdown(
    team: &TeamRecord,
    avatar_file_name: &str,
    leader_name: &str,
    member_names: &[String],
) -> String {
    [
        "---".into(),
        format!("name: {}", yaml_inline_string(&team.name)),
        format!("description: {}", yaml_inline_string(&team.description)),
        format!("avatar: {}", yaml_inline_string(avatar_file_name)),
        format!("tag: {}", yaml_inline_string(&team.tags.join("、"))),
        format!("leader: {}", yaml_inline_string(leader_name)),
        format!("member: {}", yaml_inline_string(&member_names.join("、"))),
        "model: \"\"".into(),
        "-----------".into(),
        String::new(),
        team.prompt.clone(),
    ]
    .join("\n")
}

fn yaml_inline_string(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "\"\"".into()
    } else if trimmed.contains([':', '#', '"', '\'']) || trimmed.starts_with('[') {
        serde_json::to_string(trimmed).unwrap_or_else(|_| "\"\"".into())
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn export_team_files(
    paths: &WorkspacePaths,
    context: &ExportContext,
    team: &TeamRecord,
    root_dir_name: &str,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, AppError> {
    let team_dir_name = sanitize_export_dir_name(&team.name);
    let base_dir = if team_dir_name == root_dir_name {
        root_dir_name.to_string()
    } else {
        format!("{root_dir_name}/{team_dir_name}")
    };
    let mut files = Vec::new();
    let (avatar_name, avatar_bytes) = context
        .avatar_payloads
        .get(&format!("team:{}", team.id))
        .map(|(name, _content_type, bytes)| (name.clone(), bytes.clone()))
        .ok_or_else(|| AppError::not_found("team avatar payload"))?;
    files.push(encode_file(
        &format!("{base_dir}/{avatar_name}"),
        content_type_for_export(&avatar_name),
        avatar_bytes,
    ));
    let member_names = team
        .member_agent_ids
        .iter()
        .filter_map(|agent_id| context.agents.iter().find(|agent| &agent.id == agent_id))
        .map(|agent| agent.name.clone())
        .collect::<Vec<_>>();
    let leader_name = team
        .leader_agent_id
        .as_ref()
        .and_then(|agent_id| context.agents.iter().find(|agent| &agent.id == agent_id))
        .map(|agent| agent.name.clone())
        .unwrap_or_default();
    files.push(encode_file(
        &format!("{base_dir}/{team_dir_name}.md"),
        "text/markdown",
        render_team_markdown(team, &avatar_name, &leader_name, &member_names).into_bytes(),
    ));
    for agent_id in &team.member_agent_ids {
        let Some(agent) = context.agents.iter().find(|item| &item.id == agent_id) else {
            continue;
        };
        files.extend(export_agent_files(
            paths,
            context,
            agent,
            Some(&team_dir_name),
            root_dir_name,
        )?);
    }
    export_owner_skill_and_mcp_files(
        context,
        team,
        &format!("{root_dir_name}/{team_dir_name}"),
        &mut files,
    )?;
    Ok(files)
}

pub(crate) fn export_agent_files(
    _paths: &WorkspacePaths,
    context: &ExportContext,
    agent: &AgentRecord,
    team_dir_name: Option<&str>,
    root_dir_name: &str,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, AppError> {
    let agent_dir_name = sanitize_export_dir_name(&agent.name);
    let base_dir = match team_dir_name {
        Some(team_dir_name) => format!("{root_dir_name}/{team_dir_name}/{agent_dir_name}"),
        None if agent_dir_name == root_dir_name => root_dir_name.to_string(),
        None => format!("{root_dir_name}/{agent_dir_name}"),
    };
    let mut files = Vec::new();
    let (avatar_name, avatar_bytes) = context
        .avatar_payloads
        .get(&format!("agent:{}", agent.id))
        .map(|(name, _content_type, bytes)| (name.clone(), bytes.clone()))
        .ok_or_else(|| AppError::not_found("agent avatar payload"))?;
    files.push(encode_file(
        &format!("{base_dir}/{avatar_name}"),
        content_type_for_export(&avatar_name),
        avatar_bytes,
    ));
    let exported_skill_slugs = agent
        .skill_ids
        .iter()
        .filter_map(|skill_id| {
            context
                .skill_paths
                .get(skill_id)
                .and_then(|path| path.file_name().and_then(|value| value.to_str()))
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    files.push(encode_file(
        &format!("{base_dir}/{agent_dir_name}.md"),
        "text/markdown",
        render_agent_markdown(agent, &avatar_name, &exported_skill_slugs).into_bytes(),
    ));
    export_owner_skill_and_mcp_files(context, agent, &base_dir, &mut files)?;
    Ok(files)
}

trait ExportOwner {
    fn skill_ids(&self) -> &[String];
    fn mcp_server_names(&self) -> &[String];
}

impl ExportOwner for AgentRecord {
    fn skill_ids(&self) -> &[String] {
        &self.skill_ids
    }

    fn mcp_server_names(&self) -> &[String] {
        &self.mcp_server_names
    }
}

impl ExportOwner for TeamRecord {
    fn skill_ids(&self) -> &[String] {
        &self.skill_ids
    }

    fn mcp_server_names(&self) -> &[String] {
        &self.mcp_server_names
    }
}

fn export_owner_skill_and_mcp_files<T: ExportOwner>(
    context: &ExportContext,
    owner: &T,
    owner_dir: &str,
    files: &mut Vec<WorkspaceDirectoryUploadEntry>,
) -> Result<(), AppError> {
    for skill_id in owner.skill_ids() {
        if let Some(skill_root) = context.skill_paths.get(skill_id) {
            let skill_dir_name = skill_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill");
            for (relative_path, bytes) in read_directory_files(skill_root)? {
                files.push(encode_file(
                    &format!("{owner_dir}/skills/{skill_dir_name}/{relative_path}"),
                    content_type_for_export(&relative_path),
                    bytes,
                ));
            }
            continue;
        }

        if let Some(asset) = context.builtin_skill_assets.get(skill_id) {
            for (relative_path, bytes) in &asset.files {
                files.push(encode_file(
                    &format!("{owner_dir}/skills/{}/{relative_path}", asset.slug),
                    content_type_for_export(relative_path),
                    bytes.clone(),
                ));
            }
        }
    }

    for server_name in owner.mcp_server_names() {
        let Some(config) = context.mcp_configs.get(server_name) else {
            continue;
        };
        files.push(encode_file(
            &format!("{owner_dir}/mcps/{server_name}.json"),
            "application/json",
            serde_json::to_vec_pretty(config)?,
        ));
    }
    Ok(())
}

fn sanitize_export_dir_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::from("asset");
    }
    trimmed
        .chars()
        .map(|character| {
            if matches!(character, '/' | '\\' | ':') {
                '-'
            } else {
                character
            }
        })
        .collect()
}

pub(crate) fn build_export_context(
    connection: &Connection,
    paths: &WorkspacePaths,
    target: AssetTargetScope<'_>,
    input: ExportWorkspaceAgentBundleInput,
) -> Result<ExportContext, AppError> {
    let all_agents = load_agents(connection)?;
    let all_teams = load_teams(connection)?;

    let mut agents = Vec::new();
    let mut teams = Vec::new();
    match &target {
        AssetTargetScope::Workspace => {
            teams.extend(all_teams.into_iter().filter(|team| {
                team.project_id.is_none() && input.team_ids.iter().any(|id| id == &team.id)
            }));
            agents.extend(all_agents.into_iter().filter(|agent| {
                agent.project_id.is_none() && input.agent_ids.iter().any(|id| id == &agent.id)
            }));
            if input.mode == "batch" && input.team_ids.is_empty() && input.agent_ids.is_empty() {
                teams = load_teams(connection)?
                    .into_iter()
                    .filter(|item| item.project_id.is_none())
                    .collect();
                agents = load_agents(connection)?
                    .into_iter()
                    .filter(|item| item.project_id.is_none())
                    .collect();
            }
        }
        AssetTargetScope::Project(project_id) => {
            let project_id = project_id.to_string();
            let linked_agent_ids = load_project_linked_ids(
                connection,
                "SELECT agent_id FROM project_agent_links WHERE project_id = ?1",
                project_id.as_str(),
            )?;
            let linked_team_ids = load_project_linked_ids(
                connection,
                "SELECT team_id FROM project_team_links WHERE project_id = ?1",
                project_id.as_str(),
            )?;

            teams.extend(load_teams(connection)?.into_iter().filter(|team| {
                input.team_ids.iter().any(|id| id == &team.id)
                    && (team.project_id.as_deref() == Some(project_id.as_str())
                        || linked_team_ids.contains(&team.id))
            }));
            agents.extend(load_agents(connection)?.into_iter().filter(|agent| {
                input.agent_ids.iter().any(|id| id == &agent.id)
                    && (agent.project_id.as_deref() == Some(project_id.as_str())
                        || linked_agent_ids.contains(&agent.id))
            }));
            if input.mode == "batch" && input.team_ids.is_empty() && input.agent_ids.is_empty() {
                teams = load_teams(connection)?
                    .into_iter()
                    .filter(|team| {
                        team.project_id.as_deref() == Some(project_id.as_str())
                            || linked_team_ids.contains(&team.id)
                    })
                    .collect();
                agents = load_agents(connection)?
                    .into_iter()
                    .filter(|agent| {
                        agent.project_id.as_deref() == Some(project_id.as_str())
                            || linked_agent_ids.contains(&agent.id)
                    })
                    .collect();
            }
        }
    }

    let mut seen_agent_ids = BTreeSet::new();
    agents.retain(|agent| seen_agent_ids.insert(agent.id.clone()));
    let mut seen_team_ids = BTreeSet::new();
    teams.retain(|team| seen_team_ids.insert(team.id.clone()));
    for team in &teams {
        for member_agent_id in &team.member_agent_ids {
            if seen_agent_ids.contains(member_agent_id) {
                continue;
            }
            if let Some(agent) = load_agents(connection)?
                .into_iter()
                .find(|candidate| &candidate.id == member_agent_id)
            {
                seen_agent_ids.insert(agent.id.clone());
                agents.push(agent);
            }
        }
    }

    let skill_paths = resolve_skill_paths(paths, &agents, &teams)?;
    let builtin_skill_assets = resolve_builtin_skill_assets(&agents, &teams)?;
    let mcp_configs = resolve_mcp_configs(paths, &target, &agents, &teams)?;
    let avatar_payloads = resolve_avatar_payloads(paths, &target, &agents, &teams)?;
    let descriptors = load_scoped_bundle_asset_descriptors(connection, &target)?
        .into_values()
        .collect::<Vec<_>>();
    let asset_state = crate::agent_bundle::manifest_v2::build_bundle_asset_state(
        paths,
        &target,
        &skill_paths,
        &mcp_configs,
    )?;
    let root_dir_name = if input.mode == "single"
        && teams.len() == 1
        && agents
            .iter()
            .all(|agent| teams[0].member_agent_ids.contains(&agent.id))
    {
        sanitize_export_dir_name(&teams[0].name)
    } else if input.mode == "single" && agents.len() == 1 && teams.is_empty() {
        sanitize_export_dir_name(&agents[0].name)
    } else {
        String::from("agent-bundle")
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
        descriptors,
        skill_paths,
        builtin_skill_assets,
        mcp_configs,
        avatar_payloads,
        asset_state,
        bundle_manifest,
        translation_report,
        issues: Vec::new(),
    })
}

fn load_project_linked_ids(
    connection: &Connection,
    statement: &str,
    project_id: &str,
) -> Result<BTreeSet<String>, AppError> {
    let mut stmt = connection
        .prepare(statement)
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![project_id], |row| row.get::<_, String>(0))
        .map_err(|error| AppError::database(error.to_string()))?;
    let ids = rows
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(ids)
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
        roots.push(crate::resources_skills::SkillCatalogRoot {
            source: crate::resources_skills::SkillDefinitionSource::WorkspaceManaged,
            path: paths.project_skills_root(project_id),
            origin: crate::resources_skills::SkillSourceOrigin::SkillsDir,
        });
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
) -> Result<HashMap<String, (String, String, Vec<u8>)>, AppError> {
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
) -> Result<(String, String, Vec<u8>), AppError> {
    if let Some(avatar_path) = avatar_path {
        let absolute_path = paths.root.join(avatar_path);
        if absolute_path.is_file() {
            let file_name = absolute_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("avatar.png")
                .to_string();
            if let Some(content_type) = content_type_for_avatar(&file_name) {
                return Ok((
                    file_name,
                    content_type.to_string(),
                    fs::read(absolute_path)?,
                ));
            }
        }
    }
    default_avatar_payload(owner_kind, seed_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog_hash_id;
    use crate::infra_state::{
        ensure_agent_record_columns, ensure_bundle_asset_descriptor_columns,
        ensure_team_record_columns,
    };
    use octopus_core::{
        ExportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundleInput,
        ImportWorkspaceAgentBundlePreviewInput,
    };

    fn encoded_file(relative_path: &str, content: &str) -> WorkspaceDirectoryUploadEntry {
        WorkspaceDirectoryUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .into(),
            content_type: "text/markdown".into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn encoded_json_file(relative_path: &str, content: &str) -> WorkspaceDirectoryUploadEntry {
        WorkspaceDirectoryUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .into(),
            content_type: "application/json".into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn ensure_test_tables(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE agents (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    project_id TEXT,
                    scope TEXT NOT NULL,
                    name TEXT NOT NULL,
                    avatar_path TEXT,
                    personality TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    prompt TEXT NOT NULL,
                    builtin_tool_keys TEXT NOT NULL,
                    skill_ids TEXT NOT NULL,
                    mcp_server_names TEXT NOT NULL,
                    description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE teams (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    project_id TEXT,
                    scope TEXT NOT NULL,
                    name TEXT NOT NULL,
                    avatar_path TEXT,
                    personality TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    prompt TEXT NOT NULL,
                    builtin_tool_keys TEXT NOT NULL,
                    skill_ids TEXT NOT NULL,
                    mcp_server_names TEXT NOT NULL,
                    leader_agent_id TEXT,
                    member_agent_ids TEXT NOT NULL,
                    description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE project_agent_links (
                    workspace_id TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    linked_at INTEGER NOT NULL
                );
                CREATE TABLE project_team_links (
                    workspace_id TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    team_id TEXT NOT NULL,
                    linked_at INTEGER NOT NULL
                );",
            )
            .expect("tables");
        ensure_agent_record_columns(connection).expect("agent columns");
        ensure_team_record_columns(connection).expect("team columns");
        ensure_bundle_asset_descriptor_columns(connection).expect("descriptor columns");
        crate::agent_bundle::shared::ensure_import_source_tables(connection)
            .expect("import source tables");
    }

    fn test_agent_record(
        id: &str,
        workspace_id: &str,
        project_id: Option<&str>,
        scope: &str,
        name: &str,
        personality: &str,
        tags: Vec<String>,
        prompt: &str,
        builtin_tool_keys: Vec<String>,
        skill_ids: Vec<String>,
        mcp_server_names: Vec<String>,
        description: &str,
    ) -> AgentRecord {
        let task_domains = normalize_task_domains(tags.clone());
        AgentRecord {
            id: id.into(),
            workspace_id: workspace_id.into(),
            project_id: project_id.map(str::to_string),
            scope: scope.into(),
            name: name.into(),
            avatar_path: None,
            avatar: None,
            personality: personality.into(),
            tags,
            prompt: prompt.into(),
            builtin_tool_keys: builtin_tool_keys.clone(),
            skill_ids: skill_ids.clone(),
            mcp_server_names: mcp_server_names.clone(),
            task_domains: task_domains.clone(),
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: default_model_strategy(),
            capability_policy: capability_policy_from_sources(
                &builtin_tool_keys,
                &skill_ids,
                &mcp_server_names,
            ),
            permission_envelope: default_permission_envelope(),
            memory_policy: default_agent_memory_policy(),
            delegation_policy: default_agent_delegation_policy(),
            approval_preference: default_approval_preference(),
            output_contract: default_output_contract(),
            shared_capability_policy: default_agent_shared_capability_policy(),
            integration_source: None,
            trust_metadata: default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: default_asset_import_metadata(),
            description: description.into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        }
    }

    #[test]
    fn preview_supports_standalone_agent_root_and_yaml_arrays() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let preview = crate::agent_bundle::preview_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundlePreviewInput {
                files: vec![
                    encoded_file(
                        "财务分析师/财务分析师.md",
                        "---\nname: 财务分析师\ndescription: 财务分析\ncharacter: 数字敏感\ntools: [\"ALL\"]\nskills: [\"shared-skill\"]\nmcps: [\"ops\"]\n---\n\n# 角色定义\n财务专家\n",
                    ),
                    encoded_file(
                        "财务分析师/skills/shared-skill/SKILL.md",
                        "---\nname: shared-skill\ndescription: shared\n---\n\n# Shared\n",
                    ),
                    encoded_json_file(
                        "财务分析师/mcps/ops.json",
                        r#"{"type":"http","url":"https://ops.example.test/mcp"}"#,
                    ),
                ],
            },
        )
        .expect("preview");

        assert_eq!(preview.detected_agent_count, 1);
        assert_eq!(preview.detected_team_count, 0);
        assert_eq!(preview.unique_skill_count, 1);
        assert_eq!(preview.unique_mcp_count, 1);
        assert_eq!(preview.agents[0].mcp_server_names, vec!["ops"]);
    }

    #[test]
    fn preview_supports_team_bundle_and_reference_only_mcp_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let preview = crate::agent_bundle::preview_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundlePreviewInput {
                files: vec![
                    encoded_file(
                        "财务部/财务部.md",
                        "---\nname: 财务部\ndescription: 财务团队\nleader: 财务负责人\nmember: 财务负责人、财务分析师\n---\n\n# leader职责\n负责统筹\n",
                    ),
                    encoded_file(
                        "财务部/财务负责人/财务负责人.md",
                        "---\nname: 财务负责人\ndescription: 负责人\nskills: []\nmcps: []\n---\n\n# 角色定义\n负责人\n",
                    ),
                    encoded_file(
                        "财务部/财务分析师/财务分析师.md",
                        "---\nname: 财务分析师\ndescription: 分析师\nmcps: [\"shared-ops\"]\n---\n\n# 角色定义\n分析师\n",
                    ),
                ],
            },
        )
        .expect("preview");

        assert_eq!(preview.detected_team_count, 1);
        assert_eq!(preview.detected_agent_count, 2);
        assert_eq!(preview.teams.len(), 1);
        assert_eq!(preview.teams[0].member_names.len(), 2);
        assert!(preview.issues.iter().any(|item| item.scope == "mcp"));
    }

    #[test]
    fn export_single_agent_uses_agent_root_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let agent_id = "agent-1";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "财务分析师",
            "数字敏感",
            vec!["财务".into()],
            "# 角色定义\n财务专家\n",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "负责财务分析",
        );
        write_agent_record(&connection, &record, false).expect("write agent");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert_eq!(exported.root_dir_name, "财务分析师");
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务分析师/财务分析师.md"));
    }

    #[test]
    fn export_project_bundle_includes_project_skill_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let project_id = "project-finance";
        let skill_slug = "project-skill";
        let skill_root = paths.project_skills_root(project_id).join(skill_slug);
        fs::create_dir_all(&skill_root).expect("create skill root");
        fs::write(
            skill_root.join("SKILL.md"),
            "---\nname: project-skill\ndescription: Project scoped skill\n---\n\n# Overview\n\nproject only\n",
        )
        .expect("write skill");

        let project_agent_id = "agent-project-1";
        let project_agent = test_agent_record(
            project_agent_id,
            "ws-local",
            Some(project_id),
            "project",
            "项目财务分析师",
            "项目财务",
            vec!["项目".into()],
            "# 角色定义\n项目财务分析\n",
            Vec::new(),
            vec![managed_skill_id(
                &AssetTargetScope::Project(project_id),
                skill_slug,
            )],
            Vec::new(),
            "项目级技能",
        );
        write_agent_record(&connection, &project_agent, false).expect("write project agent");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project(project_id),
            ExportWorkspaceAgentBundleInput {
                mode: "batch".into(),
                agent_ids: Vec::new(),
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert!(exported.files.iter().any(|file| {
            file.relative_path == "agent-bundle/项目财务分析师/skills/project-skill/SKILL.md"
        }));
    }

    #[test]
    fn export_project_bundle_materializes_linked_workspace_builtin_dependencies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let builtin_skill = crate::agent_bundle::list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .find(|asset| asset.slug == "financial-calculator")
            .expect("financial-calculator builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);
        let builtin_mcp = crate::agent_bundle::find_builtin_mcp_asset("finance-data")
            .expect("builtin mcp lookup")
            .expect("finance-data builtin mcp");

        let agent_id = "agent-linked-workspace";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "财务联动员工",
            "处理项目联动财务任务",
            vec!["财务".into()],
            "# 角色定义\n处理项目财务联动\n",
            vec!["bash".into()],
            vec![builtin_skill_id],
            vec![builtin_mcp.server_name.clone()],
            "工作区级财务员工",
        );
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params!["ws-local", "proj-finance", agent_id, timestamp_now() as i64],
            )
            .expect("insert project agent link");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-finance"),
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        assert_eq!(exported.root_dir_name, "财务联动员工");
        assert!(exported
            .files
            .iter()
            .any(|file| file.relative_path == "财务联动员工/.octopus/manifest.json"));
        assert!(exported.files.iter().any(|file| {
            file.relative_path == "财务联动员工/skills/financial-calculator/SKILL.md"
        }));
        assert!(exported
            .files
            .iter()
            .any(|file| { file.relative_path == "财务联动员工/mcps/finance-data.json" }));
        assert!(exported.files.iter().any(|file| {
            file.relative_path.starts_with("财务联动员工/")
                && (file.relative_path.ends_with(".png")
                    || file.relative_path.ends_with(".jpg")
                    || file.relative_path.ends_with(".jpeg")
                    || file.relative_path.ends_with(".webp"))
        }));
    }

    #[test]
    fn export_project_bundle_roundtrips_via_project_import() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let builtin_skill = crate::agent_bundle::list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .find(|asset| asset.slug == "financial-calculator")
            .expect("financial-calculator builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);

        let agent_id = "agent-linked-roundtrip";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "导出回导员工",
            "负责导出回导验证",
            vec!["回归".into()],
            "# 角色定义\n导出后重新导入\n",
            vec!["bash".into()],
            vec![builtin_skill_id],
            vec!["finance-data".into()],
            "验证项目导出闭包",
        );
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params!["ws-local", "proj-export", agent_id, timestamp_now() as i64],
            )
            .expect("insert project agent link");

        let exported = crate::agent_bundle::export_assets(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-export"),
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        let imported = crate::agent_bundle::execute_import(
            &connection,
            &paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Project("proj-import"),
            ImportWorkspaceAgentBundleInput {
                files: exported.files,
            },
        )
        .expect("import");

        assert_eq!(imported.failure_count, 0);
        assert_eq!(imported.agent_count, 1);
        assert_eq!(imported.team_count, 0);
        assert_eq!(imported.skill_count, 1);
        assert_eq!(imported.mcp_count, 1);
        assert_eq!(imported.avatar_count, 1);
    }

    #[test]
    fn export_import_roundtrips_asset_state_metadata_for_managed_skill_and_mcp() {
        let source_temp = tempfile::tempdir().expect("source tempdir");
        let source_paths = WorkspacePaths::new(source_temp.path());
        source_paths.ensure_layout().expect("source layout");
        let source_connection = Connection::open(source_paths.db_path.clone()).expect("source db");
        ensure_test_tables(&source_connection);

        let skill_slug = "managed-roundtrip";
        let skill_dir = source_paths.managed_skills_dir.join(skill_slug);
        fs::create_dir_all(&skill_dir).expect("managed skill dir");
        fs::write(
            skill_dir.join(SKILL_FRONTMATTER_FILE),
            "---\nname: managed-roundtrip\ndescription: Managed roundtrip skill.\n---\n",
        )
        .expect("write managed skill");

        let skill_source_key = format!("skill:data/skills/{skill_slug}/{SKILL_FRONTMATTER_FILE}");
        let mcp_server_name = "roundtrip-mcp";
        let mcp_source_key = format!("mcp:{mcp_server_name}");

        let mut source_asset_state =
            crate::resources_skills::load_workspace_asset_state_document(&source_paths)
                .expect("load source asset state");
        crate::resources_skills::set_workspace_asset_enabled(
            &mut source_asset_state,
            &skill_source_key,
            false,
        );
        crate::resources_skills::set_workspace_asset_trusted(
            &mut source_asset_state,
            &skill_source_key,
            true,
        );
        crate::resources_skills::set_workspace_asset_enabled(
            &mut source_asset_state,
            &mcp_source_key,
            false,
        );
        crate::resources_skills::set_workspace_asset_trusted(
            &mut source_asset_state,
            &mcp_source_key,
            true,
        );
        crate::resources_skills::save_workspace_asset_state_document(
            &source_paths,
            &source_asset_state,
        )
        .expect("save source asset state");

        crate::resources_skills::write_workspace_runtime_document(
            &source_paths,
            &serde_json::Map::from_iter([(
                "mcpServers".to_string(),
                json!({
                    mcp_server_name: {
                        "transport": "stdio",
                        "command": "roundtrip-mcp",
                        "args": ["serve"]
                    }
                }),
            )]),
        )
        .expect("write workspace runtime document");

        let agent_id = "agent-managed-roundtrip";
        let record = test_agent_record(
            agent_id,
            "ws-local",
            None,
            "workspace",
            "Managed Roundtrip",
            "Verifies asset metadata export and import",
            vec!["roundtrip".into()],
            "# Role\nVerify asset metadata roundtrip.\n",
            vec!["bash".into()],
            vec![managed_skill_id(&AssetTargetScope::Workspace, skill_slug)],
            vec![mcp_server_name.into()],
            "Managed asset metadata roundtrip",
        );
        write_agent_record(&source_connection, &record, false).expect("write source agent");

        let exported = crate::agent_bundle::export_assets(
            &source_connection,
            &source_paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export source assets");
        assert!(
            exported
                .files
                .iter()
                .any(|file| file.relative_path.ends_with(".octopus/asset-state.json")),
            "bundle should carry serialized asset metadata"
        );

        let destination_temp = tempfile::tempdir().expect("destination tempdir");
        let destination_paths = WorkspacePaths::new(destination_temp.path());
        destination_paths
            .ensure_layout()
            .expect("destination layout");
        let destination_connection =
            Connection::open(destination_paths.db_path.clone()).expect("destination db");
        ensure_test_tables(&destination_connection);

        crate::agent_bundle::execute_import(
            &destination_connection,
            &destination_paths,
            "ws-local",
            crate::agent_bundle::BundleTarget::Workspace,
            ImportWorkspaceAgentBundleInput {
                files: exported.files,
            },
        )
        .expect("import destination assets");

        let destination_asset_state: JsonValue = serde_json::from_str(
            &fs::read_to_string(&destination_paths.workspace_asset_state_path)
                .expect("destination asset state file"),
        )
        .expect("parse destination asset state");
        assert_eq!(
            destination_asset_state["assets"][skill_source_key.as_str()]["enabled"],
            JsonValue::Bool(false)
        );
        assert_eq!(
            destination_asset_state["assets"][skill_source_key.as_str()]["trusted"],
            JsonValue::Bool(true)
        );
        assert_eq!(
            destination_asset_state["assets"][mcp_source_key.as_str()]["enabled"],
            JsonValue::Bool(false)
        );
        assert_eq!(
            destination_asset_state["assets"][mcp_source_key.as_str()]["trusted"],
            JsonValue::Bool(true)
        );
    }
}
