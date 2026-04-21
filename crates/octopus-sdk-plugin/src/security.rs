use std::path::{Path, PathBuf};

use crate::{manifest::PluginManifest, PluginComponent, PluginError};

pub fn validate_security_gates(
    manifest_path: &Path,
    manifest: &PluginManifest,
) -> Result<(), PluginError> {
    validate_plugin_id(&manifest.id)?;
    ensure_not_world_writable(manifest_path)?;

    let root = manifest_path
        .parent()
        .ok_or_else(|| PluginError::PathNotFound {
            path: manifest_path.to_path_buf(),
        })?;

    for path in component_paths(manifest) {
        ensure_path_within_root(root, &path)?;
    }

    Ok(())
}

pub fn validate_plugin_id(id: &str) -> Result<(), PluginError> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');

    if valid {
        Ok(())
    } else {
        Err(PluginError::ManifestValidationError {
            cause: "invalid plugin id".into(),
        })
    }
}

pub fn ensure_path_within_root(root: &Path, relative_path: &Path) -> Result<(), PluginError> {
    let root = root.canonicalize().map_err(|_| PluginError::PathNotFound {
        path: root.to_path_buf(),
    })?;
    let resolved =
        root.join(relative_path)
            .canonicalize()
            .map_err(|_| PluginError::PathNotFound {
                path: root.join(relative_path),
            })?;

    if resolved.starts_with(&root) {
        Ok(())
    } else {
        Err(PluginError::PathEscape { path: resolved })
    }
}

pub fn ensure_not_world_writable(path: &Path) -> Result<(), PluginError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = path
            .metadata()
            .map_err(|_| PluginError::PathNotFound { path: path.into() })?;
        if metadata.permissions().mode() & 0o002 != 0 {
            return Err(PluginError::WorldWritable { path: path.into() });
        }
    }

    Ok(())
}

fn component_paths(manifest: &PluginManifest) -> Vec<PathBuf> {
    manifest
        .components
        .iter()
        .filter_map(|component| match component {
            PluginComponent::Command(decl) => Some(decl.path.clone()),
            PluginComponent::Agent(decl) => Some(decl.manifest_path.clone()),
            PluginComponent::OutputStyle(decl) => Some(decl.template_path.clone()),
            PluginComponent::McpServer(decl) => Some(decl.manifest_path.clone()),
            PluginComponent::ContextEngine(decl) => Some(decl.entrypoint.clone()),
            PluginComponent::MemoryBackend(decl) => Some(decl.entrypoint.clone()),
            _ => None,
        })
        .collect()
}
