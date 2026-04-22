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
        "image/svg+xml" => "svg",
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
        Some("svg") => Some("image/svg+xml"),
        _ => None,
    }
}

