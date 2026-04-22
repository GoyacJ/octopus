use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use octopus_core::{
    CapabilityAssetManifest, CapabilityManagementEntry, CapabilityManagementProjection,
    McpServerPackageManifest, SkillPackageManifest, WorkspaceToolCatalogEntry,
    WorkspaceToolConsumerSummary,
};
use octopus_sdk_mcp::{
    discover_mcp_server_capabilities_best_effort, mcp_endpoint as sdk_mcp_endpoint,
    parse_mcp_server_config, parse_mcp_servers, qualified_mcp_resource_name,
    DiscoveredMcpServerCapabilities, McpServerConfig,
};
use octopus_sdk_tools::{builtin_tool_catalog, BuiltinToolPermission};

use crate::{
    agent_assets::BuiltinSkillAsset,
    agent_bundle::{
        find_builtin_mcp_asset, find_builtin_skill_asset_by_id, list_builtin_agent_templates,
        list_builtin_mcp_assets, list_builtin_skill_assets, list_builtin_team_templates,
    },
};

const BUILTIN_SKILL_SOURCE_ORIGIN: &str = "builtin_bundle";
const REQUIRED_CONFIGURED_MODEL_FIELDS: &[&str] = &["providerId", "modelId", "name"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceCapabilityAssetStateDocument {
    #[serde(default)]
    pub(super) assets: BTreeMap<String, WorkspaceCapabilityAssetMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceCapabilityAssetMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) trusted: Option<bool>,
}

impl WorkspaceCapabilityAssetMetadata {
    fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.trusted.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SkillDefinitionSource {
    WorkspaceManaged,
    ProjectManaged,
    ProjectClaw,
    ProjectCodex,
    ProjectClaude,
    UserClawConfigHome,
    UserCodexHome,
    UserClaw,
    UserCodex,
    UserClaude,
}

impl SkillDefinitionSource {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::WorkspaceManaged => "workspace-managed",
            Self::ProjectManaged => "project-managed",
            Self::ProjectClaw => "project-claw",
            Self::ProjectCodex => "project-codex",
            Self::ProjectClaude => "project-claude",
            Self::UserClawConfigHome => "user-claw-config-home",
            Self::UserCodexHome => "user-codex-home",
            Self::UserClaw => "user-claw-home",
            Self::UserCodex => "user-codex-home-legacy",
            Self::UserClaude => "user-claude-home",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SkillSourceOrigin {
    SkillsDir,
    LegacyCommandsDir,
}

impl SkillSourceOrigin {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SkillsDir => "skills_dir",
            Self::LegacyCommandsDir => "legacy_commands_dir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SkillCatalogRoot {
    pub(super) source: SkillDefinitionSource,
    pub(super) path: PathBuf,
    pub(super) origin: SkillSourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SkillCatalogEntry {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) source: SkillDefinitionSource,
    pub(super) origin: SkillSourceOrigin,
    pub(super) path: PathBuf,
    pub(super) shadowed_by: Option<String>,
}

pub(super) fn normalize_required_permission(permission: BuiltinToolPermission) -> Option<String> {
    match permission {
        BuiltinToolPermission::ReadOnly => Some("readonly".into()),
        BuiltinToolPermission::WorkspaceWrite => Some("workspace-write".into()),
        BuiltinToolPermission::DangerFullAccess => Some("danger-full-access".into()),
    }
}

fn capability_asset_import_status(entry: &WorkspaceToolCatalogEntry) -> String {
    match entry.kind.as_str() {
        "builtin" => "not-importable".into(),
        "skill" => {
            if entry.workspace_owned == Some(true)
                || entry.owner_scope.as_deref() == Some("project")
            {
                "managed".into()
            } else {
                "copy-required".into()
            }
        }
        _ => {
            if entry.scope.as_deref() == Some("builtin") {
                "copy-required".into()
            } else {
                "managed".into()
            }
        }
    }
}

fn capability_asset_export_status(entry: &WorkspaceToolCatalogEntry) -> String {
    if entry.kind == "builtin" {
        "not-exportable".into()
    } else if entry.management.can_edit || entry.management.can_delete {
        "exportable".into()
    } else {
        "readonly".into()
    }
}

fn capability_asset_state(entry: &WorkspaceToolCatalogEntry) -> String {
    if entry.disabled {
        return "disabled".into();
    }
    match entry.kind.as_str() {
        "builtin" => "builtin".into(),
        "skill" => {
            if entry.shadowed_by.is_some() {
                "shadowed".into()
            } else if entry.owner_scope.as_deref() == Some("project") {
                "project".into()
            } else if entry.source_origin.as_deref() == Some(BUILTIN_SKILL_SOURCE_ORIGIN) {
                "builtin".into()
            } else if entry.workspace_owned == Some(true) {
                if entry.owner_scope.as_deref() == Some("workspace") {
                    "workspace".into()
                } else {
                    "managed".into()
                }
            } else {
                "external".into()
            }
        }
        _ => entry.scope.clone().unwrap_or_else(|| "workspace".into()),
    }
}

fn capability_asset_id(entry: &WorkspaceToolCatalogEntry) -> String {
    entry.asset_id.clone().unwrap_or_else(|| entry.id.clone())
}

fn capability_id(entry: &WorkspaceToolCatalogEntry) -> String {
    entry
        .capability_id
        .clone()
        .unwrap_or_else(|| entry.id.clone())
}

fn capability_source_kind(entry: &WorkspaceToolCatalogEntry) -> String {
    entry
        .source_kind
        .clone()
        .unwrap_or_else(|| match entry.kind.as_str() {
            "skill" => {
                if entry.source_origin.as_deref() == Some(BUILTIN_SKILL_SOURCE_ORIGIN) {
                    "bundled_skill".into()
                } else {
                    "local_skill".into()
                }
            }
            "mcp" => "mcp_tool".into(),
            _ => "builtin".into(),
        })
}

fn capability_execution_kind(entry: &WorkspaceToolCatalogEntry) -> String {
    entry
        .execution_kind
        .clone()
        .unwrap_or_else(|| match entry.kind.as_str() {
            "skill" => "prompt_skill".into(),
            "mcp" => {
                if entry.resource_uri.is_some() {
                    "resource".into()
                } else {
                    "tool".into()
                }
            }
            _ => "tool".into(),
        })
}

fn unique_sorted(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn capability_asset_name(entry: &WorkspaceToolCatalogEntry) -> String {
    if entry.kind == "mcp" {
        entry
            .server_name
            .clone()
            .unwrap_or_else(|| entry.name.clone())
    } else {
        entry.name.clone()
    }
}

fn capability_asset_description(entry: &WorkspaceToolCatalogEntry) -> String {
    if entry.kind == "mcp" {
        if entry.scope.as_deref() == Some("builtin") {
            "Builtin MCP server template.".into()
        } else {
            "Configured MCP server.".into()
        }
    } else {
        entry.description.clone()
    }
}

fn capability_asset_manifest(entries: &[&WorkspaceToolCatalogEntry]) -> CapabilityAssetManifest {
    let entry = entries
        .first()
        .copied()
        .expect("capability asset manifest requires at least one entry");
    CapabilityAssetManifest {
        asset_id: capability_asset_id(entry),
        workspace_id: entry.workspace_id.clone(),
        source_key: entry.source_key.clone(),
        kind: entry.kind.clone(),
        source_kinds: unique_sorted(entries.iter().map(|item| capability_source_kind(item))),
        execution_kinds: unique_sorted(entries.iter().map(|item| capability_execution_kind(item))),
        name: capability_asset_name(entry),
        description: capability_asset_description(entry),
        display_path: entry.display_path.clone(),
        owner_scope: entry.owner_scope.clone(),
        owner_id: entry.owner_id.clone(),
        owner_label: entry.owner_label.clone(),
        required_permission: entry.required_permission.clone(),
        management: entry.management.clone(),
        installed: true,
        enabled: !entry.disabled,
        health: entry.availability.clone(),
        state: capability_asset_state(entry),
        import_status: capability_asset_import_status(entry),
        export_status: capability_asset_export_status(entry),
    }
}

fn capability_management_entry(entry: &WorkspaceToolCatalogEntry) -> CapabilityManagementEntry {
    let asset = capability_asset_manifest(&[entry]);
    CapabilityManagementEntry {
        id: entry.id.clone(),
        asset_id: asset.asset_id.clone(),
        capability_id: capability_id(entry),
        workspace_id: entry.workspace_id.clone(),
        name: entry.name.clone(),
        kind: entry.kind.clone(),
        source_kind: capability_source_kind(entry),
        execution_kind: capability_execution_kind(entry),
        description: entry.description.clone(),
        required_permission: entry.required_permission.clone(),
        availability: entry.availability.clone(),
        source_key: entry.source_key.clone(),
        display_path: entry.display_path.clone(),
        disabled: entry.disabled,
        management: entry.management.clone(),
        builtin_key: entry.builtin_key.clone(),
        active: entry.active,
        shadowed_by: entry.shadowed_by.clone(),
        source_origin: entry.source_origin.clone(),
        workspace_owned: entry.workspace_owned,
        relative_path: entry.relative_path.clone(),
        server_name: entry.server_name.clone(),
        endpoint: entry.endpoint.clone(),
        tool_names: entry.tool_names.clone(),
        resource_uri: entry.resource_uri.clone(),
        status_detail: entry.status_detail.clone(),
        scope: entry.scope.clone(),
        owner_scope: entry.owner_scope.clone(),
        owner_id: entry.owner_id.clone(),
        owner_label: entry.owner_label.clone(),
        consumers: entry.consumers.clone(),
        installed: asset.installed,
        enabled: asset.enabled,
        health: asset.health,
        state: asset.state,
        import_status: asset.import_status,
        export_status: asset.export_status,
    }
}

fn skill_package_manifest(entry: &WorkspaceToolCatalogEntry) -> Option<SkillPackageManifest> {
    if entry.kind != "skill" {
        return None;
    }
    let asset = capability_asset_manifest(&[entry]);
    let package_kind = if entry.owner_scope.as_deref() == Some("project") {
        "project"
    } else if entry.source_origin.as_deref() == Some(BUILTIN_SKILL_SOURCE_ORIGIN) {
        "builtin"
    } else if entry.workspace_owned == Some(true) {
        "workspace"
    } else {
        "external"
    };
    Some(SkillPackageManifest {
        asset_id: asset.asset_id,
        workspace_id: asset.workspace_id,
        source_key: asset.source_key,
        kind: "skill".into(),
        source_kinds: asset.source_kinds,
        execution_kinds: asset.execution_kinds,
        name: asset.name,
        description: asset.description,
        display_path: asset.display_path,
        owner_scope: asset.owner_scope,
        owner_id: asset.owner_id,
        owner_label: asset.owner_label,
        required_permission: asset.required_permission,
        management: asset.management,
        installed: asset.installed,
        enabled: asset.enabled,
        health: asset.health,
        state: asset.state,
        import_status: asset.import_status,
        export_status: asset.export_status,
        package_kind: package_kind.into(),
        active: entry.active.unwrap_or(false),
        shadowed_by: entry.shadowed_by.clone(),
        source_origin: entry
            .source_origin
            .clone()
            .unwrap_or_else(|| "skills_dir".into()),
        workspace_owned: entry.workspace_owned.unwrap_or(false),
        relative_path: entry.relative_path.clone(),
    })
}

fn mcp_server_package_manifest(
    entries: &[&WorkspaceToolCatalogEntry],
) -> Option<McpServerPackageManifest> {
    let entry = entries.first().copied()?;
    if entry.kind != "mcp" {
        return None;
    }
    let asset = capability_asset_manifest(entries);
    let scope = entry.scope.clone().unwrap_or_else(|| "workspace".into());
    Some(McpServerPackageManifest {
        asset_id: asset.asset_id,
        workspace_id: asset.workspace_id,
        source_key: asset.source_key,
        kind: "mcp".into(),
        source_kinds: asset.source_kinds,
        execution_kinds: asset.execution_kinds,
        name: asset.name,
        description: asset.description,
        display_path: asset.display_path,
        owner_scope: asset.owner_scope,
        owner_id: asset.owner_id,
        owner_label: asset.owner_label,
        required_permission: asset.required_permission,
        management: asset.management,
        installed: asset.installed,
        enabled: asset.enabled,
        health: asset.health,
        state: asset.state,
        import_status: asset.import_status,
        export_status: asset.export_status,
        package_kind: scope.clone(),
        server_name: entry.server_name.clone().unwrap_or_default(),
        endpoint: entry.endpoint.clone().unwrap_or_default(),
        tool_names: unique_sorted(
            entries
                .iter()
                .flat_map(|item| item.tool_names.clone().unwrap_or_default()),
        ),
        prompt_names: unique_sorted(
            entries
                .iter()
                .filter(|item| capability_source_kind(item) == "mcp_prompt")
                .map(|item| item.name.clone()),
        ),
        resource_uris: unique_sorted(entries.iter().filter_map(|item| item.resource_uri.clone())),
        scope,
        status_detail: entry.status_detail.clone(),
    })
}

fn capability_management_projection(
    entries: Vec<WorkspaceToolCatalogEntry>,
) -> CapabilityManagementProjection {
    let management_entries = entries
        .iter()
        .map(capability_management_entry)
        .collect::<Vec<_>>();
    let mut grouped_entries = BTreeMap::<String, Vec<&WorkspaceToolCatalogEntry>>::new();
    for entry in &entries {
        grouped_entries
            .entry(format!(
                "{}::{}",
                capability_asset_id(entry),
                entry.source_key
            ))
            .or_default()
            .push(entry);
    }
    CapabilityManagementProjection {
        assets: grouped_entries
            .values()
            .map(|group| capability_asset_manifest(group))
            .collect(),
        skill_packages: entries.iter().filter_map(skill_package_manifest).collect(),
        mcp_server_packages: grouped_entries
            .values()
            .filter_map(|group| mcp_server_package_manifest(group))
            .collect(),
        entries: management_entries,
    }
}

pub(super) fn display_path(path: &Path, workspace_root: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(workspace_root) {
        return relative.to_string_lossy().replace('\\', "/");
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        if let Ok(relative) = path.strip_prefix(&home) {
            let suffix = relative.to_string_lossy().replace('\\', "/");
            return if suffix.is_empty() {
                "~".into()
            } else {
                format!("~/{}", suffix)
            };
        }
    }

    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn discover_skill_roots(cwd: &Path) -> Vec<SkillCatalogRoot> {
    let mut roots = Vec::new();

    push_unique_skill_root(
        &mut roots,
        SkillDefinitionSource::WorkspaceManaged,
        cwd.join("data").join("skills"),
        SkillSourceOrigin::SkillsDir,
    );

    for ancestor in cwd.ancestors() {
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaw,
            ancestor.join(".claw").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectCodex,
            ancestor.join(".codex").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaude,
            ancestor.join(".claude").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaw,
            ancestor.join(".claw").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectCodex,
            ancestor.join(".codex").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectClaude,
            ancestor.join(".claude").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Ok(claw_config_home) = env::var("CLAW_CONFIG_HOME") {
        let claw_config_home = PathBuf::from(claw_config_home);
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClawConfigHome,
            claw_config_home.join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClawConfigHome,
            claw_config_home.join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Ok(codex_home) = env::var("CODEX_HOME") {
        let codex_home = PathBuf::from(codex_home);
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodexHome,
            codex_home.join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodexHome,
            codex_home.join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaw,
            home.join(".claw").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaw,
            home.join(".claw").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodex,
            home.join(".codex").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserCodex,
            home.join(".codex").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaude,
            home.join(".claude").join("skills"),
            SkillSourceOrigin::SkillsDir,
        );
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::UserClaude,
            home.join(".claude").join("commands"),
            SkillSourceOrigin::LegacyCommandsDir,
        );
    }

    roots
}

pub(super) fn discover_catalog_skill_roots(
    paths: &WorkspacePaths,
    projects: &[ProjectRecord],
) -> Vec<SkillCatalogRoot> {
    let mut roots = discover_skill_roots(&paths.root);
    let mut sorted_projects = projects.to_vec();
    sorted_projects.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    for project in &sorted_projects {
        push_unique_skill_root(
            &mut roots,
            SkillDefinitionSource::ProjectManaged,
            paths.project_skills_root(&project.id),
            SkillSourceOrigin::SkillsDir,
        );
    }
    roots
}

pub(super) fn push_unique_skill_root(
    roots: &mut Vec<SkillCatalogRoot>,
    source: SkillDefinitionSource,
    path: PathBuf,
    origin: SkillSourceOrigin,
) {
    if path.is_dir() && !roots.iter().any(|existing| existing.path == path) {
        roots.push(SkillCatalogRoot {
            source,
            path,
            origin,
        });
    }
}

pub(super) fn load_skills_from_roots(
    roots: &[SkillCatalogRoot],
) -> Result<Vec<SkillCatalogEntry>, AppError> {
    let mut skills = Vec::new();
    let mut active_sources = BTreeMap::<String, String>::new();

    for root in roots {
        let mut root_skills = Vec::new();
        for entry in fs::read_dir(&root.path)? {
            let entry = entry?;
            match root.origin {
                SkillSourceOrigin::SkillsDir => {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let skill_path = entry.path().join("SKILL.md");
                    if !skill_path.is_file() {
                        continue;
                    }
                    let contents = fs::read_to_string(&skill_path)?;
                    let (name, description) = parse_skill_frontmatter(&contents);
                    root_skills.push(SkillCatalogEntry {
                        name: name
                            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string()),
                        description,
                        source: root.source,
                        origin: root.origin,
                        path: skill_path,
                        shadowed_by: None,
                    });
                }
                SkillSourceOrigin::LegacyCommandsDir => {
                    let path = entry.path();
                    let markdown_path = if path.is_dir() {
                        let skill_path = path.join("SKILL.md");
                        if !skill_path.is_file() {
                            continue;
                        }
                        skill_path
                    } else if path
                        .extension()
                        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("md"))
                    {
                        path
                    } else {
                        continue;
                    };

                    let contents = fs::read_to_string(&markdown_path)?;
                    let fallback_name = markdown_path.file_stem().map_or_else(
                        || entry.file_name().to_string_lossy().to_string(),
                        |stem| stem.to_string_lossy().to_string(),
                    );
                    let (name, description) = parse_skill_frontmatter(&contents);
                    root_skills.push(SkillCatalogEntry {
                        name: name.unwrap_or(fallback_name),
                        description,
                        source: root.source,
                        origin: root.origin,
                        path: markdown_path,
                        shadowed_by: None,
                    });
                }
            }
        }

        root_skills.sort_by(|left, right| left.name.cmp(&right.name));
        for mut skill in root_skills {
            let key = skill.name.to_ascii_lowercase();
            if let Some(existing) = active_sources.get(&key) {
                skill.shadowed_by = Some(existing.clone());
            } else {
                active_sources.insert(key, skill.source.key().into());
            }
            skills.push(skill);
        }
    }

    Ok(skills)
}

pub(super) fn parse_skill_frontmatter(contents: &str) -> (Option<String>, Option<String>) {
    let mut lines = contents.lines();
    if lines.next().map(str::trim) != Some("---") {
        return (None, None);
    }

    let mut name = None;
    let mut description = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            let value = unquote_frontmatter_value(value.trim());
            if !value.is_empty() {
                name = Some(value);
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("description:") {
            let value = unquote_frontmatter_value(value.trim());
            if !value.is_empty() {
                description = Some(value);
            }
        }
    }

    (name, description)
}

pub(super) fn unquote_frontmatter_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|trimmed| trimmed.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|trimmed| trimmed.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim()
        .to_string()
}

pub(super) fn load_workspace_runtime_document(
    paths: &WorkspacePaths,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    load_runtime_document(&workspace_config_path)
}

pub(super) fn load_runtime_document(
    path: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(serde_json::Map::new());
            }
            let parsed: serde_json::Value = serde_json::from_str(trimmed)?;
            parsed.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input(format!(
                    "{} must contain a top-level JSON object",
                    path.display()
                ))
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn validate_workspace_runtime_document(
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    if let Some(mcp_servers) = document.get("mcpServers") {
        mcp_servers.as_object().ok_or_else(|| {
            AppError::invalid_input("invalid runtime config: mcpServers must be a JSON object")
        })?;
    }

    if let Some(configured_models) = document.get("configuredModels") {
        let Some(configured_models) = configured_models.as_object() else {
            return Err(AppError::invalid_input(
                "invalid runtime config: configuredModels must be a JSON object",
            ));
        };
        for (configured_model_id, entry) in configured_models {
            let Some(entry_object) = entry.as_object() else {
                return Err(AppError::invalid_input(format!(
                    "invalid runtime config: configuredModels.{configured_model_id} must be a JSON object"
                )));
            };
            for field in REQUIRED_CONFIGURED_MODEL_FIELDS {
                if entry_object
                    .get(*field)
                    .and_then(serde_json::Value::as_str)
                    .is_none()
                {
                    return Err(AppError::invalid_input(format!(
                        "invalid runtime config: configuredModels.{configured_model_id}.{field} is required"
                    )));
                }
            }
        }
    }

    Ok(())
}

pub(super) fn write_workspace_runtime_document(
    paths: &WorkspacePaths,
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.runtime_config_dir)?;
    let rendered = serde_json::to_vec_pretty(&serde_json::Value::Object(document.clone()))?;
    fs::write(paths.runtime_config_dir.join("workspace.json"), rendered)?;
    Ok(())
}

pub(super) fn workspace_relative_path(path: &Path, workspace_root: &Path) -> Option<String> {
    path.strip_prefix(workspace_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

pub(super) fn catalog_hash_id(prefix: &str, value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{prefix}-{:x}", hasher.finish())
}

pub(super) fn skill_source_key(path: &Path, workspace_root: &Path) -> String {
    format!("skill:{}", display_path(path, workspace_root))
}

pub(super) fn load_workspace_asset_state_document(
    paths: &WorkspacePaths,
) -> Result<WorkspaceCapabilityAssetStateDocument, AppError> {
    match fs::read_to_string(&paths.workspace_asset_state_path) {
        Ok(raw) => {
            if raw.trim().is_empty() {
                return Ok(WorkspaceCapabilityAssetStateDocument::default());
            }
            Ok(serde_json::from_str(&raw)?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(WorkspaceCapabilityAssetStateDocument::default())
        }
        Err(error) => Err(error.into()),
    }
}

pub(super) fn save_workspace_asset_state_document(
    paths: &WorkspacePaths,
    document: &WorkspaceCapabilityAssetStateDocument,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.asset_config_dir)?;
    fs::write(
        &paths.workspace_asset_state_path,
        serde_json::to_string_pretty(document)?,
    )?;
    Ok(())
}

pub(super) fn workspace_asset_is_disabled(
    document: &WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
) -> bool {
    matches!(
        document
            .assets
            .get(source_key)
            .and_then(|metadata| metadata.enabled),
        Some(false)
    )
}

fn normalize_workspace_asset_state_document(document: &mut WorkspaceCapabilityAssetStateDocument) {
    document.assets.retain(|_, metadata| !metadata.is_empty());
}

pub(super) fn set_workspace_asset_enabled(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    enabled: bool,
) {
    let metadata = document.assets.entry(source_key.to_string()).or_default();
    metadata.enabled = if enabled { None } else { Some(false) };
    normalize_workspace_asset_state_document(document);
}

pub(super) fn set_workspace_asset_trusted(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    trusted: bool,
) {
    let metadata = document.assets.entry(source_key.to_string()).or_default();
    metadata.trusted = Some(trusted);
    normalize_workspace_asset_state_document(document);
}

pub(super) fn remove_workspace_asset_metadata(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
) {
    document.assets.remove(source_key);
}

pub(super) fn validate_skill_slug(slug: &str) -> Result<String, AppError> {
    let slug = slug.trim();
    if slug.is_empty() {
        return Err(AppError::invalid_input("skill slug is required"));
    }
    if slug.contains('/') || slug.contains('\\') || slug.contains("..") {
        return Err(AppError::invalid_input(
            "skill slug must be a safe relative directory name",
        ));
    }
    if !slug
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
    {
        return Err(AppError::invalid_input(
            "skill slug may only contain letters, numbers, hyphen, underscore, or dot",
        ));
    }
    Ok(slug.to_string())
}

pub(super) fn workspace_owned_skill_root(paths: &WorkspacePaths) -> PathBuf {
    paths.managed_skills_dir.clone()
}

pub(super) fn is_workspace_owned_skill(
    relative_path: Option<&str>,
    origin: SkillSourceOrigin,
) -> bool {
    relative_path.is_some_and(|value| value.starts_with("data/skills/"))
        && origin == SkillSourceOrigin::SkillsDir
}

pub(super) fn project_owned_skill_project_id(
    relative_path: Option<&str>,
    origin: SkillSourceOrigin,
) -> Option<String> {
    if origin != SkillSourceOrigin::SkillsDir {
        return None;
    }
    let relative_path = relative_path?;
    let mut parts = relative_path.split('/');
    match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some("data"), Some("projects"), Some(project_id), Some("skills"), Some(_)) => {
            Some(project_id.to_string())
        }
        _ => None,
    }
}

pub(super) fn skill_root_path(
    path: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<PathBuf, AppError> {
    match source_origin {
        SkillSourceOrigin::SkillsDir => path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| AppError::invalid_input("workspace skill path is invalid")),
        SkillSourceOrigin::LegacyCommandsDir => Ok(path.to_path_buf()),
    }
}

pub(super) fn build_skill_tree_node(
    path: &Path,
    root: &Path,
) -> Result<WorkspaceSkillTreeNode, AppError> {
    let metadata = fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("."));
    let relative = path
        .strip_prefix(root)
        .ok()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| name.clone());

    if metadata.is_dir() {
        let mut children = fs::read_dir(path)?
            .map(|entry| entry.map(|item| item.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort_by(|left, right| {
            let left_is_dir = left.is_dir();
            let right_is_dir = right.is_dir();
            right_is_dir
                .cmp(&left_is_dir)
                .then_with(|| left.file_name().cmp(&right.file_name()))
        });
        return Ok(WorkspaceSkillTreeNode {
            path: relative,
            name,
            kind: "directory".into(),
            children: Some(
                children
                    .iter()
                    .map(|child| build_skill_tree_node(child, root))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            byte_size: None,
            is_text: None,
        });
    }

    let bytes = fs::read(path)?;
    let is_text = std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0);
    Ok(WorkspaceSkillTreeNode {
        path: relative,
        name,
        kind: "file".into(),
        children: None,
        byte_size: Some(metadata.len()),
        is_text: Some(is_text),
    })
}

pub(super) fn build_skill_tree(
    root: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<Vec<WorkspaceSkillTreeNode>, AppError> {
    match source_origin {
        SkillSourceOrigin::SkillsDir => {
            let mut children = fs::read_dir(root)?
                .map(|entry| entry.map(|item| item.path()))
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by(|left, right| {
                let left_is_dir = left.is_dir();
                let right_is_dir = right.is_dir();
                right_is_dir
                    .cmp(&left_is_dir)
                    .then_with(|| left.file_name().cmp(&right.file_name()))
            });
            children
                .iter()
                .map(|child| build_skill_tree_node(child, root))
                .collect()
        }
        SkillSourceOrigin::LegacyCommandsDir => Ok(vec![build_skill_tree_node(root, root)?]),
    }
}

pub(super) fn collect_tree_files(
    skill_root: &Path,
    node: &WorkspaceSkillTreeNode,
    collected: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), AppError> {
    if node.kind == "directory" {
        for child in node.children.as_deref().unwrap_or(&[]) {
            collect_tree_files(skill_root, child, collected)?;
        }
        return Ok(());
    }

    let path = skill_root.join(&node.path);
    collected.push((node.path.clone(), fs::read(path)?));
    Ok(())
}

pub(super) fn content_type_for_skill_file(path: &Path, is_text: bool) -> Option<String> {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "md" => Some("text/markdown".into()),
        Some(extension) if matches!(extension.as_str(), "json") => Some("application/json".into()),
        Some(extension) if matches!(extension.as_str(), "yaml" | "yml") => {
            Some("application/yaml".into())
        }
        Some(extension)
            if matches!(
                extension.as_str(),
                "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "toml" | "txt" | "css" | "html" | "vue"
            ) =>
        {
            Some("text/plain".into())
        }
        _ if is_text => Some("text/plain".into()),
        _ => None,
    }
}

pub(super) fn language_for_skill_file(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "md" => Some("markdown".into()),
        Some(extension) if extension == "json" => Some("json".into()),
        Some(extension) if matches!(extension.as_str(), "yaml" | "yml") => Some("yaml".into()),
        Some(extension) if extension == "rs" => Some("rust".into()),
        Some(extension) if matches!(extension.as_str(), "ts" | "tsx") => Some("typescript".into()),
        Some(extension) if matches!(extension.as_str(), "js" | "jsx") => Some("javascript".into()),
        Some(extension) if extension == "py" => Some("python".into()),
        Some(extension) if extension == "vue" => Some("vue".into()),
        Some(extension) if extension == "toml" => Some("toml".into()),
        _ => None,
    }
}

pub(super) fn validate_skill_file_relative_path(relative_path: &str) -> Result<String, AppError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("skill file path is required"));
    }
    let path = Path::new(trimmed);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(AppError::invalid_input(
                    "skill file path must stay within the skill root",
                ));
            }
        }
    }
    Ok(trimmed.replace('\\', "/"))
}

pub(super) fn resolve_skill_file_path(
    skill_root: &Path,
    source_origin: SkillSourceOrigin,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    let relative_path = validate_skill_file_relative_path(relative_path)?;
    match source_origin {
        SkillSourceOrigin::SkillsDir => Ok(skill_root.join(relative_path)),
        SkillSourceOrigin::LegacyCommandsDir => {
            if relative_path
                != skill_root
                    .file_name()
                    .map(|value| value.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default()
            {
                return Err(AppError::not_found("workspace skill file"));
            }
            Ok(skill_root.to_path_buf())
        }
    }
}

pub(super) fn skill_file_document_from_path(
    workspace_root: &Path,
    skill_id: &str,
    source_key: &str,
    skill_root: &Path,
    source_origin: SkillSourceOrigin,
    path: &Path,
    readonly: bool,
) -> Result<WorkspaceSkillFileDocument, AppError> {
    let metadata = fs::metadata(path)?;
    let bytes = fs::read(path)?;
    let is_text = std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0);
    let content = if is_text {
        Some(String::from_utf8(bytes).map_err(|error| AppError::invalid_input(error.to_string()))?)
    } else {
        None
    };
    let relative_path = match source_origin {
        SkillSourceOrigin::SkillsDir => path
            .strip_prefix(skill_root)
            .ok()
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            }),
        SkillSourceOrigin::LegacyCommandsDir => path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    };

    Ok(WorkspaceSkillFileDocument {
        skill_id: skill_id.into(),
        source_key: source_key.into(),
        path: relative_path,
        display_path: display_path(path, workspace_root),
        byte_size: metadata.len(),
        is_text,
        content,
        content_type: content_type_for_skill_file(path, is_text),
        language: language_for_skill_file(path),
        readonly,
    })
}

pub(super) fn skill_tree_document_from_path(
    workspace_root: &Path,
    skill_id: &str,
    source_key: &str,
    path: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<WorkspaceSkillTreeDocument, AppError> {
    let root = skill_root_path(path, source_origin)?;
    let root_display_path = display_path(&root, workspace_root);
    Ok(WorkspaceSkillTreeDocument {
        skill_id: skill_id.into(),
        source_key: source_key.into(),
        display_path: display_path(path, workspace_root),
        root_path: root_display_path,
        tree: build_skill_tree(&root, source_origin)?,
    })
}

pub(super) fn write_skill_tree_files(
    skill_dir: &Path,
    files: &[(String, Vec<u8>)],
) -> Result<(), AppError> {
    for (relative_path, bytes) in files {
        let target_path = skill_dir.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
    }
    Ok(())
}

pub(super) fn normalize_uploaded_files(
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    if files.is_empty() {
        return Err(AppError::invalid_input("skill import files are required"));
    }
    let mut normalized = Vec::new();
    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        let bytes = BASE64_STANDARD
            .decode(&file.data_base64)
            .map_err(|error| AppError::invalid_input(format!("invalid file data: {error}")))?;
        normalized.push((relative_path, bytes));
    }

    let has_root_skill = normalized.iter().any(|(path, _)| path == "SKILL.md");
    if has_root_skill {
        return Ok(normalized);
    }

    let prefixes = normalized
        .iter()
        .filter_map(|(path, _)| path.split('/').next().map(ToOwned::to_owned))
        .collect::<std::collections::BTreeSet<_>>();
    if prefixes.len() != 1 {
        return Err(AppError::invalid_input(
            "skill import must contain SKILL.md at the root or under a single top-level directory",
        ));
    }
    let prefix = prefixes.iter().next().cloned().unwrap_or_default();
    let stripped = normalized
        .into_iter()
        .map(|(path, bytes)| {
            let stripped = path
                .strip_prefix(&format!("{prefix}/"))
                .ok_or_else(|| AppError::invalid_input("invalid imported file path"))?;
            Ok((stripped.to_string(), bytes))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if !stripped.iter().any(|(path, _)| path == "SKILL.md") {
        return Err(AppError::invalid_input(
            "imported skill must contain SKILL.md at the root",
        ));
    }
    Ok(stripped)
}

pub(super) fn extract_archive_entries(
    archive: &ImportWorkspaceSkillArchiveInput,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let bytes = BASE64_STANDARD
        .decode(&archive.archive.data_base64)
        .map_err(|error| AppError::invalid_input(format!("invalid archive data: {error}")))?;
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)
        .map_err(|error: zip::result::ZipError| AppError::invalid_input(error.to_string()))?;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|error: zip::result::ZipError| AppError::invalid_input(error.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let relative_path = validate_skill_file_relative_path(file.name())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        files.push((relative_path, bytes));
    }
    normalize_archive_entries(&files)
}

pub(super) fn normalize_archive_entries(
    files: &[(String, Vec<u8>)],
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let entries = files
        .iter()
        .map(|(path, bytes)| WorkspaceDirectoryUploadEntry {
            relative_path: path.clone(),
            file_name: Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            content_type: String::new(),
            data_base64: BASE64_STANDARD.encode(bytes),
            byte_size: bytes.len() as u64,
        })
        .collect::<Vec<_>>();
    normalize_uploaded_files(&entries)
}

pub(super) fn skill_document_from_path(
    workspace_root: &Path,
    path: &Path,
    source_origin: SkillSourceOrigin,
) -> Result<WorkspaceSkillDocument, AppError> {
    let content = fs::read_to_string(path)?;
    let (name, description) = parse_skill_frontmatter(&content);
    let display_path_value = display_path(path, workspace_root);
    let relative_path = workspace_relative_path(path, workspace_root);
    let workspace_owned = is_workspace_owned_skill(relative_path.as_deref(), source_origin);
    let root = skill_root_path(path, source_origin)?;

    Ok(WorkspaceSkillDocument {
        id: catalog_hash_id("skill", &display_path_value),
        source_key: skill_source_key(path, workspace_root),
        name: name.unwrap_or_else(|| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| display_path_value.clone())
        }),
        description: description.unwrap_or_default(),
        content,
        display_path: display_path_value,
        root_path: display_path(&root, workspace_root),
        tree: build_skill_tree(&root, source_origin)?,
        source_origin: source_origin.as_str().into(),
        workspace_owned,
        relative_path,
    })
}

fn builtin_skill_source_key(asset: &BuiltinSkillAsset) -> String {
    format!("skill:{}", asset.display_path)
}

fn builtin_skill_document(asset: &BuiltinSkillAsset) -> Result<WorkspaceSkillDocument, AppError> {
    let content = asset
        .files
        .iter()
        .find(|(path, _)| path == "SKILL.md")
        .ok_or_else(|| AppError::not_found("workspace skill"))?
        .1
        .clone();
    let content =
        String::from_utf8(content).map_err(|error| AppError::invalid_input(error.to_string()))?;

    Ok(WorkspaceSkillDocument {
        id: catalog_hash_id("skill", &asset.display_path),
        source_key: builtin_skill_source_key(asset),
        name: asset.name.clone(),
        description: asset.description.clone(),
        content,
        display_path: asset.display_path.clone(),
        root_path: asset.root_display_path.clone(),
        tree: build_embedded_skill_tree(&asset.files),
        source_origin: BUILTIN_SKILL_SOURCE_ORIGIN.into(),
        workspace_owned: false,
        relative_path: None,
    })
}

fn builtin_skill_tree_document(
    asset: &BuiltinSkillAsset,
) -> Result<WorkspaceSkillTreeDocument, AppError> {
    Ok(WorkspaceSkillTreeDocument {
        skill_id: catalog_hash_id("skill", &asset.display_path),
        source_key: builtin_skill_source_key(asset),
        display_path: asset.display_path.clone(),
        root_path: asset.root_display_path.clone(),
        tree: build_embedded_skill_tree(&asset.files),
    })
}

fn builtin_skill_file_document(
    asset: &BuiltinSkillAsset,
    relative_path: &str,
) -> Result<WorkspaceSkillFileDocument, AppError> {
    let relative_path = validate_skill_file_relative_path(relative_path)?;
    let (_, bytes) = asset
        .files
        .iter()
        .find(|(path, _)| path == &relative_path)
        .ok_or_else(|| AppError::not_found("workspace skill file"))?;
    let is_text = std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0);
    let content = if is_text {
        Some(
            String::from_utf8(bytes.clone())
                .map_err(|error| AppError::invalid_input(error.to_string()))?,
        )
    } else {
        None
    };
    let synthetic_path = Path::new(&relative_path);

    Ok(WorkspaceSkillFileDocument {
        skill_id: catalog_hash_id("skill", &asset.display_path),
        source_key: builtin_skill_source_key(asset),
        path: relative_path.clone(),
        display_path: format!("{}/{}", asset.root_display_path, relative_path),
        byte_size: bytes.len() as u64,
        is_text,
        content,
        content_type: content_type_for_skill_file(synthetic_path, is_text),
        language: language_for_skill_file(synthetic_path),
        readonly: true,
    })
}

#[derive(Default)]
struct EmbeddedSkillTreeDir {
    dirs: BTreeMap<String, EmbeddedSkillTreeDir>,
    files: BTreeMap<String, (u64, bool)>,
}

fn build_embedded_skill_tree(files: &[(String, Vec<u8>)]) -> Vec<WorkspaceSkillTreeNode> {
    let mut root = EmbeddedSkillTreeDir::default();
    for (path, bytes) in files {
        insert_embedded_skill_file(&mut root, path, bytes);
    }
    embedded_skill_children("", &root)
}

fn insert_embedded_skill_file(root: &mut EmbeddedSkillTreeDir, path: &str, bytes: &[u8]) {
    let mut parts = path.split('/').peekable();
    let mut current = root;
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            current = current.dirs.entry(part.to_string()).or_default();
        } else {
            let is_text = std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0);
            current
                .files
                .insert(part.to_string(), (bytes.len() as u64, is_text));
        }
    }
}

fn embedded_skill_children(
    parent_path: &str,
    node: &EmbeddedSkillTreeDir,
) -> Vec<WorkspaceSkillTreeNode> {
    let mut children = Vec::new();
    for (name, child) in &node.dirs {
        let path = if parent_path.is_empty() {
            name.clone()
        } else {
            format!("{parent_path}/{name}")
        };
        children.push(WorkspaceSkillTreeNode {
            path: path.clone(),
            name: name.clone(),
            kind: "directory".into(),
            children: Some(embedded_skill_children(&path, child)),
            byte_size: None,
            is_text: None,
        });
    }
    for (name, (byte_size, is_text)) in &node.files {
        let path = if parent_path.is_empty() {
            name.clone()
        } else {
            format!("{parent_path}/{name}")
        };
        children.push(WorkspaceSkillTreeNode {
            path,
            name: name.clone(),
            kind: "file".into(),
            children: None,
            byte_size: Some(*byte_size),
            is_text: Some(*is_text),
        });
    }
    children
}

pub(super) fn rewrite_skill_frontmatter_name(
    path: &Path,
    skill_name: &str,
) -> Result<(), AppError> {
    let existing = fs::read_to_string(path)?;
    let ends_with_newline = existing.ends_with('\n');
    let mut lines = existing.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.first().map(|line| line.trim()) != Some("---") {
        return Ok(());
    }

    let mut closing_index = None;
    let mut name_index = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" {
            closing_index = Some(index);
            break;
        }
        if trimmed.starts_with("name:") {
            name_index = Some(index);
        }
    }

    let Some(closing_index) = closing_index else {
        return Ok(());
    };

    let normalized_name = format!("name: {skill_name}");
    if let Some(index) = name_index {
        if lines[index] == normalized_name {
            return Ok(());
        }
        lines[index] = normalized_name;
    } else {
        lines.insert(closing_index, normalized_name);
    }

    let mut updated = lines.join("\n");
    if ends_with_newline {
        updated.push('\n');
    }
    fs::write(path, updated)?;
    Ok(())
}

pub(super) fn ensure_object_value<'a>(
    value: &'a mut serde_json::Value,
    field_name: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    value
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input(format!("{field_name} must be a JSON object")))
}

pub(super) fn ensure_top_level_object<'a>(
    document: &'a mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    let value = document
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    ensure_object_value(value, key)
}

pub(super) fn mcp_scope_label() -> &'static str {
    "workspace"
}

fn mcp_source_key(scope: &str, owner_id: Option<&str>, server_name: &str) -> String {
    match (scope, owner_id) {
        ("project", Some(project_id)) => format!("mcp:project:{project_id}:{server_name}"),
        _ => format!("mcp:{server_name}"),
    }
}

fn extract_mcp_server_configs(
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

fn mcp_endpoint_from_document(config: &serde_json::Value) -> String {
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

struct ToolConsumerMaps {
    builtin: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
    skills: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
    mcps: HashMap<String, Vec<WorkspaceToolConsumerSummary>>,
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

fn clone_non_empty_consumers(
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

fn build_tool_consumer_maps(
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

async fn discover_mcp_server_capabilities(
    servers: &BTreeMap<String, McpServerConfig>,
) -> BTreeMap<String, DiscoveredMcpServerCapabilities> {
    let mut supported = BTreeMap::new();
    let mut discovered = BTreeMap::new();

    for (server_name, config) in servers {
        if matches!(config, McpServerConfig::Stdio(_)) {
            supported.insert(server_name.clone(), config.clone());
            continue;
        }

        discovered.insert(
            server_name.clone(),
            DiscoveredMcpServerCapabilities {
                availability: "attention".into(),
                status_detail: Some(format!(
                    "transport {} is not supported by capability management discovery",
                    config.transport_label()
                )),
                ..DiscoveredMcpServerCapabilities::default()
            },
        );
    }

    discovered.extend(discover_mcp_server_capabilities_best_effort(&supported).await);
    discovered
}

#[allow(clippy::too_many_arguments)]
fn append_mcp_catalog_entries(
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

impl InfraWorkspaceService {
    pub(super) fn find_skill_catalog_entry(
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

    pub(super) fn get_workspace_skill_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        if let Some(asset) = find_builtin_skill_asset_by_id(skill_id)? {
            return builtin_skill_document(&asset);
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    pub(super) fn get_workspace_skill_tree_document(
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

    pub(super) fn get_workspace_skill_file_document(
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

    pub(super) fn ensure_workspace_owned_skill_entry(
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

    pub(super) fn import_skill_files_to_managed_root(
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

    pub(super) fn load_mcp_server_document(
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

    pub(super) fn save_workspace_runtime_document(
        &self,
        document: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AppError> {
        validate_workspace_runtime_document(&document)?;
        write_workspace_runtime_document(&self.state.paths, &document)
    }

    pub(super) async fn build_tool_catalog_entries(
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
        let project_mcp_configs = projects
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
        let project_mcp_servers = project_mcp_configs
            .iter()
            .map(|(project_id, configs)| {
                let parsed = configs
                    .iter()
                    .filter_map(|(server_name, config)| {
                        parse_mcp_server_config(config).map(|parsed| (server_name.clone(), parsed))
                    })
                    .collect::<BTreeMap<_, _>>();
                (project_id.clone(), parsed)
            })
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
        let workspace_mcp_capabilities =
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
        let builtin_mcp_capabilities = discover_mcp_server_capabilities(&builtin_mcp_servers).await;

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

        let mut project_mcp_capabilities = HashMap::new();
        for (project_id, servers) in &project_mcp_servers {
            project_mcp_capabilities.insert(
                project_id.clone(),
                discover_mcp_server_capabilities(servers).await,
            );
        }

        for project in &projects {
            let project_configs = project_mcp_configs
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

    pub(super) async fn build_capability_management_projection(
        &self,
    ) -> Result<CapabilityManagementProjection, AppError> {
        let entries = self.build_tool_catalog_entries().await?;
        Ok(capability_management_projection(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn management_capabilities() -> WorkspaceToolManagementCapabilities {
        WorkspaceToolManagementCapabilities {
            can_disable: true,
            can_edit: true,
            can_delete: true,
        }
    }

    fn workspace_mcp_entry(
        id: &str,
        name: &str,
        source_kind: &str,
        execution_kind: &str,
        tool_names: Vec<&str>,
        resource_uri: Option<&str>,
    ) -> WorkspaceToolCatalogEntry {
        serde_json::from_value(json!({
            "id": id,
            "assetId": "mcp-asset-ops",
            "capabilityId": id,
            "workspaceId": "ws-test",
            "name": name,
            "kind": "mcp",
            "sourceKind": source_kind,
            "executionKind": execution_kind,
            "description": format!("Capability for {name}"),
            "requiredPermission": null,
            "availability": "healthy",
            "sourceKey": "mcp:ops",
            "displayPath": "config/runtime/workspace.json",
            "disabled": false,
            "management": management_capabilities(),
            "builtinKey": null,
            "active": null,
            "shadowedBy": null,
            "sourceOrigin": null,
            "workspaceOwned": null,
            "relativePath": null,
            "serverName": "ops",
            "endpoint": "https://ops.example.com/mcp",
            "toolNames": tool_names,
            "resourceUri": resource_uri,
            "statusDetail": null,
            "scope": "workspace",
            "ownerScope": "workspace",
            "ownerId": "ws-test",
            "ownerLabel": "Workspace",
            "consumers": []
        }))
        .expect("mcp entry should deserialize")
    }

    #[test]
    fn capability_management_projection_groups_mcp_capabilities_by_asset() {
        let projection = capability_management_projection(vec![
            workspace_mcp_entry(
                "mcp_tool__ops__tail_logs",
                "tail_logs",
                "mcp_tool",
                "tool",
                vec!["tail_logs"],
                None,
            ),
            workspace_mcp_entry(
                "mcp_prompt__ops__deploy_review",
                "deploy_review",
                "mcp_prompt",
                "prompt_skill",
                Vec::new(),
                None,
            ),
            workspace_mcp_entry(
                "mcp_resource__ops__guide_txt",
                "Ops Guide",
                "mcp_resource",
                "resource",
                Vec::new(),
                Some("file://ops-guide.txt"),
            ),
        ]);

        let projection_json =
            serde_json::to_value(&projection).expect("projection should serialize to JSON");

        let entries = projection_json["entries"]
            .as_array()
            .expect("entries should serialize as array");
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|entry| {
            entry["sourceKind"] == "mcp_tool" && entry["executionKind"] == "tool"
        }));
        assert!(entries.iter().any(|entry| {
            entry["sourceKind"] == "mcp_prompt" && entry["executionKind"] == "prompt_skill"
        }));
        assert!(entries.iter().any(|entry| {
            entry["sourceKind"] == "mcp_resource" && entry["executionKind"] == "resource"
        }));

        let assets = projection_json["assets"]
            .as_array()
            .expect("assets should serialize as array");
        assert_eq!(
            assets.len(),
            1,
            "three MCP capabilities should collapse into one server asset"
        );
        assert_eq!(
            assets[0]["sourceKinds"],
            json!(["mcp_prompt", "mcp_resource", "mcp_tool"])
        );
        assert_eq!(
            assets[0]["executionKinds"],
            json!(["prompt_skill", "resource", "tool"])
        );

        let packages = projection_json["mcpServerPackages"]
            .as_array()
            .expect("mcp packages should serialize as array");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0]["toolNames"], json!(["tail_logs"]));
        assert_eq!(packages[0]["promptNames"], json!(["deploy_review"]));
        assert_eq!(packages[0]["resourceUris"], json!(["file://ops-guide.txt"]));
    }
}
