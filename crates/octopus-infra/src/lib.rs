use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    timestamp_now, AppError, ArtifactRecord, AuditRecord, AuthorizationDecision,
    ClientAppRecord, CostLedgerEntry, InboxItemRecord, KnowledgeEntryRecord,
    LoginRequest, LoginResponse, ProjectRecord, SessionRecord, SystemBootstrapStatus,
    TraceEventRecord, UserRecord, WorkspaceMembershipRecord, WorkspaceSummary,
    DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
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
    artifacts: Mutex<Vec<ArtifactRecord>>,
    inbox: Mutex<Vec<InboxItemRecord>>,
    knowledge: Mutex<Vec<KnowledgeEntryRecord>>,
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
        artifacts: Mutex::new(Vec::new()),
        inbox: Mutex::new(Vec::new()),
        knowledge: Mutex::new(Vec::new()),
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
            .knowledge
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .clone())
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
        assert_eq!(paths.audit_log_dir, temp.path().join("logs/audit"));
        assert_eq!(paths.db_path, temp.path().join("data/main.db"));
    }
}
