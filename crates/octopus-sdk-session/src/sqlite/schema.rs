use rusqlite::Connection;

pub(crate) fn initialize(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            config_snapshot_id TEXT NOT NULL,
            effective_config_hash TEXT NOT NULL,
            head_event_id TEXT NOT NULL,
            usage_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(session_id) REFERENCES sessions(session_id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_events_session_seq
            ON events(session_id, seq);
        ",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use uuid::Uuid;

    use crate::SqliteJsonlSessionStore;

    #[test]
    fn creates_sessions_and_events_tables_on_open() {
        let root =
            std::env::temp_dir().join(format!("octopus-sdk-session-schema-{}", Uuid::new_v4()));
        let db_path = root.join("data").join("main.db");
        let jsonl_root = root.join("runtime").join("events");

        fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");
        let _store =
            SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");

        let connection = Connection::open(&db_path).expect("sqlite db should open");
        let tables = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('sessions', 'events') ORDER BY name",
            )
            .expect("query should prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("table names should load");

        assert_eq!(tables, vec!["events".to_string(), "sessions".to_string()]);
    }
}
