use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceCapabilityAssetStateDocument {
    #[serde(default)]
    pub(crate) assets: BTreeMap<String, WorkspaceCapabilityAssetMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceCapabilityAssetMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) trusted: Option<bool>,
}

impl WorkspaceCapabilityAssetMetadata {
    pub(crate) fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.trusted.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkillDefinitionSource {
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
    pub(crate) fn key(self) -> &'static str {
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
pub(crate) enum SkillSourceOrigin {
    SkillsDir,
    LegacyCommandsDir,
}

impl SkillSourceOrigin {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SkillsDir => "skills_dir",
            Self::LegacyCommandsDir => "legacy_commands_dir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCatalogRoot {
    pub(crate) source: SkillDefinitionSource,
    pub(crate) path: PathBuf,
    pub(crate) origin: SkillSourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCatalogEntry {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) source: SkillDefinitionSource,
    pub(crate) origin: SkillSourceOrigin,
    pub(crate) path: PathBuf,
    pub(crate) shadowed_by: Option<String>,
}

pub(crate) fn normalize_required_permission(permission: BuiltinToolPermission) -> Option<String> {
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

pub(crate) fn capability_management_projection(
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

pub(crate) fn display_path(path: &Path, workspace_root: &Path) -> String {
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

pub(crate) fn discover_skill_roots(cwd: &Path) -> Vec<SkillCatalogRoot> {
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

pub(crate) fn discover_catalog_skill_roots(
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

pub(crate) fn push_unique_skill_root(
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

pub(crate) fn load_skills_from_roots(
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

pub(crate) fn parse_skill_frontmatter(contents: &str) -> (Option<String>, Option<String>) {
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

pub(crate) fn unquote_frontmatter_value(value: &str) -> String {
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
