use super::*;

pub(crate) fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<Vec<String>, AppError> {
    let mut stmt = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(columns)
}

pub(crate) fn ensure_columns(
    connection: &Connection,
    table_name: &str,
    definitions: &[(&str, &str)],
) -> Result<(), AppError> {
    let columns = table_columns(connection, table_name)?;

    for (name, definition) in definitions {
        if columns.iter().any(|column| column == name) {
            continue;
        }

        connection
            .execute(
                &format!("ALTER TABLE {table_name} ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(crate) fn ensure_user_avatar_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "users",
        &[
            ("avatar_path", "TEXT"),
            ("avatar_content_type", "TEXT"),
            ("avatar_byte_size", "INTEGER"),
            ("avatar_content_hash", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_project_task_run_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "project_task_runs",
        &[("pending_approval_id", "TEXT")],
    )
}

pub(crate) fn json_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(AppError::from)
}

fn merge_json_with_defaults(
    base: serde_json::Value,
    patch: serde_json::Value,
) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let merged = merge_json_with_defaults(
                    base_map.remove(&key).unwrap_or(serde_json::Value::Null),
                    patch_value,
                );
                base_map.insert(key, merged);
            }
            serde_json::Value::Object(base_map)
        }
        (base, serde_json::Value::Null) => base,
        (_, patch) => patch,
    }
}

pub(crate) fn parse_json_or_default<T, F>(raw: &str, default: F) -> T
where
    T: serde::de::DeserializeOwned + Serialize,
    F: FnOnce() -> T,
{
    let default_value = default();
    let merged = serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|patch| {
            serde_json::to_value(&default_value)
                .ok()
                .map(|base| merge_json_with_defaults(base, patch))
        })
        .unwrap_or(serde_json::Value::Null);
    serde_json::from_value(merged).unwrap_or(default_value)
}

pub(crate) fn drop_legacy_access_control_tables(connection: &Connection) -> Result<(), AppError> {
    for table in ["memberships", "roles", "permissions", "menus"] {
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }
    Ok(())
}

pub(crate) fn reset_legacy_sessions_table(connection: &Connection) -> Result<(), AppError> {
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sessions' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .is_some();
    if !table_exists {
        return Ok(());
    }

    let expected_columns = [
        "id",
        "workspace_id",
        "user_id",
        "client_app_id",
        "token",
        "status",
        "created_at",
        "expires_at",
    ];
    let mut pragma = connection
        .prepare("PRAGMA table_info(sessions)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = pragma
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if columns != expected_columns {
        connection
            .execute("DROP TABLE sessions", [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}
