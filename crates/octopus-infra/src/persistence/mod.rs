mod schema;

use octopus_core::AppError;
use octopus_persistence::{Database, DbError};

use crate::WorkspacePaths;

pub(crate) use schema::*;

pub(crate) fn map_db_error(error: DbError) -> AppError {
    match error {
        DbError::Io(error) => AppError::from(error),
        DbError::Sqlite(error) => AppError::database(error.to_string()),
    }
}

pub(crate) fn open_workspace_database(paths: &WorkspacePaths) -> Result<Database, AppError> {
    Database::open(&paths.db_path).map_err(map_db_error)
}
