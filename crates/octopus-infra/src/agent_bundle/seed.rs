use std::fs;

use octopus_core::AppError;
use rusqlite::Connection;

use crate::WorkspacePaths;

pub(crate) use super::shared::ensure_import_source_tables;

pub(crate) fn seed_bundled_agent_bundle(
    _connection: &Connection,
    _paths: &WorkspacePaths,
    _workspace_id: &str,
) -> Result<Vec<String>, AppError> {
    Ok(Vec::new())
}

pub(crate) fn workspace_has_managed_skills(paths: &WorkspacePaths) -> Result<bool, AppError> {
    if !paths.managed_skills_dir.is_dir() {
        return Ok(false);
    }
    Ok(fs::read_dir(&paths.managed_skills_dir)?.next().is_some())
}
