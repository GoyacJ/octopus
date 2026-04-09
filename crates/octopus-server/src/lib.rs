use std::{
    collections::HashMap,
    env, fs,
    path::{Path as StdPath, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_stream::stream;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use octopus_core::{
    connection_profile_from_host_workspace_connection, create_default_notification_unread_summary,
    default_host_update_status, host_workspace_connection_record_from_profile,
    normalize_connection_base_url, normalize_notification_filter_scope,
    normalize_runtime_permission_mode_label, notification_list_response_from_records,
    timestamp_now, AgentRecord, ApiErrorDetail, ApiErrorEnvelope, AppError, AutomationRecord,
    ChangeCurrentUserPasswordRequest, ChangeCurrentUserPasswordResponse, ClientAppRecord,
    ConnectionProfile, ConversationRecord, CopyWorkspaceSkillToManagedInput,
    CreateHostWorkspaceConnectionInput, CreateNotificationInput, CreateProjectRequest,
    CreateWorkspaceResourceFolderInput, CreateWorkspaceResourceInput, CreateWorkspaceSkillInput,
    CreateWorkspaceUserRequest, DesktopBackendConnection, HealthcheckBackendStatus,
    HealthcheckStatus, HostReleaseSummary, HostState, HostUpdateStatus,
    HostWorkspaceConnectionRecord, ImportWorkspaceAgentBundleInput,
    ImportWorkspaceAgentBundlePreview, ImportWorkspaceAgentBundlePreviewInput,
    ImportWorkspaceAgentBundleResult, ImportWorkspaceSkillArchiveInput,
    ImportWorkspaceSkillFolderInput, KnowledgeRecord, LoginRequest, MenuRecord,
    ModelCatalogSnapshot, NotificationFilter, NotificationListResponse, NotificationRecord,
    NotificationUnreadSummary, PermissionRecord, PetConversationBinding, PetPresenceState,
    PetWorkspaceSnapshot, ProjectAgentLinkInput, ProjectAgentLinkRecord, ProjectDashboardSnapshot,
    ProjectRecord, ProjectTeamLinkInput, ProjectTeamLinkRecord, ProviderCredentialRecord,
    RegisterWorkspaceOwnerRequest, RegisterWorkspaceOwnerResponse, ResolveRuntimeApprovalInput,
    RoleRecord, RuntimeConfigPatch, RuntimeConfigValidationResult,
    RuntimeConfiguredModelProbeInput, RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig,
    SavePetPresenceInput, SessionRecord, ShellBootstrap, ShellPreferences, SubmitRuntimeTurnInput,
    TeamRecord, ToolRecord, UpdateCurrentUserProfileRequest, UpdateProjectRequest,
    UpdateWorkspaceResourceInput, UpdateWorkspaceSkillFileInput, UpdateWorkspaceSkillInput,
    UpdateWorkspaceUserRequest, UpsertAgentInput, UpsertTeamInput, UpsertWorkspaceMcpServerInput,
    UserCenterAlertRecord, UserCenterOverviewSnapshot, UserRecordSummary, WorkspaceActivityRecord,
    WorkspaceMcpServerDocument, WorkspaceMetricRecord, WorkspaceOverviewSnapshot,
    WorkspaceResourceRecord, WorkspaceSkillDocument, WorkspaceSkillFileDocument,
    WorkspaceSkillTreeDocument, WorkspaceToolCatalogSnapshot, WorkspaceToolDisablePatch,
};
use octopus_platform::PlatformServices;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct ServerState {
    pub services: PlatformServices,
    pub host_auth_token: String,
    pub transport_security: String,
    pub idempotency_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub host_state: HostState,
    pub host_connections: Vec<ConnectionProfile>,
    pub host_preferences_path: PathBuf,
    pub host_workspace_connections_path: PathBuf,
    pub host_default_preferences: ShellPreferences,
    pub backend_connection: DesktopBackendConnection,
}

#[derive(Debug)]
struct ApiError {
    source: AppError,
    request_id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostUpdateCheckRequestPayload {
    channel: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductUpdateConfig {
    formal_endpoint: Option<String>,
    preview_endpoint: Option<String>,
    pubkey: Option<String>,
    #[serde(rename = "releaseRepo")]
    _release_repo: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct RemoteUpdateManifest {
    version: Option<String>,
    notes: Option<String>,
    #[serde(rename = "pub_date")]
    pub_date: Option<String>,
    channel: Option<String>,
    #[serde(alias = "notes_url", alias = "notesUrl")]
    notes_url: Option<String>,
}

const UPDATE_ENDPOINT_FORMAL_ENV: &str = "OCTOPUS_UPDATE_ENDPOINT_FORMAL";
const UPDATE_ENDPOINT_PREVIEW_ENV: &str = "OCTOPUS_UPDATE_ENDPOINT_PREVIEW";
const UPDATE_PUBKEY_ENV: &str = "OCTOPUS_UPDATE_PUBKEY";
const BUILTIN_UPDATER_CONFIG: &str =
    include_str!("../../../apps/desktop/src-tauri/updater.config.json");

impl ApiError {
    fn new(source: AppError, request_id: impl Into<String>) -> Self {
        Self {
            source,
            request_id: request_id.into(),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self::new(value, format!("req-{}", timestamp_now()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, retryable) = match &self.source {
            AppError::Auth(message) if message.contains("expired") => {
                (StatusCode::UNAUTHORIZED, "SESSION_EXPIRED", false)
            }
            AppError::Auth(message)
                if message.contains("access denied")
                    || message.contains("no matching role permission")
                    || message.contains("workspace scope mismatch") =>
            {
                (StatusCode::FORBIDDEN, "FORBIDDEN", false)
            }
            AppError::Auth(_) => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED", false),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", false),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", false),
            AppError::InvalidInput(_) => (StatusCode::BAD_REQUEST, "INVALID_INPUT", false),
            AppError::Database(_) | AppError::Runtime(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "UNAVAILABLE", true)
            }
            AppError::Io(_)
            | AppError::Json(_)
            | AppError::TomlDeserialize(_)
            | AppError::TomlSerialize(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", true)
            }
        };
        let body = Json(ApiErrorEnvelope {
            error: ApiErrorDetail {
                code: code.into(),
                message: self.source.to_string(),
                details: None,
                request_id: self.request_id.clone(),
                retryable,
            },
        });
        let mut response = (status, body).into_response();
        if let Ok(value) = HeaderValue::from_str(&self.request_id) {
            response
                .headers_mut()
                .insert(header::HeaderName::from_static("x-request-id"), value);
        }
        response
    }
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after: Option<String>,
}

const HEADER_REQUEST_ID: &str = "x-request-id";
const HEADER_WORKSPACE_ID: &str = "x-workspace-id";
const HEADER_IDEMPOTENCY_KEY: &str = "idempotency-key";
const HEADER_LAST_EVENT_ID: &str = "last-event-id";

fn build_cors_layer(transport_security: &str) -> CorsLayer {
    let allow_origin = if transport_security == "loopback" {
        AllowOrigin::predicate(|origin, _| is_allowed_loopback_origin(origin))
    } else {
        AllowOrigin::predicate(|_, _| false)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static(HEADER_LAST_EVENT_ID),
            HeaderName::from_static(HEADER_REQUEST_ID),
            HeaderName::from_static(HEADER_WORKSPACE_ID),
            HeaderName::from_static(HEADER_IDEMPOTENCY_KEY),
        ])
}

fn is_allowed_loopback_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };

    origin == "http://127.0.0.1"
        || origin.starts_with("http://127.0.0.1:")
        || origin == "http://localhost"
        || origin.starts_with("http://localhost:")
        || origin == "http://tauri.localhost"
        || origin.starts_with("http://tauri.localhost:")
        || origin == "http://[::1]"
        || origin.starts_with("http://[::1]:")
        || origin == "https://127.0.0.1"
        || origin.starts_with("https://127.0.0.1:")
        || origin == "https://localhost"
        || origin.starts_with("https://localhost:")
        || origin == "https://tauri.localhost"
        || origin.starts_with("https://tauri.localhost:")
        || origin == "https://[::1]"
        || origin.starts_with("https://[::1]:")
        || origin == "tauri://localhost"
}

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get(HEADER_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("req-{}", timestamp_now()))
}

fn insert_request_id(response: &mut Response, request_id: &str) {
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response
            .headers_mut()
            .insert(header::HeaderName::from_static(HEADER_REQUEST_ID), value);
    }
}

fn idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_IDEMPOTENCY_KEY)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn last_event_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_LAST_EVENT_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn workspace_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_WORKSPACE_ID)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn load_idempotent_response(
    state: &ServerState,
    scope: &str,
    request_id: &str,
) -> Result<Option<Response>, ApiError> {
    let cache = state.idempotency_cache.lock().map_err(|_| {
        ApiError::new(
            AppError::runtime("idempotency cache mutex poisoned"),
            request_id,
        )
    })?;
    let Some(body) = cache.get(scope).cloned() else {
        return Ok(None);
    };
    drop(cache);

    let mut response = Json(body).into_response();
    insert_request_id(&mut response, request_id);
    Ok(Some(response))
}

fn store_idempotent_response<T: serde::Serialize>(
    state: &ServerState,
    scope: &str,
    value: &T,
    request_id: &str,
) -> Result<(), ApiError> {
    let payload = serde_json::to_value(value)
        .map_err(|error| ApiError::new(AppError::Json(error), request_id))?;
    let mut cache = state.idempotency_cache.lock().map_err(|_| {
        ApiError::new(
            AppError::runtime("idempotency cache mutex poisoned"),
            request_id,
        )
    })?;
    cache.insert(scope.to_string(), payload);
    Ok(())
}

fn idempotency_scope(
    session: &SessionRecord,
    operation: &str,
    resource: &str,
    key: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        session.workspace_id,
        session.user_id,
        operation,
        format!("{resource}:{key}")
    )
}

pub fn build_router(state: ServerState) -> Router {
    let cors_layer = build_cors_layer(&state.transport_security);

    Router::new()
        .route("/health", get(healthcheck))
        .route("/api/v1/host/bootstrap", get(host_bootstrap))
        .route("/api/v1/host/health", get(host_healthcheck))
        .route(
            "/api/v1/host/preferences",
            get(load_host_preferences_route).put(save_host_preferences_route),
        )
        .route(
            "/api/v1/host/update-status",
            get(get_host_update_status_route),
        )
        .route("/api/v1/host/update-check", post(check_host_update_route))
        .route(
            "/api/v1/host/update-download",
            post(download_host_update_route),
        )
        .route(
            "/api/v1/host/update-install",
            post(install_host_update_route),
        )
        .route(
            "/api/v1/host/workspace-connections",
            get(list_host_workspace_connections_route).post(create_host_workspace_connection_route),
        )
        .route(
            "/api/v1/host/workspace-connections/:connection_id",
            delete(delete_host_workspace_connection_route),
        )
        .route(
            "/api/v1/host/notifications",
            get(list_host_notifications_route).post(create_host_notification_route),
        )
        .route(
            "/api/v1/host/notifications/:notification_id/read",
            post(mark_host_notification_read_route),
        )
        .route(
            "/api/v1/host/notifications/read-all",
            post(mark_all_host_notifications_read_route),
        )
        .route(
            "/api/v1/host/notifications/:notification_id/dismiss-toast",
            post(dismiss_host_notification_toast_route),
        )
        .route(
            "/api/v1/host/notifications/unread-summary",
            get(get_host_notification_unread_summary_route),
        )
        .route("/api/v1/system/health", get(healthcheck))
        .route("/api/v1/system/bootstrap", get(system_bootstrap))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register-owner", post(register_owner))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/apps", get(list_apps).post(register_app))
        .route("/api/v1/workspace", get(workspace))
        .route("/api/v1/workspace/overview", get(workspace_overview))
        .route(
            "/api/v1/workspace/resources",
            get(workspace_resources).post(create_workspace_resource),
        )
        .route(
            "/api/v1/workspace/resources/:resource_id",
            patch(update_workspace_resource).delete(delete_workspace_resource),
        )
        .route("/api/v1/workspace/knowledge", get(workspace_knowledge))
        .route("/api/v1/workspace/pet", get(workspace_pet_snapshot))
        .route(
            "/api/v1/workspace/pet/presence",
            patch(save_workspace_pet_presence),
        )
        .route(
            "/api/v1/workspace/pet/conversation",
            put(bind_workspace_pet_conversation),
        )
        .route(
            "/api/v1/workspace/agents",
            get(list_agents).post(create_agent),
        )
        .route(
            "/api/v1/workspace/agents/import-preview",
            post(preview_import_agent_bundle_route),
        )
        .route(
            "/api/v1/workspace/agents/import",
            post(import_agent_bundle_route),
        )
        .route(
            "/api/v1/workspace/agents/:agent_id",
            patch(update_agent).delete(delete_agent),
        )
        .route("/api/v1/workspace/teams", get(list_teams).post(create_team))
        .route(
            "/api/v1/workspace/teams/:team_id",
            patch(update_team).delete(delete_team),
        )
        .route(
            "/api/v1/workspace/catalog/models",
            get(workspace_catalog_models),
        )
        .route(
            "/api/v1/workspace/catalog/provider-credentials",
            get(workspace_provider_credentials),
        )
        .route(
            "/api/v1/workspace/catalog/tool-catalog",
            get(workspace_tool_catalog),
        )
        .route(
            "/api/v1/workspace/catalog/tool-catalog/disable",
            patch(workspace_tool_catalog_disable),
        )
        .route(
            "/api/v1/workspace/catalog/skills",
            post(create_workspace_skill_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id",
            get(get_workspace_skill_route)
                .patch(update_workspace_skill_route)
                .delete(delete_workspace_skill_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/import-archive",
            post(import_workspace_skill_archive_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/import-folder",
            post(import_workspace_skill_folder_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/tree",
            get(get_workspace_skill_tree_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/files/*relative_path",
            get(get_workspace_skill_file_route).patch(update_workspace_skill_file_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/copy-to-managed",
            post(copy_workspace_skill_to_managed_route),
        )
        .route(
            "/api/v1/workspace/catalog/mcp-servers",
            post(create_workspace_mcp_server_route),
        )
        .route(
            "/api/v1/workspace/catalog/mcp-servers/:server_name",
            get(get_workspace_mcp_server_route)
                .patch(update_workspace_mcp_server_route)
                .delete(delete_workspace_mcp_server_route),
        )
        .route(
            "/api/v1/workspace/catalog/tools",
            get(list_tools).post(create_tool),
        )
        .route(
            "/api/v1/workspace/catalog/tools/:tool_id",
            patch(update_tool).delete(delete_tool),
        )
        .route(
            "/api/v1/workspace/automations",
            get(list_automations).post(create_automation),
        )
        .route(
            "/api/v1/workspace/automations/:automation_id",
            patch(update_automation).delete(delete_automation),
        )
        .route(
            "/api/v1/workspace/user-center/overview",
            get(user_center_overview),
        )
        .route(
            "/api/v1/workspace/user-center/profile/runtime-config",
            get(get_user_runtime_config_route).patch(save_user_runtime_config_route),
        )
        .route(
            "/api/v1/workspace/user-center/profile/runtime-config/validate",
            post(validate_user_runtime_config_route),
        )
        .route(
            "/api/v1/workspace/user-center/profile",
            patch(update_current_user_profile_route),
        )
        .route(
            "/api/v1/workspace/user-center/profile/password",
            post(change_current_user_password_route),
        )
        .route(
            "/api/v1/workspace/rbac/users",
            get(list_users).post(create_user),
        )
        .route(
            "/api/v1/workspace/rbac/users/:user_id",
            patch(update_user).delete(delete_user),
        )
        .route(
            "/api/v1/workspace/rbac/roles",
            get(list_roles).post(create_role),
        )
        .route(
            "/api/v1/workspace/rbac/roles/:role_id",
            patch(update_role).delete(delete_role),
        )
        .route(
            "/api/v1/workspace/rbac/permissions",
            get(list_permissions).post(create_permission),
        )
        .route(
            "/api/v1/workspace/rbac/permissions/:permission_id",
            patch(update_permission).delete(delete_permission),
        )
        .route(
            "/api/v1/workspace/rbac/menus",
            get(list_menus).post(create_menu),
        )
        .route("/api/v1/workspace/rbac/menus/:menu_id", patch(update_menu))
        .route("/api/v1/projects", get(projects).post(create_project))
        .route("/api/v1/projects/:project_id", patch(update_project))
        .route(
            "/api/v1/projects/:project_id/dashboard",
            get(project_dashboard),
        )
        .route(
            "/api/v1/projects/:project_id/runtime-config",
            get(get_project_runtime_config_route).patch(save_project_runtime_config_route),
        )
        .route(
            "/api/v1/projects/:project_id/runtime-config/validate",
            post(validate_project_runtime_config_route),
        )
        .route(
            "/api/v1/projects/:project_id/resources",
            get(project_resources).post(create_project_resource),
        )
        .route(
            "/api/v1/projects/:project_id/resources/folder",
            post(create_project_resource_folder),
        )
        .route(
            "/api/v1/projects/:project_id/resources/:resource_id",
            patch(update_project_resource).delete(delete_project_resource),
        )
        .route(
            "/api/v1/projects/:project_id/knowledge",
            get(project_knowledge),
        )
        .route(
            "/api/v1/projects/:project_id/pet",
            get(project_pet_snapshot),
        )
        .route(
            "/api/v1/projects/:project_id/pet/presence",
            patch(save_project_pet_presence),
        )
        .route(
            "/api/v1/projects/:project_id/pet/conversation",
            put(bind_project_pet_conversation),
        )
        .route(
            "/api/v1/projects/:project_id/agent-links",
            get(list_project_agent_links).post(link_project_agent),
        )
        .route(
            "/api/v1/projects/:project_id/agent-links/:agent_id",
            delete(unlink_project_agent),
        )
        .route(
            "/api/v1/projects/:project_id/team-links",
            get(list_project_team_links).post(link_project_team),
        )
        .route(
            "/api/v1/projects/:project_id/team-links/:team_id",
            delete(unlink_project_team),
        )
        .route("/api/v1/inbox", get(inbox))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/knowledge", get(knowledge))
        .route("/api/v1/audit", get(audit))
        .nest("/api/v1/runtime", runtime_routes())
        .layer(cors_layer)
        .with_state(state)
}

fn runtime_routes() -> Router<ServerState> {
    Router::new()
        .route("/bootstrap", get(runtime_bootstrap))
        .route("/config", get(get_runtime_config))
        .route("/config/validate", post(validate_runtime_config_route))
        .route(
            "/config/configured-models/probe",
            post(probe_runtime_configured_model_route),
        )
        .route("/config/scopes/:scope", patch(save_runtime_config_route))
        .route(
            "/sessions",
            get(list_runtime_sessions).post(create_runtime_session),
        )
        .route(
            "/sessions/:session_id",
            get(get_runtime_session).delete(delete_runtime_session),
        )
        .route("/sessions/:session_id/turns", post(submit_runtime_turn))
        .route(
            "/sessions/:session_id/approvals/:approval_id",
            post(resolve_runtime_approval),
        )
        .route("/sessions/:session_id/events", get(runtime_events))
}

async fn healthcheck(
    State(state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    Ok(Json(build_healthcheck_status(&state)))
}

async fn system_bootstrap(
    State(state): State<ServerState>,
) -> Result<Json<octopus_core::SystemBootstrapStatus>, ApiError> {
    let mut payload = state.services.workspace.system_bootstrap().await?;
    payload.transport_security = state.transport_security.clone();
    Ok(Json(payload))
}

fn build_healthcheck_status(state: &ServerState) -> HealthcheckStatus {
    HealthcheckStatus {
        status: "ok".into(),
        host: state.host_state.platform.clone(),
        mode: state.host_state.mode.clone(),
        cargo_workspace: state.host_state.cargo_workspace,
        backend: HealthcheckBackendStatus {
            state: state.backend_connection.state.clone(),
            transport: state.backend_connection.transport.clone(),
        },
    }
}

fn load_host_preferences(state: &ServerState) -> Result<ShellPreferences, ApiError> {
    match fs::read_to_string(&state.host_preferences_path) {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|error| ApiError::from(AppError::from(error)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(state.host_default_preferences.clone())
        }
        Err(error) => Err(ApiError::from(AppError::from(error))),
    }
}

fn save_host_preferences(
    state: &ServerState,
    preferences: &ShellPreferences,
) -> Result<ShellPreferences, ApiError> {
    if let Some(parent) = state.host_preferences_path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }
    fs::write(
        &state.host_preferences_path,
        serde_json::to_vec_pretty(preferences)
            .map_err(|error| ApiError::from(AppError::from(error)))?,
    )
    .map_err(|error| ApiError::from(AppError::from(error)))?;
    Ok(preferences.clone())
}

fn normalize_host_update_channel(value: Option<&str>, fallback: &str) -> String {
    match value.map(str::trim) {
        Some("preview") => "preview".into(),
        Some("formal") => "formal".into(),
        _ => match fallback.trim() {
            "preview" => "preview".into(),
            _ => "formal".into(),
        },
    }
}

fn default_browser_host_update_status(state: &ServerState, channel: &str) -> HostUpdateStatus {
    let mut status = default_host_update_status(state.host_state.app_version.clone(), channel);
    let config = update_runtime_config(channel);
    status.capabilities.can_check = config.endpoint.is_some();
    status.capabilities.can_download = false;
    status.capabilities.can_install = false;
    status.capabilities.supports_channels = true;
    status
}

async fn load_host_update_status(
    state: &ServerState,
    requested_channel: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    refresh_browser_host_update_status(state, &channel).await
}

async fn check_host_update(
    state: &ServerState,
    requested_channel: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    refresh_browser_host_update_status(state, &channel).await
}

fn unsupported_host_update_action(
    state: &ServerState,
    requested_channel: Option<&str>,
    error_code: &str,
    error_message: &str,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    let mut status = default_browser_host_update_status(state, &channel);
    status.state = "error".into();
    status.error_code = Some(error_code.into());
    status.error_message = Some(error_message.into());
    Ok(status)
}

async fn refresh_browser_host_update_status(
    state: &ServerState,
    channel: &str,
) -> Result<HostUpdateStatus, ApiError> {
    let runtime_config = update_runtime_config(channel);
    refresh_browser_host_update_status_with_endpoint(
        state,
        channel,
        runtime_config.endpoint.as_deref(),
    )
    .await
}

async fn refresh_browser_host_update_status_with_endpoint(
    state: &ServerState,
    channel: &str,
    endpoint: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let mut status = default_browser_host_update_status(state, channel);
    let Some(endpoint) = endpoint else {
        return Ok(status);
    };

    status.last_checked_at = Some(timestamp_now());

    match fetch_remote_update_manifest(&endpoint).await {
        Ok(manifest) => {
            let latest_version = manifest
                .version
                .clone()
                .unwrap_or_else(|| state.host_state.app_version.clone());
            let latest_channel = manifest
                .channel
                .clone()
                .unwrap_or_else(|| normalize_host_update_channel(Some(channel), channel));
            status.latest_release = Some(HostReleaseSummary {
                version: latest_version.clone(),
                channel: latest_channel,
                published_at: manifest
                    .pub_date
                    .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
                notes: manifest.notes,
                notes_url: manifest.notes_url,
            });
            status.state = if latest_version == state.host_state.app_version {
                "up_to_date".into()
            } else {
                "update_available".into()
            };
            Ok(status)
        }
        Err(error) => {
            status.state = "error".into();
            status.error_code = Some("UPDATE_CHECK_FAILED".into());
            status.error_message = Some(format!("无法连接更新服务，请稍后重试。{error}"));
            Ok(status)
        }
    }
}

async fn fetch_remote_update_manifest(endpoint: &str) -> Result<RemoteUpdateManifest, AppError> {
    let response = Client::new()
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, "octopus-browser-host")
        .send()
        .await
        .map_err(|error| AppError::runtime(format!("failed to fetch update manifest: {error}")))?;
    let response = response
        .error_for_status()
        .map_err(|error| AppError::runtime(format!("update manifest request failed: {error}")))?;
    response
        .json::<RemoteUpdateManifest>()
        .await
        .map_err(|error| AppError::runtime(format!("failed to parse update manifest: {error}")))
}

fn update_runtime_config(channel: &str) -> UpdateRuntimeConfig {
    let built_in = load_product_update_config();
    UpdateRuntimeConfig {
        endpoint: env_var(update_endpoint_env(channel))
            .or_else(|| built_in.endpoint_for_channel(channel)),
        _pubkey: env_var(UPDATE_PUBKEY_ENV).or_else(|| built_in.pubkey()),
    }
}

#[derive(Clone, Default)]
struct UpdateRuntimeConfig {
    endpoint: Option<String>,
    _pubkey: Option<String>,
}

fn update_endpoint_env(channel: &str) -> &'static str {
    match normalize_host_update_channel(Some(channel), "formal").as_str() {
        "preview" => UPDATE_ENDPOINT_PREVIEW_ENV,
        _ => UPDATE_ENDPOINT_FORMAL_ENV,
    }
}

fn env_var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_product_update_config() -> ProductUpdateConfig {
    serde_json::from_str::<ProductUpdateConfig>(BUILTIN_UPDATER_CONFIG)
        .unwrap_or_default()
        .normalized()
}

impl ProductUpdateConfig {
    fn normalized(mut self) -> Self {
        self.formal_endpoint = normalize_optional_string(self.formal_endpoint);
        self.preview_endpoint = normalize_optional_string(self.preview_endpoint);
        self.pubkey = normalize_optional_string(self.pubkey);
        self._release_repo = normalize_optional_string(self._release_repo);
        self
    }

    fn endpoint_for_channel(&self, channel: &str) -> Option<String> {
        match normalize_host_update_channel(Some(channel), "formal").as_str() {
            "preview" => self.preview_endpoint.clone(),
            _ => self.formal_endpoint.clone(),
        }
    }

    fn pubkey(&self) -> Option<String> {
        self.pubkey.clone()
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn load_remote_host_workspace_connections(
    state: &ServerState,
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    match fs::read_to_string(&state.host_workspace_connections_path) {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|error| ApiError::from(AppError::from(error)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(error) => Err(ApiError::from(AppError::from(error))),
    }
}

fn save_remote_host_workspace_connections(
    state: &ServerState,
    connections: &[HostWorkspaceConnectionRecord],
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    if let Some(parent) = state.host_workspace_connections_path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }
    fs::write(
        &state.host_workspace_connections_path,
        serde_json::to_vec_pretty(connections)
            .map_err(|error| ApiError::from(AppError::from(error)))?,
    )
    .map_err(|error| ApiError::from(AppError::from(error)))?;
    Ok(connections.to_vec())
}

fn list_host_workspace_connections(
    state: &ServerState,
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    let mut connections = state
        .host_connections
        .iter()
        .map(|connection| {
            host_workspace_connection_record_from_profile(
                connection,
                Some(&state.backend_connection),
            )
        })
        .collect::<Vec<_>>();
    connections.extend(load_remote_host_workspace_connections(state)?);
    Ok(connections)
}

fn create_host_workspace_connection(
    state: &ServerState,
    input: CreateHostWorkspaceConnectionInput,
) -> Result<HostWorkspaceConnectionRecord, ApiError> {
    let mut existing_connections = load_remote_host_workspace_connections(state)?;
    let normalized_base_url = normalize_connection_base_url(&input.base_url);

    if let Some(existing) = existing_connections.iter_mut().find(|connection| {
        normalize_connection_base_url(&connection.base_url) == normalized_base_url
            && connection.workspace_id == input.workspace_id
    }) {
        existing.label = input.label;
        existing.base_url = normalized_base_url;
        existing.transport_security = input.transport_security;
        existing.auth_mode = input.auth_mode;
        existing.last_used_at = Some(timestamp_now());
        existing.status = "connected".into();
        let persisted = existing.clone();
        save_remote_host_workspace_connections(state, &existing_connections)?;
        return Ok(persisted);
    }

    let created = HostWorkspaceConnectionRecord {
        workspace_connection_id: format!("conn-remote-{}-{}", input.workspace_id, timestamp_now()),
        workspace_id: input.workspace_id,
        label: input.label,
        base_url: normalized_base_url,
        transport_security: input.transport_security,
        auth_mode: input.auth_mode,
        last_used_at: Some(timestamp_now()),
        status: "connected".into(),
    };
    existing_connections.push(created.clone());
    save_remote_host_workspace_connections(state, &existing_connections)?;
    Ok(created)
}

fn delete_host_workspace_connection(
    state: &ServerState,
    connection_id: &str,
) -> Result<(), ApiError> {
    if state
        .host_connections
        .iter()
        .any(|connection| connection.id == connection_id)
    {
        return Err(ApiError::from(AppError::invalid_input(
            "local workspace connection cannot be deleted",
        )));
    }

    let next_connections = load_remote_host_workspace_connections(state)?
        .into_iter()
        .filter(|connection| connection.workspace_connection_id != connection_id)
        .collect::<Vec<_>>();
    save_remote_host_workspace_connections(state, &next_connections)?;
    Ok(())
}

fn host_notifications_db_path(state: &ServerState) -> PathBuf {
    state
        .host_preferences_path
        .parent()
        .unwrap_or_else(|| StdPath::new("."))
        .join("data/main.db")
}

fn open_host_notifications_db(state: &ServerState) -> Result<Connection, ApiError> {
    let path = host_notifications_db_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }

    let connection = Connection::open(path)
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY NOT NULL,
                scope_kind TEXT NOT NULL,
                scope_owner_id TEXT,
                level TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                read_at INTEGER,
                toast_visible_until INTEGER,
                route_to TEXT,
                action_label TEXT
            );",
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    Ok(connection)
}

fn map_notification(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRecord> {
    Ok(NotificationRecord {
        id: row.get(0)?,
        scope_kind: row.get(1)?,
        scope_owner_id: row.get(2)?,
        level: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        source: row.get(6)?,
        created_at: row.get::<_, i64>(7)? as u64,
        read_at: row.get::<_, Option<i64>>(8)?.map(|value| value as u64),
        toast_visible_until: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
        route_to: row.get(10)?,
        action_label: row.get(11)?,
    })
}

fn list_host_notifications(
    state: &ServerState,
    filter: NotificationFilter,
) -> Result<NotificationListResponse, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let scope = normalize_notification_filter_scope(filter.scope.as_deref());
    let mut statement = if scope.is_some() {
        connection
            .prepare(
                "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 FROM notifications
                 WHERE scope_kind = ?1
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
    } else {
        connection
            .prepare(
                "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 FROM notifications
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
    };

    let mapped = if let Some(scope) = scope {
        statement.query_map(params![scope], map_notification)
    } else {
        statement.query_map([], map_notification)
    }
    .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    let notifications = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    Ok(notification_list_response_from_records(notifications))
}

fn create_host_notification(
    state: &ServerState,
    input: CreateNotificationInput,
) -> Result<NotificationRecord, ApiError> {
    let now = timestamp_now();
    let scope_kind = match input.scope_kind.trim() {
        "workspace" => "workspace",
        "user" => "user",
        _ => "app",
    };
    let notification = NotificationRecord {
        id: format!("notif-{}", Uuid::new_v4()),
        scope_kind: scope_kind.into(),
        scope_owner_id: input.scope_owner_id,
        level: if input.level.trim().is_empty() {
            "info".into()
        } else {
            input.level
        },
        title: if input.title.trim().is_empty() {
            "Notification".into()
        } else {
            input.title
        },
        body: input.body,
        source: if input.source.trim().is_empty() {
            "system".into()
        } else {
            input.source
        },
        created_at: now,
        read_at: None,
        toast_visible_until: input.toast_duration_ms.map(|duration| now + duration),
        route_to: input.route_to,
        action_label: input.action_label,
    };

    let connection = open_host_notifications_db(state)?;
    connection
        .execute(
            "INSERT INTO notifications (
                id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                notification.id,
                notification.scope_kind,
                notification.scope_owner_id,
                notification.level,
                notification.title,
                notification.body,
                notification.source,
                notification.created_at as i64,
                notification.read_at.map(|value| value as i64),
                notification.toast_visible_until.map(|value| value as i64),
                notification.route_to,
                notification.action_label,
            ],
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    get_host_notification(state, &notification.id)
}

fn get_host_notification(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .query_row(
            "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
             FROM notifications
             WHERE id = ?1",
            params![notification_id],
            map_notification,
        )
        .optional()
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("notification {notification_id} not found"))))
}

fn get_host_notification_unread_summary(
    state: &ServerState,
) -> Result<NotificationUnreadSummary, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let mut statement = connection
        .prepare("SELECT scope_kind, COUNT(*) FROM notifications WHERE read_at IS NULL GROUP BY scope_kind")
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    let counts = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    let mut summary = create_default_notification_unread_summary();
    for item in counts {
        let (scope, count) =
            item.map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
        let count = count.max(0) as u64;
        summary.total += count;
        match scope.as_str() {
            "workspace" => summary.by_scope.workspace += count,
            "user" => summary.by_scope.user += count,
            _ => summary.by_scope.app += count,
        }
    }

    Ok(summary)
}

fn mark_host_notification_read(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .execute(
            "UPDATE notifications
             SET read_at = COALESCE(read_at, ?2)
             WHERE id = ?1",
            params![notification_id, timestamp_now() as i64],
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    get_host_notification(state, notification_id)
}

fn mark_all_host_notifications_read(
    state: &ServerState,
    filter: NotificationFilter,
) -> Result<NotificationUnreadSummary, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let scope = normalize_notification_filter_scope(filter.scope.as_deref());
    if let Some(scope) = scope {
        connection
            .execute(
                "UPDATE notifications
                 SET read_at = COALESCE(read_at, ?2)
                 WHERE scope_kind = ?1",
                params![scope, timestamp_now() as i64],
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    } else {
        connection
            .execute(
                "UPDATE notifications
                 SET read_at = COALESCE(read_at, ?1)",
                params![timestamp_now() as i64],
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    }

    get_host_notification_unread_summary(state)
}

fn dismiss_host_notification_toast(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .execute(
            "UPDATE notifications
             SET toast_visible_until = NULL
             WHERE id = ?1",
            params![notification_id],
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    get_host_notification(state, notification_id)
}

fn ensure_host_authorized(
    state: &ServerState,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<(), ApiError> {
    let token = extract_bearer(headers)
        .ok_or_else(|| ApiError::new(AppError::auth("missing bearer token"), request_id))?;
    if token != state.host_auth_token {
        return Err(ApiError::new(
            AppError::auth("invalid bearer token"),
            request_id,
        ));
    }
    Ok(())
}

async fn host_bootstrap(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ShellBootstrap>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    let connections = list_host_workspace_connections(&state)?
        .iter()
        .map(connection_profile_from_host_workspace_connection)
        .collect::<Vec<_>>();

    Ok(Json(ShellBootstrap {
        host_state: state.host_state.clone(),
        preferences: load_host_preferences(&state)?,
        connections,
        backend: Some(state.backend_connection.clone()),
    }))
}

async fn host_healthcheck(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(build_healthcheck_status(&state)))
}

async fn load_host_preferences_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ShellPreferences>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(load_host_preferences(&state)?))
}

async fn save_host_preferences_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(preferences): Json<ShellPreferences>,
) -> Result<Json<ShellPreferences>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(save_host_preferences(&state, &preferences)?))
}

async fn get_host_update_status_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(load_host_update_status(&state, None).await?))
}

async fn check_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<HostUpdateCheckRequestPayload>,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(
        check_host_update(&state, request.channel.as_deref()).await?,
    ))
}

async fn download_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(unsupported_host_update_action(
        &state,
        None,
        "UPDATE_DOWNLOAD_UNSUPPORTED",
        "当前环境不支持应用内下载安装更新。",
    )?))
}

async fn install_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(unsupported_host_update_action(
        &state,
        None,
        "UPDATE_INSTALL_UNSUPPORTED",
        "当前环境不支持应用内安装更新。",
    )?))
}

async fn list_host_workspace_connections_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<HostWorkspaceConnectionRecord>>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(list_host_workspace_connections(&state)?))
}

async fn create_host_workspace_connection_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateHostWorkspaceConnectionInput>,
) -> Result<Json<HostWorkspaceConnectionRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(create_host_workspace_connection(&state, input)?))
}

async fn delete_host_workspace_connection_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(connection_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    delete_host_workspace_connection(&state, &connection_id)?;
    Ok(Json(()))
}

async fn list_host_notifications_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(filter): Query<NotificationFilter>,
) -> Result<Json<NotificationListResponse>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(list_host_notifications(&state, filter)?))
}

async fn create_host_notification_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateNotificationInput>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(create_host_notification(&state, input)?))
}

async fn mark_host_notification_read_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(mark_host_notification_read(&state, &notification_id)?))
}

async fn mark_all_host_notifications_read_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(filter): Json<NotificationFilter>,
) -> Result<Json<NotificationUnreadSummary>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(mark_all_host_notifications_read(&state, filter)?))
}

async fn dismiss_host_notification_toast_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(dismiss_host_notification_toast(
        &state,
        &notification_id,
    )?))
}

async fn get_host_notification_unread_summary_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<NotificationUnreadSummary>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(get_host_notification_unread_summary(&state)?))
}

async fn login(
    State(state): State<ServerState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<octopus_core::LoginResponse>, ApiError> {
    Ok(Json(state.services.auth.login(request).await?))
}

async fn register_owner(
    State(state): State<ServerState>,
    Json(request): Json<RegisterWorkspaceOwnerRequest>,
) -> Result<Json<RegisterWorkspaceOwnerResponse>, ApiError> {
    Ok(Json(state.services.auth.register_owner(request).await?))
}

async fn logout(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    state.services.auth.logout(&session.token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<SessionRecord>, ApiError> {
    Ok(Json(authenticate_session(&state, &headers).await?))
}

async fn list_apps(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ClientAppRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.read", None).await?;
    Ok(Json(state.services.app_registry.list_apps().await?))
}

async fn register_app(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(app): Json<ClientAppRecord>,
) -> Result<Json<ClientAppRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.write", None).await?;
    Ok(Json(state.services.app_registry.register_app(app).await?))
}

async fn workspace(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::WorkspaceSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.workspace_summary().await?))
}

async fn workspace_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceOverviewSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;

    let workspace = state.services.workspace.workspace_summary().await?;
    let projects = state.services.workspace.list_projects().await?;
    let conversations = list_conversation_records(&state, None).await?;
    let recent_activity = list_activity_records(&state, None).await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let agents = state.services.workspace.list_agents().await?;

    Ok(Json(WorkspaceOverviewSnapshot {
        workspace,
        metrics: vec![
            metric_record("projects", "Projects", projects.len()),
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        projects,
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

async fn projects(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ProjectRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_projects().await?))
}

fn validate_create_project_request(
    request: CreateProjectRequest,
) -> Result<CreateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    Ok(CreateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        assignments: request.assignments,
    })
}

fn validate_update_project_request(
    request: UpdateProjectRequest,
) -> Result<UpdateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    let status = request.status.trim();
    if status != "active" && status != "archived" {
        return Err(AppError::invalid_input("project status must be active or archived").into());
    }

    Ok(UpdateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        status: status.into(),
        assignments: request.assignments,
    })
}

async fn create_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    let request = validate_create_project_request(request)?;
    Ok(Json(
        state.services.workspace.create_project(request).await?,
    ))
}

async fn update_project(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    let request = validate_update_project_request(request)?;
    Ok(Json(
        state
            .services
            .workspace
            .update_project(&project_id, request)
            .await?,
    ))
}

async fn project_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectDashboardSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;

    let project = lookup_project(&state, &project_id).await?;
    let conversations = list_conversation_records(&state, Some(&project_id)).await?;
    let recent_activity = list_activity_records(&state, Some(&project_id)).await?;
    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let knowledge = state
        .services
        .workspace
        .list_project_knowledge(&project_id)
        .await?;
    let agents = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == Some(project_id.as_str()))
        .collect::<Vec<_>>();

    Ok(Json(ProjectDashboardSnapshot {
        project,
        metrics: vec![
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity: recent_activity.into_iter().take(8).collect(),
    }))
}

async fn workspace_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_workspace_resources().await?,
    ))
}

async fn project_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_resources(&project_id)
            .await?,
    ))
}

async fn create_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .create_workspace_resource(&workspace_id, input)
        .await?;
    Ok(Json(record))
}

async fn update_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let record = state
        .services
        .workspace
        .update_workspace_resource(&workspace_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

async fn delete_workspace_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(resource_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    state
        .services
        .workspace
        .delete_workspace_resource(&workspace_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let record = state
        .services
        .workspace
        .create_project_resource(&project_id, input)
        .await?;
    Ok(Json(record))
}

async fn create_project_resource_folder(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<CreateWorkspaceResourceFolderInput>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let records = state
        .services
        .workspace
        .create_project_resource_folder(&project_id, input)
        .await?;
    Ok(Json(records))
}

async fn update_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceResourceInput>,
) -> Result<Json<WorkspaceResourceRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    let record = state
        .services
        .workspace
        .update_project_resource(&project_id, &resource_id, input)
        .await?;
    Ok(Json(record))
}

async fn delete_project_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, resource_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", Some(&project_id)).await?;
    state
        .services
        .workspace
        .delete_project_resource(&project_id, &resource_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn workspace_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_workspace_knowledge().await?,
    ))
}

async fn project_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_knowledge(&project_id)
            .await?,
    ))
}

async fn workspace_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_pet_snapshot()
            .await?,
    ))
}

async fn project_pet_snapshot(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<PetWorkspaceSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_project_pet_snapshot(&project_id)
            .await?,
    ))
}

async fn save_workspace_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_workspace_pet_presence(input)
            .await?,
    ))
}

async fn save_project_pet_presence(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<SavePetPresenceInput>,
) -> Result<Json<PetPresenceState>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .save_project_pet_presence(&project_id, input)
            .await?,
    ))
}

async fn bind_workspace_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_workspace_pet_conversation(input)
            .await?,
    ))
}

async fn bind_project_pet_conversation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<octopus_core::BindPetConversationInput>,
) -> Result<Json<PetConversationBinding>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .bind_project_pet_conversation(&project_id, input)
            .await?,
    ))
}

async fn list_agents(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_agents().await?))
}

async fn create_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
    )
    .await?;
    Ok(Json(state.services.workspace.create_agent(input).await?))
}

async fn preview_import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundlePreviewInput>,
) -> Result<Json<ImportWorkspaceAgentBundlePreview>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .preview_import_agent_bundle(input)
            .await?,
    ))
}

async fn import_agent_bundle_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceAgentBundleInput>,
) -> Result<Json<ImportWorkspaceAgentBundleResult>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.write", None).await?;
    Ok(Json(
        state.services.workspace.import_agent_bundle(input).await?,
    ))
}

async fn update_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<UpsertAgentInput>,
) -> Result<Json<AgentRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_agent(&agent_id, input)
            .await?,
    ))
}

async fn delete_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_agent(&agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_teams(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_teams().await?))
}

async fn create_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
    )
    .await?;
    Ok(Json(state.services.workspace.create_team(input).await?))
}

async fn update_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
    Json(input): Json<UpsertTeamInput>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        input.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_team(&team_id, input)
            .await?,
    ))
}

async fn delete_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_team(&team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_project_agent_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectAgentLinkRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_agent_links(&project_id)
            .await?,
    ))
}

async fn link_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectAgentLinkInput>,
) -> Result<Json<ProjectAgentLinkRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_agent(input).await?,
    ))
}

async fn unlink_project_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    state
        .services
        .workspace
        .unlink_project_agent(&project_id, &agent_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_project_team_links(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectTeamLinkRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .workspace
            .list_project_team_links(&project_id)
            .await?,
    ))
}

async fn link_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectTeamLinkInput>,
) -> Result<Json<ProjectTeamLinkRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    if input.project_id != project_id {
        return Err(ApiError::from(AppError::invalid_input(
            "project_id in path and body must match",
        )));
    }
    Ok(Json(
        state.services.workspace.link_project_team(input).await?,
    ))
}

async fn unlink_project_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((project_id, team_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    state
        .services
        .workspace
        .unlink_project_team(&project_id, &team_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn workspace_catalog_models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.runtime_registry.catalog_snapshot().await?,
    ))
}

async fn workspace_provider_credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderCredentialRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.list_provider_credentials().await?,
    ))
}

async fn workspace_tool_catalog(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceToolCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.get_tool_catalog().await?))
}

async fn workspace_tool_catalog_disable(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<WorkspaceToolDisablePatch>,
) -> Result<Json<WorkspaceToolCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .set_tool_catalog_disabled(patch)
            .await?,
    ))
}

async fn get_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill(&skill_id)
            .await?,
    ))
}

async fn get_workspace_skill_tree_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<Json<WorkspaceSkillTreeDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_tree(&skill_id)
            .await?,
    ))
}

async fn get_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_skill_file(&skill_id, &relative_path)
            .await?,
    ))
}

async fn create_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_skill(input)
            .await?,
    ))
}

async fn import_workspace_skill_archive_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillArchiveInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_archive(input)
            .await?,
    ))
}

async fn import_workspace_skill_folder_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<ImportWorkspaceSkillFolderInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .import_workspace_skill_folder(input)
            .await?,
    ))
}

async fn update_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<UpdateWorkspaceSkillInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill(&skill_id, input)
            .await?,
    ))
}

async fn update_workspace_skill_file_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((skill_id, relative_path)): Path<(String, String)>,
    Json(input): Json<UpdateWorkspaceSkillFileInput>,
) -> Result<Json<WorkspaceSkillFileDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_skill_file(&skill_id, &relative_path, input)
            .await?,
    ))
}

async fn copy_workspace_skill_to_managed_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
    Json(input): Json<CopyWorkspaceSkillToManagedInput>,
) -> Result<Json<WorkspaceSkillDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .copy_workspace_skill_to_managed(&skill_id, input)
            .await?,
    ))
}

async fn delete_workspace_skill_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_workspace_skill(&skill_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .get_workspace_mcp_server(&server_name)
            .await?,
    ))
}

async fn create_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .create_workspace_mcp_server(input)
            .await?,
    ))
}

async fn update_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
    Json(input): Json<UpsertWorkspaceMcpServerInput>,
) -> Result<Json<WorkspaceMcpServerDocument>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_workspace_mcp_server(&server_name, input)
            .await?,
    ))
}

async fn delete_workspace_mcp_server_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(server_name): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_workspace_mcp_server(&server_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ToolRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_tools().await?))
}

async fn create_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_tool(record).await?))
}

async fn update_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(record): Json<ToolRecord>,
) -> Result<Json<ToolRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_tool(&tool_id, record)
            .await?,
    ))
}

async fn delete_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_tool(&tool_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_automations(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AutomationRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_automations().await?))
}

async fn create_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        record.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state.services.workspace.create_automation(record).await?,
    ))
}

async fn update_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_authorized_session(
        &state,
        &headers,
        "workspace.read",
        record.project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_automation(&automation_id, record)
            .await?,
    ))
}

async fn delete_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_automation(&automation_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn user_center_overview(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<UserCenterOverviewSnapshot>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    let users = state.services.workspace.list_users().await?;
    let roles = state.services.workspace.list_roles().await?;
    let permissions = state.services.workspace.list_permissions().await?;
    let menus = state.services.workspace.list_menus().await?;
    let current_user = users
        .iter()
        .find(|record| record.id == session.user_id)
        .cloned()
        .ok_or_else(|| ApiError::new(AppError::not_found("current user"), request_id(&headers)))?;

    let role_names = roles
        .iter()
        .filter(|record| {
            current_user
                .role_ids
                .iter()
                .any(|role_id| role_id == &record.id)
        })
        .map(|record| record.name.clone())
        .collect::<Vec<_>>();
    let quick_links = menus
        .iter()
        .filter(|record| record.source == "user-center" && record.status == "active")
        .cloned()
        .collect::<Vec<_>>();

    Ok(Json(UserCenterOverviewSnapshot {
        workspace_id: session.workspace_id.clone(),
        current_user,
        role_names,
        metrics: vec![
            metric_record("users", "Users", users.len()),
            metric_record("roles", "Roles", roles.len()),
            metric_record("permissions", "Permissions", permissions.len()),
            metric_record("menus", "Menus", menus.len()),
        ],
        alerts: build_user_center_alerts(&session, &permissions),
        quick_links,
    }))
}

async fn list_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserRecordSummary>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_users().await?))
}

async fn create_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkspaceUserRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_user(request).await?))
}

async fn update_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateWorkspaceUserRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_user(&user_id, request)
            .await?,
    ))
}

async fn delete_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    if session.user_id == user_id {
        return Err(ApiError::from(AppError::invalid_input(
            "current user cannot be deleted",
        )));
    }
    state.services.workspace.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_current_user_profile_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentUserProfileRequest>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_current_user_profile(&session.user_id, request)
            .await?,
    ))
}

async fn change_current_user_password_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ChangeCurrentUserPasswordRequest>,
) -> Result<Json<ChangeCurrentUserPasswordResponse>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .change_current_user_password(&session.user_id, request)
            .await?,
    ))
}

async fn list_roles(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RoleRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_roles().await?))
}

async fn create_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<RoleRecord>,
) -> Result<Json<RoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_role(record).await?))
}

async fn update_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
    Json(record): Json<RoleRecord>,
) -> Result<Json<RoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_role(&role_id, record)
            .await?,
    ))
}

async fn delete_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state.services.workspace.delete_role(&role_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_permissions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PermissionRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_permissions().await?))
}

async fn create_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<PermissionRecord>,
) -> Result<Json<PermissionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state.services.workspace.create_permission(record).await?,
    ))
}

async fn update_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(permission_id): Path<String>,
    Json(record): Json<PermissionRecord>,
) -> Result<Json<PermissionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_permission(&permission_id, record)
            .await?,
    ))
}

async fn delete_permission(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(permission_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    state
        .services
        .workspace
        .delete_permission(&permission_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_menus(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_menus().await?))
}

async fn create_menu(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(record): Json<MenuRecord>,
) -> Result<Json<MenuRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_menu(record).await?))
}

async fn update_menu(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
    Json(record): Json<MenuRecord>,
) -> Result<Json<MenuRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .workspace
            .update_menu(&menu_id, record)
            .await?,
    ))
}

async fn inbox(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::InboxItemRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.inbox.list_inbox().await?))
}

async fn artifacts(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ArtifactRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.artifact.list_artifacts().await?))
}

async fn knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::KnowledgeEntryRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.knowledge.list_knowledge().await?))
}

async fn audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::AuditRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "audit.read", None).await?;
    Ok(Json(state.services.observation.list_audit_records().await?))
}

async fn runtime_bootstrap(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<octopus_core::RuntimeBootstrap>, ApiError> {
    ensure_authorized_session(&state, &headers, "runtime.read", None).await?;
    Ok(Json(state.services.runtime_session.bootstrap().await?))
}

async fn get_runtime_config(
    State(state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    Ok(Json(state.services.runtime_config.get_config().await?))
}

async fn validate_runtime_config_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    Ok(Json(
        state.services.runtime_config.validate_config(patch).await?,
    ))
}

async fn probe_runtime_configured_model_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Json(input): Json<RuntimeConfiguredModelProbeInput>,
) -> Result<Json<RuntimeConfiguredModelProbeResult>, ApiError> {
    Ok(Json(
        state
            .services
            .runtime_config
            .probe_configured_model(input)
            .await?,
    ))
}

async fn save_runtime_config_route(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    Path(scope): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    Ok(Json(
        state
            .services
            .runtime_config
            .save_config(&scope, patch)
            .await?,
    ))
}

async fn get_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_project_config(&project_id, &session.user_id)
            .await?,
    ))
}

async fn validate_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

async fn save_project_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_project_config(&project_id, &session.user_id, patch)
            .await?,
    ))
}

async fn get_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .get_user_config(&session.user_id)
            .await?,
    ))
}

async fn validate_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeConfigValidationResult>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .validate_user_config(&session.user_id, patch)
            .await?,
    ))
}

async fn save_user_runtime_config_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(patch): Json<RuntimeConfigPatch>,
) -> Result<Json<RuntimeEffectiveConfig>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(
        state
            .services
            .runtime_config
            .save_user_config(&session.user_id, patch)
            .await?,
    ))
}

fn metric_record(id: &str, label: &str, value: usize) -> WorkspaceMetricRecord {
    WorkspaceMetricRecord {
        id: id.into(),
        label: label.into(),
        value: value.to_string(),
        helper: None,
        tone: None,
    }
}

async fn lookup_project(state: &ServerState, project_id: &str) -> Result<ProjectRecord, ApiError> {
    state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))
}

async fn list_conversation_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<ConversationRecord>, ApiError> {
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(sessions
        .into_iter()
        .filter(|record| project_id.map(|id| record.project_id == id).unwrap_or(true))
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
        })
        .collect())
}

async fn list_activity_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<WorkspaceActivityRecord>, ApiError> {
    let mut records = state.services.observation.list_audit_records().await?;
    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(records
        .into_iter()
        .filter(|record| {
            project_id
                .map(|id| record.project_id.as_deref() == Some(id))
                .unwrap_or(true)
        })
        .map(|record| WorkspaceActivityRecord {
            id: record.id,
            title: record.action,
            description: format!(
                "{} {} {}",
                record.actor_type, record.actor_id, record.outcome
            ),
            timestamp: record.created_at,
        })
        .collect())
}

fn build_user_center_alerts(
    session: &SessionRecord,
    permissions: &[PermissionRecord],
) -> Vec<UserCenterAlertRecord> {
    let mut alerts = Vec::new();
    if session.scope_project_ids.is_empty() {
        alerts.push(UserCenterAlertRecord {
            id: "alert-workspace-scope".into(),
            title: "Workspace scope active".into(),
            description: "Current session can access the full workspace scope.".into(),
            severity: "low".into(),
        });
    }
    if permissions.is_empty() {
        alerts.push(UserCenterAlertRecord {
            id: "alert-missing-permissions".into(),
            title: "RBAC not configured".into(),
            description: "No permissions are available for the current workspace.".into(),
            severity: "medium".into(),
        });
    }
    alerts
}

async fn list_runtime_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::RuntimeSessionSummary>>, ApiError> {
    ensure_authorized_session(&state, &headers, "runtime.read", None).await?;
    Ok(Json(state.services.runtime_session.list_sessions().await?))
}

async fn create_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<octopus_core::CreateRuntimeSessionInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = normalize_project_scope(&input.project_id);
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.read",
        project_id,
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers).map(|key| {
        idempotency_scope(
            &session,
            "runtime.create_session",
            &input.conversation_id,
            &key,
        )
    });
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let detail = state
        .services
        .runtime_session
        .create_session(input, &session.user_id)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &detail, &request_id)?;
    }

    let mut response = Json(detail).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

async fn get_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<octopus_core::RuntimeSessionDetail>, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(&state, &headers, "runtime.read", project_id.as_deref()).await?;
    Ok(Json(
        state
            .services
            .runtime_session
            .get_session(&session_id)
            .await?,
    ))
}

async fn delete_runtime_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(&state, &headers, "runtime.read", project_id.as_deref()).await?;
    state
        .services
        .runtime_session
        .delete_session(&session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn submit_runtime_turn(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(mut input): Json<SubmitRuntimeTurnInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    normalize_runtime_submit_input(&mut input)?;
    let session = ensure_runtime_submit(
        &state,
        &headers,
        Some(&input),
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.submit_turn", &session_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .submit_turn(&session_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &run, &request_id)?;
    }

    let mut response = Json(run).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

async fn resolve_runtime_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, approval_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeApprovalInput>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    let session = ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.resolve_approval",
        project_id.as_deref(),
        &request_id,
    )
    .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.resolve_approval", &approval_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let run = state
        .services
        .runtime_execution
        .resolve_approval(&session_id, &approval_id, input)
        .await?;
    if let Some(scope) = idempotency_scope.as_deref() {
        store_idempotent_response(&state, scope, &run, &request_id)?;
    }

    let mut response = Json(run).into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;

    let replay_after = query.after.or_else(|| last_event_id(&headers));

    if !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?;
        let mut response = Json(events).into_response();
        insert_request_id(&mut response, &request_id);
        return Ok(response);
    }

    let replay_events = if replay_after.is_some() {
        state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?
    } else {
        Vec::new()
    };
    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream = stream! {
        for event in replay_events {
            if let Ok(data) = serde_json::to_string(&event) {
                yield Ok::<Event, std::convert::Infallible>(
                    Event::default()
                        .event(event.event_type.clone())
                        .id(event.id.clone())
                        .data(data)
                );
            }
        }

        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok::<Event, std::convert::Infallible>(
                            Event::default()
                                .event(event.event_type.clone())
                                .id(event.id.clone())
                                .data(data)
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}

async fn authenticate_session(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    authenticate_session_with_request_id(state, headers, &request_id).await
}

async fn authenticate_session_with_request_id(
    state: &ServerState,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let token = extract_bearer(headers)
        .ok_or_else(|| ApiError::new(AppError::auth("missing bearer token"), request_id))?;
    let session = state
        .services
        .auth
        .lookup_session(&token)
        .await?
        .ok_or_else(|| ApiError::new(AppError::auth("invalid bearer token"), request_id))?;
    if let Some(workspace_id) = workspace_header(headers) {
        if workspace_id != session.workspace_id {
            return Err(ApiError::new(
                AppError::auth("workspace scope mismatch"),
                request_id,
            ));
        }
    }
    Ok(session)
}

async fn ensure_authorized_session(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    ensure_authorized_session_with_request_id(state, headers, capability, project_id, &request_id)
        .await
}

async fn ensure_authorized_session_with_request_id(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let session = authenticate_session_with_request_id(state, headers, request_id).await?;
    authorize_session(state, &session, capability, project_id, request_id).await?;
    Ok(session)
}

async fn authorize_session(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<(), ApiError> {
    let decision = state
        .services
        .rbac
        .authorize(session, capability, project_id)
        .await?;
    if !decision.allowed {
        return Err(ApiError::new(
            AppError::auth(decision.reason.unwrap_or_else(|| "access denied".into())),
            request_id,
        ));
    }
    Ok(())
}

async fn ensure_runtime_submit(
    state: &ServerState,
    headers: &HeaderMap,
    input: Option<&SubmitRuntimeTurnInput>,
    project_id: Option<&str>,
    request_id: &str,
) -> Result<SessionRecord, ApiError> {
    let session = ensure_authorized_session_with_request_id(
        state,
        headers,
        "runtime.submit_turn",
        project_id,
        request_id,
    )
    .await?;
    if let Some(input) = input {
        if input.permission_mode.is_empty() {
            return Err(ApiError::new(
                AppError::invalid_input("permission mode is required"),
                request_id,
            ));
        }
    }
    Ok(session)
}

fn normalize_runtime_submit_input(input: &mut SubmitRuntimeTurnInput) -> Result<(), ApiError> {
    input.configured_model_id = input
        .configured_model_id
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    input.model_id = input
        .model_id
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized =
        normalize_runtime_permission_mode_label(&input.permission_mode).ok_or_else(|| {
            ApiError::from(AppError::invalid_input(format!(
                "unsupported permission mode: {}",
                input.permission_mode
            )))
        })?;
    input.permission_mode = normalized.to_string();
    Ok(())
}

async fn runtime_project_scope(
    state: &ServerState,
    session_id: &str,
) -> Result<Option<String>, ApiError> {
    let detail = state
        .services
        .runtime_session
        .get_session(session_id)
        .await?;
    Ok(normalize_project_scope(&detail.summary.project_id).map(ToOwned::to_owned))
}

fn normalize_project_scope(project_id: &str) -> Option<&str> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id)
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(ToOwned::to_owned)
}

fn accepts_sse(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/event-stream"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request},
        routing::get,
        Json,
    };
    use octopus_core::{
        ApiErrorEnvelope, AuditRecord, ClientAppRecord, CreateRuntimeSessionInput,
        CreateWorkspaceUserRequest, InboxItemRecord, KnowledgeEntryRecord, LoginRequest,
        LoginResponse, RegisterWorkspaceOwnerRequest, RegisterWorkspaceOwnerResponse,
        ResolveRuntimeApprovalInput, RuntimeConfigPatch, RuntimeConfigValidationResult,
        RuntimeEffectiveConfig, RuntimeEventEnvelope, RuntimeRunSnapshot, RuntimeSessionDetail,
        SessionRecord, SubmitRuntimeTurnInput,
    };
    use octopus_infra::{build_infra_bundle, InfraBundle};
    use octopus_platform::{ObservationService, PlatformServices};
    use octopus_runtime_adapter::{MockRuntimeModelExecutor, RuntimeAdapter};
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use tokio_stream::StreamExt;
    use tower::ServiceExt;

    use super::*;

    #[derive(Clone)]
    struct TestHarness {
        router: Router,
        infra: InfraBundle,
        state: ServerState,
    }

    fn test_harness() -> TestHarness {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let preferences_path = root.join("shell-preferences.json");
        let workspace_connections_path = root.join("shell-workspace-connections.json");
        std::mem::forget(temp);
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let runtime = Arc::new(RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        ));
        let services = PlatformServices {
            workspace: infra.workspace.clone(),
            auth: infra.auth.clone(),
            app_registry: infra.app_registry.clone(),
            rbac: infra.rbac.clone(),
            runtime_session: runtime.clone(),
            runtime_execution: runtime.clone(),
            runtime_config: runtime.clone(),
            runtime_registry: runtime.clone(),
            artifact: infra.artifact.clone(),
            inbox: infra.inbox.clone(),
            knowledge: infra.knowledge.clone(),
            observation: infra.observation.clone(),
        };
        let state = ServerState {
            services,
            host_auth_token: "desktop-test-token".into(),
            transport_security: "loopback".into(),
            idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
            host_state: octopus_core::default_host_state("0.1.0-test".into(), true),
            host_connections: octopus_core::default_connection_stubs(),
            host_preferences_path: preferences_path,
            host_workspace_connections_path: workspace_connections_path,
            host_default_preferences: octopus_core::default_preferences(
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
            ),
            backend_connection: DesktopBackendConnection {
                base_url: Some("http://127.0.0.1:43127".into()),
                auth_token: Some("desktop-test-token".into()),
                state: "ready".into(),
                transport: "http".into(),
            },
        };
        let router = build_router(state.clone());

        TestHarness {
            router,
            infra,
            state,
        }
    }

    async fn decode_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json body")
    }

    async fn register_owner_session(router: &Router, client_app_id: &str) -> SessionRecord {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/auth/register-owner")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RegisterWorkspaceOwnerRequest {
                            client_app_id: client_app_id.into(),
                            username: "owner".into(),
                            display_name: "Workspace Owner".into(),
                            password: "owner-owner".into(),
                            confirm_password: "owner-owner".into(),
                            avatar: octopus_core::AvatarUploadPayload {
                                file_name: "owner-avatar.png".into(),
                                content_type: "image/png".into(),
                                data_base64: "iVBORw0KGgo=".into(),
                                byte_size: 8,
                            },
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RegisterWorkspaceOwnerResponse>(response)
            .await
            .session
    }

    async fn login_owner_session(router: &Router, client_app_id: &str) -> SessionRecord {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            client_app_id: client_app_id.into(),
                            username: "owner".into(),
                            password: "owner-owner".into(),
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        if response.status() == StatusCode::UNAUTHORIZED {
            return register_owner_session(router, client_app_id).await;
        }
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<LoginResponse>(response).await.session
    }

    async fn create_member_session(router: &Router, client_app_id: &str) -> SessionRecord {
        let owner = login_owner_session(router, client_app_id).await;
        let create_user_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/workspace/rbac/users")
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateWorkspaceUserRequest {
                            username: "member-alpha".into(),
                            display_name: "Member Alpha".into(),
                            status: "active".into(),
                            role_ids: vec!["role-member".into()],
                            scope_project_ids: Vec::new(),
                            avatar: None,
                            use_default_avatar: Some(true),
                            password: Some("member-member".into()),
                            confirm_password: Some("member-member".into()),
                            use_default_password: Some(false),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_user_response.status(), StatusCode::OK);

        let login_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            client_app_id: client_app_id.into(),
                            username: "member-alpha".into(),
                            password: "member-member".into(),
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(login_response.status(), StatusCode::OK);
        decode_json::<LoginResponse>(login_response).await.session
    }

    async fn create_runtime_session(
        router: &Router,
        token: &str,
        title: &str,
        idempotency_key: Option<&str>,
    ) -> RuntimeSessionDetail {
        let mut request = Request::builder();
        request = request
            .method(Method::POST)
            .uri("/api/v1/runtime/sessions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(idempotency_key) = idempotency_key {
            request = request.header("Idempotency-Key", idempotency_key);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(
                        serde_json::to_vec(&CreateRuntimeSessionInput {
                            conversation_id: "conv-1".into(),
                            project_id: "proj-redesign".into(),
                            title: title.into(),
                            session_kind: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeSessionDetail>(response).await
    }

    async fn create_runtime_session_for_project(
        router: &Router,
        token: &str,
        title: &str,
        project_id: &str,
    ) -> RuntimeSessionDetail {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/runtime/sessions")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateRuntimeSessionInput {
                            conversation_id: "conv-1".into(),
                            project_id: project_id.into(),
                            title: title.into(),
                            session_kind: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeSessionDetail>(response).await
    }

    async fn get_runtime_config(router: &Router, token: &str) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/runtime/config")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeEffectiveConfig>(response).await
    }

    async fn get_runtime_config_without_session(router: &Router) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/runtime/config")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeEffectiveConfig>(response).await
    }

    async fn get_tool_catalog(router: &Router, token: &str) -> Value {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace/catalog/tool-catalog")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<Value>(response).await
    }

    async fn patch_tool_catalog_disabled(router: &Router, token: &str, body: Value) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/api/v1/workspace/catalog/tool-catalog/disable")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn create_workspace_skill(router: &Router, token: &str, body: Value) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/workspace/catalog/skills")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_workspace_skill(router: &Router, token: &str, skill_id: &str) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_workspace_skill_tree(router: &Router, token: &str, skill_id: &str) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}/tree"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_workspace_skill_file(
        router: &Router,
        token: &str,
        skill_id: &str,
        relative_path: &str,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!(
                        "/api/v1/workspace/catalog/skills/{skill_id}/files/{relative_path}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn update_workspace_skill_file(
        router: &Router,
        token: &str,
        skill_id: &str,
        relative_path: &str,
        body: Value,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!(
                        "/api/v1/workspace/catalog/skills/{skill_id}/files/{relative_path}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn copy_workspace_skill_to_managed(
        router: &Router,
        token: &str,
        skill_id: &str,
        body: Value,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/workspace/catalog/skills/{skill_id}/copy-to-managed"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn import_workspace_skill_folder(router: &Router, token: &str, body: Value) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/workspace/catalog/skills/import-folder")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn update_workspace_skill(
        router: &Router,
        token: &str,
        skill_id: &str,
        body: Value,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn delete_workspace_skill(router: &Router, token: &str, skill_id: &str) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn create_workspace_mcp_server(router: &Router, token: &str, body: Value) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/workspace/catalog/mcp-servers")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_workspace_mcp_server(router: &Router, token: &str, server_name: &str) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!(
                        "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn update_workspace_mcp_server(
        router: &Router,
        token: &str,
        server_name: &str,
        body: Value,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!(
                        "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn delete_workspace_mcp_server(
        router: &Router,
        token: &str,
        server_name: &str,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!(
                        "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn get_model_catalog(router: &Router, token: &str) -> Value {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace/catalog/models")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<Value>(response).await
    }

    async fn create_project(router: &Router, token: &str, body: Value) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn update_project(
        router: &Router,
        token: &str,
        project_id: &str,
        body: Value,
    ) -> Response {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/projects/{project_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    async fn list_projects(router: &Router, token: &str) -> Value {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<Value>(response).await
    }

    async fn validate_runtime_config_without_session(
        router: &Router,
        patch: RuntimeConfigPatch,
    ) -> RuntimeConfigValidationResult {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/runtime/config/validate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeConfigValidationResult>(response).await
    }

    async fn probe_runtime_configured_model_without_session(
        router: &Router,
        input: RuntimeConfiguredModelProbeInput,
    ) -> RuntimeConfiguredModelProbeResult {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/runtime/config/configured-models/probe")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&input).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeConfiguredModelProbeResult>(response).await
    }

    async fn save_runtime_config(
        router: &Router,
        token: &str,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/runtime/config/scopes/{scope}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeEffectiveConfig>(response).await
    }

    async fn save_runtime_config_without_session(
        router: &Router,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/runtime/config/scopes/{scope}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        serde_json::from_slice::<RuntimeEffectiveConfig>(&body).expect("runtime config json")
    }

    async fn get_project_runtime_config(
        router: &Router,
        token: &str,
        project_id: &str,
    ) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeEffectiveConfig>(response).await
    }

    async fn save_project_runtime_config(
        router: &Router,
        token: &str,
        project_id: &str,
        patch: RuntimeConfigPatch,
    ) -> RuntimeEffectiveConfig {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeEffectiveConfig>(response).await
    }

    async fn submit_turn(
        router: &Router,
        token: &str,
        session_id: &str,
        permission_mode: &str,
        idempotency_key: Option<&str>,
    ) -> RuntimeRunSnapshot {
        submit_turn_with_input(
            router,
            token,
            session_id,
            SubmitRuntimeTurnInput {
                content: "hello".into(),
                model_id: Some("claude-sonnet-4-5".into()),
                configured_model_id: None,
                permission_mode: permission_mode.into(),
                actor_kind: None,
                actor_id: None,
            },
            idempotency_key,
        )
        .await
    }

    async fn submit_turn_with_input(
        router: &Router,
        token: &str,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
        idempotency_key: Option<&str>,
    ) -> RuntimeRunSnapshot {
        let mut request = Request::builder();
        request = request
            .method(Method::POST)
            .uri(format!("/api/v1/runtime/sessions/{session_id}/turns"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(idempotency_key) = idempotency_key {
            request = request.header("Idempotency-Key", idempotency_key);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&input).expect("json")))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeRunSnapshot>(response).await
    }

    async fn runtime_session_detail(
        router: &Router,
        token: &str,
        session_id: &str,
    ) -> RuntimeSessionDetail {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/runtime/sessions/{session_id}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<RuntimeSessionDetail>(response).await
    }

    async fn runtime_events_after(
        router: &Router,
        token: &str,
        session_id: &str,
        after: &str,
    ) -> Vec<RuntimeEventEnvelope> {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/runtime/sessions/{session_id}/events?after={after}"
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<Vec<RuntimeEventEnvelope>>(response).await
    }

    async fn next_sse_event(
        router: &Router,
        token: &str,
        session_id: &str,
        last_event_id: Option<&str>,
        emit_turn: bool,
    ) -> RuntimeEventEnvelope {
        let mut request = Request::builder();
        request = request
            .uri(format!("/api/v1/runtime/sessions/{session_id}/events"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::ACCEPT, "text/event-stream");
        if let Some(last_event_id) = last_event_id {
            request = request.header("Last-Event-ID", last_event_id);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );

        let mut stream = response.into_body().into_data_stream();
        if emit_turn {
            submit_turn(router, token, session_id, "ask", None).await;
        }

        let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .expect("sse event timeout")
            .expect("sse chunk")
            .expect("sse bytes");
        let payload = String::from_utf8(chunk.to_vec()).expect("utf8");
        let data = payload
            .lines()
            .find_map(|line| line.strip_prefix("data:"))
            .map(str::trim)
            .expect("sse data line");
        serde_json::from_str(data).expect("sse envelope")
    }

    #[tokio::test]
    async fn health_route_reports_ready_backend() {
        let response = test_harness()
            .router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn system_bootstrap_and_login_are_public_entrypoints() {
        let harness = test_harness();
        let bootstrap_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/system/bootstrap")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(bootstrap_response.status(), StatusCode::OK);
        let bootstrap: serde_json::Value = decode_json(bootstrap_response).await;
        assert_eq!(bootstrap["setupRequired"], true);
        assert_eq!(bootstrap["ownerReady"], false);

        let register_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/auth/register-owner")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RegisterWorkspaceOwnerRequest {
                            client_app_id: "octopus-desktop".into(),
                            username: "owner".into(),
                            display_name: "Workspace Owner".into(),
                            password: "owner-owner".into(),
                            confirm_password: "owner-owner".into(),
                            avatar: octopus_core::AvatarUploadPayload {
                                file_name: "owner-avatar.png".into(),
                                content_type: "image/png".into(),
                                data_base64: "iVBORw0KGgo=".into(),
                                byte_size: 8,
                            },
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(register_response.status(), StatusCode::OK);

        let login_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            client_app_id: "octopus-desktop".into(),
                            username: "owner".into(),
                            password: "owner-owner".into(),
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(login_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn project_management_routes_create_and_update_projects() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let create_response = create_project(
            &harness.router,
            &session.token,
            json!({
                "name": "Agent Studio",
                "description": "Project management workspace surface.",
                "assignments": {
                    "models": {
                        "configuredModelIds": ["anthropic-primary"],
                        "defaultConfiguredModelId": "anthropic-primary"
                    },
                    "tools": {
                        "sourceKeys": ["builtin:bash"]
                    },
                    "agents": {
                        "agentIds": ["agent-architect"],
                        "teamIds": ["team-studio"]
                    }
                }
            }),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created: Value = decode_json(create_response).await;
        assert_eq!(created["name"], "Agent Studio");
        assert_eq!(created["status"], "active");
        assert_eq!(
            created["assignments"]["models"]["configuredModelIds"],
            json!(["anthropic-primary"])
        );
        assert_eq!(
            created["assignments"]["tools"]["sourceKeys"],
            json!(["builtin:bash"])
        );

        let created_id = created["id"].as_str().expect("project id");
        let update_response = update_project(
            &harness.router,
            &session.token,
            created_id,
            json!({
                "name": "Agent Studio",
                "description": "Updated project workspace surface.",
                "status": "archived",
                "assignments": {
                    "models": {
                        "configuredModelIds": ["anthropic-alt"],
                        "defaultConfiguredModelId": "anthropic-alt"
                    },
                    "tools": {
                        "sourceKeys": ["builtin:bash", "mcp:ops"]
                    },
                    "agents": {
                        "agentIds": ["agent-architect"],
                        "teamIds": []
                    }
                }
            }),
        )
        .await;
        assert_eq!(update_response.status(), StatusCode::OK);
        let updated: Value = decode_json(update_response).await;
        assert_eq!(updated["status"], "archived");
        assert_eq!(updated["description"], "Updated project workspace surface.");
        assert_eq!(
            updated["assignments"]["models"]["configuredModelIds"],
            json!(["anthropic-alt"])
        );
        assert_eq!(
            updated["assignments"]["tools"]["sourceKeys"],
            json!(["builtin:bash", "mcp:ops"])
        );
    }

    #[tokio::test]
    async fn project_management_routes_reject_blank_names() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let response = create_project(
            &harness.router,
            &session.token,
            json!({
                "name": "   ",
                "description": "Project management workspace surface."
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error: ApiErrorEnvelope = decode_json(response).await;
        assert_eq!(error.error.code, "INVALID_INPUT");
    }

    #[tokio::test]
    async fn project_management_routes_protect_the_last_active_project() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let create_response = create_project(
            &harness.router,
            &session.token,
            json!({
                "name": "Workspace Governance",
                "description": "RBAC, menu policies, and audit automation."
            }),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created: Value = decode_json(create_response).await;
        let governance_project_id = created["id"].as_str().expect("project id").to_string();

        let first_archive = update_project(
            &harness.router,
            &session.token,
            "proj-redesign",
            json!({
                "name": "Desktop Redesign",
                "description": "Real workspace API migration for the desktop surface.",
                "status": "archived"
            }),
        )
        .await;
        assert_eq!(first_archive.status(), StatusCode::OK);

        let second_archive = update_project(
            &harness.router,
            &session.token,
            &governance_project_id,
            json!({
                "name": "Workspace Governance",
                "description": "RBAC, menu policies, and audit automation.",
                "status": "archived"
            }),
        )
        .await;
        assert_eq!(second_archive.status(), StatusCode::BAD_REQUEST);

        let projects = list_projects(&harness.router, &session.token).await;
        let active_count = projects
            .as_array()
            .expect("projects array")
            .iter()
            .filter(|project| project["status"] == "active")
            .count();
        assert_eq!(active_count, 1);
    }

    #[tokio::test]
    async fn workspace_tool_catalog_returns_runtime_backed_entries() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let skill_dir = harness.infra.paths.root.join("data/skills/help");
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: help\ndescription: Helpful local skill.\n---\n\nUse this skill to help.\n",
        )
        .expect("skill file");

        let _ = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "mcpServers": {
                        "ops": {
                            "type": "http",
                            "url": "https://ops.example.test/mcp"
                        }
                    }
                }),
            },
        )
        .await;

        let payload = get_tool_catalog(&harness.router, &token).await;
        let entries = payload["entries"].as_array().expect("entries array");

        assert!(entries
            .iter()
            .any(|entry| entry["kind"] == "builtin" && entry["name"] == "bash"));
        let skill_entry = entries
            .iter()
            .find(|entry| entry["kind"] == "skill" && entry["name"] == "help")
            .expect("skill entry");
        assert_eq!(skill_entry["disabled"], Value::Bool(false));
        assert_eq!(skill_entry["workspaceOwned"], Value::Bool(true));
        assert_eq!(
            skill_entry["relativePath"],
            Value::String("data/skills/help/SKILL.md".into())
        );
        assert_eq!(skill_entry["management"]["canEdit"], Value::Bool(true));
        assert_eq!(skill_entry["management"]["canDelete"], Value::Bool(true));
        assert_eq!(skill_entry["management"]["canDisable"], Value::Bool(true));
        assert!(entries
            .iter()
            .any(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops"));
        let builtin_entry = entries
            .iter()
            .find(|entry| entry["kind"] == "builtin" && entry["name"] == "bash")
            .expect("builtin entry");
        assert_eq!(builtin_entry["disabled"], Value::Bool(false));
        assert_eq!(builtin_entry["management"]["canEdit"], Value::Bool(false));
        assert_eq!(builtin_entry["management"]["canDelete"], Value::Bool(false));
        assert_eq!(builtin_entry["management"]["canDisable"], Value::Bool(true));
    }

    #[tokio::test]
    async fn workspace_tool_catalog_disable_route_persists_runtime_overrides() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let skill_dir = harness.infra.paths.root.join("data/skills/help");
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: help\ndescription: Helpful local skill.\n---\n",
        )
        .expect("skill file");

        let _ = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "mcpServers": {
                        "ops": {
                            "type": "http",
                            "url": "https://ops.example.test/mcp"
                        }
                    }
                }),
            },
        )
        .await;

        let before = get_tool_catalog(&harness.router, &token).await;
        let entries = before["entries"].as_array().expect("entries array");
        let builtin_source_key = entries
            .iter()
            .find(|entry| entry["kind"] == "builtin" && entry["name"] == "bash")
            .and_then(|entry| entry["sourceKey"].as_str())
            .expect("builtin source key")
            .to_string();
        let skill_source_key = entries
            .iter()
            .find(|entry| entry["kind"] == "skill" && entry["name"] == "help")
            .and_then(|entry| entry["sourceKey"].as_str())
            .expect("skill source key")
            .to_string();
        let mcp_source_key = entries
            .iter()
            .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops")
            .and_then(|entry| entry["sourceKey"].as_str())
            .expect("mcp source key")
            .to_string();

        for source_key in [&builtin_source_key, &skill_source_key, &mcp_source_key] {
            let response = patch_tool_catalog_disabled(
                &harness.router,
                &token,
                json!({
                    "sourceKey": source_key,
                    "disabled": true
                }),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        let after = get_tool_catalog(&harness.router, &token).await;
        let entries = after["entries"].as_array().expect("entries array");
        for source_key in [&builtin_source_key, &skill_source_key, &mcp_source_key] {
            let entry = entries
                .iter()
                .find(|entry| entry["sourceKey"] == Value::String(source_key.clone()))
                .expect("updated entry");
            assert_eq!(entry["disabled"], Value::Bool(true));
        }

        let written = std::fs::read_to_string(
            harness
                .infra
                .paths
                .runtime_config_dir
                .join("workspace.json"),
        )
        .expect("workspace config");
        assert!(written.contains("\"toolCatalog\""));
        assert!(written.contains(&builtin_source_key));
        assert!(written.contains(&skill_source_key));
        assert!(written.contains(&mcp_source_key));
    }

    #[tokio::test]
    async fn workspace_skill_routes_create_update_and_delete_workspace_owned_skills() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let create_response = create_workspace_skill(
            &harness.router,
            &token,
            json!({
                "slug": "ops-helper",
                "content": "---\nname: ops-helper\ndescription: First version.\n---\n\nUse this skill to help ops.\n"
            }),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created: Value = decode_json(create_response).await;
        let skill_id = created["id"].as_str().expect("skill id").to_string();
        assert_eq!(created["name"], "ops-helper");
        assert_eq!(created["workspaceOwned"], Value::Bool(true));

        let skill_path = harness
            .infra
            .paths
            .root
            .join("data/skills/ops-helper/SKILL.md");
        assert!(skill_path.exists());

        let get_response = get_workspace_skill(&harness.router, &token, &skill_id).await;
        assert_eq!(get_response.status(), StatusCode::OK);
        let fetched: Value = decode_json(get_response).await;
        assert!(fetched["tree"].is_array());

        let update_response = update_workspace_skill_file(
            &harness.router,
            &token,
            &skill_id,
            "SKILL.md",
            json!({
                "content": "---\nname: ops-helper\ndescription: Updated version.\n---\n\nUse this skill to help ops better.\n"
            }),
        )
        .await;
        assert_eq!(update_response.status(), StatusCode::OK);
        let updated: Value = decode_json(update_response).await;
        assert_eq!(updated["content"], Value::String("---\nname: ops-helper\ndescription: Updated version.\n---\n\nUse this skill to help ops better.\n".into()));

        let written = std::fs::read_to_string(&skill_path).expect("skill file");
        assert!(written.contains("Updated version."));

        let delete_response = delete_workspace_skill(&harness.router, &token, &skill_id).await;
        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
        assert!(!skill_path.exists());
    }

    #[tokio::test]
    async fn workspace_skill_routes_expose_tree_and_file_documents_for_managed_skills() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let skill_dir = harness.infra.paths.root.join("data/skills/ops-helper");
        std::fs::create_dir_all(skill_dir.join("templates")).expect("skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: ops-helper\ndescription: Managed skill.\n---\n",
        )
        .expect("skill file");
        std::fs::write(skill_dir.join("templates/prompt.md"), "# Prompt\n").expect("text file");
        std::fs::write(skill_dir.join("icon.bin"), [0_u8, 159, 146, 150]).expect("binary file");

        let payload = get_tool_catalog(&harness.router, &token).await;
        let skill_id = payload["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["kind"] == "skill" && entry["name"] == "ops-helper")
            .and_then(|entry| entry["id"].as_str())
            .expect("skill id")
            .to_string();

        let tree_response = get_workspace_skill_tree(&harness.router, &token, &skill_id).await;
        assert_eq!(tree_response.status(), StatusCode::OK);
        let tree: Value = decode_json(tree_response).await;
        let nodes = tree["tree"].as_array().expect("tree nodes");
        assert!(nodes.iter().any(|node| node["path"] == "SKILL.md"));
        assert!(nodes.iter().any(|node| node["path"] == "templates"));

        let file_response =
            get_workspace_skill_file(&harness.router, &token, &skill_id, "templates/prompt.md")
                .await;
        assert_eq!(file_response.status(), StatusCode::OK);
        let file: Value = decode_json(file_response).await;
        assert_eq!(file["path"], "templates/prompt.md");
        assert_eq!(file["isText"], Value::Bool(true));
        assert_eq!(file["content"], "# Prompt\n");

        let binary_response =
            get_workspace_skill_file(&harness.router, &token, &skill_id, "icon.bin").await;
        assert_eq!(binary_response.status(), StatusCode::OK);
        let binary: Value = decode_json(binary_response).await;
        assert_eq!(binary["isText"], Value::Bool(false));
        assert_eq!(binary["content"], Value::Null);
    }

    #[tokio::test]
    async fn workspace_skill_routes_reject_mutating_non_workspace_owned_entries() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let skill_dir = harness
            .infra
            .paths
            .root
            .join(".claude/skills/external-help");
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: external-help\ndescription: External skill.\n---\n",
        )
        .expect("skill file");

        let payload = get_tool_catalog(&harness.router, &token).await;
        let skill_id = payload["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["kind"] == "skill" && entry["name"] == "external-help")
            .and_then(|entry| entry["id"].as_str())
            .expect("external skill id")
            .to_string();

        let update_response = update_workspace_skill(
            &harness.router,
            &token,
            &skill_id,
            json!({
                "content": "---\nname: external-help\ndescription: Updated.\n---\n"
            }),
        )
        .await;
        assert_eq!(update_response.status(), StatusCode::BAD_REQUEST);

        let delete_response = delete_workspace_skill(&harness.router, &token, &skill_id).await;
        assert_eq!(delete_response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn workspace_skill_routes_copy_external_skill_to_managed_root() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let skill_dir = harness
            .infra
            .paths
            .root
            .join(".claude/skills/external-help");
        std::fs::create_dir_all(skill_dir.join("templates")).expect("skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: external-help\ndescription: External skill.\n---\n",
        )
        .expect("skill file");
        std::fs::write(skill_dir.join("templates/prompt.md"), "hello\n").expect("template file");

        let payload = get_tool_catalog(&harness.router, &token).await;
        let skill_id = payload["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["kind"] == "skill" && entry["name"] == "external-help")
            .and_then(|entry| entry["id"].as_str())
            .expect("external skill id")
            .to_string();

        let response = copy_workspace_skill_to_managed(
            &harness.router,
            &token,
            &skill_id,
            json!({ "slug": "external-help-copy" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let copied: Value = decode_json(response).await;
        assert_eq!(copied["workspaceOwned"], Value::Bool(true));
        assert_eq!(
            copied["relativePath"],
            Value::String("data/skills/external-help-copy/SKILL.md".into())
        );

        let copied_skill_root = harness
            .infra
            .paths
            .root
            .join("data/skills/external-help-copy");
        assert!(copied_skill_root.join("SKILL.md").exists());
        assert!(copied_skill_root.join("templates/prompt.md").exists());
    }

    #[tokio::test]
    async fn workspace_skill_routes_import_folder_into_managed_root() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let response = import_workspace_skill_folder(
            &harness.router,
            &token,
            json!({
                "slug": "imported-skill",
                "files": [
                    {
                        "relativePath": "wrapped/SKILL.md",
                        "fileName": "SKILL.md",
                        "contentType": "text/markdown",
                        "dataBase64": "LS0tCm5hbWU6IGltcG9ydGVkLXNraWxsCmRlc2NyaXB0aW9uOiBJbXBvcnRlZCBza2lsbC4KLS0tCg==",
                        "byteSize": 58
                    },
                    {
                        "relativePath": "wrapped/templates/prompt.md",
                        "fileName": "prompt.md",
                        "contentType": "text/markdown",
                        "dataBase64": "IyBQcm9tcHQK",
                        "byteSize": 9
                    }
                ]
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let imported: Value = decode_json(response).await;
        assert_eq!(
            imported["relativePath"],
            Value::String("data/skills/imported-skill/SKILL.md".into())
        );
        assert!(harness
            .infra
            .paths
            .root
            .join("data/skills/imported-skill/templates/prompt.md")
            .exists());
    }

    #[tokio::test]
    async fn workspace_mcp_routes_create_update_and_delete_servers() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let create_response = create_workspace_mcp_server(
            &harness.router,
            &token,
            json!({
                "serverName": "ops",
                "config": {
                    "type": "http",
                    "url": "https://ops.example.test/mcp"
                }
            }),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created: Value = decode_json(create_response).await;
        assert_eq!(created["serverName"], "ops");

        let get_response = get_workspace_mcp_server(&harness.router, &token, "ops").await;
        assert_eq!(get_response.status(), StatusCode::OK);
        let fetched: Value = decode_json(get_response).await;
        assert_eq!(fetched["config"]["url"], "https://ops.example.test/mcp");

        let update_response = update_workspace_mcp_server(
            &harness.router,
            &token,
            "ops",
            json!({
                "serverName": "ops",
                "config": {
                    "type": "http",
                    "url": "https://ops-alt.example.test/mcp"
                }
            }),
        )
        .await;
        assert_eq!(update_response.status(), StatusCode::OK);
        let updated: Value = decode_json(update_response).await;
        assert_eq!(updated["config"]["url"], "https://ops-alt.example.test/mcp");

        let delete_response = delete_workspace_mcp_server(&harness.router, &token, "ops").await;
        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

        let payload = get_tool_catalog(&harness.router, &token).await;
        assert!(!payload["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .any(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops"));
    }

    #[tokio::test]
    async fn workspace_model_catalog_returns_registry_snapshot_shape() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let payload = get_model_catalog(&harness.router, &token).await;

        assert!(
            payload.get("providers").is_some(),
            "missing providers snapshot"
        );
        assert!(payload.get("models").is_some(), "missing models snapshot");
        assert!(
            payload.get("defaultSelections").is_some(),
            "missing default selections"
        );
        assert!(payload.get("diagnostics").is_some(), "missing diagnostics");
    }

    #[tokio::test]
    async fn workspace_model_catalog_reflects_runtime_registry_overrides_without_restart() {
        let harness = test_harness();
        let token = register_owner_session(&harness.router, "octopus-desktop")
            .await
            .token;

        let _saved = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "modelRegistry": {
                        "providers": {
                            "deepseek": {
                                "providerId": "deepseek",
                                "label": "DeepSeek",
                                "enabled": true,
                                "surfaces": [
                                    {
                                        "surface": "conversation",
                                        "protocolFamily": "openai_chat",
                                        "authStrategy": "bearer",
                                        "baseUrl": "https://api.deepseek.com",
                                        "baseUrlPolicy": "allow_override"
                                    }
                                ]
                            }
                        },
                        "models": {
                            "deepseek-chat": {
                                "modelId": "deepseek-chat",
                                "providerId": "deepseek",
                                "label": "DeepSeek Chat",
                                "family": "deepseek-chat",
                                "track": "latest_alias",
                                "enabled": true,
                                "surfaceBindings": [
                                    {
                                        "surface": "conversation",
                                        "protocolFamily": "openai_chat"
                                    }
                                ],
                                "capabilities": ["streaming", "tool_calling"],
                                "metadata": {
                                    "source": "workspace-override"
                                }
                            }
                        },
                        "defaultSelections": {
                            "conversation": {
                                "providerId": "deepseek",
                                "modelId": "deepseek-chat",
                                "surface": "conversation"
                            }
                        }
                    }
                }),
            },
        )
        .await;

        let payload = get_model_catalog(&harness.router, &token).await;
        let models = payload["models"].as_array().expect("models array");
        let defaults = &payload["defaultSelections"];

        assert!(models
            .iter()
            .any(|model| model["modelId"] == "deepseek-chat"));
        assert_eq!(defaults["conversation"]["modelId"], "deepseek-chat");
        assert_eq!(defaults["conversation"]["providerId"], "deepseek");
    }

    #[tokio::test]
    async fn host_routes_require_a_valid_host_token_and_persist_preferences() {
        let harness = test_harness();

        let unauthorized = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/bootstrap")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let bootstrap_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/bootstrap")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(bootstrap_response.status(), StatusCode::OK);
        let bootstrap: serde_json::Value = decode_json(bootstrap_response).await;
        assert_eq!(bootstrap["hostState"]["platform"], "tauri");
        assert_eq!(bootstrap["backend"]["state"], "ready");

        let update_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/v1/host/preferences")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&octopus_core::ShellPreferences {
                            theme: "dark".into(),
                            locale: "en-US".into(),
                            font_size: 15,
                            font_family: "Inter, sans-serif".into(),
                            font_style: "sans".into(),
                            compact_sidebar: true,
                            left_sidebar_collapsed: true,
                            right_sidebar_collapsed: false,
                            update_channel: "preview".into(),
                            default_workspace_id: "ws-local".into(),
                            last_visited_route:
                                "/workspaces/ws-local/overview?project=proj-redesign".into(),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(update_response.status(), StatusCode::OK);

        let preferences_response = harness
            .router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/preferences")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(preferences_response.status(), StatusCode::OK);
        let preferences = decode_json::<octopus_core::ShellPreferences>(preferences_response).await;
        assert_eq!(preferences.theme, "dark");
        assert_eq!(preferences.locale, "en-US");
        assert_eq!(preferences.update_channel, "preview");
        assert!(preferences.left_sidebar_collapsed);
    }

    #[tokio::test]
    async fn host_update_routes_return_browser_safe_contracts() {
        let manifest_router = Router::new()
            .route(
                "/formal/latest.json",
                get(|| async {
                    Json(json!({
                        "version": "0.2.0",
                        "notes": "Formal release body",
                        "pub_date": "2026-04-08T11:30:00Z",
                        "channel": "formal",
                        "notesUrl": "https://github.com/GoyacJ/octopus/releases/tag/v0.2.0",
                        "platforms": {
                            "darwin-aarch64": {
                                "signature": "formal-signature",
                                "url": "https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Lobster.app.tar.gz"
                            }
                        }
                    }))
                }),
            )
            .route(
                "/preview/latest.json",
                get(|| async {
                    Json(json!({
                        "version": "0.2.0-preview.4",
                        "notes": "Preview release body",
                        "pub_date": "2026-04-09T09:15:00Z",
                        "channel": "preview",
                        "notesUrl": "https://github.com/GoyacJ/octopus/releases/tag/v0.2.0-preview.4",
                        "platforms": {
                            "darwin-aarch64": {
                                "signature": "preview-signature",
                                "url": "https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Lobster.app.tar.gz"
                            }
                        }
                    }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("manifest listener");
        let address = listener.local_addr().expect("manifest addr");
        let manifest_server = tokio::spawn(async move {
            axum::serve(listener, manifest_router)
                .await
                .expect("manifest server");
        });

        let harness = test_harness();
        let formal_status = refresh_browser_host_update_status_with_endpoint(
            &harness.state,
            "formal",
            Some(&format!("http://{address}/formal/latest.json")),
        )
        .await
        .expect("formal update status");
        assert_eq!(formal_status.current_version, "0.1.0-test");
        assert_eq!(formal_status.current_channel, "formal");
        assert_eq!(formal_status.state, "update_available");
        assert_eq!(
            formal_status
                .latest_release
                .as_ref()
                .map(|release| release.version.as_str()),
            Some("0.2.0")
        );
        assert_eq!(
            formal_status
                .latest_release
                .as_ref()
                .and_then(|release| release.notes_url.as_deref()),
            Some("https://github.com/GoyacJ/octopus/releases/tag/v0.2.0")
        );
        assert!(!formal_status.capabilities.can_download);
        assert!(!formal_status.capabilities.can_install);
        assert!(formal_status.capabilities.supports_channels);
        assert!(formal_status.last_checked_at.is_some());

        let preview_status = refresh_browser_host_update_status_with_endpoint(
            &harness.state,
            "preview",
            Some(&format!("http://{address}/preview/latest.json")),
        )
        .await
        .expect("preview update status");
        assert_eq!(preview_status.current_channel, "preview");
        assert_eq!(preview_status.state, "update_available");
        assert_eq!(
            preview_status
                .latest_release
                .as_ref()
                .map(|release| release.version.as_str()),
            Some("0.2.0-preview.4")
        );
        assert!(preview_status.last_checked_at.is_some());

        let download_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/host/update-download")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(download_response.status(), StatusCode::OK);
        let download_status: serde_json::Value = decode_json(download_response).await;
        assert_eq!(download_status["state"], "error");
        assert_eq!(download_status["errorCode"], "UPDATE_DOWNLOAD_UNSUPPORTED");

        let install_response = harness
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/host/update-install")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(install_response.status(), StatusCode::OK);
        let install_status: serde_json::Value = decode_json(install_response).await;
        assert_eq!(install_status["state"], "error");
        assert_eq!(install_status["errorCode"], "UPDATE_INSTALL_UNSUPPORTED");
        manifest_server.abort();
    }

    #[tokio::test]
    async fn host_notification_routes_roundtrip_and_preserve_history() {
        let harness = test_harness();

        let create_workspace_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/host/notifications")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateNotificationInput {
                            scope_kind: "workspace".into(),
                            scope_owner_id: Some("ws-local".into()),
                            level: "success".into(),
                            title: "Workspace synced".into(),
                            body: "The workspace is ready.".into(),
                            source: "workspace-store".into(),
                            toast_duration_ms: Some(30_000),
                            route_to: Some("/workspaces/ws-local/overview".into()),
                            action_label: Some("Open workspace".into()),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_workspace_response.status(), StatusCode::OK);
        let workspace_notification =
            decode_json::<NotificationRecord>(create_workspace_response).await;

        let create_user_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/host/notifications")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateNotificationInput {
                            scope_kind: "user".into(),
                            scope_owner_id: Some("user-local".into()),
                            level: "warning".into(),
                            title: "Profile incomplete".into(),
                            body: "Add your preferred contact details.".into(),
                            source: "user-center".into(),
                            toast_duration_ms: Some(15_000),
                            route_to: None,
                            action_label: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(create_user_response.status(), StatusCode::OK);
        let user_notification = decode_json::<NotificationRecord>(create_user_response).await;

        let unread_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/notifications/unread-summary")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unread_response.status(), StatusCode::OK);
        let unread = decode_json::<NotificationUnreadSummary>(unread_response).await;
        assert_eq!(unread.total, 2);
        assert_eq!(unread.by_scope.workspace, 1);
        assert_eq!(unread.by_scope.user, 1);

        let filtered_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/notifications?scope=workspace")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(filtered_response.status(), StatusCode::OK);
        let filtered = decode_json::<NotificationListResponse>(filtered_response).await;
        assert_eq!(filtered.notifications.len(), 1);
        assert_eq!(filtered.notifications[0].id, workspace_notification.id);
        assert_eq!(filtered.unread.total, 1);

        let marked_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&format!(
                        "/api/v1/host/notifications/{}/read",
                        workspace_notification.id
                    ))
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(marked_response.status(), StatusCode::OK);
        let marked = decode_json::<NotificationRecord>(marked_response).await;
        assert!(marked.read_at.is_some());

        let dismissed_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&format!(
                        "/api/v1/host/notifications/{}/dismiss-toast",
                        workspace_notification.id
                    ))
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(dismissed_response.status(), StatusCode::OK);
        let dismissed = decode_json::<NotificationRecord>(dismissed_response).await;
        assert_eq!(dismissed.toast_visible_until, None);

        let mark_all_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/host/notifications/read-all")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&NotificationFilter {
                            scope: Some("user".into()),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(mark_all_response.status(), StatusCode::OK);
        let next_summary = decode_json::<NotificationUnreadSummary>(mark_all_response).await;
        assert_eq!(next_summary.total, 0);

        let listed_response = harness
            .router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/host/notifications?scope=all")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(listed_response.status(), StatusCode::OK);
        let listed = decode_json::<NotificationListResponse>(listed_response).await;
        assert_eq!(listed.notifications.len(), 2);
        assert_eq!(listed.notifications[0].id, user_notification.id);
        assert_eq!(listed.notifications[1].id, workspace_notification.id);
        assert!(listed
            .notifications
            .iter()
            .all(|notification| notification.read_at.is_some()));
        assert_eq!(listed.notifications[1].toast_visible_until, None);
    }

    #[tokio::test]
    async fn host_routes_accept_browser_cors_preflight_for_local_dev_origin() {
        let response = test_harness()
            .router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/v1/host/bootstrap")
                    .header(header::ORIGIN, "http://127.0.0.1:15420")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .header(
                        header::ACCESS_CONTROL_REQUEST_HEADERS,
                        "authorization,content-type",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("http://127.0.0.1:15420")
        );
    }

    #[tokio::test]
    async fn workspace_routes_accept_packaged_tauri_https_origin() {
        let response = test_harness()
            .router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/v1/system/bootstrap")
                    .header(header::ORIGIN, "https://tauri.localhost")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .header(
                        header::ACCESS_CONTROL_REQUEST_HEADERS,
                        "x-request-id,content-type",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("https://tauri.localhost")
        );
    }

    #[tokio::test]
    async fn workspace_routes_accept_packaged_tauri_http_origin() {
        let response = test_harness()
            .router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/v1/auth/register-owner")
                    .header(header::ORIGIN, "http://tauri.localhost")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(
                        header::ACCESS_CONTROL_REQUEST_HEADERS,
                        "content-type,x-request-id",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("http://tauri.localhost")
        );
    }

    #[tokio::test]
    async fn legacy_runtime_aliases_are_not_available() {
        let harness = test_harness();
        let response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/runtime/sessions")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn host_token_is_rejected_for_protected_workspace_and_runtime_routes() {
        let harness = test_harness();
        let workspace_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspace")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(workspace_response.status(), StatusCode::UNAUTHORIZED);
        let workspace_error = decode_json::<ApiErrorEnvelope>(workspace_response).await;
        assert_eq!(workspace_error.error.code, "UNAUTHENTICATED");

        let runtime_response = harness
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/runtime/sessions")
                    .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateRuntimeSessionInput {
                            conversation_id: "conv-1".into(),
                            project_id: "proj-redesign".into(),
                            title: "Session".into(),
                            session_kind: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(runtime_response.status(), StatusCode::UNAUTHORIZED);
        let runtime_error = decode_json::<ApiErrorEnvelope>(runtime_response).await;
        assert_eq!(runtime_error.error.code, "UNAUTHENTICATED");
    }

    #[tokio::test]
    async fn apps_routes_roundtrip_through_http_contract() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let list_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/apps")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(list_response.status(), StatusCode::OK);
        let listed = decode_json::<Vec<ClientAppRecord>>(list_response).await;
        assert!(listed.iter().any(|record| record.id == "octopus-web"));

        let register_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/apps")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ClientAppRecord {
                            id: "octopus-desktop-preview".into(),
                            name: "Octopus Desktop Preview".into(),
                            platform: "desktop".into(),
                            status: "active".into(),
                            first_party: true,
                            allowed_origins: vec!["http://127.0.0.1".into()],
                            allowed_hosts: vec!["127.0.0.1".into()],
                            session_policy: "session_token".into(),
                            default_scopes: vec!["workspace".into(), "runtime".into()],
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(register_response.status(), StatusCode::OK);
        let registered = decode_json::<ClientAppRecord>(register_response).await;
        assert_eq!(registered.id, "octopus-desktop-preview");
    }

    #[tokio::test]
    async fn audit_inbox_and_knowledge_routes_return_transport_records() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let audit_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(audit_response.status(), StatusCode::OK);
        let _audit_records = decode_json::<Vec<AuditRecord>>(audit_response).await;

        let inbox_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/inbox")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(inbox_response.status(), StatusCode::OK);
        let _inbox_records = decode_json::<Vec<InboxItemRecord>>(inbox_response).await;

        let knowledge_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/knowledge")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(knowledge_response.status(), StatusCode::OK);
        let _knowledge_records = decode_json::<Vec<KnowledgeEntryRecord>>(knowledge_response).await;
    }

    #[tokio::test]
    async fn apps_audit_inbox_and_knowledge_routes_reject_non_owner_sessions() {
        let harness = test_harness();
        let member = create_member_session(&harness.router, "octopus-desktop").await;

        let apps_read = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/apps")
                    .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(apps_read.status(), StatusCode::FORBIDDEN);
        let apps_read_error = decode_json::<ApiErrorEnvelope>(apps_read).await;
        assert_eq!(apps_read_error.error.code, "FORBIDDEN");

        let apps_write = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/apps")
                    .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ClientAppRecord {
                            id: "octopus-member-preview".into(),
                            name: "Octopus Member Preview".into(),
                            platform: "desktop".into(),
                            status: "active".into(),
                            first_party: true,
                            allowed_origins: vec!["http://127.0.0.1".into()],
                            allowed_hosts: vec!["127.0.0.1".into()],
                            session_policy: "session_token".into(),
                            default_scopes: vec!["workspace".into()],
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(apps_write.status(), StatusCode::FORBIDDEN);
        let apps_write_error = decode_json::<ApiErrorEnvelope>(apps_write).await;
        assert_eq!(apps_write_error.error.code, "FORBIDDEN");

        let audit_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit")
                    .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(audit_response.status(), StatusCode::FORBIDDEN);
        let audit_error = decode_json::<ApiErrorEnvelope>(audit_response).await;
        assert_eq!(audit_error.error.code, "FORBIDDEN");

        let inbox_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/inbox")
                    .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(inbox_response.status(), StatusCode::FORBIDDEN);
        let inbox_error = decode_json::<ApiErrorEnvelope>(inbox_response).await;
        assert_eq!(inbox_error.error.code, "FORBIDDEN");

        let knowledge_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/knowledge")
                    .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(knowledge_response.status(), StatusCode::FORBIDDEN);
        let knowledge_error = decode_json::<ApiErrorEnvelope>(knowledge_response).await;
        assert_eq!(knowledge_error.error.code, "FORBIDDEN");
    }

    #[tokio::test]
    async fn runtime_session_flow_supports_json_event_polling_and_observation_with_session_token() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created =
            create_runtime_session(&harness.router, &session.token, "Session", None).await;

        let run = submit_turn(
            &harness.router,
            &session.token,
            &created.summary.id,
            "ask",
            None,
        )
        .await;
        assert_eq!(run.status, "waiting_approval");

        let events_response = harness
            .router
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/events?after=missing",
                        created.summary.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(events_response.status(), StatusCode::OK);
        let events = decode_json::<Vec<RuntimeEventEnvelope>>(events_response).await;
        assert!(events
            .iter()
            .any(|event| event.event_type == "runtime.approval.requested"));
        assert!(events
            .iter()
            .any(|event| event.event_type == "runtime.run.updated"));

        let trace_events = harness
            .infra
            .observation
            .list_trace_events()
            .await
            .expect("trace events");
        let audit_records = harness
            .infra
            .observation
            .list_audit_records()
            .await
            .expect("audit records");
        assert!(trace_events
            .iter()
            .any(|event| event.event_kind == "turn_submitted"));
        assert!(audit_records
            .iter()
            .any(|record| record.action == "runtime.submit_turn"));
    }

    #[tokio::test]
    async fn runtime_submit_turn_executes_model_and_records_resolved_target() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created =
            create_runtime_session(&harness.router, &session.token, "Execution Session", None)
                .await;

        let run = submit_turn_with_input(
            &harness.router,
            &session.token,
            &created.summary.id,
            SubmitRuntimeTurnInput {
                content: "Reply with a short acknowledgement.".into(),
                model_id: Some("claude-sonnet-4-5".into()),
                configured_model_id: None,
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
            None,
        )
        .await;

        assert_eq!(run.status, "completed");

        let detail =
            runtime_session_detail(&harness.router, &session.token, &created.summary.id).await;
        let assistant_message = detail
            .messages
            .iter()
            .find(|message| message.sender_type == "assistant")
            .expect("assistant message");
        assert!(!assistant_message.content.is_empty());

        let run_value = serde_json::to_value(&run).expect("serialize run");
        assert_eq!(run_value["resolvedTarget"]["providerId"], "anthropic");
        assert_eq!(run_value["resolvedTarget"]["modelId"], "claude-sonnet-4-5");
        assert_eq!(run_value["resolvedTarget"]["surface"], "conversation");
    }

    #[tokio::test]
    async fn workspace_model_catalog_exposes_configured_models_and_submit_turn_accepts_configured_model_id(
    ) {
        let harness = test_harness();
        let owner = register_owner_session(&harness.router, "octopus-desktop").await;

        let _saved = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "anthropic-primary": {
                            "configuredModelId": "anthropic-primary",
                            "name": "Claude Primary",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_API_KEY",
                            "enabled": true,
                            "source": "workspace"
                        },
                        "anthropic-alt": {
                            "configuredModelId": "anthropic-alt",
                            "name": "Claude Alt",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_ALT_API_KEY",
                            "baseUrl": "https://anthropic.alt.example.test",
                            "enabled": true,
                            "source": "workspace"
                        }
                    },
                    "defaultSelections": {
                        "conversation": {
                            "configuredModelId": "anthropic-primary",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "surface": "conversation"
                        }
                    }
                }),
            },
        )
        .await;

        let catalog_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace/catalog/models")
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(catalog_response.status(), StatusCode::OK);
        let catalog_value = decode_json::<serde_json::Value>(catalog_response).await;
        let configured_models = catalog_value["configuredModels"]
            .as_array()
            .expect("configured models array");
        assert!(configured_models
            .iter()
            .any(|model| model["configuredModelId"] == "anthropic-primary"
                && model["name"] == "Claude Primary"));
        assert!(configured_models
            .iter()
            .any(|model| model["configuredModelId"] == "anthropic-alt"
                && model["name"] == "Claude Alt"));
        assert_eq!(
            catalog_value["defaultSelections"]["conversation"]["configuredModelId"],
            "anthropic-primary"
        );

        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session(
            &harness.router,
            &session.token,
            "Configured Model Session",
            None,
        )
        .await;

        let response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/turns",
                        created.summary.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "content": "Reply with a short acknowledgement.",
                            "configuredModelId": "anthropic-alt",
                            "permissionMode": "readonly"
                        }))
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let run_value = serde_json::from_slice::<serde_json::Value>(&body).expect("run json");
        assert_eq!(run_value["configuredModelId"], "anthropic-alt");
        assert_eq!(
            run_value["resolvedTarget"]["configuredModelId"],
            "anthropic-alt"
        );
        assert_eq!(
            run_value["resolvedTarget"]["configuredModelName"],
            "Claude Alt"
        );
        assert_eq!(
            run_value["resolvedTarget"]["credentialRef"],
            "env:ANTHROPIC_ALT_API_KEY"
        );
    }

    #[tokio::test]
    async fn workspace_model_catalog_exposes_token_usage_and_runtime_blocks_exhausted_quota() {
        let harness = test_harness();
        let owner = register_owner_session(&harness.router, "octopus-desktop").await;

        let _saved = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "quota-model": {
                            "configuredModelId": "quota-model",
                            "name": "Quota Model",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_API_KEY",
                            "tokenQuota": {
                                "totalTokens": 32
                            },
                            "enabled": true,
                            "source": "workspace"
                        }
                    }
                }),
            },
        )
        .await;

        let first_session =
            create_runtime_session(&harness.router, &owner.token, "Quota Session 1", None).await;
        let first_run = submit_turn_with_input(
            &harness.router,
            &owner.token,
            &first_session.summary.id,
            SubmitRuntimeTurnInput {
                content: "Use the whole quota.".into(),
                model_id: None,
                configured_model_id: Some("quota-model".into()),
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
            None,
        )
        .await;
        assert_eq!(first_run.consumed_tokens, Some(32));

        let catalog_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/workspace/catalog/models")
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(catalog_response.status(), StatusCode::OK);
        let catalog_value = decode_json::<serde_json::Value>(catalog_response).await;
        let quota_model = catalog_value["configuredModels"]
            .as_array()
            .expect("configured models array")
            .iter()
            .find(|model| model["configuredModelId"] == "quota-model")
            .cloned()
            .expect("quota model in catalog");
        assert_eq!(quota_model["tokenQuota"]["totalTokens"], 32);
        assert_eq!(quota_model["tokenUsage"]["usedTokens"], 32);
        assert_eq!(quota_model["tokenUsage"]["remainingTokens"], 0);
        assert_eq!(quota_model["tokenUsage"]["exhausted"], true);

        let second_session =
            create_runtime_session(&harness.router, &owner.token, "Quota Session 2", None).await;
        let response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/turns",
                        second_session.summary.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "content": "This should be blocked.",
                            "configuredModelId": "quota-model",
                            "permissionMode": "readonly"
                        }))
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = decode_json::<ApiErrorEnvelope>(response).await;
        assert_eq!(error.error.code, "INVALID_INPUT");
        assert!(error
            .error
            .message
            .contains("has reached its total token limit"));
    }

    #[tokio::test]
    async fn runtime_submit_turn_rejects_unknown_registry_model() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session(
            &harness.router,
            &session.token,
            "Unknown Model Session",
            None,
        )
        .await;

        let response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/turns",
                        created.summary.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&SubmitRuntimeTurnInput {
                            content: "hello".into(),
                            model_id: Some("missing-model".into()),
                            configured_model_id: None,
                            permission_mode: "readonly".into(),
                            actor_kind: None,
                            actor_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = decode_json::<ApiErrorEnvelope>(response).await;
        assert_eq!(error.error.code, "INVALID_INPUT");
        assert!(error.error.message.contains("missing-model"));
    }

    #[tokio::test]
    async fn project_runtime_model_settings_filter_allowed_models_and_override_default_selection() {
        let harness = test_harness();
        let owner = register_owner_session(&harness.router, "octopus-desktop").await;

        let _saved_workspace = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "anthropic-primary": {
                            "configuredModelId": "anthropic-primary",
                            "name": "Claude Primary",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_API_KEY",
                            "enabled": true,
                            "source": "workspace"
                        },
                        "anthropic-alt": {
                            "configuredModelId": "anthropic-alt",
                            "name": "Claude Alt",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_ALT_API_KEY",
                            "enabled": true,
                            "source": "workspace"
                        }
                    },
                    "defaultSelections": {
                        "conversation": {
                            "configuredModelId": "anthropic-primary",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "surface": "conversation"
                        }
                    }
                }),
            },
        )
        .await;

        let create_project_response = create_project(
            &harness.router,
            &owner.token,
            json!({
                "name": "Project Runtime Models",
                "description": "Project-specific model selection coverage.",
                "assignments": {
                    "models": {
                        "configuredModelIds": ["anthropic-alt"],
                        "defaultConfiguredModelId": "anthropic-alt"
                    }
                }
            }),
        )
        .await;
        assert_eq!(create_project_response.status(), StatusCode::OK);
        let created_project = decode_json::<Value>(create_project_response).await;
        let project_id = created_project["id"]
            .as_str()
            .expect("project id")
            .to_string();

        let _saved_project = save_project_runtime_config(
            &harness.router,
            &owner.token,
            &project_id,
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "projectSettings": {
                        "models": {
                            "allowedConfiguredModelIds": ["anthropic-alt"],
                            "defaultConfiguredModelId": "anthropic-alt"
                        }
                    }
                }),
            },
        )
        .await;

        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session_for_project(
            &harness.router,
            &session.token,
            "Project Model Session",
            &project_id,
        )
        .await;

        let defaulted_run = submit_turn_with_input(
            &harness.router,
            &session.token,
            &created.summary.id,
            SubmitRuntimeTurnInput {
                content: "Reply with a short acknowledgement.".into(),
                model_id: None,
                configured_model_id: None,
                permission_mode: "readonly".into(),
                actor_kind: None,
                actor_id: None,
            },
            None,
        )
        .await;
        assert_eq!(
            defaulted_run
                .resolved_target
                .expect("resolved target")
                .configured_model_id,
            "anthropic-alt"
        );

        let denied_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/turns",
                        created.summary.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&SubmitRuntimeTurnInput {
                            content: "Try the workspace default.".into(),
                            model_id: None,
                            configured_model_id: Some("anthropic-primary".into()),
                            permission_mode: "readonly".into(),
                            actor_kind: None,
                            actor_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(denied_response.status(), StatusCode::BAD_REQUEST);
        let error = decode_json::<ApiErrorEnvelope>(denied_response).await;
        assert_eq!(error.error.code, "INVALID_INPUT");
        assert!(error.error.message.contains("anthropic-primary"));
    }

    #[tokio::test]
    async fn project_runtime_settings_reject_unassigned_tools_and_agents() {
        let harness = test_harness();
        let owner = register_owner_session(&harness.router, "octopus-desktop").await;

        let create_project_response = create_project(
            &harness.router,
            &owner.token,
            json!({
                "name": "Project Runtime Actors",
                "description": "Project-specific tool and actor assignment coverage.",
                "assignments": {
                    "tools": {
                        "sourceKeys": ["builtin:bash"]
                    },
                    "agents": {
                        "agentIds": ["agent-architect"],
                        "teamIds": ["team-studio"]
                    }
                }
            }),
        )
        .await;
        assert_eq!(create_project_response.status(), StatusCode::OK);
        let created_project = decode_json::<Value>(create_project_response).await;
        let project_id = created_project["id"]
            .as_str()
            .expect("project id")
            .to_string();

        let invalid_tool_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RuntimeConfigPatch {
                            scope: "project".into(),
                            patch: json!({
                                "projectSettings": {
                                    "tools": {
                                        "enabledSourceKeys": ["builtin:terminal"],
                                        "overrides": {
                                            "builtin:terminal": { "permissionMode": "readonly" }
                                        }
                                    }
                                }
                            }),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(invalid_tool_response.status(), StatusCode::BAD_REQUEST);
        let invalid_tool_error = decode_json::<ApiErrorEnvelope>(invalid_tool_response).await;
        assert!(invalid_tool_error
            .error
            .message
            .contains("unassigned sourceKey `builtin:terminal`"));

        let invalid_agent_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                    .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RuntimeConfigPatch {
                            scope: "project".into(),
                            patch: json!({
                                "projectSettings": {
                                    "agents": {
                                        "enabledAgentIds": ["agent-reviewer"],
                                        "enabledTeamIds": ["team-studio"]
                                    }
                                }
                            }),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(invalid_agent_response.status(), StatusCode::BAD_REQUEST);
        let invalid_agent_error = decode_json::<ApiErrorEnvelope>(invalid_agent_response).await;
        assert!(invalid_agent_error
            .error
            .message
            .contains("unassigned agent `agent-reviewer`"));
    }

    #[tokio::test]
    async fn runtime_events_support_sse_and_polling_consistency_for_session_tokens() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created =
            create_runtime_session(&harness.router, &session.token, "SSE Session", None).await;
        let initial_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            "missing",
        )
        .await;
        let baseline_event = initial_events.last().expect("baseline event").id.clone();

        let sse_event = next_sse_event(
            &harness.router,
            &session.token,
            &created.summary.id,
            None,
            true,
        )
        .await;
        let polled_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            &baseline_event,
        )
        .await;

        assert!(polled_events.iter().any(|event| event.id == sse_event.id));
        assert!(polled_events
            .iter()
            .any(|event| event.event_type == "runtime.approval.requested"));
    }

    #[tokio::test]
    async fn runtime_events_support_sse_backlog_replay_with_last_event_id() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created =
            create_runtime_session(&harness.router, &session.token, "Replay Session", None).await;
        submit_turn(
            &harness.router,
            &session.token,
            &created.summary.id,
            "ask",
            None,
        )
        .await;

        let initial_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            "missing",
        )
        .await;
        let baseline_event = initial_events.first().expect("baseline event").id.clone();

        let replayed_event = next_sse_event(
            &harness.router,
            &session.token,
            &created.summary.id,
            Some(&baseline_event),
            false,
        )
        .await;

        assert_ne!(replayed_event.id, baseline_event);
        assert!(replayed_event.sequence > 1);
        assert_eq!(replayed_event.event_type, "runtime.message.created");
    }

    #[tokio::test]
    async fn runtime_mutations_replay_when_the_same_idempotency_key_is_reused() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created_first = create_runtime_session(
            &harness.router,
            &session.token,
            "Idempotent Session",
            Some("idem-session-1"),
        )
        .await;
        let created_second = create_runtime_session(
            &harness.router,
            &session.token,
            "Idempotent Session",
            Some("idem-session-1"),
        )
        .await;

        assert_eq!(created_first.summary.id, created_second.summary.id);

        let first_run = submit_turn(
            &harness.router,
            &session.token,
            &created_first.summary.id,
            "ask",
            Some("idem-turn-1"),
        )
        .await;
        let second_run = submit_turn(
            &harness.router,
            &session.token,
            &created_first.summary.id,
            "ask",
            Some("idem-turn-1"),
        )
        .await;

        assert_eq!(first_run.id, second_run.id);
        let sessions_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/runtime/sessions")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(sessions_response.status(), StatusCode::OK);
        let sessions =
            decode_json::<Vec<octopus_core::RuntimeSessionSummary>>(sessions_response).await;
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn protected_routes_reject_workspace_scope_mismatch() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workspace")
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header("X-Workspace-Id", "ws-other")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let error = decode_json::<ApiErrorEnvelope>(response).await;
        assert_eq!(error.error.code, "FORBIDDEN");
    }

    #[tokio::test]
    async fn approval_resolution_updates_run_status_and_observation_records() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let approved_session =
            create_runtime_session(&harness.router, &session.token, "Approve Session", None).await;
        submit_turn(
            &harness.router,
            &session.token,
            &approved_session.summary.id,
            "ask",
            None,
        )
        .await;
        let detail = runtime_session_detail(
            &harness.router,
            &session.token,
            &approved_session.summary.id,
        )
        .await;
        let approval = detail.pending_approval.expect("pending approval");
        let approve_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/approvals/{}",
                        approved_session.summary.id, approval.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ResolveRuntimeApprovalInput {
                            decision: "approve".into(),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(approve_response.status(), StatusCode::OK);
        let approved_run = decode_json::<RuntimeRunSnapshot>(approve_response).await;
        assert_eq!(approved_run.status, "completed");
        let approved_detail = runtime_session_detail(
            &harness.router,
            &session.token,
            &approved_session.summary.id,
        )
        .await;
        assert!(approved_detail
            .messages
            .iter()
            .any(|message| message.sender_type == "assistant"));

        let approved_events = runtime_events_after(
            &harness.router,
            &session.token,
            &approved_session.summary.id,
            "missing",
        )
        .await;
        let approved_resolution = approved_events
            .iter()
            .find(|event| event.event_type == "runtime.approval.resolved")
            .expect("approval resolved event");
        assert_eq!(
            approved_resolution
                .approval
                .as_ref()
                .expect("approved payload")
                .status,
            "approved"
        );

        let rejected_session =
            create_runtime_session(&harness.router, &session.token, "Reject Session", None).await;
        submit_turn(
            &harness.router,
            &session.token,
            &rejected_session.summary.id,
            "ask",
            None,
        )
        .await;
        let reject_detail = runtime_session_detail(
            &harness.router,
            &session.token,
            &rejected_session.summary.id,
        )
        .await;
        let reject_approval = reject_detail.pending_approval.expect("pending approval");
        let reject_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/api/v1/runtime/sessions/{}/approvals/{}",
                        rejected_session.summary.id, reject_approval.id
                    ))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ResolveRuntimeApprovalInput {
                            decision: "reject".into(),
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(reject_response.status(), StatusCode::OK);
        let rejected_run = decode_json::<RuntimeRunSnapshot>(reject_response).await;
        assert_eq!(rejected_run.status, "blocked");

        let rejected_events = runtime_events_after(
            &harness.router,
            &session.token,
            &rejected_session.summary.id,
            "missing",
        )
        .await;
        let rejected_resolution = rejected_events
            .iter()
            .find(|event| event.event_type == "runtime.approval.resolved")
            .expect("approval resolved event");
        assert_eq!(
            rejected_resolution
                .approval
                .as_ref()
                .expect("rejected payload")
                .status,
            "rejected"
        );

        let trace_events = harness
            .infra
            .observation
            .list_trace_events()
            .await
            .expect("trace events");
        let audit_records = harness
            .infra
            .observation
            .list_audit_records()
            .await
            .expect("audit records");
        assert!(trace_events
            .iter()
            .any(|event| event.event_kind == "approval_resolved"));
        assert!(audit_records
            .iter()
            .any(|record| record.action == "runtime.resolve_approval"));
    }

    #[tokio::test]
    async fn runtime_config_routes_load_validate_and_save_scoped_documents() {
        let harness = test_harness();

        let initial = get_runtime_config_without_session(&harness.router).await;
        assert!(initial.validation.valid);
        assert_eq!(initial.sources.len(), 1);
        assert!(initial
            .sources
            .iter()
            .any(|source| source.scope == "workspace" && !source.exists));

        let validation = validate_runtime_config_without_session(
            &harness.router,
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: serde_json::json!({
                    "model": "claude-sonnet-4-5",
                    "permissions": {
                        "defaultMode": "plan"
                    }
                }),
            },
        )
        .await;
        assert!(validation.valid);

        let saved = save_runtime_config_without_session(
            &harness.router,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: serde_json::json!({
                    "model": "claude-sonnet-4-5",
                    "permissions": {
                        "defaultMode": "plan"
                    }
                }),
            },
        )
        .await;

        assert_eq!(
            saved.effective_config.get("model"),
            Some(&serde_json::json!("claude-sonnet-4-5"))
        );
        assert!(saved.sources.iter().any(|source| {
            source.scope == "workspace"
                && source.source_key == "workspace"
                && source.display_path == "config/runtime/workspace.json"
                && source.exists
        }));

        let workspace_settings = harness
            .infra
            .paths
            .config_dir
            .join("runtime")
            .join("workspace.json");
        let written =
            std::fs::read_to_string(workspace_settings).expect("workspace settings written");
        assert!(written.contains("\"model\": \"claude-sonnet-4-5\""));
        assert!(written.contains("\"defaultMode\": \"plan\""));
    }

    #[tokio::test]
    async fn runtime_config_probe_route_executes_real_configured_model_request() {
        let harness = test_harness();

        let probe = probe_runtime_configured_model_without_session(
            &harness.router,
            RuntimeConfiguredModelProbeInput {
                scope: "workspace".into(),
                configured_model_id: "anthropic-primary".into(),
                patch: serde_json::json!({
                    "configuredModels": {
                        "anthropic-primary": {
                            "configuredModelId": "anthropic-primary",
                            "name": "Claude Primary",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_API_KEY",
                            "enabled": true,
                            "source": "workspace"
                        }
                    }
                }),
            },
        )
        .await;

        assert!(probe.valid);
        assert!(probe.reachable);
        assert_eq!(probe.configured_model_id, "anthropic-primary");
        assert_eq!(
            probe.configured_model_name.as_deref(),
            Some("Claude Primary")
        );
        assert_eq!(probe.consumed_tokens, Some(32));
        assert!(probe.errors.is_empty());
    }

    #[tokio::test]
    async fn runtime_config_routes_expose_workspace_relative_source_metadata() {
        let harness = test_harness();

        let config = get_runtime_config_without_session(&harness.router).await;
        let serialized = serde_json::to_value(&config).expect("serialize config");

        let workspace_source = config
            .sources
            .iter()
            .find(|source| source.scope == "workspace")
            .expect("workspace source");

        assert_eq!(workspace_source.source_key, "workspace");
        assert_eq!(
            workspace_source.display_path,
            "config/runtime/workspace.json"
        );
        assert!(workspace_source.owner_id.is_none());
        assert!(serialized.to_string().contains("\"displayPath\""));
        assert!(!serialized.to_string().contains("\"path\""));
    }

    #[tokio::test]
    async fn runtime_public_workspace_config_route_stays_workspace_only() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let runtime_dir = harness.infra.paths.runtime_config_dir.clone();
        std::fs::create_dir_all(harness.infra.paths.runtime_user_config_dir.clone())
            .expect("user runtime dir");
        std::fs::write(
            harness
                .infra
                .paths
                .runtime_user_config_dir
                .join(format!("{}.json", session.user_id)),
            r#"{
              "model": "user-model",
              "provider": {
                "defaultModel": "user-default"
              }
            }"#,
        )
        .expect("write user settings");
        std::fs::write(
            runtime_dir.join("workspace.json"),
            r#"{
              "model": "workspace-model"
            }"#,
        )
        .expect("write workspace settings");

        let config = get_runtime_config_without_session(&harness.router).await;
        assert_eq!(
            config
                .sources
                .iter()
                .map(|source| source.scope.clone())
                .collect::<Vec<_>>(),
            vec!["workspace".to_string()]
        );
        assert_eq!(
            config.effective_config.get("model"),
            Some(&json!("workspace-model"))
        );
        assert_eq!(config.effective_config.get("provider"), None);
    }

    #[tokio::test]
    async fn project_runtime_config_routes_include_current_user_precedence() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let project_id = "proj-redesign";

        std::fs::write(
            harness
                .infra
                .paths
                .runtime_user_config_dir
                .join(format!("{}.json", session.user_id)),
            r#"{
              "model": "user-model",
              "provider": {
                "defaultModel": "user-default"
              },
              "permissions": {
                "defaultMode": "readonly"
              }
            }"#,
        )
        .expect("write user settings");
        std::fs::write(
            harness
                .infra
                .paths
                .runtime_config_dir
                .join("workspace.json"),
            r#"{
              "model": "workspace-model",
              "permissions": {
                "defaultMode": "plan"
              }
            }"#,
        )
        .expect("write workspace settings");
        std::fs::write(
            harness
                .infra
                .paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
            r#"{
              "model": "project-model"
            }"#,
        )
        .expect("write project settings");

        let fetched = get_project_runtime_config(&harness.router, &session.token, project_id).await;
        assert_eq!(
            fetched
                .sources
                .iter()
                .map(|source| source.source_key.clone())
                .collect::<Vec<_>>(),
            vec![
                format!("user:{}", session.user_id),
                "workspace".to_string(),
                format!("project:{project_id}"),
            ]
        );
        assert_eq!(
            fetched.effective_config.get("model"),
            Some(&json!("project-model"))
        );
        assert_eq!(
            fetched.effective_config.pointer("/permissions/defaultMode"),
            Some(&json!("plan"))
        );
        assert_eq!(
            fetched.effective_config.pointer("/provider/defaultModel"),
            Some(&json!("user-default"))
        );

        let saved = save_project_runtime_config(
            &harness.router,
            &session.token,
            project_id,
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: json!({
                    "provider": {
                        "defaultModel": "project-default"
                    }
                }),
            },
        )
        .await;
        assert_eq!(
            saved.effective_config.pointer("/provider/defaultModel"),
            Some(&json!("project-default"))
        );
        assert_eq!(
            saved
                .sources
                .iter()
                .map(|source| source.source_key.clone())
                .collect::<Vec<_>>(),
            vec![
                format!("user:{}", session.user_id),
                "workspace".to_string(),
                format!("project:{project_id}"),
            ]
        );
    }

    #[tokio::test]
    async fn runtime_config_routes_redact_plaintext_secrets_from_api_payloads() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let project_dir = harness.infra.paths.config_dir.join("runtime");
        std::fs::create_dir_all(&project_dir).expect("workspace settings dir");
        std::fs::write(
            project_dir.join("workspace.json"),
            r#"{
              "provider": {
                "apiKey": "super-secret-key"
              },
              "mcpServers": {
                "remote": {
                  "type": "http",
                  "url": "https://example.test/mcp",
                  "headers": {
                    "Authorization": "Bearer secret-token"
                  }
                }
              }
            }"#,
        )
        .expect("write project settings");

        let config = get_runtime_config(&harness.router, &session.token).await;
        let project_source = config
            .sources
            .iter()
            .find(|source| {
                source.scope == "workspace"
                    && source.display_path == "config/runtime/workspace.json"
            })
            .expect("workspace source");

        assert_eq!(
            project_source
                .document
                .as_ref()
                .and_then(|document| document.get("provider"))
                .and_then(|provider| provider.get("apiKey")),
            Some(&serde_json::json!("***"))
        );
        assert!(config.secret_references.iter().any(|secret| {
            secret.path.ends_with("provider.apiKey") && secret.status == "inline-redacted"
        }));
        assert!(config.secret_references.iter().any(|secret| {
            secret
                .path
                .ends_with("mcpServers.remote.headers.Authorization")
                && secret.status == "inline-redacted"
        }));
    }

    #[tokio::test]
    async fn runtime_session_creation_persists_config_snapshot_and_sqlite_projection() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        std::fs::write(
            harness
                .infra
                .paths
                .runtime_user_config_dir
                .join(format!("{}.json", session.user_id)),
            r#"{
              "model": "user-model"
            }"#,
        )
        .expect("write user settings");
        let _saved = save_runtime_config(
            &harness.router,
            &session.token,
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: serde_json::json!({
                    "model": "workspace-model",
                    "permissions": {
                        "defaultMode": "plan"
                    }
                }),
            },
        )
        .await;
        let _project_saved = save_project_runtime_config(
            &harness.router,
            &session.token,
            "proj-redesign",
            RuntimeConfigPatch {
                scope: "project".into(),
                patch: serde_json::json!({
                    "model": "project-model"
                }),
            },
        )
        .await;

        let created =
            create_runtime_session(&harness.router, &session.token, "Projection Session", None)
                .await;
        assert!(!created.summary.config_snapshot_id.is_empty());
        assert!(!created.summary.effective_config_hash.is_empty());
        assert!(created
            .summary
            .started_from_scope_set
            .iter()
            .zip(["user", "workspace", "project"])
            .all(|(actual, expected)| actual == expected));
        assert_eq!(
            created.summary.started_from_scope_set,
            vec![
                "user".to_string(),
                "workspace".to_string(),
                "project".to_string()
            ]
        );
        assert_eq!(
            created.run.config_snapshot_id,
            created.summary.config_snapshot_id
        );

        let connection =
            Connection::open(&harness.infra.paths.db_path).expect("open runtime projection db");

        let stored_snapshot_id: String = connection
            .query_row(
                "SELECT config_snapshot_id FROM runtime_session_projections WHERE id = ?1",
                [&created.summary.id],
                |row| row.get(0),
            )
            .expect("runtime session projection");
        assert_eq!(stored_snapshot_id, created.summary.config_snapshot_id);

        let stored_hash: String = connection
            .query_row(
                "SELECT effective_config_hash FROM runtime_config_snapshots WHERE id = ?1",
                [&created.summary.config_snapshot_id],
                |row| row.get(0),
            )
            .expect("runtime config snapshot");
        assert_eq!(stored_hash, created.summary.effective_config_hash);

        let stored_source_refs: String = connection
            .query_row(
                "SELECT source_refs FROM runtime_config_snapshots WHERE id = ?1",
                [&created.summary.config_snapshot_id],
                |row| row.get(0),
            )
            .expect("runtime config source refs");
        assert_eq!(
            stored_source_refs,
            serde_json::json!([
                format!("user:{}", session.user_id),
                "workspace",
                "project:proj-redesign"
            ])
            .to_string()
        );
    }
}
