use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{
    capability_policy_from_sources, default_agent_asset_role, default_agent_delegation_policy,
    default_agent_memory_policy, default_agent_shared_capability_policy,
    default_approval_preference, default_artifact_handoff_policy, default_asset_import_metadata,
    default_asset_trust_metadata, default_mailbox_policy, default_model_strategy,
    default_output_contract, default_permission_envelope, default_shared_memory_policy,
    default_team_delegation_policy, default_team_memory_policy,
    default_team_shared_capability_policy, normalize_task_domains, team_topology_from_refs,
    timestamp_now, workflow_affordance_from_task_domains, AgentRecord, AppError,
    AssetBundleManifestV2, AssetDependency, AssetDependencyResolution, AssetTranslationReport,
    BundleAssetDescriptorRecord, DefaultModelStrategy, ExportWorkspaceAgentBundleInput,
    ImportIssue, TeamRecord, WorkspaceDirectoryUploadEntry, ASSET_MANIFEST_REVISION_V2,
};
use octopus_sdk_tools::builtin_tool_catalog;
use rusqlite::{params, Connection};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use sha2::{Digest, Sha256};

use crate::{
    infra_state::{
        agent_avatar, load_agents, load_bundle_asset_descriptor_records, load_projects, load_teams,
    },
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
const IGNORED_TEMPLATE_ROOTS: &[&str] = &["系统通用", "管理层与PMO"];
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
static BUILTIN_BUNDLE_ASSET_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates");

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
    pub(crate) model: Option<String>,
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
    pub(crate) leader_agent_source_id: Option<String>,
    pub(crate) member_agent_source_ids: Vec<String>,
    pub(crate) model: Option<String>,
}

#[allow(clippy::struct_field_names)]
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
    pub(crate) model: Option<String>,
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
    pub(crate) model: Option<String>,
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
    pub(crate) model: Option<String>,
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
    pub(crate) model: Option<String>,
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
    pub(crate) skill_paths: HashMap<String, PathBuf>,
    pub(crate) builtin_skill_assets: HashMap<String, BuiltinSkillAsset>,
    pub(crate) mcp_configs: HashMap<String, JsonValue>,
    pub(crate) avatar_payloads: HashMap<String, Option<(String, String, Vec<u8>)>>,
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

#[derive(Debug, Default)]
struct ParsedPackage {
    agents: Vec<ParsedAgent>,
    teams: Vec<ParsedTeam>,
    skills: Vec<ParsedSkillSource>,
    mcps: Vec<ParsedMcpSource>,
    avatars: Vec<ParsedAssetAvatar>,
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

include!("agent_assets/parse_bundle.rs");
include!("agent_assets/builtin_catalog.rs");
include!("agent_assets/normalize_and_frontmatter.rs");
include!("agent_assets/asset_state_and_ids.rs");
include!("agent_assets/records_and_actions.rs");
include!("agent_assets/runtime_docs_and_persistence.rs");
include!("agent_assets/export_files.rs");
include!("agent_assets/export_context.rs");

#[cfg(test)]
mod tests;
