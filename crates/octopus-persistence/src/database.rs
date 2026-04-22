use std::{
    fs,
    path::{Path, PathBuf},
};

use octopus_core::AppError;
use rusqlite::{params, Connection};

use crate::migrations::Migration;

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
    migrations: &'static [Migration],
}

impl Database {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, AppError> {
        let path = path.into();
        ensure_parent_dir(&path)?;
        Ok(Self {
            path,
            migrations: &[],
        })
    }

    #[must_use]
    pub fn with_migrations(mut self, migrations: &'static [Migration]) -> Self {
        self.migrations = migrations;
        self
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn acquire(&self) -> Result<Connection, AppError> {
        Connection::open(&self.path).map_err(|error| AppError::database(error.to_string()))
    }

    pub fn run_migrations(&self) -> Result<(), AppError> {
        let connection = self.acquire()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS __octopus_persistence_migrations (
                    key TEXT PRIMARY KEY NOT NULL,
                    applied_at INTEGER NOT NULL
                );",
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        for migration in self.migrations {
            let already_applied = connection
                .query_row(
                    "SELECT 1 FROM __octopus_persistence_migrations WHERE key = ?1",
                    params![migration.key],
                    |row| row.get::<_, i64>(0),
                )
                .map(|_| true)
                .or_else(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => Ok(false),
                    other => Err(other),
                })
                .map_err(|error| AppError::database(error.to_string()))?;
            if already_applied {
                continue;
            }

            (migration.apply)(&connection)?;
            connection
                .execute(
                    "INSERT INTO __octopus_persistence_migrations (key, applied_at)
                     VALUES (?1, unixepoch())",
                    params![migration.key],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        Ok(())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
