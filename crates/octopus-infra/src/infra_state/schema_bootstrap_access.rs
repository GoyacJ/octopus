use super::*;

pub(crate) fn apply_access_schema_batch(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS org_units (
              id TEXT PRIMARY KEY,
              parent_id TEXT,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS positions (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_groups (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_org_assignments (
              user_id TEXT NOT NULL,
              org_unit_id TEXT NOT NULL,
              is_primary INTEGER NOT NULL,
              position_ids TEXT NOT NULL,
              user_group_ids TEXT NOT NULL,
              PRIMARY KEY (user_id, org_unit_id)
            );
            CREATE TABLE IF NOT EXISTS access_roles (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_codes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS role_bindings (
              id TEXT PRIMARY KEY,
              role_id TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS data_policies (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              scope_type TEXT NOT NULL,
              project_ids TEXT NOT NULL,
              tags TEXT NOT NULL,
              classifications TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resource_policies (
              id TEXT PRIMARY KEY,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              action_name TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS menu_policies (
              menu_id TEXT PRIMARY KEY,
              enabled INTEGER NOT NULL,
              order_value INTEGER NOT NULL,
              group_key TEXT,
              visibility TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS protected_resources (
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              resource_subtype TEXT,
              project_id TEXT,
              tags TEXT NOT NULL,
              classification TEXT NOT NULL,
              owner_subject_type TEXT,
              owner_subject_id TEXT,
              PRIMARY KEY (resource_type, resource_id)
            );
            CREATE TABLE IF NOT EXISTS audit_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              actor_type TEXT NOT NULL,
              actor_id TEXT NOT NULL,
              action TEXT NOT NULL,
              resource TEXT NOT NULL,
              outcome TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS trace_events (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              session_id TEXT,
              event_kind TEXT NOT NULL,
              title TEXT NOT NULL,
              detail TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
