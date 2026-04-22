use super::*;

pub(crate) fn apply_workspace_schema(connection: &Connection) -> Result<(), AppError> {
    drop_legacy_access_control_tables(connection)?;
    reset_legacy_sessions_table(connection)?;
    apply_core_schema_batch(connection)?;
    apply_access_schema_batch(connection)?;
    apply_runtime_schema_batch(connection)?;
    ensure_user_avatar_columns(connection)?;
    ensure_agent_record_columns(connection)?;
    ensure_pet_agent_extension_columns(connection)?;
    ensure_pet_projection_columns(connection)?;
    ensure_team_record_columns(connection)?;
    ensure_bundle_asset_descriptor_columns(connection)?;
    ensure_project_assignment_columns(connection)?;
    ensure_project_promotion_request_table(connection)?;
    ensure_project_deletion_request_table(connection)?;
    ensure_project_agent_link_table(connection)?;
    ensure_project_team_link_table(connection)?;
    ensure_project_task_run_columns(connection)?;
    ensure_runtime_config_snapshot_columns(connection)?;
    ensure_runtime_session_projection_columns(connection)?;
    ensure_runtime_run_projection_columns(connection)?;
    ensure_runtime_phase_four_projection_tables(connection)?;
    ensure_runtime_memory_projection_tables(connection)?;
    ensure_cost_entry_columns(connection)?;
    ensure_resource_columns(connection)?;
    ensure_knowledge_columns(connection)?;
    agent_seed::ensure_import_source_tables(connection)?;
    Ok(())
}

pub(crate) fn initialize_database(paths: &WorkspacePaths) -> Result<(), AppError> {
    crate::persistence::workspace_database(paths)?.run_migrations()
}

pub(crate) fn seed_defaults(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection = crate::persistence::open_connection(paths)?;

    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_exists.is_none() {
        let default_project_resource_directory =
            paths.default_project_resource_directory(DEFAULT_PROJECT_ID);
        let default_project_assignments = serde_json::to_string(&default_project_assignments())?;
        let default_permission_overrides =
            serde_json::to_string(&default_project_permission_overrides())?;
        let default_linked_assets =
            serde_json::to_string(&empty_project_linked_workspace_assets())?;
        let default_member_user_ids = serde_json::to_string(&vec!["user-owner".to_string()])?;
        connection
            .execute(
                "INSERT INTO projects
                 (id, workspace_id, name, status, description, resource_directory, leader_agent_id, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
                    default_project_resource_directory,
                    Option::<String>::None,
                    Some(default_project_assignments),
                    "user-owner",
                    default_member_user_ids,
                    default_permission_overrides,
                    default_linked_assets,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    let resources_exist: Option<String> = connection
        .query_row("SELECT id FROM resources LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if resources_exist.is_none() {
        for record in default_workspace_resources() {
            connection
                .execute(
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.kind,
                        record.name,
                        record.location,
                        record.origin,
                        record.scope,
                        record.visibility,
                        record.owner_user_id,
                        record.storage_path,
                        record.content_type,
                        record.byte_size.map(|value| value as i64),
                        record.preview_kind,
                        record.status,
                        record.updated_at as i64,
                        serde_json::to_string(&record.tags)?,
                        record.source_artifact_id,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        fs::create_dir_all(&paths.workspace_resources_dir)?;
        fs::write(
            paths.workspace_resources_dir.join("workspace-handbook.md"),
            "# Workspace Handbook\n\nShared operating rules for this workspace.\n",
        )?;
        fs::create_dir_all(
            paths
                .project_resources_dir(DEFAULT_PROJECT_ID)
                .join("delivery-board"),
        )?;
    }

    let knowledge_exists: Option<String> = connection
        .query_row("SELECT id FROM knowledge_records LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if knowledge_exists.is_none() {
        for record in default_knowledge_records() {
            connection
                .execute(
                    "INSERT INTO knowledge_records (id, workspace_id, project_id, title, summary, kind, scope, status, visibility, owner_user_id, source_type, source_ref, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.summary,
                        record.kind,
                        record.scope,
                        record.status,
                        record.visibility,
                        record.owner_user_id,
                        record.source_type,
                        record.source_ref,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let models_exist: Option<String> = connection
        .query_row("SELECT id FROM model_catalog LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if models_exist.is_none() {
        for record in default_model_catalog() {
            connection
                .execute(
                    "INSERT INTO model_catalog (id, workspace_id, label, provider, description, recommended_for, availability, default_permission)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.label,
                        record.provider,
                        record.description,
                        record.recommended_for,
                        record.availability,
                        record.default_permission,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let provider_credentials_exist: Option<String> = connection
        .query_row("SELECT id FROM provider_credentials LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if provider_credentials_exist.is_none() {
        for record in default_provider_credentials() {
            connection
                .execute(
                    "INSERT INTO provider_credentials (id, workspace_id, provider, name, base_url, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.provider,
                        record.name,
                        record.base_url,
                        record.status,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let tools_exist: Option<String> = connection
        .query_row("SELECT id FROM tools LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if tools_exist.is_none() {
        for record in default_tool_records() {
            connection
                .execute(
                    "INSERT INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.kind,
                        record.name,
                        record.description,
                        record.status,
                        record.permission_mode,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    connection
        .execute(
            "INSERT OR IGNORE INTO org_units (id, parent_id, code, name, status)
             VALUES ('org-root', NULL, 'root', 'Root Organization', 'active')",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    for app in default_client_apps() {
        connection
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    app.id,
                    app.name,
                    app.platform,
                    app.status,
                    if app.first_party { 1 } else { 0 },
                    serde_json::to_string(&app.allowed_origins)?,
                    serde_json::to_string(&app.allowed_hosts)?,
                    app.session_policy,
                    serde_json::to_string(&app.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}
