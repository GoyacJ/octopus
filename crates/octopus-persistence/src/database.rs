use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use thiserror::Error;

use crate::migrations::{run_migration_profile, MigrationProfile};

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn acquire(&self) -> Result<Connection, DbError> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    pub fn run_migrations(&self, profile: MigrationProfile) -> Result<(), DbError> {
        let connection = self.acquire()?;
        run_migration_profile(&connection, profile)?;
        Ok(())
    }
}
