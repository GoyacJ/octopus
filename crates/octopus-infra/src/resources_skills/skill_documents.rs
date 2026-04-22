use super::*;

pub(crate) fn validate_skill_slug(slug: &str) -> Result<String, AppError> {
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

pub(crate) fn workspace_owned_skill_root(paths: &WorkspacePaths) -> PathBuf {
    paths.managed_skills_dir.clone()
}

pub(crate) fn is_workspace_owned_skill(
    relative_path: Option<&str>,
    origin: SkillSourceOrigin,
) -> bool {
    relative_path.is_some_and(|value| value.starts_with("data/skills/"))
        && origin == SkillSourceOrigin::SkillsDir
}

pub(crate) fn project_owned_skill_project_id(
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

pub(crate) fn skill_root_path(
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

pub(crate) fn build_skill_tree_node(
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

pub(crate) fn build_skill_tree(
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

pub(crate) fn collect_tree_files(
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

pub(crate) fn content_type_for_skill_file(path: &Path, is_text: bool) -> Option<String> {
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

pub(crate) fn language_for_skill_file(path: &Path) -> Option<String> {
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

pub(crate) fn validate_skill_file_relative_path(relative_path: &str) -> Result<String, AppError> {
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

pub(crate) fn resolve_skill_file_path(
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

pub(crate) fn skill_file_document_from_path(
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

pub(crate) fn skill_tree_document_from_path(
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

pub(crate) fn write_skill_tree_files(
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

pub(crate) fn normalize_uploaded_files(
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

pub(crate) fn extract_archive_entries(
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

pub(crate) fn normalize_archive_entries(
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

pub(crate) fn skill_document_from_path(
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

pub(crate) fn builtin_skill_source_key(asset: &BuiltinSkillAsset) -> String {
    format!("skill:{}", asset.display_path)
}

pub(crate) fn builtin_skill_document(
    asset: &BuiltinSkillAsset,
) -> Result<WorkspaceSkillDocument, AppError> {
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

pub(crate) fn builtin_skill_tree_document(
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

pub(crate) fn builtin_skill_file_document(
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

pub(crate) fn rewrite_skill_frontmatter_name(
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
