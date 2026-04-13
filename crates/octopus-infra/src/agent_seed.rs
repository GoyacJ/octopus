use octopus_core::AppError;
use rusqlite::Connection;

use crate::WorkspacePaths;

pub(crate) fn ensure_import_source_tables(connection: &Connection) -> Result<(), AppError> {
    crate::agent_bundle::seed::ensure_import_source_tables(connection)
}

pub(crate) fn seed_bundled_agent_bundle(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
) -> Result<Vec<String>, AppError> {
    crate::agent_bundle::seed::seed_bundled_agent_bundle(connection, paths, workspace_id)
}

pub(crate) fn workspace_has_managed_skills(paths: &WorkspacePaths) -> Result<bool, AppError> {
    crate::agent_bundle::seed::workspace_has_managed_skills(paths)
}
