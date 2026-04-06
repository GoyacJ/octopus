use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    timestamp_now, AgentRecord, AppError, ArtifactRecord, AuditRecord, AuthorizationDecision,
    AutomationRecord, ClientAppRecord, CostLedgerEntry, InboxItemRecord,
    KnowledgeEntryRecord, KnowledgeRecord, LoginRequest, LoginResponse, MenuRecord,
    ModelCatalogRecord, PermissionRecord, ProjectRecord, ProviderCredentialRecord,
    RoleRecord, SessionRecord, SystemBootstrapStatus, TeamRecord, ToolRecord,
    TraceEventRecord, UserRecord, UserRecordSummary, WorkspaceMembershipRecord,
    WorkspaceResourceRecord, WorkspaceSummary, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use octopus_platform::{
    AppRegistryService, ArtifactService, AuthService, InboxService, KnowledgeService,
    ObservationService, RbacService, WorkspaceService,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub workspace_config: PathBuf,
    pub app_registry_config: PathBuf,
    pub db_path: PathBuf,
    pub blobs_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub knowledge_dir: PathBuf,
    pub inbox_dir: PathBuf,
    pub runtime_sessions_dir: PathBuf,
    pub runtime_events_dir: PathBuf,
    pub runtime_traces_dir: PathBuf,
    pub runtime_approvals_dir: PathBuf,
    pub runtime_cache_dir: PathBuf,
    pub audit_log_dir: PathBuf,
    pub server_log_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let config_dir = root.join("config");
        let data_dir = root.join("data");
        let runtime_dir = root.join("runtime");
        let logs_dir = root.join("logs");
        let tmp_dir = root.join("tmp");
        let blobs_dir = data_dir.join("blobs");
        let artifacts_dir = data_dir.join("artifacts");
        let knowledge_dir = data_dir.join("knowledge");
        let inbox_dir = data_dir.join("inbox");
        let runtime_sessions_dir = runtime_dir.join("sessions");
        let runtime_events_dir = runtime_dir.join("events");
        let runtime_traces_dir = runtime_dir.join("traces");
        let runtime_approvals_dir = runtime_dir.join("approvals");
        let runtime_cache_dir = runtime_dir.join("cache");
        let audit_log_dir = logs_dir.join("audit");
        let server_log_dir = logs_dir.join("server");

        Self {
            workspace_config: config_dir.join("workspace.toml"),
            app_registry_config: config_dir.join("app-registry.toml"),
            db_path: data_dir.join("main.db"),
            root,
            config_dir,
            data_dir,
            runtime_dir,
            logs_dir,
            tmp_dir,
            blobs_dir,
            artifacts_dir,
            knowledge_dir,
            inbox_dir,
            runtime_sessions_dir,
            runtime_events_dir,
            runtime_traces_dir,
            runtime_approvals_dir,
            runtime_cache_dir,
            audit_log_dir,
            server_log_dir,
        }
    }

    pub fn ensure_layout(&self) -> Result<(), AppError> {
        for path in [
            &self.root,
            &self.config_dir,
            &self.data_dir,
            &self.runtime_dir,
            &self.logs_dir,
            &self.tmp_dir,
            &self.blobs_dir,
            &self.artifacts_dir,
            &self.knowledge_dir,
            &self.inbox_dir,
            &self.runtime_sessions_dir,
            &self.runtime_events_dir,
            &self.runtime_traces_dir,
            &self.runtime_approvals_dir,
            &self.runtime_cache_dir,
            &self.audit_log_dir,
            &self.server_log_dir,
        ] {
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceConfigFile {
    id: String,
    name: String,
    slug: String,
    deployment: String,
    bootstrap_status: String,
    owner_user_id: Option<String>,
    host: String,
    listen_address: String,
    default_project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppRegistryFile {
    apps: Vec<ClientAppRecord>,
}

#[derive(Debug, Clone)]
struct StoredUser {
    record: UserRecord,
    password_hash: String,
    membership: WorkspaceMembershipRecord,
}

#[derive(Debug)]
struct InfraState {
    paths: WorkspacePaths,
    workspace: WorkspaceSummary,
    users: Mutex<Vec<StoredUser>>,
    apps: Mutex<Vec<ClientAppRecord>>,
    sessions: Mutex<Vec<SessionRecord>>,
    projects: Mutex<Vec<ProjectRecord>>,
    resources: Mutex<Vec<WorkspaceResourceRecord>>,
    knowledge_records: Mutex<Vec<KnowledgeRecord>>,
    agents: Mutex<Vec<AgentRecord>>,
    teams: Mutex<Vec<TeamRecord>>,
    model_catalog: Mutex<Vec<ModelCatalogRecord>>,
    provider_credentials: Mutex<Vec<ProviderCredentialRecord>>,
    tools: Mutex<Vec<ToolRecord>>,
    automations: Mutex<Vec<AutomationRecord>>,
    roles: Mutex<Vec<RoleRecord>>,
    permissions: Mutex<Vec<PermissionRecord>>,
    menus: Mutex<Vec<MenuRecord>>,
    artifacts: Mutex<Vec<ArtifactRecord>>,
    inbox: Mutex<Vec<InboxItemRecord>>,
    trace_events: Mutex<Vec<TraceEventRecord>>,
    audit_records: Mutex<Vec<AuditRecord>>,
    cost_entries: Mutex<Vec<CostLedgerEntry>>,
}

impl InfraState {
    fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.paths.db_path).map_err(|error| AppError::database(error.to_string()))
    }
}

#[derive(Clone)]
pub struct InfraWorkspaceService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraAuthService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraAppRegistryService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraRbacService {
    _state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraArtifactService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraInboxService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraKnowledgeService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraObservationService {
    state: Arc<InfraState>,
}

#[derive(Clone)]
pub struct InfraBundle {
    pub paths: WorkspacePaths,
    pub workspace: Arc<InfraWorkspaceService>,
    pub auth: Arc<InfraAuthService>,
    pub app_registry: Arc<InfraAppRegistryService>,
    pub rbac: Arc<InfraRbacService>,
    pub artifact: Arc<InfraArtifactService>,
    pub inbox: Arc<InfraInboxService>,
    pub knowledge: Arc<InfraKnowledgeService>,
    pub observation: Arc<InfraObservationService>,
}

pub fn initialize_workspace(root: impl Into<PathBuf>) -> Result<WorkspacePaths, AppError> {
    let paths = WorkspacePaths::new(root);
    paths.ensure_layout()?;
    initialize_workspace_config(&paths)?;
    initialize_app_registry(&paths)?;
    initialize_database(&paths)?;
    seed_defaults(&paths)?;
    Ok(paths)
}

pub fn build_infra_bundle(root: impl Into<PathBuf>) -> Result<InfraBundle, AppError> {
    let paths = initialize_workspace(root)?;
    let state = Arc::new(load_state(paths.clone())?);

    Ok(InfraBundle {
        paths: paths.clone(),
        workspace: Arc::new(InfraWorkspaceService {
            state: Arc::clone(&state),
        }),
        auth: Arc::new(InfraAuthService {
            state: Arc::clone(&state),
        }),
        app_registry: Arc::new(InfraAppRegistryService {
            state: Arc::clone(&state),
        }),
        rbac: Arc::new(InfraRbacService {
            _state: Arc::clone(&state),
        }),
        artifact: Arc::new(InfraArtifactService {
            state: Arc::clone(&state),
        }),
        inbox: Arc::new(InfraInboxService {
            state: Arc::clone(&state),
        }),
        knowledge: Arc::new(InfraKnowledgeService {
            state: Arc::clone(&state),
        }),
        observation: Arc::new(InfraObservationService { state }),
    })
}

fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.workspace_config.exists() {
        return Ok(());
    }

    let config = WorkspaceConfigFile {
        id: DEFAULT_WORKSPACE_ID.into(),
        name: "Octopus Local Workspace".into(),
        slug: "local-workspace".into(),
        deployment: "local".into(),
        bootstrap_status: "setup_required".into(),
        owner_user_id: Some("user-owner".into()),
        host: "127.0.0.1".into(),
        listen_address: "127.0.0.1".into(),
        default_project_id: DEFAULT_PROJECT_ID.into(),
    };
    fs::write(&paths.workspace_config, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn initialize_app_registry(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.app_registry_config.exists() {
        return Ok(());
    }

    let registry = AppRegistryFile {
        apps: default_client_apps(),
    };
    fs::write(&paths.app_registry_config, toml::to_string_pretty(&registry)?)?;
    Ok(())
}

fn initialize_database(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (
              id TEXT PRIMARY KEY,
              username TEXT NOT NULL UNIQUE,
              display_name TEXT NOT NULL,
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
              description TEXT NOT NULL
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
              tags TEXT NOT NULL
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
              title TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS teams (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              member_ids TEXT NOT NULL,
              updated_at INTEGER NOT NULL
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
              metric TEXT NOT NULL,
              amount INTEGER NOT NULL,
              unit TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_config_snapshots (
              id TEXT PRIMARY KEY,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              source_paths TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_session_projections (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              title TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              last_message_preview TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              detail_json TEXT NOT NULL
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

    Ok(())
}

fn seed_defaults(paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection =
        Connection::open(&paths.db_path).map_err(|error| AppError::database(error.to_string()))?;

    let owner_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM users WHERE id = ?1",
            params!["user-owner"],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if owner_exists.is_none() {
        let now = timestamp_now();
        connection
            .execute(
                "INSERT INTO users (id, username, display_name, status, password_hash, password_state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "user-owner",
                    "owner",
                    "Workspace Owner",
                    "active",
                    hash_password("owner"),
                    "reset-required",
                    now as i64,
                    now as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    DEFAULT_WORKSPACE_ID,
                    "user-owner",
                    serde_json::to_string(&vec!["owner"])?,
                    "all-projects",
                    serde_json::to_string(&Vec::<String>::new())?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

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
                "INSERT INTO projects (id, workspace_id, name, status, description)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
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
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let knowledge_exists: Option<String> = connection
        .query_row("SELECT id FROM knowledge_records LIMIT 1", [], |row| row.get(0))
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

    let agents_exist: Option<String> = connection
        .query_row("SELECT id FROM agents LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if agents_exist.is_none() {
        for record in default_agent_records() {
            connection
                .execute(
                    "INSERT INTO agents (id, workspace_id, project_id, scope, name, title, description, status, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.scope,
                        record.name,
                        record.title,
                        record.description,
                        record.status,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let teams_exist: Option<String> = connection
        .query_row("SELECT id FROM teams LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if teams_exist.is_none() {
        for record in default_team_records() {
            connection
                .execute(
                    "INSERT INTO teams (id, workspace_id, project_id, scope, name, description, status, member_ids, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.scope,
                        record.name,
                        record.description,
                        record.status,
                        serde_json::to_string(&record.member_ids)?,
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
        .query_row("SELECT id FROM provider_credentials LIMIT 1", [], |row| row.get(0))
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

    let automations_exist: Option<String> = connection
        .query_row("SELECT id FROM automations LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if automations_exist.is_none() {
        for record in default_automation_records() {
            connection
                .execute(
                    "INSERT INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.description,
                        record.cadence,
                        record.owner_type,
                        record.owner_id,
                        record.status,
                        record.next_run_at.map(|value| value as i64),
                        record.last_run_at.map(|value| value as i64),
                        record.output,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

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

    let menus_exist: Option<String> = connection
        .query_row("SELECT id FROM menus LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if menus_exist.is_none() {
        for record in default_menu_records() {
            connection
                .execute(
                    "INSERT INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
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

fn load_state(paths: WorkspacePaths) -> Result<InfraState, AppError> {
    let workspace_file: WorkspaceConfigFile = toml::from_str(&fs::read_to_string(
        &paths.workspace_config,
    )?)?;
    let workspace = WorkspaceSummary {
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
    let projects = load_projects(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let agents = load_agents(&connection)?;
    let teams = load_teams(&connection)?;
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

    Ok(InfraState {
        paths,
        workspace,
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
        resources: Mutex::new(resources),
        knowledge_records: Mutex::new(knowledge_records),
        agents: Mutex::new(agents),
        teams: Mutex::new(teams),
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
    })
}

fn load_users(connection: &Connection) -> Result<Vec<StoredUser>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT u.id, u.username, u.display_name, u.status, u.password_hash, u.password_state, u.created_at, u.updated_at,
                    m.workspace_id, m.role_ids, m.scope_mode, m.scope_project_ids
             FROM users u
             LEFT JOIN memberships m ON m.user_id = u.id",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_ids_raw: String = row.get(9)?;
            let scope_project_ids_raw: String = row.get(11)?;
            Ok(StoredUser {
                record: UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    status: row.get(3)?,
                    password_state: row.get(5)?,
                    created_at: row.get::<_, i64>(6)? as u64,
                    updated_at: row.get::<_, i64>(7)? as u64,
                },
                password_hash: row.get(4)?,
                membership: WorkspaceMembershipRecord {
                    workspace_id: row.get(8)?,
                    user_id: row.get(0)?,
                    role_ids: serde_json::from_str(&role_ids_raw).unwrap_or_default(),
                    scope_mode: row.get(10)?,
                    scope_project_ids: serde_json::from_str(&scope_project_ids_raw)
                        .unwrap_or_default(),
                },
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT id, workspace_id, name, status, description FROM projects")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_resources(connection: &Connection) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags FROM resources",
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
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_knowledge_records(connection: &Connection) -> Result<Vec<KnowledgeRecord>, AppError> {
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

fn load_agents(connection: &Connection) -> Result<Vec<AgentRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, scope, name, title, description, status, updated_at FROM agents",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AgentRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                title: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                updated_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_teams(connection: &Connection) -> Result<Vec<TeamRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, scope, name, description, status, member_ids, updated_at FROM teams",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let member_ids_raw: String = row.get(7)?;
            Ok(TeamRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                description: row.get(5)?,
                status: row.get(6)?,
                member_ids: serde_json::from_str(&member_ids_raw).unwrap_or_default(),
                updated_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_model_catalog(connection: &Connection) -> Result<Vec<ModelCatalogRecord>, AppError> {
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

fn load_provider_credentials(connection: &Connection) -> Result<Vec<ProviderCredentialRecord>, AppError> {
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

fn load_tools(connection: &Connection) -> Result<Vec<ToolRecord>, AppError> {
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

fn load_automations(connection: &Connection) -> Result<Vec<AutomationRecord>, AppError> {
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

fn load_roles(connection: &Connection) -> Result<Vec<RoleRecord>, AppError> {
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

fn load_permissions(connection: &Connection) -> Result<Vec<PermissionRecord>, AppError> {
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

fn load_menus(connection: &Connection) -> Result<Vec<MenuRecord>, AppError> {
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

fn load_sessions(connection: &Connection) -> Result<Vec<SessionRecord>, AppError> {
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
                scope_project_ids: serde_json::from_str(&scope_project_ids_raw)
                    .unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn load_trace_events(connection: &Connection) -> Result<Vec<TraceEventRecord>, AppError> {
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

fn load_audit_records(connection: &Connection) -> Result<Vec<AuditRecord>, AppError> {
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

fn load_cost_entries(connection: &Connection) -> Result<Vec<CostLedgerEntry>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, run_id, metric, amount, unit, created_at
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
                metric: row.get(4)?,
                amount: row.get(5)?,
                unit: row.get(6)?,
                created_at: row.get::<_, i64>(7)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

fn default_workspace_resources() -> Vec<WorkspaceResourceRecord> {
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
        },
    ]
}

fn default_knowledge_records() -> Vec<KnowledgeRecord> {
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

fn default_agent_records() -> Vec<AgentRecord> {
    let now = timestamp_now();
    vec![
        AgentRecord {
            id: "agent-orchestrator".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            name: "Workspace Orchestrator".into(),
            title: "Route work across the workspace".into(),
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
            title: "Drive the default project".into(),
            description: "Tracks project work, runtime sessions, and follow-up actions.".into(),
            status: "active".into(),
            updated_at: now,
        },
    ]
}

fn default_team_records() -> Vec<TeamRecord> {
    let now = timestamp_now();
    vec![
        TeamRecord {
            id: "team-workspace-core".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            name: "Workspace Core".into(),
            description: "Maintains workspace-wide operating standards and governance.".into(),
            status: "active".into(),
            member_ids: vec!["user-owner".into(), "agent-orchestrator".into()],
            updated_at: now,
        },
    ]
}

fn default_model_catalog() -> Vec<ModelCatalogRecord> {
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

fn default_provider_credentials() -> Vec<ProviderCredentialRecord> {
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

fn default_tool_records() -> Vec<ToolRecord> {
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

fn default_automation_records() -> Vec<AutomationRecord> {
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

fn default_permission_records() -> Vec<PermissionRecord> {
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

fn default_menu_records() -> Vec<MenuRecord> {
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
            id: "menu-workspace-resources".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Resources".into(),
            route_name: Some("workspace-resources".into()),
            status: "active".into(),
            order: 20,
        },
        MenuRecord {
            id: "menu-workspace-knowledge".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Knowledge".into(),
            route_name: Some("workspace-knowledge".into()),
            status: "active".into(),
            order: 30,
        },
        MenuRecord {
            id: "menu-workspace-agents".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Agents".into(),
            route_name: Some("workspace-agents".into()),
            status: "active".into(),
            order: 40,
        },
        MenuRecord {
            id: "menu-workspace-teams".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Teams".into(),
            route_name: Some("workspace-teams".into()),
            status: "active".into(),
            order: 50,
        },
        MenuRecord {
            id: "menu-workspace-models".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Models".into(),
            route_name: Some("workspace-models".into()),
            status: "active".into(),
            order: 60,
        },
        MenuRecord {
            id: "menu-workspace-tools".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Tools".into(),
            route_name: Some("workspace-tools".into()),
            status: "active".into(),
            order: 70,
        },
        MenuRecord {
            id: "menu-workspace-automations".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "Automations".into(),
            route_name: Some("workspace-automations".into()),
            status: "active".into(),
            order: 80,
        },
        MenuRecord {
            id: "menu-workspace-user-center".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: None,
            source: "main-sidebar".into(),
            label: "User Center".into(),
            route_name: Some("workspace-user-center".into()),
            status: "active".into(),
            order: 90,
        },
        MenuRecord {
            id: "menu-workspace-user-center-profile".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Profile".into(),
            route_name: Some("workspace-user-center-profile".into()),
            status: "active".into(),
            order: 100,
        },
        MenuRecord {
            id: "menu-workspace-user-center-users".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Users".into(),
            route_name: Some("workspace-user-center-users".into()),
            status: "active".into(),
            order: 110,
        },
        MenuRecord {
            id: "menu-workspace-user-center-roles".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Roles".into(),
            route_name: Some("workspace-user-center-roles".into()),
            status: "active".into(),
            order: 120,
        },
        MenuRecord {
            id: "menu-workspace-user-center-permissions".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Permissions".into(),
            route_name: Some("workspace-user-center-permissions".into()),
            status: "active".into(),
            order: 130,
        },
        MenuRecord {
            id: "menu-workspace-user-center-menus".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            parent_id: Some("menu-workspace-user-center".into()),
            source: "user-center".into(),
            label: "Menus".into(),
            route_name: Some("workspace-user-center-menus".into()),
            status: "active".into(),
            order: 140,
        },
    ]
}

fn default_role_records() -> Vec<RoleRecord> {
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
        menu_ids: default_menu_records().into_iter().map(|record| record.id).collect(),
    }]
}

fn to_user_summary(user: &StoredUser) -> UserRecordSummary {
    UserRecordSummary {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        status: user.record.status.clone(),
        role_ids: user.membership.role_ids.clone(),
        scope_project_ids: user.membership.scope_project_ids.clone(),
    }
}

fn default_client_apps() -> Vec<ClientAppRecord> {
    vec![
        ClientAppRecord {
            id: "octopus-desktop".into(),
            name: "Octopus Desktop".into(),
            platform: "desktop".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-web".into(),
            name: "Octopus Web".into(),
            platform: "web".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: vec!["http://127.0.0.1".into(), "http://localhost".into()],
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-mobile".into(),
            name: "Octopus Mobile".into(),
            platform: "mobile".into(),
            status: "disabled".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: Vec::new(),
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into()],
        },
    ]
}

fn hash_password(password: &str) -> String {
    format!("plain::{password}")
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash == hash_password(password)
}

fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    raw.push_str(&serde_json::to_string(value)?);
    raw.push('\n');
    fs::write(path, raw)?;
    Ok(())
}

impl InfraWorkspaceService {
    fn now() -> u64 {
        timestamp_now()
    }

    fn replace_or_push<T, F>(items: &mut Vec<T>, value: T, matcher: F)
    where
        F: Fn(&T) -> bool,
    {
        if let Some(existing) = items.iter_mut().find(|item| matcher(item)) {
            *existing = value;
        } else {
            items.push(value);
        }
    }
}

#[async_trait]
impl WorkspaceService for InfraWorkspaceService {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError> {
        Ok(SystemBootstrapStatus {
            workspace: self.state.workspace.clone(),
            setup_required: self.state.workspace.bootstrap_status == "setup_required",
            owner_ready: self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("workspace users mutex poisoned"))?
                .iter()
                .any(|user| user.record.id == "user-owner"),
            registered_apps: self
                .state
                .apps
                .lock()
                .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
                .clone(),
            protocol_version: "2026-04-06".into(),
            api_base_path: "/api/v1".into(),
            transport_security: "loopback".into(),
            auth_mode: "session-token".into(),
            capabilities: octopus_core::WorkspaceCapabilitySet {
                polling: true,
                sse: true,
                idempotency: true,
                reconnect: true,
                event_replay: true,
            },
        })
    }

    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError> {
        Ok(self.state.workspace.clone())
    }

    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError> {
        Ok(self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone())
    }

    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .clone())
    }

    async fn list_project_resources(&self, project_id: &str) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .clone())
    }

    async fn list_project_knowledge(&self, project_id: &str) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError> {
        Ok(self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .clone())
    }

    async fn create_agent(&self, mut record: AgentRecord) -> Result<AgentRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("agent-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT INTO agents (id, workspace_id, project_id, scope, name, title, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.title,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut agents = self.state.agents.lock().map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        agents.push(record.clone());
        Ok(record)
    }

    async fn update_agent(&self, agent_id: &str, mut record: AgentRecord) -> Result<AgentRecord, AppError> {
        record.id = agent_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, title, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.title,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut agents = self.state.agents.lock().map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        Self::replace_or_push(&mut agents, record.clone(), |item| item.id == agent_id);
        Ok(record)
    }

    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError> {
        self.state.open_db()?.execute("DELETE FROM agents WHERE id = ?1", params![agent_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state.agents.lock().map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .retain(|item| item.id != agent_id);
        Ok(())
    }

    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError> {
        Ok(self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .clone())
    }

    async fn create_team(&self, mut record: TeamRecord) -> Result<TeamRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("team-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT INTO teams (id, workspace_id, project_id, scope, name, description, status, member_ids, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.description,
                record.status,
                serde_json::to_string(&record.member_ids)?,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut teams = self.state.teams.lock().map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        teams.push(record.clone());
        Ok(record)
    }

    async fn update_team(&self, team_id: &str, mut record: TeamRecord) -> Result<TeamRecord, AppError> {
        record.id = team_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO teams (id, workspace_id, project_id, scope, name, description, status, member_ids, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.description,
                record.status,
                serde_json::to_string(&record.member_ids)?,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut teams = self.state.teams.lock().map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        Self::replace_or_push(&mut teams, record.clone(), |item| item.id == team_id);
        Ok(record)
    }

    async fn delete_team(&self, team_id: &str) -> Result<(), AppError> {
        self.state.open_db()?.execute("DELETE FROM teams WHERE id = ?1", params![team_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state.teams.lock().map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .retain(|item| item.id != team_id);
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError> {
        Ok(self
            .state
            .model_catalog
            .lock()
            .map_err(|_| AppError::runtime("model catalog mutex poisoned"))?
            .clone())
    }

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        Ok(self
            .state
            .provider_credentials
            .lock()
            .map_err(|_| AppError::runtime("provider credentials mutex poisoned"))?
            .clone())
    }

    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError> {
        Ok(self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .clone())
    }

    async fn create_tool(&self, mut record: ToolRecord) -> Result<ToolRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("tool-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
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
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self.state.tools.lock().map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        tools.push(record.clone());
        Ok(record)
    }

    async fn update_tool(&self, tool_id: &str, mut record: ToolRecord) -> Result<ToolRecord, AppError> {
        record.id = tool_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
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
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self.state.tools.lock().map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        Self::replace_or_push(&mut tools, record.clone(), |item| item.id == tool_id);
        Ok(record)
    }

    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError> {
        self.state.open_db()?.execute("DELETE FROM tools WHERE id = ?1", params![tool_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state.tools.lock().map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .retain(|item| item.id != tool_id);
        Ok(())
    }

    async fn list_automations(&self) -> Result<Vec<AutomationRecord>, AppError> {
        Ok(self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .clone())
    }

    async fn create_automation(&self, mut record: AutomationRecord) -> Result<AutomationRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("automation-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self.state.automations.lock().map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        automations.push(record.clone());
        Ok(record)
    }

    async fn update_automation(&self, automation_id: &str, mut record: AutomationRecord) -> Result<AutomationRecord, AppError> {
        record.id = automation_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self.state.automations.lock().map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        Self::replace_or_push(&mut automations, record.clone(), |item| item.id == automation_id);
        Ok(record)
    }

    async fn delete_automation(&self, automation_id: &str) -> Result<(), AppError> {
        self.state.open_db()?.execute("DELETE FROM automations WHERE id = ?1", params![automation_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state.automations.lock().map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .retain(|item| item.id != automation_id);
        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<UserRecordSummary>, AppError> {
        Ok(self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .map(to_user_summary)
            .collect())
    }

    async fn create_user(&self, mut record: UserRecordSummary) -> Result<UserRecordSummary, AppError> {
        if record.id.is_empty() {
            record.id = format!("user-{}", Uuid::new_v4());
        }
        let now = Self::now();
        let user_record = UserRecord {
            id: record.id.clone(),
            username: record.username.clone(),
            display_name: record.display_name.clone(),
            status: record.status.clone(),
            password_state: "reset-required".into(),
            created_at: now,
            updated_at: now,
        };
        let membership = WorkspaceMembershipRecord {
            workspace_id: self.state.workspace.id.clone(),
            user_id: record.id.clone(),
            role_ids: record.role_ids.clone(),
            scope_mode: if record.scope_project_ids.is_empty() {
                "all-projects".into()
            } else {
                "selected-projects".into()
            },
            scope_project_ids: record.scope_project_ids.clone(),
        };

        self.state.open_db()?.execute(
            "INSERT INTO users (id, username, display_name, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                user_record.id,
                user_record.username,
                user_record.display_name,
                user_record.status,
                hash_password("changeme"),
                user_record.password_state,
                user_record.created_at as i64,
                user_record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                membership.workspace_id,
                membership.user_id,
                serde_json::to_string(&membership.role_ids)?,
                membership.scope_mode,
                serde_json::to_string(&membership.scope_project_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut users = self.state.users.lock().map_err(|_| AppError::runtime("users mutex poisoned"))?;
        users.push(StoredUser {
            record: user_record,
            password_hash: hash_password("changeme"),
            membership,
        });
        Ok(record)
    }

    async fn update_user(&self, user_id: &str, mut record: UserRecordSummary) -> Result<UserRecordSummary, AppError> {
        record.id = user_id.into();
        let now = Self::now();

        self.state.open_db()?.execute(
            "UPDATE users SET username = ?2, display_name = ?3, status = ?4, updated_at = ?5 WHERE id = ?1",
            params![
                user_id,
                record.username,
                record.display_name,
                record.status,
                now as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                self.state.workspace.id,
                user_id,
                serde_json::to_string(&record.role_ids)?,
                if record.scope_project_ids.is_empty() { "all-projects" } else { "selected-projects" },
                serde_json::to_string(&record.scope_project_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut users = self.state.users.lock().map_err(|_| AppError::runtime("users mutex poisoned"))?;
        if let Some(existing) = users.iter_mut().find(|item| item.record.id == user_id) {
            existing.record.username = record.username.clone();
            existing.record.display_name = record.display_name.clone();
            existing.record.status = record.status.clone();
            existing.record.updated_at = now;
            existing.membership.role_ids = record.role_ids.clone();
            existing.membership.scope_project_ids = record.scope_project_ids.clone();
            existing.membership.scope_mode = if record.scope_project_ids.is_empty() {
                "all-projects".into()
            } else {
                "selected-projects".into()
            };
        }
        Ok(record)
    }

    async fn list_roles(&self) -> Result<Vec<RoleRecord>, AppError> {
        Ok(self
            .state
            .roles
            .lock()
            .map_err(|_| AppError::runtime("roles mutex poisoned"))?
            .clone())
    }

    async fn create_role(&self, mut record: RoleRecord) -> Result<RoleRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("role-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut roles = self.state.roles.lock().map_err(|_| AppError::runtime("roles mutex poisoned"))?;
        roles.push(record.clone());
        Ok(record)
    }

    async fn update_role(&self, role_id: &str, mut record: RoleRecord) -> Result<RoleRecord, AppError> {
        record.id = role_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO roles (id, workspace_id, name, code, description, status, permission_ids, menu_ids)
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut roles = self.state.roles.lock().map_err(|_| AppError::runtime("roles mutex poisoned"))?;
        Self::replace_or_push(&mut roles, record.clone(), |item| item.id == role_id);
        Ok(record)
    }

    async fn list_permissions(&self) -> Result<Vec<PermissionRecord>, AppError> {
        Ok(self
            .state
            .permissions
            .lock()
            .map_err(|_| AppError::runtime("permissions mutex poisoned"))?
            .clone())
    }

    async fn create_permission(&self, mut record: PermissionRecord) -> Result<PermissionRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("permission-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut permissions = self.state.permissions.lock().map_err(|_| AppError::runtime("permissions mutex poisoned"))?;
        permissions.push(record.clone());
        Ok(record)
    }

    async fn update_permission(&self, permission_id: &str, mut record: PermissionRecord) -> Result<PermissionRecord, AppError> {
        record.id = permission_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO permissions (id, workspace_id, name, code, description, status, kind, target_type, target_ids, action, member_permission_ids)
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut permissions = self.state.permissions.lock().map_err(|_| AppError::runtime("permissions mutex poisoned"))?;
        Self::replace_or_push(&mut permissions, record.clone(), |item| item.id == permission_id);
        Ok(record)
    }

    async fn list_menus(&self) -> Result<Vec<MenuRecord>, AppError> {
        Ok(self
            .state
            .menus
            .lock()
            .map_err(|_| AppError::runtime("menus mutex poisoned"))?
            .clone())
    }

    async fn create_menu(&self, mut record: MenuRecord) -> Result<MenuRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("menu-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut menus = self.state.menus.lock().map_err(|_| AppError::runtime("menus mutex poisoned"))?;
        menus.push(record.clone());
        Ok(record)
    }

    async fn update_menu(&self, menu_id: &str, mut record: MenuRecord) -> Result<MenuRecord, AppError> {
        record.id = menu_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace.id.clone();
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO menus (id, workspace_id, parent_id, source, label, route_name, status, order_value)
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
        ).map_err(|error| AppError::database(error.to_string()))?;
        let mut menus = self.state.menus.lock().map_err(|_| AppError::runtime("menus mutex poisoned"))?;
        Self::replace_or_push(&mut menus, record.clone(), |item| item.id == menu_id);
        Ok(record)
    }
}

#[async_trait]
impl AuthService for InfraAuthService {
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError> {
        let app = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == request.client_app_id)
            .cloned()
            .ok_or_else(|| AppError::auth("client app is not registered"))?;
        if app.status != "active" {
            return Err(AppError::auth("client app is disabled"));
        }

        let user = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .find(|user| user.record.username == request.username)
            .cloned()
            .ok_or_else(|| AppError::auth("invalid credentials"))?;
        if !verify_password(&request.password, &user.password_hash) {
            return Err(AppError::auth("invalid credentials"));
        }

        let session = SessionRecord {
            id: format!("sess-{}", Uuid::new_v4()),
            workspace_id: self.state.workspace.id.clone(),
            user_id: user.record.id.clone(),
            client_app_id: request.client_app_id,
            token: Uuid::new_v4().to_string(),
            status: "active".into(),
            created_at: timestamp_now(),
            expires_at: None,
            role_ids: user.membership.role_ids.clone(),
            scope_project_ids: user.membership.scope_project_ids.clone(),
        };

        self.state
            .open_db()?
            .execute(
                "INSERT INTO sessions (id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at, role_ids, scope_project_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session.id,
                    session.workspace_id,
                    session.user_id,
                    session.client_app_id,
                    session.token,
                    session.status,
                    session.created_at as i64,
                    session.expires_at.map(|value| value as i64),
                    serde_json::to_string(&session.role_ids)?,
                    serde_json::to_string(&session.scope_project_ids)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .push(session.clone());

        Ok(LoginResponse {
            session,
            workspace: self.state.workspace.clone(),
        })
    }

    async fn logout(&self, token: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE sessions SET status = 'revoked' WHERE token = ?1",
                params![token],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(session) = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter_mut()
            .find(|session| session.token == token)
        {
            session.status = "revoked".into();
        }

        Ok(())
    }

    async fn session(&self, token: &str) -> Result<SessionRecord, AppError> {
        self.lookup_session(token)
            .await?
            .ok_or_else(|| AppError::auth("session token is invalid"))
    }

    async fn lookup_session(&self, token: &str) -> Result<Option<SessionRecord>, AppError> {
        Ok(self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter()
            .find(|session| session.token == token && session.status == "active")
            .cloned())
    }
}

#[async_trait]
impl AppRegistryService for InfraAppRegistryService {
    async fn list_apps(&self) -> Result<Vec<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .clone())
    }

    async fn register_app(
        &self,
        record: ClientAppRecord,
    ) -> Result<ClientAppRecord, AppError> {
        if !record.first_party {
            return Err(AppError::invalid_input(
                "phase one only accepts first-party client apps",
            ));
        }

        self.state
            .open_db()?
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.name,
                    record.platform,
                    record.status,
                    if record.first_party { 1 } else { 0 },
                    serde_json::to_string(&record.allowed_origins)?,
                    serde_json::to_string(&record.allowed_hosts)?,
                    record.session_policy,
                    serde_json::to_string(&record.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut apps = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?;
        if let Some(existing) = apps.iter_mut().find(|app| app.id == record.id) {
            *existing = record.clone();
        } else {
            apps.push(record.clone());
        }
        let registry = AppRegistryFile { apps: apps.clone() };
        fs::write(
            &self.state.paths.app_registry_config,
            toml::to_string_pretty(&registry)?,
        )?;

        Ok(record)
    }

    async fn find_app(&self, app_id: &str) -> Result<Option<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == app_id)
            .cloned())
    }
}

#[async_trait]
impl RbacService for InfraRbacService {
    async fn authorize(
        &self,
        session: &SessionRecord,
        _capability: &str,
        project_id: Option<&str>,
    ) -> Result<AuthorizationDecision, AppError> {
        if session.role_ids.iter().any(|role| role == "owner") {
            return Ok(AuthorizationDecision {
                allowed: project_id
                    .map(|project| {
                        session.scope_project_ids.is_empty()
                            || session.scope_project_ids.iter().any(|item| item == project)
                    })
                    .unwrap_or(true),
                reason: None,
            });
        }

        Ok(AuthorizationDecision {
            allowed: false,
            reason: Some("no matching role permission".into()),
        })
    }
}

#[async_trait]
impl ArtifactService for InfraArtifactService {
    async fn list_artifacts(&self) -> Result<Vec<ArtifactRecord>, AppError> {
        Ok(self
            .state
            .artifacts
            .lock()
            .map_err(|_| AppError::runtime("artifacts mutex poisoned"))?
            .clone())
    }
}

#[async_trait]
impl InboxService for InfraInboxService {
    async fn list_inbox(&self) -> Result<Vec<InboxItemRecord>, AppError> {
        Ok(self
            .state
            .inbox
            .lock()
            .map_err(|_| AppError::runtime("inbox mutex poisoned"))?
            .clone())
    }
}

#[async_trait]
impl KnowledgeService for InfraKnowledgeService {
    async fn list_knowledge(&self) -> Result<Vec<KnowledgeEntryRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .map(|record| KnowledgeEntryRecord {
                id: record.id.clone(),
                workspace_id: record.workspace_id.clone(),
                project_id: record.project_id.clone(),
                title: record.title.clone(),
                scope: if record.project_id.is_some() {
                    "project".into()
                } else {
                    "workspace".into()
                },
                status: record.status.clone(),
                source_type: record.source_type.clone(),
                source_ref: record.source_ref.clone(),
                updated_at: record.updated_at,
            })
            .collect())
    }
}

#[async_trait]
impl ObservationService for InfraObservationService {
    async fn list_trace_events(&self) -> Result<Vec<TraceEventRecord>, AppError> {
        Ok(self
            .state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .clone())
    }

    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, AppError> {
        Ok(self
            .state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .clone())
    }

    async fn list_cost_entries(&self) -> Result<Vec<CostLedgerEntry>, AppError> {
        Ok(self
            .state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .clone())
    }

    async fn append_trace(&self, record: TraceEventRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO trace_events (id, workspace_id, project_id, run_id, session_id, event_kind, title, detail, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.session_id,
                    record.event_kind,
                    record.title,
                    record.detail,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self.state.paths.runtime_traces_dir.join("trace-events.jsonl"),
            &record,
        )?;
        self.state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_audit(&self, record: AuditRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO audit_records (id, workspace_id, project_id, actor_type, actor_id, action, resource, outcome, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.actor_type,
                    record.actor_id,
                    record.action,
                    record.resource,
                    record.outcome,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self.state.paths.audit_log_dir.join("audit-records.jsonl"),
            &record,
        )?;
        self.state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_cost(&self, record: CostLedgerEntry) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO cost_entries (id, workspace_id, project_id, run_id, metric, amount, unit, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.metric,
                    record.amount,
                    record.unit,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self.state.paths.server_log_dir.join("cost-ledger.jsonl"),
            &record,
        )?;
        self.state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .push(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{build_infra_bundle, initialize_workspace, WorkspacePaths};
    use octopus_platform::WorkspaceService;

    #[test]
    fn workspace_initialization_creates_expected_layout_and_defaults() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = initialize_workspace(temp.path()).expect("workspace initialized");

        for path in [
            &paths.config_dir,
            &paths.data_dir,
            &paths.runtime_dir,
            &paths.logs_dir,
            &paths.tmp_dir,
            &paths.blobs_dir,
            &paths.artifacts_dir,
            &paths.knowledge_dir,
            &paths.inbox_dir,
            &paths.runtime_sessions_dir,
            &paths.runtime_events_dir,
            &paths.runtime_traces_dir,
            &paths.runtime_approvals_dir,
            &paths.runtime_cache_dir,
            &paths.audit_log_dir,
            &paths.server_log_dir,
        ] {
            assert!(path.exists(), "missing {}", path.display());
        }
        assert!(paths.workspace_config.exists());
        assert!(paths.app_registry_config.exists());
        assert!(paths.db_path.exists());

        let workspace_toml =
            std::fs::read_to_string(&paths.workspace_config).expect("workspace toml");
        assert!(workspace_toml.contains("listen_address = \"127.0.0.1\""));
        assert!(workspace_toml.contains("bootstrap_status = \"setup_required\""));
    }

    #[test]
    fn bundle_exposes_bootstrap_owner_and_registered_apps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let bootstrap = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.system_bootstrap())
            .expect("bootstrap");

        assert!(bootstrap.setup_required);
        assert!(bootstrap.owner_ready);
        assert!(bootstrap
            .registered_apps
            .iter()
            .any(|app| app.id == "octopus-desktop"));
    }

    #[test]
    fn workspace_paths_follow_unified_workspace_layout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());

        assert_eq!(paths.runtime_sessions_dir, temp.path().join("runtime/sessions"));
        assert_eq!(paths.runtime_events_dir, temp.path().join("runtime/events"));
        assert_eq!(paths.audit_log_dir, temp.path().join("logs/audit"));
        assert_eq!(paths.db_path, temp.path().join("data/main.db"));
    }
}
