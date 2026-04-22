use super::*;

pub(crate) fn load_workspace_runtime_document(
    paths: &WorkspacePaths,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let workspace_config_path = paths.runtime_config_dir.join("workspace.json");
    load_runtime_document(&workspace_config_path)
}

pub(crate) fn load_runtime_document(
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

pub(crate) fn validate_workspace_runtime_document(
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

pub(crate) fn write_workspace_runtime_document(
    paths: &WorkspacePaths,
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.runtime_config_dir)?;
    let rendered = serde_json::to_vec_pretty(&serde_json::Value::Object(document.clone()))?;
    fs::write(paths.runtime_config_dir.join("workspace.json"), rendered)?;
    Ok(())
}

pub(crate) fn workspace_relative_path(path: &Path, workspace_root: &Path) -> Option<String> {
    path.strip_prefix(workspace_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn catalog_hash_id(prefix: &str, value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{prefix}-{:x}", hasher.finish())
}

pub(crate) fn skill_source_key(path: &Path, workspace_root: &Path) -> String {
    format!("skill:{}", display_path(path, workspace_root))
}

pub(crate) fn load_workspace_asset_state_document(
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

pub(crate) fn save_workspace_asset_state_document(
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

pub(crate) fn workspace_asset_is_disabled(
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

pub(crate) fn set_workspace_asset_enabled(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    enabled: bool,
) {
    let metadata = document.assets.entry(source_key.to_string()).or_default();
    metadata.enabled = if enabled { None } else { Some(false) };
    normalize_workspace_asset_state_document(document);
}

pub(crate) fn set_workspace_asset_trusted(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    trusted: bool,
) {
    let metadata = document.assets.entry(source_key.to_string()).or_default();
    metadata.trusted = Some(trusted);
    normalize_workspace_asset_state_document(document);
}

pub(crate) fn remove_workspace_asset_metadata(
    document: &mut WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
) {
    document.assets.remove(source_key);
}

pub(crate) fn ensure_object_value<'a>(
    value: &'a mut serde_json::Value,
    field_name: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    value
        .as_object_mut()
        .ok_or_else(|| AppError::invalid_input(format!("{field_name} must be a JSON object")))
}

pub(crate) fn ensure_top_level_object<'a>(
    document: &'a mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AppError> {
    let value = document
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    ensure_object_value(value, key)
}
