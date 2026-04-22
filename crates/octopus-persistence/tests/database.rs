use octopus_persistence::{Database, MigrationProfile};

#[test]
fn acquire_enables_foreign_keys() {
    let temp = tempfile::tempdir().expect("tempdir");
    let database = Database::open(&temp.path().join("data/main.db")).expect("database");
    let connection = database.acquire().expect("connection");
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("pragma");
    assert_eq!(foreign_keys, 1);
}

#[test]
fn runtime_secret_profile_creates_runtime_secret_records_table() {
    let temp = tempfile::tempdir().expect("tempdir");
    let database = Database::open(&temp.path().join("data/main.db")).expect("database");

    database
        .run_migrations(MigrationProfile::RuntimeSecrets)
        .expect("migrations");

    let connection = database.acquire().expect("connection");
    let exists: String = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'runtime_secret_records'",
            [],
            |row| row.get(0),
        )
        .expect("table exists");
    assert_eq!(exists, "runtime_secret_records");
}

#[test]
fn host_notifications_profile_creates_notifications_table() {
    let temp = tempfile::tempdir().expect("tempdir");
    let database = Database::open(&temp.path().join("data/main.db")).expect("database");

    database
        .run_migrations(MigrationProfile::HostNotifications)
        .expect("migrations");

    let connection = database.acquire().expect("connection");
    let exists: String = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'notifications'",
            [],
            |row| row.get(0),
        )
        .expect("table exists");
    assert_eq!(exists, "notifications");
}
