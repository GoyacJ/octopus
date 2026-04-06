use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_WORKSPACE_ID: &str = "ws-local";
pub const DEFAULT_PROJECT_ID: &str = "proj-redesign";

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

impl AppError {
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

pub trait PreferencesPort: Send + Sync {
    fn load_preferences(&self) -> Result<ShellPreferences, AppError>;
    fn save_preferences(&self, preferences: &ShellPreferences)
        -> Result<ShellPreferences, AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostState {
    pub platform: String,
    pub mode: String,
    pub app_version: String,
    pub cargo_workspace: bool,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackendConnection {
    pub base_url: Option<String>,
    pub auth_token: Option<String>,
    pub state: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckBackendStatus {
    pub state: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckStatus {
    pub status: String,
    pub host: String,
    pub mode: String,
    pub cargo_workspace: bool,
    pub backend: HealthcheckBackendStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellPreferences {
    pub theme: String,
    pub locale: String,
    pub compact_sidebar: bool,
    pub left_sidebar_collapsed: bool,
    pub right_sidebar_collapsed: bool,
    pub default_workspace_id: String,
    pub last_visited_route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub mode: String,
    pub label: String,
    pub workspace_id: String,
    pub base_url: Option<String>,
    pub state: String,
    pub last_sync_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub client_app_id: String,
    pub username: String,
    pub password: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session: SessionRecord,
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppRecord {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub status: String,
    pub first_party: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub session_policy: String,
    pub default_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub password_state: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMembershipRecord {
    pub workspace_id: String,
    pub user_id: String,
    pub role_ids: Vec<String>,
    pub scope_mode: String,
    pub scope_project_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub client_app_id: String,
    pub token: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub role_ids: Vec<String>,
    pub scope_project_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub deployment: String,
    pub bootstrap_status: String,
    pub owner_user_id: Option<String>,
    pub host: String,
    pub listen_address: String,
    pub default_project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub status: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub session_id: String,
    pub status: String,
    pub current_step: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub model_id: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionSummary {
    pub id: String,
    pub conversation_id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub updated_at: u64,
    pub last_message_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRunSnapshot {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub status: String,
    pub current_step: String,
    pub started_at: u64,
    pub updated_at: u64,
    pub model_id: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMessage {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub sender_type: String,
    pub sender_label: String,
    pub content: String,
    pub timestamp: u64,
    pub model_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTraceItem {
    pub id: String,
    pub session_id: String,
    pub run_id: String,
    pub conversation_id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub tone: String,
    pub timestamp: u64,
    pub actor: String,
    pub related_message_id: Option<String>,
    pub related_tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub tool_name: String,
    pub summary: String,
    pub detail: String,
    pub risk_level: String,
    pub created_at: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventEnvelope {
    pub id: String,
    pub kind: String,
    pub session_id: String,
    pub conversation_id: String,
    pub run_id: Option<String>,
    pub emitted_at: u64,
    pub run: Option<RuntimeRunSnapshot>,
    pub message: Option<RuntimeMessage>,
    pub trace: Option<RuntimeTraceItem>,
    pub approval: Option<ApprovalRequestRecord>,
    pub decision: Option<String>,
    pub summary: Option<RuntimeSessionSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionDetail {
    pub summary: RuntimeSessionSummary,
    pub run: RuntimeRunSnapshot,
    pub messages: Vec<RuntimeMessage>,
    pub trace: Vec<RuntimeTraceItem>,
    pub pending_approval: Option<ApprovalRequestRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBootstrap {
    pub provider: ProviderConfig,
    pub sessions: Vec<RuntimeSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeSessionInput {
    pub conversation_id: String,
    pub project_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRuntimeTurnInput {
    pub content: String,
    pub model_id: String,
    pub permission_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeApprovalInput {
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub status: String,
    pub latest_version: u32,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntryRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub scope: String,
    pub status: String,
    pub source_type: String,
    pub source_ref: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub item_type: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceEventRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub event_kind: String,
    pub title: String,
    pub detail: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub actor_type: String,
    pub actor_id: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CostLedgerEntry {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub metric: String,
    pub amount: i64,
    pub unit: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemBootstrapStatus {
    pub workspace: WorkspaceSummary,
    pub setup_required: bool,
    pub owner_ready: bool,
    pub registered_apps: Vec<ClientAppRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

pub fn timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn default_host_state(app_version: String, cargo_workspace: bool) -> HostState {
    HostState {
        platform: "tauri".into(),
        mode: "local".into(),
        app_version,
        cargo_workspace,
        shell: "tauri2".into(),
    }
}

pub fn default_preferences(
    default_workspace_id: impl Into<String>,
    default_project_id: impl Into<String>,
) -> ShellPreferences {
    let workspace_id = default_workspace_id.into();
    let project_id = default_project_id.into();

    ShellPreferences {
        theme: "system".into(),
        locale: "zh-CN".into(),
        compact_sidebar: false,
        left_sidebar_collapsed: false,
        right_sidebar_collapsed: false,
        default_workspace_id: workspace_id.clone(),
        last_visited_route: format!(
            "/workspaces/{workspace_id}/overview?project={project_id}"
        ),
    }
}

pub fn default_connection_stubs() -> Vec<ConnectionProfile> {
    vec![ConnectionProfile {
        id: "conn-local".into(),
        mode: "local".into(),
        label: "Local Runtime".into(),
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        base_url: None,
        state: "local-ready".into(),
        last_sync_at: None,
    }]
}
