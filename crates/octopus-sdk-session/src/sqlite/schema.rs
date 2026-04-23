use rusqlite::Connection;

pub(crate) const SESSIONS_TABLE: &str = "runtime_session_store_sessions";
pub(crate) const EVENTS_TABLE: &str = "runtime_session_store_events";
const EVENTS_SESSION_SEQ_INDEX: &str = "idx_runtime_session_store_events_session_seq";
const LEGACY_SESSIONS_TABLE: &str = "sessions";
const LEGACY_EVENTS_TABLE: &str = "events";
const DEFAULT_PLUGINS_SNAPSHOT_JSON: &str = r#"{"api_version":"","plugins":[]}"#;

pub(crate) fn initialize(connection: &Connection) -> Result<(), rusqlite::Error> {
    migrate_legacy_tables(connection)?;

    connection.execute_batch(&format!(
        "
        CREATE TABLE IF NOT EXISTS {sessions_table} (
            session_id TEXT PRIMARY KEY,
            working_dir TEXT NOT NULL DEFAULT '.',
            permission_mode TEXT NOT NULL DEFAULT 'default',
            model TEXT NOT NULL DEFAULT 'main',
            config_snapshot_id TEXT NOT NULL,
            effective_config_hash TEXT NOT NULL,
            token_budget INTEGER NOT NULL DEFAULT 8192,
            plugins_snapshot_json TEXT NOT NULL DEFAULT '{default_plugins_snapshot_json}',
            head_event_id TEXT NOT NULL,
            usage_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS {events_table} (
            event_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(session_id) REFERENCES {sessions_table}(session_id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS {events_session_seq_index}
            ON {events_table}(session_id, seq);
        ",
        sessions_table = SESSIONS_TABLE,
        events_table = EVENTS_TABLE,
        events_session_seq_index = EVENTS_SESSION_SEQ_INDEX,
        default_plugins_snapshot_json = DEFAULT_PLUGINS_SNAPSHOT_JSON,
    ))?;

    if !has_column(connection, SESSIONS_TABLE, "plugins_snapshot_json")? {
        connection.execute(
            &format!(
                "ALTER TABLE {SESSIONS_TABLE} ADD COLUMN plugins_snapshot_json TEXT NOT NULL DEFAULT '{DEFAULT_PLUGINS_SNAPSHOT_JSON}'"
            ),
            [],
        )?;
    }
    if !has_column(connection, SESSIONS_TABLE, "working_dir")? {
        connection.execute(
            &format!(
                "ALTER TABLE {SESSIONS_TABLE} ADD COLUMN working_dir TEXT NOT NULL DEFAULT '.'"
            ),
            [],
        )?;
    }
    if !has_column(connection, SESSIONS_TABLE, "permission_mode")? {
        connection.execute(
            &format!(
                "ALTER TABLE {SESSIONS_TABLE} ADD COLUMN permission_mode TEXT NOT NULL DEFAULT 'default'"
            ),
            [],
        )?;
    }
    if !has_column(connection, SESSIONS_TABLE, "model")? {
        connection.execute(
            &format!("ALTER TABLE {SESSIONS_TABLE} ADD COLUMN model TEXT NOT NULL DEFAULT 'main'"),
            [],
        )?;
    }
    if !has_column(connection, SESSIONS_TABLE, "token_budget")? {
        connection.execute(
            &format!("ALTER TABLE {SESSIONS_TABLE} ADD COLUMN token_budget INTEGER NOT NULL DEFAULT 8192"),
            [],
        )?;
    }

    Ok(())
}

fn migrate_legacy_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    let renamed_sessions = if table_exists(connection, LEGACY_SESSIONS_TABLE)?
        && !table_exists(connection, SESSIONS_TABLE)?
        && has_column(connection, LEGACY_SESSIONS_TABLE, "session_id")?
    {
        connection.execute(
            &format!("ALTER TABLE {LEGACY_SESSIONS_TABLE} RENAME TO {SESSIONS_TABLE}"),
            [],
        )?;
        true
    } else {
        false
    };

    if renamed_sessions
        && table_exists(connection, LEGACY_EVENTS_TABLE)?
        && !table_exists(connection, EVENTS_TABLE)?
        && has_column(connection, LEGACY_EVENTS_TABLE, "session_id")?
        && has_column(connection, LEGACY_EVENTS_TABLE, "seq")?
        && has_column(connection, LEGACY_EVENTS_TABLE, "payload")?
    {
        connection.execute(
            &format!("ALTER TABLE {LEGACY_EVENTS_TABLE} RENAME TO {EVENTS_TABLE}"),
            [],
        )?;
    }

    Ok(())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, rusqlite::Error> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .map(|()| true)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            other => Err(other),
        })
}

fn has_column(connection: &Connection, table: &str, column: &str) -> Result<bool, rusqlite::Error> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;

    for value in rows {
        if value? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::{params, Connection};
    use uuid::Uuid;

    use super::{EVENTS_TABLE, SESSIONS_TABLE};
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
                &format!(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('{sessions_table}', '{events_table}') ORDER BY name",
                    sessions_table = SESSIONS_TABLE,
                    events_table = EVENTS_TABLE,
                ),
            )
            .expect("query should prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("table names should load");

        assert_eq!(
            tables,
            vec![EVENTS_TABLE.to_string(), SESSIONS_TABLE.to_string()]
        );
    }

    #[test]
    fn leaves_auth_sessions_table_untouched_when_opening_store() {
        let root =
            std::env::temp_dir().join(format!("octopus-sdk-session-auth-{}", Uuid::new_v4()));
        let db_path = root.join("data").join("main.db");
        let jsonl_root = root.join("runtime").join("events");

        fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");
        let connection = Connection::open(&db_path).expect("sqlite db should open");
        connection
            .execute_batch(
                "
                CREATE TABLE sessions (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    user_id TEXT NOT NULL,
                    client_app_id TEXT NOT NULL,
                    token TEXT NOT NULL UNIQUE,
                    status TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    expires_at INTEGER
                );
                ",
            )
            .expect("auth sessions table should create");
        drop(connection);

        let _store =
            SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");

        let connection = Connection::open(&db_path).expect("sqlite db should reopen");
        let auth_columns = load_columns(&connection, "sessions");
        let runtime_tables = connection
            .prepare(
                &format!(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('{sessions_table}', '{events_table}') ORDER BY name",
                    sessions_table = SESSIONS_TABLE,
                    events_table = EVENTS_TABLE,
                ),
            )
            .expect("query should prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("table names should load");

        assert_eq!(
            auth_columns,
            vec![
                "id".to_string(),
                "workspace_id".to_string(),
                "user_id".to_string(),
                "client_app_id".to_string(),
                "token".to_string(),
                "status".to_string(),
                "created_at".to_string(),
                "expires_at".to_string(),
            ]
        );
        assert_eq!(
            runtime_tables,
            vec![EVENTS_TABLE.to_string(), SESSIONS_TABLE.to_string()]
        );
    }

    #[test]
    fn migrates_legacy_runtime_tables_to_namespaced_tables() {
        let root =
            std::env::temp_dir().join(format!("octopus-sdk-session-legacy-{}", Uuid::new_v4()));
        let db_path = root.join("data").join("main.db");
        let jsonl_root = root.join("runtime").join("events");

        fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");
        let connection = Connection::open(&db_path).expect("sqlite db should open");
        connection
            .execute_batch(
                "
                CREATE TABLE sessions (
                    session_id TEXT PRIMARY KEY,
                    working_dir TEXT NOT NULL,
                    permission_mode TEXT NOT NULL,
                    model TEXT NOT NULL,
                    config_snapshot_id TEXT NOT NULL,
                    effective_config_hash TEXT NOT NULL,
                    token_budget INTEGER NOT NULL,
                    plugins_snapshot_json TEXT NOT NULL,
                    head_event_id TEXT NOT NULL,
                    usage_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );

                CREATE TABLE events (
                    event_id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    kind TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                ",
            )
            .expect("legacy runtime tables should create");
        connection
            .execute(
                "
                INSERT INTO sessions (
                    session_id,
                    working_dir,
                    permission_mode,
                    model,
                    config_snapshot_id,
                    effective_config_hash,
                    token_budget,
                    plugins_snapshot_json,
                    head_event_id,
                    usage_json,
                    created_at,
                    updated_at
                )
                VALUES (?1, '.', 'default', 'main', 'cfg-1', 'hash-1', 8192, '{\"api_version\":\"\",\"plugins\":[]}', 'evt-1', '{\"input_tokens\":0,\"output_tokens\":0,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}', 1, 1)
                ",
                ["session-1"],
            )
            .expect("legacy session row should insert");
        connection
            .execute(
                "
                INSERT INTO events (event_id, session_id, seq, kind, payload, created_at)
                VALUES (?1, ?2, 1, 'session_started', '{\"SessionStarted\":{\"working_dir\":\".\",\"permission_mode\":\"Default\",\"model\":\"main\",\"config_snapshot_id\":\"cfg-1\",\"effective_config_hash\":\"hash-1\",\"token_budget\":8192,\"plugins_snapshot\":null}}', 1)
                ",
                params!["evt-1", "session-1"],
            )
            .expect("legacy event row should insert");
        drop(connection);

        let _store =
            SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");

        let connection = Connection::open(&db_path).expect("sqlite db should reopen");
        let runtime_tables = connection
            .prepare(
                &format!(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('{sessions_table}', '{events_table}') ORDER BY name",
                    sessions_table = SESSIONS_TABLE,
                    events_table = EVENTS_TABLE,
                ),
            )
            .expect("query should prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("table names should load");
        let legacy_tables = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('sessions', 'events') ORDER BY name",
            )
            .expect("legacy query should prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("legacy query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("legacy table names should load");
        let runtime_session_rows: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {SESSIONS_TABLE}"),
                [],
                |row| row.get(0),
            )
            .expect("runtime session rows should query");
        let runtime_event_rows: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {EVENTS_TABLE}"), [], |row| {
                row.get(0)
            })
            .expect("runtime event rows should query");

        assert_eq!(
            runtime_tables,
            vec![EVENTS_TABLE.to_string(), SESSIONS_TABLE.to_string()]
        );
        assert!(legacy_tables.is_empty());
        assert_eq!(runtime_session_rows, 1);
        assert_eq!(runtime_event_rows, 1);
    }

    fn load_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("pragma should prepare");
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("pragma should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("columns should load")
    }
}
