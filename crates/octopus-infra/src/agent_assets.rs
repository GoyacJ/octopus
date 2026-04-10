use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{
    timestamp_now, AgentRecord, AppError, ExportWorkspaceAgentBundleInput,
    ExportWorkspaceAgentBundleResult, ImportIssue, ImportWorkspaceAgentBundleInput,
    ImportWorkspaceAgentBundlePreview, ImportWorkspaceAgentBundlePreviewInput,
    ImportWorkspaceAgentBundleResult, ImportedAgentPreviewItem, ImportedAvatarPreviewItem,
    ImportedMcpPreviewItem, ImportedSkillPreviewItem, ImportedTeamPreviewItem, TeamRecord,
    WorkspaceDirectoryUploadEntry,
};
use rusqlite::{params, Connection};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use sha2::{Digest, Sha256};

use crate::{
    infra_state::{agent_avatar, load_agents, load_teams, write_team_record},
    resources_skills::{
        catalog_hash_id, disabled_source_keys, discover_skill_roots, load_skills_from_roots,
        validate_skill_file_relative_path, validate_skill_slug, write_workspace_runtime_document,
    },
    WorkspacePaths,
};

const ISSUE_WARNING: &str = "warning";
const ISSUE_ERROR: &str = "error";
const SOURCE_SCOPE_BUNDLE: &str = "bundle";
const SOURCE_SCOPE_AGENT: &str = "agent";
const SOURCE_SCOPE_TEAM: &str = "team";
const SOURCE_SCOPE_SKILL: &str = "skill";
const SOURCE_SCOPE_MCP: &str = "mcp";
const SOURCE_SCOPE_AVATAR: &str = "avatar";
const SKILL_FRONTMATTER_FILE: &str = "SKILL.md";
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
static BUILTIN_MCP_ASSET_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/seed/builtin-assets/mcps");

pub(crate) const BUILTIN_SKILL_DISPLAY_ROOT: &str = "builtin-assets/skills";

#[derive(Debug, Clone)]
pub(crate) enum AssetTargetScope<'a> {
    Workspace,
    Project(&'a str),
}

impl AssetTargetScope<'_> {
    fn scope_label(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Project(_) => "project",
        }
    }

    fn project_id(&self) -> Option<&str> {
        match self {
            Self::Workspace => None,
            Self::Project(project_id) => Some(project_id),
        }
    }

    fn source_kind(&self) -> String {
        match self {
            Self::Workspace => String::from("user_import:workspace"),
            Self::Project(project_id) => format!("user_import:project:{project_id}"),
        }
    }

    fn skill_root(&self, paths: &WorkspacePaths) -> PathBuf {
        match self {
            Self::Workspace => paths.managed_skills_dir.clone(),
            Self::Project(project_id) => paths.project_skills_root(project_id),
        }
    }

    fn runtime_document_path(&self, paths: &WorkspacePaths) -> PathBuf {
        match self {
            Self::Workspace => paths.runtime_config_dir.join("workspace.json"),
            Self::Project(project_id) => paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
        }
    }

    fn avatar_seed_key(&self, source_id: &str) -> String {
        format!("{}:{source_id}", self.scope_label())
    }
}

#[derive(Debug, Clone)]
struct BundleFile {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinSkillAsset {
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) display_path: String,
    pub(crate) root_display_path: String,
    pub(crate) source_ids: Vec<String>,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltinMcpAsset {
    pub(crate) server_name: String,
    pub(crate) display_path: String,
    pub(crate) config: JsonValue,
}

#[derive(Debug, Clone)]
struct ParsedAssetAvatar {
    source_id: String,
    owner_kind: String,
    owner_name: String,
    file_name: String,
    content_type: String,
    bytes: Vec<u8>,
    generated: bool,
}

#[derive(Debug, Clone)]
struct ParsedAgent {
    source_id: String,
    team_name: Option<String>,
    name: String,
    description: String,
    personality: String,
    prompt: String,
    tags: Vec<String>,
    builtin_tool_keys: Vec<String>,
    skill_source_ids: Vec<String>,
    mcp_source_ids: Vec<String>,
    avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
struct ParsedTeam {
    source_id: String,
    name: String,
    description: String,
    personality: String,
    prompt: String,
    tags: Vec<String>,
    builtin_tool_keys: Vec<String>,
    skill_source_ids: Vec<String>,
    mcp_source_ids: Vec<String>,
    leader_name: Option<String>,
    member_names: Vec<String>,
    agent_source_ids: Vec<String>,
    avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
struct ParsedSkillSource {
    source_id: String,
    owner_name: String,
    name: String,
    canonical_slug: String,
    content_hash: String,
    files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
struct ParsedMcpSource {
    source_id: String,
    owner_name: String,
    server_name: String,
    content_hash: Option<String>,
    config: Option<JsonValue>,
    referenced_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportAction {
    Create,
    Update,
    Skip,
    Failed,
}

impl ImportAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Skip => "skip",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct PlannedSkill {
    slug: String,
    skill_id: String,
    name: String,
    action: ImportAction,
    content_hash: String,
    file_count: usize,
    source_ids: Vec<String>,
    consumer_names: Vec<String>,
    files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
struct PlannedMcp {
    server_name: String,
    action: ImportAction,
    content_hash: Option<String>,
    source_ids: Vec<String>,
    consumer_names: Vec<String>,
    config: Option<JsonValue>,
    referenced_only: bool,
    resolved: bool,
}

#[derive(Debug, Clone)]
struct PlannedAgent {
    source_id: String,
    agent_id: Option<String>,
    name: String,
    department: String,
    action: ImportAction,
    description: String,
    personality: String,
    prompt: String,
    tags: Vec<String>,
    builtin_tool_keys: Vec<String>,
    skill_slugs: Vec<String>,
    mcp_server_names: Vec<String>,
    avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
struct PlannedTeam {
    source_id: String,
    team_id: Option<String>,
    name: String,
    action: ImportAction,
    description: String,
    personality: String,
    prompt: String,
    tags: Vec<String>,
    builtin_tool_keys: Vec<String>,
    skill_slugs: Vec<String>,
    mcp_server_names: Vec<String>,
    leader_name: Option<String>,
    member_names: Vec<String>,
    agent_source_ids: Vec<String>,
    avatar: ParsedAssetAvatar,
}

#[derive(Debug, Clone)]
struct BundlePlan {
    departments: Vec<String>,
    detected_agent_count: u64,
    detected_team_count: u64,
    filtered_file_count: u64,
    issues: Vec<ImportIssue>,
    skills: Vec<PlannedSkill>,
    mcps: Vec<PlannedMcp>,
    agents: Vec<PlannedAgent>,
    teams: Vec<PlannedTeam>,
    avatars: Vec<ParsedAssetAvatar>,
}

#[derive(Debug, Clone)]
struct ExistingAgentImportSource {
    agent_id: String,
}

#[derive(Debug, Clone)]
struct ExistingTeamImportSource {
    team_id: String,
}

#[derive(Debug, Clone)]
struct ExistingSkillImportSource {
    skill_slug: String,
}

#[derive(Debug, Clone)]
struct ExistingManagedSkill {
    slug: String,
    content_hash: String,
}

pub(crate) fn preview_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: AssetTargetScope<'_>,
    input: ImportWorkspaceAgentBundlePreviewInput,
) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
    let plan = build_bundle_plan(connection, paths, workspace_id, target, &input.files)?;
    Ok(plan_to_preview(&plan))
}

pub(crate) fn execute_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: AssetTargetScope<'_>,
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
        if matches!(action, ImportAction::Create | ImportAction::Update) {
            if let Err(error) =
                write_managed_skill(&target.skill_root(paths), &skill.slug, &skill.files)
            {
                action = ImportAction::Failed;
                failed_skill_slugs.insert(skill.slug.clone());
                issues.push(issue(
                    ISSUE_ERROR,
                    SOURCE_SCOPE_SKILL,
                    skill.source_ids.first().cloned(),
                    format!("failed to import skill '{}': {error}", skill.slug),
                ));
            }
        }
        if action != ImportAction::Failed {
            for source_id in &skill.source_ids {
                upsert_skill_import_source(
                    connection,
                    &source_kind,
                    source_id,
                    &skill.content_hash,
                    &skill.slug,
                    now,
                )?;
            }
        }
        increment_action_counts(
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

    let existing_mcp_target = load_target_runtime_document(paths, &target)?;
    let effective_mcp_document =
        plan_mcp_document_updates(existing_mcp_target, &plan.mcps, &mut issues)?;
    write_target_runtime_document(paths, &target, &effective_mcp_document)?;

    let mut mcp_results = Vec::new();
    for mcp in &plan.mcps {
        increment_action_counts(
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

    let existing_agents = load_scoped_agents(connection, &target)?;
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
            .map(|slug| managed_skill_id(&target, slug))
            .collect::<Vec<_>>();
        let actual_action =
            resolve_agent_action(workspace_id, &target, &existing_agents, agent, &skill_ids)?;
        let mut result_action = actual_action;
        let agent_id = agent
            .agent_id
            .clone()
            .unwrap_or_else(|| deterministic_asset_id("agent", &target, &agent.source_id));
        let avatar_path = match persist_avatar(paths, &agent_id, &agent.avatar) {
            Ok(path) => path,
            Err(error) => {
                result_action = ImportAction::Failed;
                issues.push(issue(
                    ISSUE_ERROR,
                    SOURCE_SCOPE_AVATAR,
                    Some(agent.source_id.clone()),
                    format!("failed to persist agent avatar '{}': {error}", agent.name),
                ));
                None
            }
        };

        if result_action != ImportAction::Failed
            && matches!(actual_action, ImportAction::Create | ImportAction::Update)
        {
            let record = build_agent_record(
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
            if let Err(error) =
                write_agent_record(connection, &record, actual_action == ImportAction::Update)
            {
                result_action = ImportAction::Failed;
                issues.push(issue(
                    ISSUE_ERROR,
                    SOURCE_SCOPE_AGENT,
                    Some(agent.source_id.clone()),
                    format!("failed to import agent '{}': {error}", agent.name),
                ));
            } else {
                upsert_agent_import_source(
                    connection,
                    &source_kind,
                    &agent.source_id,
                    &agent_id,
                    now,
                )?;
            }
        } else if result_action != ImportAction::Failed {
            upsert_agent_import_source(connection, &source_kind, &agent.source_id, &agent_id, now)?;
        }

        increment_action_counts(
            result_action,
            &mut total_create,
            &mut total_update,
            &mut total_skip,
        );
        if result_action != ImportAction::Failed {
            agent_id_by_source.insert(agent.source_id.clone(), agent_id.clone());
        }
        agent_results.push(ImportedAgentPreviewItem {
            source_id: agent.source_id.clone(),
            agent_id: Some(agent_id),
            name: agent.name.clone(),
            department: agent.department.clone(),
            action: result_action.as_str().into(),
            skill_slugs: usable_skill_slugs,
            mcp_server_names: agent.mcp_server_names.clone(),
        });
    }

    let existing_teams = load_scoped_teams(connection, &target)?;
    let mut team_results = Vec::new();
    for team in &plan.teams {
        let skill_ids = team
            .skill_slugs
            .iter()
            .filter(|slug| !failed_skill_slugs.contains(*slug))
            .map(|slug| managed_skill_id(&target, slug))
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
        let actual_action = resolve_team_action(
            workspace_id,
            &target,
            &existing_teams,
            team,
            &skill_ids,
            leader_agent_id.as_deref(),
            &member_agent_ids,
        )?;
        let mut result_action = actual_action;
        let team_id = team
            .team_id
            .clone()
            .unwrap_or_else(|| deterministic_asset_id("team", &target, &team.source_id));
        let avatar_path = match persist_avatar(paths, &team_id, &team.avatar) {
            Ok(path) => path,
            Err(error) => {
                result_action = ImportAction::Failed;
                issues.push(issue(
                    ISSUE_ERROR,
                    SOURCE_SCOPE_AVATAR,
                    Some(team.source_id.clone()),
                    format!("failed to persist team avatar '{}': {error}", team.name),
                ));
                None
            }
        };

        if result_action != ImportAction::Failed
            && matches!(actual_action, ImportAction::Create | ImportAction::Update)
        {
            let record = build_team_record(
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
            if let Err(error) =
                write_team_record(connection, &record, actual_action == ImportAction::Update)
            {
                result_action = ImportAction::Failed;
                issues.push(issue(
                    ISSUE_ERROR,
                    SOURCE_SCOPE_TEAM,
                    Some(team.source_id.clone()),
                    format!("failed to import team '{}': {error}", team.name),
                ));
            } else {
                upsert_team_import_source(
                    connection,
                    &source_kind,
                    &team.source_id,
                    &team_id,
                    now,
                )?;
            }
        } else if result_action != ImportAction::Failed {
            upsert_team_import_source(connection, &source_kind, &team.source_id, &team_id, now)?;
        }

        increment_action_counts(
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

    Ok(ImportWorkspaceAgentBundleResult {
        departments: plan.departments.clone(),
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
            .filter(|entry| entry.severity == ISSUE_ERROR)
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
    })
}

pub(crate) fn export_assets(
    connection: &Connection,
    paths: &WorkspacePaths,
    _workspace_id: &str,
    target: AssetTargetScope<'_>,
    input: ExportWorkspaceAgentBundleInput,
) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
    let context = build_export_context(connection, paths, target, input)?;
    let root_dir_name = context.root_dir_name.clone();
    let mut files = Vec::new();

    for team in &context.teams {
        files.extend(export_team_files(paths, &context, team, &root_dir_name)?);
    }

    let team_member_ids = context
        .teams
        .iter()
        .flat_map(|team| team.member_agent_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    for agent in &context.agents {
        if team_member_ids.contains(&agent.id) {
            continue;
        }
        files.extend(export_agent_files(
            paths,
            &context,
            agent,
            None,
            &root_dir_name,
        )?);
    }

    files.push(encode_file(
        &format!("{root_dir_name}/.octopus/manifest.json"),
        "application/json",
        serde_json::to_vec_pretty(&json!({
            "mode": context.mode,
            "agentIds": context.agents.iter().map(|item| item.id.clone()).collect::<Vec<_>>(),
            "teamIds": context.teams.iter().map(|item| item.id.clone()).collect::<Vec<_>>(),
        }))?,
    ));

    Ok(ExportWorkspaceAgentBundleResult {
        root_dir_name,
        file_count: files.len() as u64,
        agent_count: context.agents.len() as u64,
        team_count: context.teams.len() as u64,
        skill_count: context.skill_paths.len() as u64,
        mcp_count: context.mcp_configs.len() as u64,
        avatar_count: context.avatar_payloads.len() as u64,
        files,
        issues: context.issues,
    })
}

#[derive(Debug, Clone)]
struct ExportContext {
    mode: String,
    root_dir_name: String,
    agents: Vec<AgentRecord>,
    teams: Vec<TeamRecord>,
    skill_paths: HashMap<String, PathBuf>,
    builtin_skill_assets: HashMap<String, BuiltinSkillAsset>,
    mcp_configs: HashMap<String, JsonValue>,
    avatar_payloads: HashMap<String, (String, String, Vec<u8>)>,
    issues: Vec<ImportIssue>,
}

fn build_bundle_plan(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: AssetTargetScope<'_>,
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<BundlePlan, AppError> {
    let (bundle_files, filtered_file_count, mut issues) = normalize_bundle_files(files)?;
    let bundle_files = strip_optional_bundle_root(bundle_files);
    let parsed = parse_bundle_files(&bundle_files, &target, &mut issues)?;
    let existing_skill_sources =
        load_existing_skill_import_sources(connection, &target.source_kind())?;
    let existing_team_sources =
        load_existing_team_import_sources(connection, &target.source_kind())?;
    let existing_agent_sources =
        load_existing_agent_import_sources(connection, &target.source_kind())?;
    let existing_managed_skills = load_existing_managed_skills(&target.skill_root(paths))?;
    let existing_target_mcp = load_target_mcp_map(paths, &target)?;
    let existing_effective_mcp = load_effective_mcp_map(paths, &target)?;
    let existing_agents = load_scoped_agents(connection, &target)?;
    let existing_teams = load_scoped_teams(connection, &target)?;

    let mut unique_skills = BTreeMap::<(String, String), PlannedSkill>::new();
    for source in &parsed.skills {
        let entry = unique_skills
            .entry((source.canonical_slug.clone(), source.content_hash.clone()))
            .or_insert_with(|| PlannedSkill {
                slug: String::new(),
                skill_id: String::new(),
                name: source.name.clone(),
                action: ImportAction::Create,
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
                (mapped_slug, ImportAction::Skip)
            } else {
                (mapped_slug, ImportAction::Update)
            }
        } else if let Some(existing) = existing_managed_skills.get(&canonical_slug) {
            if existing.content_hash == content_hash {
                (canonical_slug.clone(), ImportAction::Skip)
            } else {
                let candidate = format!("{}-{}", canonical_slug, short_hash(&content_hash));
                let action = if existing_managed_skills
                    .get(&candidate)
                    .is_some_and(|item| item.content_hash == content_hash)
                {
                    ImportAction::Skip
                } else {
                    ImportAction::Create
                };
                (candidate, action)
            }
        } else {
            (canonical_slug.clone(), ImportAction::Create)
        };

        planned.slug = slug.clone();
        planned.skill_id = managed_skill_id(&target, &slug);
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

    let mut unique_mcps = BTreeMap::<(String, Option<String>, bool), PlannedMcp>::new();
    for source in &parsed.mcps {
        let key = (
            source.server_name.clone(),
            source.content_hash.clone(),
            source.referenced_only,
        );
        let entry = unique_mcps.entry(key).or_insert_with(|| PlannedMcp {
            server_name: source.server_name.clone(),
            action: ImportAction::Create,
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
                planned.action = ImportAction::Skip;
                planned.resolved = true;
            } else {
                planned.action = ImportAction::Failed;
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_MCP,
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
                    planned.action = ImportAction::Skip;
                } else {
                    planned.server_name = format!(
                        "{}-{}",
                        planned.server_name,
                        short_hash(content_hash.as_deref().unwrap_or_default())
                    );
                    planned.action = if existing_target_mcp
                        .get(&planned.server_name)
                        .is_some_and(|candidate| candidate == config)
                    {
                        ImportAction::Skip
                    } else {
                        ImportAction::Create
                    };
                }
            } else {
                planned.action = ImportAction::Create;
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
                    deterministic_asset_id("agent", &target, &parsed_agent.source_id);
                existing_agents
                    .contains_key(&deterministic)
                    .then_some(deterministic)
            });
        let action = resolve_agent_action(
            workspace_id,
            &target,
            &existing_agents,
            &PlannedAgent {
                source_id: parsed_agent.source_id.clone(),
                agent_id: agent_id.clone(),
                name: parsed_agent.name.clone(),
                department: parsed_agent.team_name.clone().unwrap_or_default(),
                action: ImportAction::Create,
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
                .map(|slug| managed_skill_id(&target, slug))
                .collect::<Vec<_>>(),
        )?;
        planned_agents.push(PlannedAgent {
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
                agent
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| deterministic_asset_id("agent", &target, &agent.source_id)),
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
                let deterministic = deterministic_asset_id("team", &target, &parsed_team.source_id);
                existing_teams
                    .contains_key(&deterministic)
                    .then_some(deterministic)
            });
        let action = resolve_team_action(
            workspace_id,
            &target,
            &existing_teams,
            &PlannedTeam {
                source_id: parsed_team.source_id.clone(),
                team_id: team_id.clone(),
                name: parsed_team.name.clone(),
                action: ImportAction::Create,
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
                .map(|slug| managed_skill_id(&target, slug))
                .collect::<Vec<_>>(),
            leader_agent_id.as_deref(),
            &member_agent_ids,
        )?;
        planned_teams.push(PlannedTeam {
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

    Ok(BundlePlan {
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
    })
}

#[derive(Debug, Clone)]
struct ParsedBundle {
    agents: Vec<ParsedAgent>,
    teams: Vec<ParsedTeam>,
    skills: Vec<ParsedSkillSource>,
    mcps: Vec<ParsedMcpSource>,
    avatars: Vec<ParsedAssetAvatar>,
}

fn parse_bundle_files(
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedBundle, AppError> {
    let grouped = group_top_level(files);
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
        agents,
        teams,
        skills,
        mcps,
        avatars,
    })
}

pub(crate) fn list_builtin_skill_assets() -> Result<Vec<BuiltinSkillAsset>, AppError> {
    let parsed = parse_builtin_bundle()?;

    let mut unique_skills =
        BTreeMap::<(String, String), (String, String, Vec<String>, Vec<(String, Vec<u8>)>)>::new();
    for source in parsed.skills {
        let description = source
            .files
            .iter()
            .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
            .and_then(|(_, bytes)| String::from_utf8(bytes.clone()).ok())
            .and_then(|content| parse_frontmatter(&content).ok())
            .and_then(|(frontmatter, _)| yaml_string(&frontmatter, "description"))
            .unwrap_or_default();
        unique_skills
            .entry((source.canonical_slug.clone(), source.content_hash.clone()))
            .or_insert_with(|| {
                (
                    source.name.clone(),
                    description,
                    Vec::new(),
                    source.files.clone(),
                )
            })
            .2
            .push(source.source_id.clone());
    }

    let mut assigned_hash_by_slug = BTreeMap::<String, String>::new();
    let mut assets = Vec::new();
    for ((canonical_slug, content_hash), (name, description, mut source_ids, files)) in unique_skills
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
            source_ids,
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
    let parsed = parse_builtin_bundle()?;
    let skill_id_by_source = builtin_skill_id_by_source_id()?;
    let mcp_name_by_source = parsed
        .mcps
        .iter()
        .map(|mcp| (mcp.source_id.clone(), mcp.server_name.clone()))
        .collect::<HashMap<_, _>>();

    let mut records = parsed
        .agents
        .into_iter()
        .filter(|agent| agent.team_name.is_none())
        .map(|agent| AgentRecord {
            id: catalog_hash_id("builtin-agent", &agent.source_id),
            workspace_id: workspace_id.to_string(),
            project_id: None,
            scope: "workspace".into(),
            name: agent.name.clone(),
            avatar_path: None,
            avatar: avatar_data_url(&agent.avatar),
            personality: agent.personality.clone(),
            tags: agent.tags.clone(),
            prompt: agent.prompt.clone(),
            builtin_tool_keys: agent.builtin_tool_keys.clone(),
            skill_ids: agent
                .skill_source_ids
                .iter()
                .filter_map(|source_id| skill_id_by_source.get(source_id))
                .cloned()
                .collect(),
            mcp_server_names: agent
                .mcp_source_ids
                .iter()
                .filter_map(|source_id| mcp_name_by_source.get(source_id))
                .cloned()
                .collect(),
            integration_source: Some(octopus_core::WorkspaceLinkIntegrationSource {
                kind: "builtin-template".into(),
                source_id: agent.source_id,
            }),
            description: agent.description,
            status: "active".into(),
            updated_at: 0,
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(records)
}

pub(crate) fn list_builtin_team_templates(workspace_id: &str) -> Result<Vec<TeamRecord>, AppError> {
    let parsed = parse_builtin_bundle()?;
    let skill_id_by_source = builtin_skill_id_by_source_id()?;
    let mcp_name_by_source = parsed
        .mcps
        .iter()
        .map(|mcp| (mcp.source_id.clone(), mcp.server_name.clone()))
        .collect::<HashMap<_, _>>();

    let mut records = parsed
        .teams
        .into_iter()
        .map(|team| TeamRecord {
            id: catalog_hash_id("builtin-team", &team.source_id),
            workspace_id: workspace_id.to_string(),
            project_id: None,
            scope: "workspace".into(),
            name: team.name.clone(),
            avatar_path: None,
            avatar: avatar_data_url(&team.avatar),
            personality: team.personality.clone(),
            tags: team.tags.clone(),
            prompt: team.prompt.clone(),
            builtin_tool_keys: team.builtin_tool_keys.clone(),
            skill_ids: team
                .skill_source_ids
                .iter()
                .filter_map(|source_id| skill_id_by_source.get(source_id))
                .cloned()
                .collect(),
            mcp_server_names: team
                .mcp_source_ids
                .iter()
                .filter_map(|source_id| mcp_name_by_source.get(source_id))
                .cloned()
                .collect(),
            leader_agent_id: None,
            member_agent_ids: Vec::new(),
            integration_source: Some(octopus_core::WorkspaceLinkIntegrationSource {
                kind: "builtin-template".into(),
                source_id: team.source_id,
            }),
            description: team.description,
            status: "active".into(),
            updated_at: 0,
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(records)
}

pub(crate) fn extract_builtin_agent_template_files(
    agent_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    let parsed = parse_builtin_bundle()?;
    let Some(agent) = parsed
        .agents
        .iter()
        .find(|agent| {
            agent.team_name.is_none() && catalog_hash_id("builtin-agent", &agent.source_id) == agent_id
        })
    else {
        return Ok(None);
    };
    Ok(Some(encode_builtin_bundle_entries(&agent.source_id)?))
}

pub(crate) fn extract_builtin_team_template_files(
    team_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    let parsed = parse_builtin_bundle()?;
    let Some(team) = parsed
        .teams
        .iter()
        .find(|team| catalog_hash_id("builtin-team", &team.source_id) == team_id)
    else {
        return Ok(None);
    };
    Ok(Some(encode_builtin_bundle_entries(&team.source_id)?))
}

pub(crate) fn list_builtin_mcp_assets() -> Result<Vec<BuiltinMcpAsset>, AppError> {
    let mut assets = embedded_bundle_files(&BUILTIN_MCP_ASSET_DIR)?
        .into_iter()
        .filter(|file| file.relative_path.ends_with(".json"))
        .map(|file| {
            let server_name = Path::new(&file.relative_path)
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| AppError::invalid_input("invalid builtin MCP file name"))?
                .to_string();
            let config = serde_json::from_slice::<JsonValue>(&file.bytes)?;
            if !config.is_object() {
                return Err(AppError::invalid_input(
                    "builtin MCP config must be a JSON object",
                ));
            }
            Ok(BuiltinMcpAsset {
                server_name: server_name.clone(),
                display_path: format!("builtin-assets/mcps/{}.json", server_name),
                config,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    assets.sort_by(|left, right| left.server_name.cmp(&right.server_name));
    Ok(assets)
}

pub(crate) fn find_builtin_mcp_asset(
    server_name: &str,
) -> Result<Option<BuiltinMcpAsset>, AppError> {
    Ok(list_builtin_mcp_assets()?
        .into_iter()
        .find(|asset| asset.server_name == server_name))
}

fn parse_builtin_bundle() -> Result<ParsedBundle, AppError> {
    let bundle_files = embedded_bundle_files(&BUILTIN_BUNDLE_ASSET_DIR)?;
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

fn builtin_skill_id_by_source_id() -> Result<HashMap<String, String>, AppError> {
    let mut map = HashMap::new();
    for asset in list_builtin_skill_assets()? {
        let skill_id = catalog_hash_id("skill", &asset.display_path);
        for source_id in asset.source_ids {
            map.insert(source_id, skill_id.clone());
        }
    }
    Ok(map)
}

fn avatar_data_url(avatar: &ParsedAssetAvatar) -> Option<String> {
    Some(format!(
        "data:{};base64,{}",
        avatar.content_type,
        BASE64_STANDARD.encode(&avatar.bytes)
    ))
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

fn embedded_bundle_files(dir: &Dir<'_>) -> Result<Vec<BundleFile>, AppError> {
    let mut files = Vec::new();
    collect_embedded_bundle_files(dir, "", &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
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
                    .ok_or_else(|| AppError::invalid_input("invalid builtin asset file"))?;
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

fn normalize_bundle_files(
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

fn strip_optional_bundle_root(files: Vec<BundleFile>) -> Vec<BundleFile> {
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

fn managed_skill_id(target: &AssetTargetScope<'_>, slug: &str) -> String {
    let display_path = match target {
        AssetTargetScope::Workspace => format!("data/skills/{slug}/{SKILL_FRONTMATTER_FILE}"),
        AssetTargetScope::Project(project_id) => {
            format!("data/projects/{project_id}/skills/{slug}/{SKILL_FRONTMATTER_FILE}")
        }
    };
    catalog_hash_id("skill", &display_path)
}

fn deterministic_asset_id(prefix: &str, target: &AssetTargetScope<'_>, source_id: &str) -> String {
    format!(
        "{prefix}-{}",
        short_hash(&hash_text(&format!("{}:{source_id}", target.scope_label())))
    )
}

fn issue(severity: &str, scope: &str, source_id: Option<String>, message: String) -> ImportIssue {
    ImportIssue {
        severity: severity.into(),
        scope: scope.into(),
        source_id,
        message,
    }
}

fn increment_action_counts(
    action: ImportAction,
    create: &mut u64,
    update: &mut u64,
    skip: &mut u64,
) {
    match action {
        ImportAction::Create => *create += 1,
        ImportAction::Update => *update += 1,
        ImportAction::Skip => *skip += 1,
        ImportAction::Failed => {}
    }
}

fn build_agent_record(
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
        builtin_tool_keys: builtin_tool_keys.to_vec(),
        skill_ids: skill_ids.to_vec(),
        mcp_server_names: mcp_server_names.to_vec(),
        integration_source: None,
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

fn build_team_record(
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
        builtin_tool_keys: builtin_tool_keys.to_vec(),
        skill_ids: skill_ids.to_vec(),
        mcp_server_names: mcp_server_names.to_vec(),
        leader_agent_id,
        member_agent_ids,
        integration_source: None,
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

fn resolve_agent_action(
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

fn resolve_team_action(
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

fn load_scoped_agents(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, AgentRecord>, AppError> {
    Ok(load_agents(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

fn load_scoped_teams(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, TeamRecord>, AppError> {
    Ok(load_teams(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

fn load_existing_managed_skills(
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

fn write_managed_skill(
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

fn write_agent_record(
    connection: &Connection,
    record: &AgentRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    connection
        .execute(
            &format!(
                "{verb} INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
            ),
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn load_target_runtime_document(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
) -> Result<JsonMap<String, JsonValue>, AppError> {
    read_runtime_document(&target.runtime_document_path(paths))
}

fn load_target_mcp_map(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
) -> Result<BTreeMap<String, JsonValue>, AppError> {
    extract_mcp_map(&load_target_runtime_document(paths, target)?)
}

fn load_effective_mcp_map(
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

fn plan_mcp_document_updates(
    mut document: JsonMap<String, JsonValue>,
    mcps: &[PlannedMcp],
    issues: &mut Vec<ImportIssue>,
) -> Result<JsonMap<String, JsonValue>, AppError> {
    let disabled_keys = disabled_source_keys(&document);
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

    if !disabled_keys.is_empty() {
        let tool_catalog = document
            .entry("toolCatalog")
            .or_insert_with(|| JsonValue::Object(JsonMap::new()))
            .as_object_mut()
            .ok_or_else(|| AppError::invalid_input("toolCatalog must be a JSON object"))?;
        tool_catalog.insert(
            "disabledSourceKeys".into(),
            JsonValue::Array(
                disabled_keys
                    .into_iter()
                    .map(JsonValue::String)
                    .collect::<Vec<_>>(),
            ),
        );
    }

    Ok(document)
}

fn write_target_runtime_document(
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

fn persist_avatar(
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

fn upsert_skill_import_source(
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

fn upsert_agent_import_source(
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

fn upsert_team_import_source(
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

fn load_existing_skill_import_sources(
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

fn load_existing_agent_import_sources(
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

fn load_existing_team_import_sources(
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

fn plan_to_preview(plan: &BundlePlan) -> ImportWorkspaceAgentBundlePreview {
    let mut create_count = 0_u64;
    let mut update_count = 0_u64;
    let mut skip_count = 0_u64;
    for action in plan
        .skills
        .iter()
        .map(|item| item.action)
        .chain(plan.mcps.iter().map(|item| item.action))
        .chain(plan.agents.iter().map(|item| item.action))
        .chain(plan.teams.iter().map(|item| item.action))
    {
        increment_action_counts(
            action,
            &mut create_count,
            &mut update_count,
            &mut skip_count,
        );
    }

    ImportWorkspaceAgentBundlePreview {
        departments: plan.departments.clone(),
        department_count: plan.departments.len() as u64,
        detected_agent_count: plan.detected_agent_count,
        importable_agent_count: plan.agents.len() as u64,
        detected_team_count: plan.detected_team_count,
        importable_team_count: plan.teams.len() as u64,
        create_count,
        update_count,
        skip_count,
        failure_count: plan
            .issues
            .iter()
            .filter(|item| item.severity == ISSUE_ERROR)
            .count() as u64,
        unique_skill_count: plan.skills.len() as u64,
        unique_mcp_count: plan.mcps.len() as u64,
        agent_count: plan.agents.len() as u64,
        team_count: plan.teams.len() as u64,
        skill_count: plan.skills.len() as u64,
        mcp_count: plan.mcps.len() as u64,
        avatar_count: plan.avatars.len() as u64,
        filtered_file_count: plan.filtered_file_count,
        agents: plan
            .agents
            .iter()
            .map(|agent| ImportedAgentPreviewItem {
                source_id: agent.source_id.clone(),
                agent_id: agent.agent_id.clone(),
                name: agent.name.clone(),
                department: agent.department.clone(),
                action: agent.action.as_str().into(),
                skill_slugs: agent.skill_slugs.clone(),
                mcp_server_names: agent.mcp_server_names.clone(),
            })
            .collect(),
        teams: plan
            .teams
            .iter()
            .map(|team| ImportedTeamPreviewItem {
                source_id: team.source_id.clone(),
                team_id: team.team_id.clone(),
                name: team.name.clone(),
                action: team.action.as_str().into(),
                leader_name: team.leader_name.clone(),
                member_names: team.member_names.clone(),
                agent_source_ids: team.agent_source_ids.clone(),
            })
            .collect(),
        skills: plan
            .skills
            .iter()
            .map(|skill| ImportedSkillPreviewItem {
                slug: skill.slug.clone(),
                skill_id: skill.skill_id.clone(),
                name: skill.name.clone(),
                action: skill.action.as_str().into(),
                content_hash: skill.content_hash.clone(),
                file_count: skill.file_count as u64,
                source_ids: skill.source_ids.clone(),
                departments: Vec::new(),
                agent_names: skill.consumer_names.clone(),
            })
            .collect(),
        mcps: plan
            .mcps
            .iter()
            .map(|mcp| ImportedMcpPreviewItem {
                server_name: mcp.server_name.clone(),
                action: mcp.action.as_str().into(),
                content_hash: mcp.content_hash.clone(),
                source_ids: mcp.source_ids.clone(),
                consumer_names: mcp.consumer_names.clone(),
                referenced_only: mcp.referenced_only,
            })
            .collect(),
        avatars: plan
            .avatars
            .iter()
            .map(|avatar| ImportedAvatarPreviewItem {
                source_id: avatar.source_id.clone(),
                owner_kind: avatar.owner_kind.clone(),
                owner_name: avatar.owner_name.clone(),
                file_name: avatar.file_name.clone(),
                generated: avatar.generated,
            })
            .collect(),
        issues: plan.issues.clone(),
    }
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

fn export_team_files(
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

fn export_agent_files(
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

fn build_export_context(
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

    Ok(ExportContext {
        mode: input.mode,
        root_dir_name,
        agents,
        teams,
        skill_paths,
        builtin_skill_assets,
        mcp_configs,
        avatar_payloads,
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
        if let Some(asset) = find_builtin_mcp_asset(server_name)? {
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
        crate::agent_import::ensure_import_source_tables(connection).expect("import source tables");
    }

    #[test]
    fn preview_supports_standalone_agent_root_and_yaml_arrays() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_tables(&connection);

        let preview = preview_import(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Workspace,
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

        let preview = preview_import(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Workspace,
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
        let record = AgentRecord {
            id: agent_id.into(),
            workspace_id: "ws-local".into(),
            project_id: None,
            scope: "workspace".into(),
            name: "财务分析师".into(),
            avatar_path: None,
            avatar: None,
            personality: "数字敏感".into(),
            tags: vec!["财务".into()],
            prompt: "# 角色定义\n财务专家\n".into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: Vec::new(),
            mcp_server_names: Vec::new(),
            integration_source: None,
            description: "负责财务分析".into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        };
        write_agent_record(&connection, &record, false).expect("write agent");

        let exported = export_assets(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Workspace,
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
        let project_agent = AgentRecord {
            id: project_agent_id.into(),
            workspace_id: "ws-local".into(),
            project_id: Some(project_id.into()),
            scope: "project".into(),
            name: "项目财务分析师".into(),
            avatar_path: None,
            avatar: None,
            personality: "项目财务".into(),
            tags: vec!["项目".into()],
            prompt: "# 角色定义\n项目财务分析\n".into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: vec![managed_skill_id(
                &AssetTargetScope::Project(project_id),
                skill_slug,
            )],
            mcp_server_names: Vec::new(),
            integration_source: None,
            description: "项目级技能".into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        };
        write_agent_record(&connection, &project_agent, false).expect("write project agent");

        let exported = export_assets(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Project(project_id),
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

        let builtin_skill = list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .find(|asset| asset.slug == "financial-calculator")
            .expect("financial-calculator builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);
        let builtin_mcp = find_builtin_mcp_asset("finance-data")
            .expect("builtin mcp lookup")
            .expect("finance-data builtin mcp");

        let agent_id = "agent-linked-workspace";
        let record = AgentRecord {
            id: agent_id.into(),
            workspace_id: "ws-local".into(),
            project_id: None,
            scope: "workspace".into(),
            name: "财务联动员工".into(),
            avatar_path: None,
            avatar: None,
            personality: "处理项目联动财务任务".into(),
            tags: vec!["财务".into()],
            prompt: "# 角色定义\n处理项目财务联动\n".into(),
            builtin_tool_keys: vec!["bash".into()],
            skill_ids: vec![builtin_skill_id],
            mcp_server_names: vec![builtin_mcp.server_name.clone()],
            integration_source: None,
            description: "工作区级财务员工".into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        };
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params!["ws-local", "proj-finance", agent_id, timestamp_now() as i64],
            )
            .expect("insert project agent link");

        let exported = export_assets(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Project("proj-finance"),
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
        assert!(exported.files.iter().any(|file| {
            file.relative_path == "财务联动员工/mcps/finance-data.json"
        }));
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

        let builtin_skill = list_builtin_skill_assets()
            .expect("builtin skills")
            .into_iter()
            .find(|asset| asset.slug == "financial-calculator")
            .expect("financial-calculator builtin skill");
        let builtin_skill_id = catalog_hash_id("skill", &builtin_skill.display_path);

        let agent_id = "agent-linked-roundtrip";
        let record = AgentRecord {
            id: agent_id.into(),
            workspace_id: "ws-local".into(),
            project_id: None,
            scope: "workspace".into(),
            name: "导出回导员工".into(),
            avatar_path: None,
            avatar: None,
            personality: "负责导出回导验证".into(),
            tags: vec!["回归".into()],
            prompt: "# 角色定义\n导出后重新导入\n".into(),
            builtin_tool_keys: vec!["bash".into()],
            skill_ids: vec![builtin_skill_id],
            mcp_server_names: vec!["finance-data".into()],
            integration_source: None,
            description: "验证项目导出闭包".into(),
            status: "active".into(),
            updated_at: timestamp_now(),
        };
        write_agent_record(&connection, &record, false).expect("write workspace agent");
        connection
            .execute(
                "INSERT INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params!["ws-local", "proj-export", agent_id, timestamp_now() as i64],
            )
            .expect("insert project agent link");

        let exported = export_assets(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Project("proj-export"),
            ExportWorkspaceAgentBundleInput {
                mode: "single".into(),
                agent_ids: vec![agent_id.into()],
                team_ids: Vec::new(),
            },
        )
        .expect("export");

        let imported = execute_import(
            &connection,
            &paths,
            "ws-local",
            AssetTargetScope::Project("proj-import"),
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
}
