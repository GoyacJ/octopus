use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceConfigFile {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) slug: String,
    pub(super) deployment: String,
    pub(super) bootstrap_status: String,
    pub(super) owner_user_id: Option<String>,
    pub(super) host: String,
    pub(super) listen_address: String,
    pub(super) default_project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AppRegistryFile {
    pub(super) apps: Vec<ClientAppRecord>,
}

#[derive(Debug, Clone)]
pub(super) struct StoredUser {
    pub(super) record: UserRecord,
    pub(super) password_hash: String,
    pub(super) membership: WorkspaceMembershipRecord,
}

#[derive(Debug)]
pub(super) struct InfraState {
    pub(super) paths: WorkspacePaths,
    pub(super) workspace: Mutex<WorkspaceSummary>,
    pub(super) users: Mutex<Vec<StoredUser>>,
    pub(super) apps: Mutex<Vec<ClientAppRecord>>,
    pub(super) sessions: Mutex<Vec<SessionRecord>>,
    pub(super) projects: Mutex<Vec<ProjectRecord>>,
    pub(super) resources: Mutex<Vec<WorkspaceResourceRecord>>,
    pub(super) knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    pub(super) agents: Mutex<Vec<AgentRecord>>,
    pub(super) project_agent_links: Mutex<Vec<ProjectAgentLinkRecord>>,
    pub(super) teams: Mutex<Vec<TeamRecord>>,
    pub(super) project_team_links: Mutex<Vec<ProjectTeamLinkRecord>>,
    pub(super) model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    pub(super) provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    pub(super) tools: Mutex<Vec<ToolRecord>>,
    pub(super) automations: Mutex<Vec<AutomationRecord>>,
    pub(super) roles: Mutex<Vec<RoleRecord>>,
    pub(super) permissions: Mutex<Vec<PermissionRecord>>,
    pub(super) menus: Mutex<Vec<MenuRecord>>,
    pub(super) artifacts: Mutex<Vec<ArtifactRecord>>,
    pub(super) inbox: Mutex<Vec<InboxItemRecord>>,
    pub(super) trace_events: Mutex<Vec<TraceEventRecord>>,
    pub(super) audit_records: Mutex<Vec<AuditRecord>>,
    pub(super) cost_entries: Mutex<Vec<CostLedgerEntry>>,
    pub(super) workspace_pet_presence: Mutex<PetPresenceState>,
    pub(super) project_pet_presences: Mutex<Vec<(String, PetPresenceState)>>,
    pub(super) workspace_pet_binding: Mutex<Option<PetConversationBinding>>,
    pub(super) project_pet_bindings: Mutex<Vec<(String, PetConversationBinding)>>,
}

impl InfraState {
    pub(super) fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.paths.db_path).map_err(|error| AppError::database(error.to_string()))
    }

    pub(super) fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))
            .map(|workspace| workspace.clone())
    }

    pub(super) fn workspace_id(&self) -> Result<String, AppError> {
        Ok(self.workspace_snapshot()?.id)
    }

    pub(super) fn save_workspace_config(&self) -> Result<(), AppError> {
        let workspace = self.workspace_snapshot()?;
        bootstrap::save_workspace_config_file(&self.paths.workspace_config, &workspace)
    }
}

pub(super) fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.workspace_config.exists() {
        return Ok(());
    }

    let config = WorkspaceConfigFile {
        id: DEFAULT_WORKSPACE_ID.into(),
        name: "Octopus Local Workspace".into(),
        slug: "local-workspace".into(),
        deployment: "local".into(),
        bootstrap_status: "setup_required".into(),
        owner_user_id: None,
        host: "127.0.0.1".into(),
        listen_address: "127.0.0.1".into(),
        default_project_id: DEFAULT_PROJECT_ID.into(),
    };
    fs::write(&paths.workspace_config, toml::to_string_pretty(&config)?)?;
    Ok(())
}

pub(super) fn initialize_app_registry(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.app_registry_config.exists() {
        return Ok(());
    }

    let registry = AppRegistryFile {
        apps: default_client_apps(),
    };
    fs::write(
        &paths.app_registry_config,
        toml::to_string_pretty(&registry)?,
    )?;
    Ok(())
}

pub(super) fn initialize_database(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (
              id TEXT PRIMARY KEY,
              username TEXT NOT NULL UNIQUE,
              display_name TEXT NOT NULL,
              avatar_path TEXT,
              avatar_content_type TEXT,
              avatar_byte_size INTEGER,
              avatar_content_hash TEXT,
              status TEXT NOT NULL,
              password_hash TEXT NOT NULL,
              password_state TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS memberships (
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              role_ids TEXT NOT NULL,
              scope_mode TEXT NOT NULL,
              scope_project_ids TEXT NOT NULL,
              PRIMARY KEY (workspace_id, user_id)
            );
            CREATE TABLE IF NOT EXISTS client_apps (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              platform TEXT NOT NULL,
              status TEXT NOT NULL,
              first_party INTEGER NOT NULL,
              allowed_origins TEXT NOT NULL,
              allowed_hosts TEXT NOT NULL,
              session_policy TEXT NOT NULL,
              default_scopes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              client_app_id TEXT NOT NULL,
              token TEXT NOT NULL UNIQUE,
              status TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER,
              role_ids TEXT NOT NULL,
              scope_project_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              description TEXT NOT NULL,
              assignments_json TEXT
            );
            CREATE TABLE IF NOT EXISTS resources (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              location TEXT,
              origin TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              tags TEXT NOT NULL,
              source_artifact_id TEXT
            );
            CREATE TABLE IF NOT EXISTS knowledge_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              title TEXT NOT NULL,
              summary TEXT NOT NULL,
              kind TEXT NOT NULL,
              status TEXT NOT NULL,
              source_type TEXT NOT NULL,
              source_ref TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS agents (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              avatar_path TEXT,
              personality TEXT NOT NULL,
              tags TEXT NOT NULL,
              prompt TEXT NOT NULL,
              builtin_tool_keys TEXT NOT NULL,
              skill_ids TEXT NOT NULL,
              mcp_server_names TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            );
            CREATE TABLE IF NOT EXISTS teams (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              avatar_path TEXT,
              personality TEXT NOT NULL,
              tags TEXT NOT NULL,
              prompt TEXT NOT NULL,
              builtin_tool_keys TEXT NOT NULL,
              skill_ids TEXT NOT NULL,
              mcp_server_names TEXT NOT NULL,
              leader_agent_id TEXT,
              member_agent_ids TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            );
            CREATE TABLE IF NOT EXISTS model_catalog (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              label TEXT NOT NULL,
              provider TEXT NOT NULL,
              description TEXT NOT NULL,
              recommended_for TEXT NOT NULL,
              availability TEXT NOT NULL,
              default_permission TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_credentials (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              provider TEXT NOT NULL,
              name TEXT NOT NULL,
              base_url TEXT,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tools (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_mode TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS automations (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              title TEXT NOT NULL,
              description TEXT NOT NULL,
              cadence TEXT NOT NULL,
              owner_type TEXT NOT NULL,
              owner_id TEXT NOT NULL,
              status TEXT NOT NULL,
              next_run_at INTEGER,
              last_run_at INTEGER,
              output TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS roles (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              code TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_ids TEXT NOT NULL,
              menu_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS permissions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              code TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              kind TEXT NOT NULL,
              target_type TEXT,
              target_ids TEXT NOT NULL,
              action TEXT,
              member_permission_ids TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS menus (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              parent_id TEXT,
              source TEXT NOT NULL,
              label TEXT NOT NULL,
              route_name TEXT,
              status TEXT NOT NULL,
              order_value INTEGER NOT NULL
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
            CREATE TABLE IF NOT EXISTS cost_entries (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              configured_model_id TEXT,
              metric TEXT NOT NULL,
              amount INTEGER NOT NULL,
              unit TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_config_snapshots (
              id TEXT PRIMARY KEY,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              source_refs TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              effective_config_json TEXT
            );
            CREATE TABLE IF NOT EXISTS runtime_session_projections (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              title TEXT NOT NULL,
              session_kind TEXT NOT NULL DEFAULT 'project',
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              last_message_preview TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              detail_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_presence (
              scope_key TEXT PRIMARY KEY,
              project_id TEXT,
              pet_id TEXT NOT NULL,
              is_visible INTEGER NOT NULL,
              chat_open INTEGER NOT NULL,
              motion_state TEXT NOT NULL,
              unread_count INTEGER NOT NULL,
              last_interaction_at INTEGER NOT NULL,
              position_x INTEGER NOT NULL,
              position_y INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_conversation_bindings (
              scope_key TEXT PRIMARY KEY,
              project_id TEXT,
              pet_id TEXT NOT NULL,
              workspace_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_run_projections (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              status TEXT NOT NULL,
              current_step TEXT NOT NULL,
              started_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              model_id TEXT,
              next_action TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              run_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_approval_projections (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              tool_name TEXT NOT NULL,
              summary TEXT NOT NULL,
              detail TEXT NOT NULL,
              risk_level TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              status TEXT NOT NULL,
              approval_json TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    ensure_user_avatar_columns(&connection)?;
    ensure_agent_record_columns(&connection)?;
    ensure_team_record_columns(&connection)?;
    ensure_project_assignment_columns(&connection)?;
    ensure_project_agent_link_table(&connection)?;
    ensure_project_team_link_table(&connection)?;
    ensure_runtime_config_snapshot_columns(&connection)?;
    ensure_runtime_session_projection_columns(&connection)?;
    ensure_cost_entry_columns(&connection)?;
    ensure_resource_columns(&connection)?;
    agent_seed::ensure_import_source_tables(&connection)?;

    Ok(())
}

pub(super) fn seed_defaults(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    let default_menu_records = default_menu_records();

    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_exists.is_none() {
        connection
            .execute(
                "INSERT INTO projects (id, workspace_id, name, status, description, assignments_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
                    Option::<String>::None,
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
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.kind,
                        record.name,
                        record.location,
                        record.origin,
                        record.status,
                        record.updated_at as i64,
                        serde_json::to_string(&record.tags)?,
                        record.source_artifact_id,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
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
                    "INSERT INTO knowledge_records (id, workspace_id, project_id, title, summary, kind, status, source_type, source_ref, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.summary,
                        record.kind,
                        record.status,
                        record.source_type,
                        record.source_ref,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    // Builtin agent/team assets are now exposed as readonly templates and are no longer
    // materialized into editable workspace records during bootstrap.

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

    // Default automations are not seeded because builtin actors are no longer written into
    // workspace/project tables at initialization time.

    let roles_exist: Option<String> = connection
        .query_row("SELECT id FROM roles LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if roles_exist.is_none() {
        for record in default_role_records() {
            connection
                .execute(
                    "INSERT INTO roles (id, workspace_id, name, code, description, status, permission_ids, menu_ids)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.name,
                        record.code,
                        record.description,
                        record.status,
                        serde_json::to_string(&record.permission_ids)?,
                        serde_json::to_string(&record.menu_ids)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let permissions_exist: Option<String> = connection
        .query_row("SELECT id FROM permissions LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if permissions_exist.is_none() {
        for record in default_permission_records() {
            connection
                .execute(
                    "INSERT INTO permissions (id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.name,
                        record.code,
                        record.description,
                        record.status,
                        record.kind,
                        record.target_type,
                        serde_json::to_string(&record.target_ids)?,
                        record.action,
                        serde_json::to_string(&record.member_permission_ids)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    for record in &default_menu_records {
        connection
            .execute(
                "INSERT OR IGNORE INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.workspace_id,
                    record.parent_id,
                    record.source,
                    record.label,
                    record.route_name,
                    record.status,
                    record.order,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    let owner_menu_ids_raw: Option<String> = connection
        .query_row(
            "SELECT menu_ids FROM roles WHERE id = 'owner' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if let Some(owner_menu_ids_raw) = owner_menu_ids_raw {
        let mut owner_menu_ids: Vec<String> =
            serde_json::from_str(&owner_menu_ids_raw).unwrap_or_default();
        let mut changed = false;

        for menu_id in default_menu_records.iter().map(|record| record.id.as_str()) {
            if owner_menu_ids.iter().any(|existing| existing == menu_id) {
                continue;
            }
            owner_menu_ids.push(menu_id.into());
            changed = true;
        }

        if changed {
            connection
                .execute(
                    "UPDATE roles SET menu_ids = ?1 WHERE id = 'owner'",
                    params![serde_json::to_string(&owner_menu_ids)?],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

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

pub(super) fn table_columns(
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

pub(super) fn ensure_columns(
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

pub(super) fn ensure_user_avatar_columns(connection: &Connection) -> Result<(), AppError> {
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

pub(super) fn ensure_agent_record_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "agents",
        &[
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
        ],
    )
}

pub(super) fn ensure_team_record_columns(connection: &Connection) -> Result<(), AppError> {
    let columns = table_columns(connection, "teams")?;

    ensure_columns(
        connection,
        "teams",
        &[
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
            ("leader_agent_id", "TEXT"),
            ("member_agent_ids", "TEXT NOT NULL DEFAULT '[]'"),
        ],
    )?;

    if columns.iter().any(|column| column == "member_ids") {
        connection
            .execute(
                "UPDATE teams SET member_agent_ids = member_ids WHERE member_agent_ids = '[]' AND member_ids IS NOT NULL",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(super) fn write_team_record(
    connection: &Connection,
    record: &TeamRecord,
    replace: bool,
) -> Result<(), AppError> {
    let member_agent_ids_json = serde_json::to_string(&record.member_agent_ids)?;
    let has_legacy_member_ids = table_columns(connection, "teams")?
        .iter()
        .any(|column| column == "member_ids");
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = if has_legacy_member_ids {
        format!(
            "{verb} INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_ids, member_agent_ids, description, status, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
        )
    } else {
        format!(
            "{verb} INTO teams (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_agent_ids, description, status, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
        )
    };

    if has_legacy_member_ids {
        connection.execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.leader_agent_id,
                member_agent_ids_json,
                member_agent_ids_json,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
    } else {
        connection.execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.leader_agent_id,
                member_agent_ids_json,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
    }
    .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn ensure_project_assignment_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(connection, "projects", &[("assignments_json", "TEXT")])
}

pub(super) fn ensure_project_agent_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_project_team_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(super) fn ensure_runtime_config_snapshot_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("PRAGMA table_info(runtime_config_snapshots)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if columns
        .iter()
        .any(|column| column == "effective_config_json")
    {
        return Ok(());
    }

    connection
        .execute(
            "ALTER TABLE runtime_config_snapshots ADD COLUMN effective_config_json TEXT",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn ensure_runtime_session_projection_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_session_projections",
        &[
            ("session_kind", "TEXT NOT NULL DEFAULT 'project'"),
            ("last_message_preview", "TEXT"),
            ("config_snapshot_id", "TEXT NOT NULL DEFAULT ''"),
            ("effective_config_hash", "TEXT NOT NULL DEFAULT ''"),
            ("started_from_scope_set", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "detail_json",
                r#"TEXT NOT NULL DEFAULT '{"summary":{"id":"","conversationId":"","projectId":"","title":"","sessionKind":"project","status":"draft","updatedAt":0,"lastMessagePreview":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[]},"run":{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null},"messages":[],"trace":[],"pendingApproval":null}'"#,
            ),
        ],
    )
}

pub(super) fn ensure_cost_entry_columns(connection: &Connection) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("PRAGMA table_info(cost_entries)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if !columns.iter().any(|column| column == "configured_model_id") {
        connection
            .execute(
                "ALTER TABLE cost_entries ADD COLUMN configured_model_id TEXT",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(super) fn ensure_resource_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(connection, "resources", &[("source_artifact_id", "TEXT")])
}

pub(super) fn load_state(paths: WorkspacePaths) -> Result<InfraState, AppError> {
    let workspace_file: WorkspaceConfigFile =
        toml::from_str(&fs::read_to_string(&paths.workspace_config)?)?;
    let mut workspace = WorkspaceSummary {
        id: workspace_file.id,
        name: workspace_file.name,
        slug: workspace_file.slug,
        deployment: workspace_file.deployment,
        bootstrap_status: workspace_file.bootstrap_status,
        owner_user_id: workspace_file.owner_user_id,
        host: workspace_file.host,
        listen_address: workspace_file.listen_address,
        default_project_id: workspace_file.default_project_id,
    };

    let app_registry: AppRegistryFile =
        toml::from_str(&fs::read_to_string(&paths.app_registry_config)?)?;
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;
    let users = load_users(&connection)?;
    let owner_user_id = users
        .iter()
        .find(|user| {
            user.membership
                .role_ids
                .iter()
                .any(|role_id| role_id == "owner")
        })
        .map(|user| user.record.id.clone());
    let expected_bootstrap_status = if owner_user_id.is_some() {
        "ready"
    } else {
        "setup_required"
    };
    let workspace_needs_normalize = workspace.bootstrap_status != expected_bootstrap_status
        || workspace.owner_user_id != owner_user_id;
    if workspace_needs_normalize {
        workspace.bootstrap_status = expected_bootstrap_status.into();
        workspace.owner_user_id = owner_user_id;
        bootstrap::save_workspace_config_file(&paths.workspace_config, &workspace)?;
    }
    let projects = load_projects(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let agents = load_agents(&connection)?;
    let project_agent_links = load_project_agent_links(&connection)?;
    let teams = load_teams(&connection)?;
    let project_team_links = load_project_team_links(&connection)?;
    let model_catalog = load_model_catalog(&connection)?;
    let provider_credentials = load_provider_credentials(&connection)?;
    let tools = load_tools(&connection)?;
    let automations = load_automations(&connection)?;
    let roles = load_roles(&connection)?;
    let permissions = load_permissions(&connection)?;
    let menus = load_menus(&connection)?;
    let trace_events = load_trace_events(&connection)?;
    let audit_records = load_audit_records(&connection)?;
    let cost_entries = load_cost_entries(&connection)?;
    let workspace_pet_presence =
        load_pet_presence(&connection, "workspace")?.unwrap_or_else(default_workspace_pet_presence);
    let project_pet_presences = load_all_project_pet_presences(&connection)?;
    let workspace_pet_binding = load_pet_binding(&connection, "workspace")?;
    let project_pet_bindings = load_all_project_pet_bindings(&connection)?;

    Ok(InfraState {
        paths,
        workspace: Mutex::new(workspace),
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
        resources: Mutex::new(resources),
        knowledge_records: Mutex::new(knowledge_records),
        agents: Mutex::new(agents),
        project_agent_links: Mutex::new(project_agent_links),
        teams: Mutex::new(teams),
        project_team_links: Mutex::new(project_team_links),
        model_catalog: Mutex::new(model_catalog),
        provider_credentials: Mutex::new(provider_credentials),
        tools: Mutex::new(tools),
        automations: Mutex::new(automations),
        roles: Mutex::new(roles),
        permissions: Mutex::new(permissions),
        menus: Mutex::new(menus),
        artifacts: Mutex::new(Vec::new()),
        inbox: Mutex::new(Vec::new()),
        trace_events: Mutex::new(trace_events),
        audit_records: Mutex::new(audit_records),
        cost_entries: Mutex::new(cost_entries),
        workspace_pet_presence: Mutex::new(workspace_pet_presence),
        project_pet_presences: Mutex::new(project_pet_presences),
        workspace_pet_binding: Mutex::new(workspace_pet_binding),
        project_pet_bindings: Mutex::new(project_pet_bindings),
    })
}

pub(super) fn load_users(connection: &Connection) -> Result<Vec<StoredUser>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT u.id, u.username, u.display_name, u.avatar_path, u.avatar_content_type, u.avatar_byte_size, u.avatar_content_hash,
                    u.status, u.password_hash, u.password_state, u.created_at, u.updated_at,
                    m.workspace_id, m.role_ids, m.scope_mode, m.scope_project_ids
             FROM users u
             LEFT JOIN memberships m ON m.user_id = u.id",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_ids_raw: String = row.get(13)?;
            let scope_project_ids_raw: String = row.get(15)?;
            Ok(StoredUser {
                record: UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_path: row.get(3)?,
                    avatar_content_type: row.get(4)?,
                    avatar_byte_size: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                    avatar_content_hash: row.get(6)?,
                    status: row.get(7)?,
                    password_state: row.get(9)?,
                    created_at: row.get::<_, i64>(10)? as u64,
                    updated_at: row.get::<_, i64>(11)? as u64,
                },
                password_hash: row.get(8)?,
                membership: WorkspaceMembershipRecord {
                    workspace_id: row.get(12)?,
                    user_id: row.get(0)?,
                    role_ids: serde_json::from_str(&role_ids_raw).unwrap_or_default(),
                    scope_mode: row.get(14)?,
                    scope_project_ids: serde_json::from_str(&scope_project_ids_raw)
                        .unwrap_or_default(),
                },
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, status, description, assignments_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let assignments_json: Option<String> = row.get(5)?;
            let assignments = assignments_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                assignments,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn default_pet_profile() -> PetProfile {
    PetProfile {
        id: "pet-octopus".into(),
        species: "octopus".into(),
        display_name: "小章".into(),
        owner_user_id: "user-owner".into(),
        avatar_label: "Octopus mascot".into(),
        summary: "Octopus 首席吉祥物，负责卖萌和加油。".into(),
        greeting: "嗨！我是小章，今天也要加油哦！".into(),
        mood: "happy".into(),
        favorite_snack: "新鲜小虾".into(),
        prompt_hints: vec![
            "最近有什么好消息？".into(),
            "给我讲个冷笑话".into(),
            "我们要加油呀！".into(),
        ],
        fallback_asset: "octopus".into(),
        rive_asset: None,
        state_machine: None,
    }
}

pub(super) fn default_workspace_pet_presence() -> PetPresenceState {
    PetPresenceState {
        pet_id: "pet-octopus".into(),
        is_visible: true,
        chat_open: false,
        motion_state: "idle".into(),
        unread_count: 0,
        last_interaction_at: 0,
        position: PetPosition { x: 0, y: 0 },
    }
}

pub(super) fn map_pet_message(pet_id: &str, message: &octopus_core::RuntimeMessage) -> PetMessage {
    PetMessage {
        id: message.id.clone(),
        pet_id: pet_id.into(),
        sender: if message.sender_type == "assistant" {
            "pet".into()
        } else {
            "user".into()
        },
        content: message.content.clone(),
        timestamp: message.timestamp,
    }
}

pub(super) fn load_runtime_messages_for_conversation(
    connection: &Connection,
    conversation_id: &str,
    pet_id: &str,
) -> Result<Vec<PetMessage>, AppError> {
    let detail_json: Option<String> = connection
        .query_row(
            "SELECT detail_json FROM runtime_session_projections WHERE conversation_id = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![conversation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    let Some(detail_json) = detail_json else {
        return Ok(vec![]);
    };
    let detail: octopus_core::RuntimeSessionDetail = serde_json::from_str(&detail_json)?;
    Ok(detail
        .messages
        .iter()
        .map(|message| map_pet_message(pet_id, message))
        .collect())
}

pub(super) fn row_to_pet_presence(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetPresenceState> {
    Ok(PetPresenceState {
        pet_id: row.get(2)?,
        is_visible: row.get::<_, i64>(3)? != 0,
        chat_open: row.get::<_, i64>(4)? != 0,
        motion_state: row.get(5)?,
        unread_count: row.get::<_, i64>(6)? as u64,
        last_interaction_at: row.get::<_, i64>(7)? as u64,
        position: PetPosition {
            x: row.get(8)?,
            y: row.get(9)?,
        },
    })
}

pub(super) fn load_pet_presence(
    connection: &Connection,
    scope_key: &str,
) -> Result<Option<PetPresenceState>, AppError> {
    connection
        .query_row(
            "SELECT scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence WHERE scope_key = ?1",
            params![scope_key],
            row_to_pet_presence,
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_all_project_pet_presences(
    connection: &Connection,
) -> Result<Vec<(String, PetPresenceState)>, AppError> {
    let mut stmt = connection
        .prepare("SELECT scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence WHERE project_id IS NOT NULL")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row_to_pet_presence(row)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn row_to_pet_binding(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PetConversationBinding> {
    Ok(PetConversationBinding {
        pet_id: row.get(2)?,
        workspace_id: row.get(3)?,
        project_id: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        conversation_id: row.get(4)?,
        session_id: row.get(5)?,
        updated_at: row.get::<_, i64>(6)? as u64,
    })
}

pub(super) fn load_pet_binding(
    connection: &Connection,
    scope_key: &str,
) -> Result<Option<PetConversationBinding>, AppError> {
    connection
        .query_row(
            "SELECT scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings WHERE scope_key = ?1",
            params![scope_key],
            row_to_pet_binding,
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_all_project_pet_bindings(
    connection: &Connection,
) -> Result<Vec<(String, PetConversationBinding)>, AppError> {
    let mut stmt = connection
        .prepare("SELECT scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings WHERE project_id IS NOT NULL")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row_to_pet_binding(row)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_resources(
    connection: &Connection,
) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id FROM resources",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let tags_raw: String = row.get(9)?;
            Ok(WorkspaceResourceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                kind: row.get(3)?,
                name: row.get(4)?,
                location: row.get(5)?,
                origin: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get::<_, i64>(8)? as u64,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                source_artifact_id: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_knowledge_records(
    connection: &Connection,
) -> Result<Vec<KnowledgeRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, title, summary, kind, status, source_type, source_ref, updated_at FROM knowledge_records",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(KnowledgeRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                kind: row.get(5)?,
                status: row.get(6)?,
                source_type: row.get(7)?,
                source_ref: row.get(8)?,
                updated_at: row.get::<_, i64>(9)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn agent_avatar(paths: &WorkspacePaths, avatar_path: Option<&str>) -> Option<String> {
    let avatar_path = avatar_path?;
    let absolute_path = paths.root.join(avatar_path);
    let bytes = fs::read(&absolute_path).ok()?;
    let content_type = match absolute_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => return Some(avatar_path.to_string()),
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(super) fn load_agents(connection: &Connection) -> Result<Vec<AgentRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at FROM agents",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(5)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(7)?;
            let builtin_tool_keys_raw: String = row.get(9)?;
            let skill_ids_raw: String = row.get(10)?;
            let mcp_server_names_raw: String = row.get(11)?;
            Ok(AgentRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                avatar_path,
                avatar,
                personality: row.get(6)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(8)?,
                builtin_tool_keys: serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default(),
                skill_ids: serde_json::from_str(&skill_ids_raw).unwrap_or_default(),
                mcp_server_names: serde_json::from_str(&mcp_server_names_raw).unwrap_or_default(),
                integration_source: None,
                description: row.get(12)?,
                status: row.get(13)?,
                updated_at: row.get::<_, i64>(14)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_agent_links(
    connection: &Connection,
) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, agent_id, linked_at FROM project_agent_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectAgentLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                agent_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_teams(connection: &Connection) -> Result<Vec<TeamRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, leader_agent_id, member_agent_ids, description, status, updated_at FROM teams",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(5)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(7)?;
            let builtin_tool_keys_raw: String = row.get(9)?;
            let skill_ids_raw: String = row.get(10)?;
            let mcp_server_names_raw: String = row.get(11)?;
            let member_agent_ids_raw: String = row.get(13)?;
            Ok(TeamRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                avatar_path,
                avatar,
                personality: row.get(6)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(8)?,
                builtin_tool_keys: serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default(),
                skill_ids: serde_json::from_str(&skill_ids_raw).unwrap_or_default(),
                mcp_server_names: serde_json::from_str(&mcp_server_names_raw).unwrap_or_default(),
                leader_agent_id: row.get(12)?,
                member_agent_ids: serde_json::from_str(&member_agent_ids_raw).unwrap_or_default(),
                integration_source: None,
                description: row.get(14)?,
                status: row.get(15)?,
                updated_at: row.get::<_, i64>(16)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_team_links(
    connection: &Connection,
) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, team_id, linked_at FROM project_team_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTeamLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                team_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_model_catalog(
    connection: &Connection,
) -> Result<Vec<ModelCatalogRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, label, provider, description, recommended_for, availability, default_permission FROM model_catalog",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ModelCatalogRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                label: row.get(2)?,
                provider: row.get(3)?,
                description: row.get(4)?,
                recommended_for: row.get(5)?,
                availability: row.get(6)?,
                default_permission: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_provider_credentials(
    connection: &Connection,
) -> Result<Vec<ProviderCredentialRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, provider, name, base_url, status FROM provider_credentials",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProviderCredentialRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                provider: row.get(2)?,
                name: row.get(3)?,
                base_url: row.get(4)?,
                status: row.get(5)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_tools(connection: &Connection) -> Result<Vec<ToolRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, kind, name, description, status, permission_mode, updated_at FROM tools",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ToolRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                kind: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                permission_mode: row.get(6)?,
                updated_at: row.get::<_, i64>(7)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_automations(connection: &Connection) -> Result<Vec<AutomationRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output FROM automations",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AutomationRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                cadence: row.get(5)?,
                owner_type: row.get(6)?,
                owner_id: row.get(7)?,
                status: row.get(8)?,
                next_run_at: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
                last_run_at: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
                output: row.get(11)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_roles(connection: &Connection) -> Result<Vec<RoleRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, code, description, status, permission_ids, menu_ids FROM roles",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let permission_ids_raw: String = row.get(6)?;
            let menu_ids_raw: String = row.get(7)?;
            Ok(RoleRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                permission_ids: serde_json::from_str(&permission_ids_raw).unwrap_or_default(),
                menu_ids: serde_json::from_str(&menu_ids_raw).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_permissions(connection: &Connection) -> Result<Vec<PermissionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids FROM permissions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let target_ids_raw: String = row.get(8)?;
            let member_permission_ids_raw: String = row.get(10)?;
            Ok(PermissionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                kind: row.get(6)?,
                target_type: row.get(7)?,
                target_ids: serde_json::from_str(&target_ids_raw).unwrap_or_default(),
                action: row.get(9)?,
                member_permission_ids: serde_json::from_str(&member_permission_ids_raw)
                    .unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_menus(connection: &Connection) -> Result<Vec<MenuRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, parent_id, source, label, route_name, status, order_value FROM menus ORDER BY order_value ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MenuRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                parent_id: row.get(2)?,
                source: row.get(3)?,
                label: row.get(4)?,
                route_name: row.get(5)?,
                status: row.get(6)?,
                order: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_sessions(connection: &Connection) -> Result<Vec<SessionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at, role_ids, scope_project_ids
             FROM sessions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_ids_raw: String = row.get(8)?;
            let scope_project_ids_raw: String = row.get(9)?;
            Ok(SessionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                client_app_id: row.get(3)?,
                token: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
                expires_at: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
                role_ids: serde_json::from_str(&role_ids_raw).unwrap_or_default(),
                scope_project_ids: serde_json::from_str(&scope_project_ids_raw).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_trace_events(
    connection: &Connection,
) -> Result<Vec<TraceEventRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, run_id, session_id, event_kind, title, detail, created_at
             FROM trace_events ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TraceEventRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                run_id: row.get(3)?,
                session_id: row.get(4)?,
                event_kind: row.get(5)?,
                title: row.get(6)?,
                detail: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_audit_records(connection: &Connection) -> Result<Vec<AuditRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, actor_type, actor_id, action, resource, outcome, created_at
             FROM audit_records ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                actor_type: row.get(3)?,
                actor_id: row.get(4)?,
                action: row.get(5)?,
                resource: row.get(6)?,
                outcome: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_cost_entries(connection: &Connection) -> Result<Vec<CostLedgerEntry>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, run_id, configured_model_id, metric, amount, unit, created_at
             FROM cost_entries ORDER BY created_at ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CostLedgerEntry {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                run_id: row.get(3)?,
                configured_model_id: row.get(4)?,
                metric: row.get(5)?,
                amount: row.get(6)?,
                unit: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn default_workspace_resources() -> Vec<WorkspaceResourceRecord> {
    let now = timestamp_now();
    vec![
        WorkspaceResourceRecord {
            id: "res-workspace-handbook".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            kind: "file".into(),
            name: "Workspace Handbook".into(),
            location: Some("/docs/workspace-handbook.md".into()),
            origin: "source".into(),
            status: "healthy".into(),
            updated_at: now,
            tags: vec!["workspace".into(), "handbook".into()],
            source_artifact_id: None,
        },
        WorkspaceResourceRecord {
            id: "res-project-board".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            kind: "folder".into(),
            name: "Project Delivery Board".into(),
            location: Some("/projects/default".into()),
            origin: "generated".into(),
            status: "configured".into(),
            updated_at: now,
            tags: vec!["project".into(), "delivery".into()],
            source_artifact_id: None,
        },
    ]
}

pub(super) fn default_knowledge_records() -> Vec<KnowledgeRecord> {
    let now = timestamp_now();
    vec![
        KnowledgeRecord {
            id: "kn-workspace-onboarding".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            title: "Workspace onboarding".into(),
            summary: "Shared operating rules, review expectations, and release cadence for this workspace.".into(),
            kind: "shared".into(),
            status: "shared".into(),
            source_type: "artifact".into(),
            source_ref: "workspace-handbook".into(),
            updated_at: now,
        },
        KnowledgeRecord {
            id: "kn-project-brief".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            title: "Default project brief".into(),
            summary: "Project goals, runtime expectations, and delivery checkpoints.".into(),
            kind: "private".into(),
            status: "reviewed".into(),
            source_type: "run".into(),
            source_ref: "default-project".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_agent_records() -> Vec<AgentRecord> {
    let now = timestamp_now();
    vec![
        AgentRecord {
            id: "agent-orchestrator".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            name: "Workspace Orchestrator".into(),
            avatar_path: None,
            avatar: None,
            personality: "System coordinator".into(),
            tags: vec!["workspace".into(), "orchestration".into()],
            prompt: "Coordinate work across the workspace and keep execution aligned.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            integration_source: None,
            description: "Coordinates projects, approvals, and execution policies.".into(),
            status: "active".into(),
            updated_at: now,
        },
        AgentRecord {
            id: "agent-project-delivery".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            scope: "project".into(),
            name: "Project Delivery Agent".into(),
            avatar_path: None,
            avatar: None,
            personality: "Delivery lead".into(),
            tags: vec!["project".into(), "delivery".into()],
            prompt: "Track project work, runtime sessions, and follow-up actions.".into(),
            builtin_tool_keys: vec![],
            skill_ids: vec![],
            mcp_server_names: vec![],
            integration_source: None,
            description: "Tracks project work, runtime sessions, and follow-up actions.".into(),
            status: "active".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_team_records() -> Vec<TeamRecord> {
    let now = timestamp_now();
    vec![TeamRecord {
        id: "team-workspace-core".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        project_id: None,
        scope: "workspace".into(),
        name: "Workspace Core".into(),
        avatar_path: None,
        avatar: None,
        personality: "Governance team".into(),
        tags: vec!["workspace".into(), "governance".into()],
        prompt: "Maintain workspace-wide standards and governance.".into(),
        builtin_tool_keys: vec![],
        skill_ids: vec![],
        mcp_server_names: vec![],
        leader_agent_id: Some("agent-orchestrator".into()),
        member_agent_ids: vec!["agent-orchestrator".into()],
        integration_source: None,
        description: "Maintains workspace-wide operating standards and governance.".into(),
        status: "active".into(),
        updated_at: now,
    }]
}

pub(super) fn default_model_catalog() -> Vec<ModelCatalogRecord> {
    vec![
        ModelCatalogRecord {
            id: "claude-sonnet-4-5".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "Claude Sonnet 4.5".into(),
            provider: "Anthropic".into(),
            description: "Balanced reasoning model for daily runtime turns.".into(),
            recommended_for: "Planning, coding, and reviews".into(),
            availability: "healthy".into(),
            default_permission: "auto".into(),
        },
        ModelCatalogRecord {
            id: "gpt-4o".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "GPT-4o".into(),
            provider: "OpenAI".into(),
            description: "Fast multimodal model for general assistant work.".into(),
            recommended_for: "Conversation and lightweight execution".into(),
            availability: "configured".into(),
            default_permission: "auto".into(),
        },
    ]
}

pub(super) fn default_provider_credentials() -> Vec<ProviderCredentialRecord> {
    vec![
        ProviderCredentialRecord {
            id: "cred-anthropic".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "Anthropic".into(),
            name: "Anthropic Primary".into(),
            base_url: None,
            status: "healthy".into(),
        },
        ProviderCredentialRecord {
            id: "cred-openai".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "OpenAI".into(),
            name: "OpenAI Backup".into(),
            base_url: None,
            status: "unconfigured".into(),
        },
    ]
}

pub(super) fn default_tool_records() -> Vec<ToolRecord> {
    let now = timestamp_now();
    vec![
        ToolRecord {
            id: "tool-filesystem".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Filesystem".into(),
            description: "Read and write files inside the workspace boundary.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
        ToolRecord {
            id: "tool-shell".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Shell".into(),
            description: "Execute workspace commands with approval.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
    ]
}

#[allow(dead_code)]
pub(super) fn default_automation_records() -> Vec<AutomationRecord> {
    let now = timestamp_now();
    vec![AutomationRecord {
        id: "auto-daily-summary".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        project_id: Some(DEFAULT_PROJECT_ID.into()),
        title: "Daily summary".into(),
        description: "Summarize active runtime work for the default project.".into(),
        cadence: "Weekdays 09:30".into(),
        owner_type: "agent".into(),
        owner_id: "agent-project-delivery".into(),
        status: "active".into(),
        next_run_at: Some(now + 86_400_000),
        last_run_at: None,
        output: "Inbox summary".into(),
    }]
}

pub(super) fn default_permission_records() -> Vec<PermissionRecord> {
    vec![
        PermissionRecord {
            id: "perm-workspace-read".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            name: "Workspace Read".into(),
            code: "workspace.read".into(),
            description: "Read workspace-level resources and projections.".into(),
            status: "active".into(),
            kind: "atomic".into(),
            target_type: Some("workspace".into()),
            target_ids: vec![DEFAULT_WORKSPACE_ID.into()],
            action: Some("read".into()),
            member_permission_ids: Vec::new(),
        },
        PermissionRecord {
            id: "perm-runtime-read".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            name: "Runtime Read".into(),
            code: "runtime.read".into(),
            description: "Read runtime sessions and event streams.".into(),
            status: "active".into(),
            kind: "atomic".into(),
            target_type: Some("project".into()),
            target_ids: vec![DEFAULT_PROJECT_ID.into()],
            action: Some("read".into()),
            member_permission_ids: Vec::new(),
        },
    ]
}

pub(super) fn default_menu_records() -> Vec<MenuRecord> {
    vec![
        MenuRecord {
            id: "menu-workspace-overview".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Overview".into(),
            route_name: Some("workspace-overview".into()),
            status: "active".into(),
            order: 10,
        },
        MenuRecord {
            id: "menu-workspace-console".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Console".into(),
            route_name: Some("workspace-console".into()),
            status: "active".into(),
            order: 20,
        },
        MenuRecord {
            id: "menu-workspace-console-projects".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Projects".into(),
            route_name: Some("workspace-console-projects".into()),
            status: "active".into(),
            order: 30,
        },
        MenuRecord {
            id: "menu-workspace-console-knowledge".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Knowledge".into(),
            route_name: Some("workspace-console-knowledge".into()),
            status: "active".into(),
            order: 40,
        },
        MenuRecord {
            id: "menu-workspace-console-resources".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Resources".into(),
            route_name: Some("workspace-console-resources".into()),
            status: "active".into(),
            order: 50,
        },
        MenuRecord {
            id: "menu-workspace-console-agents".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Agents".into(),
            route_name: Some("workspace-console-agents".into()),
            status: "active".into(),
            order: 60,
        },
        MenuRecord {
            id: "menu-workspace-console-models".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Models".into(),
            route_name: Some("workspace-console-models".into()),
            status: "active".into(),
            order: 70,
        },
        MenuRecord {
            id: "menu-workspace-console-tools".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-console".into()),
            source: "console".into(),
            label: "Tools".into(),
            route_name: Some("workspace-console-tools".into()),
            status: "active".into(),
            order: 80,
        },
        MenuRecord {
            id: "menu-workspace-automations".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Automations".into(),
            route_name: Some("workspace-automations".into()),
            status: "active".into(),
            order: 90,
        },
        MenuRecord {
            id: "menu-workspace-permission-center".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Permission Center".into(),
            route_name: Some("workspace-permission-center".into()),
            status: "active".into(),
            order: 100,
        },
        MenuRecord {
            id: "menu-workspace-permission-center-users".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-permission-center".into()),
            source: "permission-center".into(),
            label: "Users".into(),
            route_name: Some("workspace-permission-center-users".into()),
            status: "active".into(),
            order: 110,
        },
        MenuRecord {
            id: "menu-workspace-permission-center-roles".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-permission-center".into()),
            source: "permission-center".into(),
            label: "Roles".into(),
            route_name: Some("workspace-permission-center-roles".into()),
            status: "active".into(),
            order: 120,
        },
        MenuRecord {
            id: "menu-workspace-permission-center-permissions".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-permission-center".into()),
            source: "permission-center".into(),
            label: "Permissions".into(),
            route_name: Some("workspace-permission-center-permissions".into()),
            status: "active".into(),
            order: 130,
        },
        MenuRecord {
            id: "menu-workspace-permission-center-menus".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-permission-center".into()),
            source: "permission-center".into(),
            label: "Menus".into(),
            route_name: Some("workspace-permission-center-menus".into()),
            status: "active".into(),
            order: 140,
        },
    ]
}

pub(super) fn default_role_records() -> Vec<RoleRecord> {
    vec![RoleRecord {
        id: "owner".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        name: "Owner".into(),
        code: "owner".into(),
        description: "Full workspace access.".into(),
        status: "active".into(),
        permission_ids: default_permission_records()
            .into_iter()
            .map(|record| record.id)
            .collect(),
        menu_ids: default_menu_records()
            .into_iter()
            .map(|record| record.id)
            .collect(),
    }]
}

pub(super) fn avatar_data_url(paths: &WorkspacePaths, user: &StoredUser) -> Option<String> {
    let avatar_path = user.record.avatar_path.as_ref()?;
    let Some(content_type) = user.record.avatar_content_type.as_deref() else {
        return Some(avatar_path.clone());
    };
    let Ok(bytes) = fs::read(paths.root.join(avatar_path)) else {
        return Some(avatar_path.clone());
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(super) fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("hash-{:x}", hasher.finish())
}
