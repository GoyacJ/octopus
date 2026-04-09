use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SkillDefinitionSource {
    WorkspaceManaged,
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

pub(super) fn normalize_required_permission(permission: runtime::PermissionMode) -> Option<String> {
    match permission {
        runtime::PermissionMode::ReadOnly => Some("readonly".into()),
        runtime::PermissionMode::WorkspaceWrite => Some("workspace-write".into()),
        runtime::PermissionMode::DangerFullAccess => Some("danger-full-access".into()),
        runtime::PermissionMode::Prompt | runtime::PermissionMode::Allow => None,
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

pub(super) fn load_workspace_runtime_config(
    paths: &WorkspacePaths,
) -> Result<runtime::RuntimeConfig, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    let exists = workspace_config_path.exists();
    let document = if exists {
        let contents = fs::read_to_string(&workspace_config_path)?;
        if contents.trim().is_empty() {
            Some(BTreeMap::new())
        } else {
            let parsed = runtime::JsonValue::parse(&contents)
                .map_err(|error| AppError::runtime(format!("invalid runtime config: {error}")))?;
            let object = parsed.as_object().ok_or_else(|| {
                AppError::invalid_input(format!(
                    "{} must contain a top-level JSON object",
                    workspace_config_path.display()
                ))
            })?;
            Some(object.clone())
        }
    } else {
        None
    };

    runtime::ConfigLoader::new(&paths.root, &paths.runtime_config_dir)
        .load_from_documents(&[runtime::ConfigDocument {
            source: runtime::ConfigSource::Project,
            path: workspace_config_path,
            exists,
            loaded: document.is_some(),
            document,
        }])
        .map_err(|error| AppError::runtime(error.to_string()))
}

pub(super) fn load_workspace_runtime_document(
    paths: &WorkspacePaths,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    match fs::read_to_string(&workspace_config_path) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(serde_json::Map::new());
            }
            let parsed: serde_json::Value = serde_json::from_str(trimmed)?;
            parsed.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("workspace runtime config must be a JSON object")
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn validate_workspace_runtime_document(
    paths: &WorkspacePaths,
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let rendered = serde_json::to_string(&serde_json::Value::Object(document.clone()))?;
    let parsed = runtime::JsonValue::parse(&rendered)
        .map_err(|error| AppError::invalid_input(format!("invalid runtime config: {error}")))?;
    let object = parsed
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::invalid_input("workspace runtime config must be a JSON object"))?;
    runtime::ConfigLoader::new(&paths.root, &paths.runtime_config_dir)
        .load_from_documents(&[runtime::ConfigDocument {
            source: runtime::ConfigSource::Project,
            path: paths.runtime_config_dir.join("workspace.json"),
            exists: true,
            loaded: true,
            document: Some(object),
        }])
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
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

pub(super) fn disabled_source_keys(
    document: &serde_json::Map<String, serde_json::Value>,
) -> std::collections::BTreeSet<String> {
    document
        .get("toolCatalog")
        .and_then(|value| value.as_object())
        .and_then(|tool_catalog| tool_catalog.get("disabledSourceKeys"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn set_disabled_source_keys(
    document: &mut serde_json::Map<String, serde_json::Value>,
    source_keys: &std::collections::BTreeSet<String>,
) -> Result<(), AppError> {
    let tool_catalog = document
        .entry("toolCatalog")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input("toolCatalog must be a JSON object"))?;
    tool_catalog.insert(
        "disabledSourceKeys".into(),
        serde_json::Value::Array(
            source_keys
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    Ok(())
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

pub(super) fn mcp_scope_label(_scope: runtime::ConfigSource) -> &'static str {
    "workspace"
}

pub(super) fn mcp_endpoint(config: &runtime::McpServerConfig) -> String {
    match config {
        runtime::McpServerConfig::Stdio(config) => {
            if config.args.is_empty() {
                format!("stdio: {}", config.command)
            } else {
                format!("stdio: {} {}", config.command, config.args.join(" "))
            }
        }
        runtime::McpServerConfig::Sse(config) | runtime::McpServerConfig::Http(config) => {
            config.url.clone()
        }
        runtime::McpServerConfig::Ws(config) => config.url.clone(),
        runtime::McpServerConfig::Sdk(config) => format!("sdk: {}", config.name),
        runtime::McpServerConfig::ManagedProxy(config) => config.url.clone(),
    }
}

impl InfraWorkspaceService {
    pub(super) fn find_skill_catalog_entry(
        &self,
        skill_id: &str,
    ) -> Result<SkillCatalogEntry, AppError> {
        let workspace_root = self.state.paths.root.clone();
        load_skills_from_roots(&discover_skill_roots(&workspace_root))?
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
        let entry = self.find_skill_catalog_entry(skill_id)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    pub(super) fn get_workspace_skill_tree_document(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
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
        let config = document
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .and_then(|servers| servers.get(server_name))
            .cloned()
            .ok_or_else(|| AppError::not_found("workspace mcp server"))?;
        let config = config
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::invalid_input("mcp server config must be a JSON object"))?;

        Ok(WorkspaceMcpServerDocument {
            server_name: server_name.into(),
            source_key: format!("mcp:{server_name}"),
            display_path: "config/runtime/workspace.json".into(),
            scope: "workspace".into(),
            config: serde_json::Value::Object(config),
        })
    }

    pub(super) fn save_workspace_runtime_document(
        &self,
        document: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), AppError> {
        validate_workspace_runtime_document(&self.state.paths, &document)?;
        write_workspace_runtime_document(&self.state.paths, &document)
    }

    pub(super) async fn build_tool_catalog(
        &self,
    ) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let workspace_root = self.state.paths.root.clone();
        let runtime_document = load_workspace_runtime_document(&self.state.paths)?;
        let disabled_keys = disabled_source_keys(&runtime_document);
        let mut entries = Vec::new();

        for spec in tools::mvp_tool_specs() {
            let source_key = format!("builtin:{}", spec.name);
            entries.push(WorkspaceToolCatalogEntry {
                id: format!("builtin-{}", spec.name),
                workspace_id: workspace_id.clone(),
                name: spec.name.into(),
                kind: "builtin".into(),
                description: spec.description.into(),
                required_permission: normalize_required_permission(spec.required_permission),
                availability: "healthy".into(),
                source_key: source_key.clone(),
                display_path: "runtime builtin registry".into(),
                disabled: disabled_keys.contains(&source_key),
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
                status_detail: None,
                scope: None,
            });
        }

        for skill in load_skills_from_roots(&discover_skill_roots(&workspace_root))? {
            let is_active = skill.shadowed_by.is_none();
            let source_key = skill_source_key(&skill.path, &workspace_root);
            let relative_path = workspace_relative_path(&skill.path, &workspace_root);
            let workspace_owned = is_workspace_owned_skill(relative_path.as_deref(), skill.origin);
            entries.push(WorkspaceToolCatalogEntry {
                id: catalog_hash_id("skill", &display_path(&skill.path, &workspace_root)),
                workspace_id: workspace_id.clone(),
                name: skill.name.clone(),
                kind: "skill".into(),
                description: skill.description.unwrap_or_default(),
                required_permission: None,
                availability: if is_active {
                    "healthy".into()
                } else {
                    "configured".into()
                },
                source_key: source_key.clone(),
                display_path: display_path(&skill.path, &workspace_root),
                disabled: disabled_keys.contains(&source_key),
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
                status_detail: None,
                scope: None,
            });
        }

        let runtime_config = load_workspace_runtime_config(&self.state.paths)?;
        let mut manager = runtime::McpServerManager::from_runtime_config(&runtime_config);
        let discovery_report = manager.discover_tools_best_effort().await;
        let discovered_tool_names = discovery_report.tools.iter().fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut grouped, tool| {
                grouped
                    .entry(tool.server_name.clone())
                    .or_default()
                    .push(tool.qualified_name.clone());
                grouped
            },
        );
        let failed_servers = discovery_report
            .failed_servers
            .iter()
            .map(|failure| (failure.server_name.clone(), failure.error.clone()))
            .collect::<BTreeMap<_, _>>();
        let unsupported_servers = discovery_report
            .unsupported_servers
            .iter()
            .map(|server| (server.server_name.clone(), server.reason.clone()))
            .collect::<BTreeMap<_, _>>();

        for (server_name, scoped_config) in runtime_config.mcp().servers() {
            let tool_names = discovered_tool_names
                .get(server_name)
                .cloned()
                .unwrap_or_default();
            let status_detail = failed_servers
                .get(server_name)
                .cloned()
                .or_else(|| unsupported_servers.get(server_name).cloned());
            let availability = if status_detail.is_some() {
                "attention"
            } else if tool_names.is_empty() {
                "configured"
            } else {
                "healthy"
            };
            let source_key = format!("mcp:{server_name}");

            entries.push(WorkspaceToolCatalogEntry {
                id: format!("mcp-{server_name}"),
                workspace_id: workspace_id.clone(),
                name: server_name.clone(),
                kind: "mcp".into(),
                description: "Configured MCP server.".into(),
                required_permission: None,
                availability: availability.into(),
                source_key: source_key.clone(),
                display_path: "config/runtime/workspace.json".into(),
                disabled: disabled_keys.contains(&source_key),
                management: WorkspaceToolManagementCapabilities {
                    can_disable: true,
                    can_edit: true,
                    can_delete: true,
                },
                builtin_key: None,
                active: None,
                shadowed_by: None,
                source_origin: None,
                workspace_owned: None,
                relative_path: None,
                server_name: Some(server_name.clone()),
                endpoint: Some(mcp_endpoint(&scoped_config.config)),
                tool_names: Some(tool_names),
                status_detail,
                scope: Some(mcp_scope_label(scoped_config.scope).into()),
            });
        }

        entries.sort_by(|left, right| {
            left.kind.cmp(&right.kind).then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
        });

        Ok(WorkspaceToolCatalogSnapshot { entries })
    }
}
