use rusqlite::Connection;

use crate::database::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationProfile {
    RuntimeSecrets,
    HostNotifications,
}

pub(crate) fn run_migration_profile(
    connection: &Connection,
    profile: MigrationProfile,
) -> Result<(), DbError> {
    match profile {
        MigrationProfile::RuntimeSecrets => connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS runtime_secret_records (
                reference TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                ciphertext BLOB NOT NULL,
                nonce BLOB NOT NULL,
                key_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )?,
        MigrationProfile::HostNotifications => connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY NOT NULL,
                scope_kind TEXT NOT NULL,
                scope_owner_id TEXT,
                level TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                read_at INTEGER,
                toast_visible_until INTEGER,
                route_to TEXT,
                action_label TEXT
            );",
        )?,
    }

    Ok(())
}
