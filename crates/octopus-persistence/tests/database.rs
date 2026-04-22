use octopus_core::AppError;
use octopus_persistence::{Database, Migration};
use rusqlite::Connection;
use tempfile::tempdir;

fn create_widgets_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS widgets (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL
            );",
        )
        .map_err(|error| AppError::database(error.to_string()))
}

static TEST_MIGRATIONS: &[Migration] = &[Migration {
    key: "0001-create-widgets",
    apply: create_widgets_table,
}];

#[test]
fn open_creates_parent_layout_and_runs_registered_migrations() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("data").join("main.db");

    let database = Database::open(&path)
        .expect("open")
        .with_migrations(TEST_MIGRATIONS);

    database.run_migrations().expect("migrate");

    assert!(path.exists());

    let connection = database.acquire().expect("acquire");
    let table_exists: String = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'widgets'",
            [],
            |row| row.get(0),
        )
        .expect("widgets table");
    assert_eq!(table_exists, "widgets");

    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign keys pragma");
    assert_eq!(foreign_keys, 1);
}

#[test]
fn run_migrations_is_idempotent() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("data").join("main.db");

    let database = Database::open(&path)
        .expect("open")
        .with_migrations(TEST_MIGRATIONS);

    database.run_migrations().expect("first migrate");
    database.run_migrations().expect("second migrate");

    let connection = database.acquire().expect("acquire");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM __octopus_persistence_migrations WHERE key = '0001-create-widgets'",
            [],
            |row| row.get(0),
        )
        .expect("migration count");
    assert_eq!(count, 1);
}
