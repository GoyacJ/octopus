use octopus_core::AppError;
use octopus_persistence::{Database, Migration};
use rusqlite::Connection;

use crate::{infra_state::apply_workspace_schema, workspace_paths::WorkspacePaths};

static WORKSPACE_MIGRATIONS: &[Migration] = &[Migration {
    key: "0001-workspace-schema",
    apply: apply_workspace_schema,
}];

pub(crate) fn workspace_database(paths: &WorkspacePaths) -> Result<Database, AppError> {
    Ok(paths.database()?.with_migrations(WORKSPACE_MIGRATIONS))
}

pub(crate) fn open_connection(paths: &WorkspacePaths) -> Result<Connection, AppError> {
    workspace_database(paths)?.acquire()
}
