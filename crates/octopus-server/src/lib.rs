use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
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
    routing::{get, patch, post},
    Json, Router,
};
use octopus_core::{
    normalize_runtime_permission_mode_label, timestamp_now, AgentRecord, ApiErrorDetail,
    ApiErrorEnvelope, AppError, AutomationRecord, ClientAppRecord, ConnectionProfile,
    ConversationRecord, DesktopBackendConnection, HealthcheckBackendStatus, HealthcheckStatus,
    HostState, KnowledgeRecord, LoginRequest, MenuRecord, ModelCatalogSnapshot,
    PermissionRecord, ProjectDashboardSnapshot, ProjectRecord, ProviderCredentialRecord,
    ResolveRuntimeApprovalInput, RoleRecord, SessionRecord, ShellBootstrap,
    ShellPreferences, SubmitRuntimeTurnInput, TeamRecord, ToolRecord,
    UserCenterAlertRecord, UserCenterOverviewSnapshot, UserRecordSummary,
    WorkspaceActivityRecord, WorkspaceMetricRecord, WorkspaceOverviewSnapshot,
    WorkspaceResourceRecord,
};
use octopus_platform::PlatformServices;
use serde::Deserialize;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
pub struct ServerState {
    pub services: PlatformServices,
    pub host_auth_token: String,
    pub transport_security: String,
    pub idempotency_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub host_state: HostState,
    pub host_connections: Vec<ConnectionProfile>,
    pub host_preferences_path: PathBuf,
    pub host_default_preferences: ShellPreferences,
    pub backend_connection: DesktopBackendConnection,
}

#[derive(Debug)]
struct ApiError {
    source: AppError,
    request_id: String,
}

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
        || origin == "http://[::1]"
        || origin.starts_with("http://[::1]:")
        || origin == "https://127.0.0.1"
        || origin.starts_with("https://127.0.0.1:")
        || origin == "https://localhost"
        || origin.starts_with("https://localhost:")
        || origin == "https://[::1]"
        || origin.starts_with("https://[::1]:")
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
    let cache = state
        .idempotency_cache
        .lock()
        .map_err(|_| ApiError::new(AppError::runtime("idempotency cache mutex poisoned"), request_id))?;
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
    let mut cache = state
        .idempotency_cache
        .lock()
        .map_err(|_| ApiError::new(AppError::runtime("idempotency cache mutex poisoned"), request_id))?;
    cache.insert(scope.to_string(), payload);
    Ok(())
}

fn idempotency_scope(
    session: &SessionRecord,
    operation: &str,
    resource: &str,
    key: &str,
) -> String {
    format!("{}:{}:{}:{}", session.workspace_id, session.user_id, operation, format!("{resource}:{key}"))
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
        .route("/api/v1/system/health", get(healthcheck))
        .route("/api/v1/system/bootstrap", get(system_bootstrap))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/apps", get(list_apps).post(register_app))
        .route("/api/v1/workspace", get(workspace))
        .route("/api/v1/workspace/overview", get(workspace_overview))
        .route("/api/v1/workspace/resources", get(workspace_resources))
        .route("/api/v1/workspace/knowledge", get(workspace_knowledge))
        .route("/api/v1/workspace/agents", get(list_agents).post(create_agent))
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
        .route("/api/v1/workspace/catalog/tools", get(list_tools).post(create_tool))
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
        .route("/api/v1/workspace/rbac/users", get(list_users).post(create_user))
        .route(
            "/api/v1/workspace/rbac/users/:user_id",
            patch(update_user),
        )
        .route("/api/v1/workspace/rbac/roles", get(list_roles).post(create_role))
        .route(
            "/api/v1/workspace/rbac/roles/:role_id",
            patch(update_role),
        )
        .route(
            "/api/v1/workspace/rbac/permissions",
            get(list_permissions).post(create_permission),
        )
        .route(
            "/api/v1/workspace/rbac/permissions/:permission_id",
            patch(update_permission),
        )
        .route("/api/v1/workspace/rbac/menus", get(list_menus).post(create_menu))
        .route(
            "/api/v1/workspace/rbac/menus/:menu_id",
            patch(update_menu),
        )
        .route("/api/v1/projects", get(projects))
        .route(
            "/api/v1/projects/:project_id/dashboard",
            get(project_dashboard),
        )
        .route(
            "/api/v1/projects/:project_id/resources",
            get(project_resources),
        )
        .route(
            "/api/v1/projects/:project_id/knowledge",
            get(project_knowledge),
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
        .route("/sessions", get(list_runtime_sessions).post(create_runtime_session))
        .route("/sessions/:session_id", get(get_runtime_session))
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
        Ok(raw) => serde_json::from_str(&raw)
            .map_err(|error| ApiError::from(AppError::from(error))),
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

    Ok(Json(ShellBootstrap {
        host_state: state.host_state.clone(),
        preferences: load_host_preferences(&state)?,
        connections: state.host_connections.clone(),
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

async fn login(
    State(state): State<ServerState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<octopus_core::LoginResponse>, ApiError> {
    Ok(Json(state.services.auth.login(request).await?))
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

async fn project_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectDashboardSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;

    let project = lookup_project(&state, &project_id).await?;
    let conversations = list_conversation_records(&state, Some(&project_id)).await?;
    let recent_activity = list_activity_records(&state, Some(&project_id)).await?;
    let resources = state.services.workspace.list_project_resources(&project_id).await?;
    let knowledge = state.services.workspace.list_project_knowledge(&project_id).await?;
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
    Ok(Json(state.services.workspace.list_workspace_resources().await?))
}

async fn project_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResourceRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(state.services.workspace.list_project_resources(&project_id).await?))
}

async fn workspace_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_workspace_knowledge().await?))
}

async fn project_knowledge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<KnowledgeRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", Some(&project_id)).await?;
    Ok(Json(state.services.workspace.list_project_knowledge(&project_id).await?))
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
    Json(record): Json<AgentRecord>,
) -> Result<Json<AgentRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.create_agent(record).await?))
}

async fn update_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(record): Json<AgentRecord>,
) -> Result<Json<AgentRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.update_agent(&agent_id, record).await?))
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
    Json(record): Json<TeamRecord>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.create_team(record).await?))
}

async fn update_team(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
    Json(record): Json<TeamRecord>,
) -> Result<Json<TeamRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.update_team(&team_id, record).await?))
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

async fn workspace_catalog_models(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ModelCatalogSnapshot>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(ModelCatalogSnapshot {
        models: state.services.workspace.list_models().await?,
        provider_credentials: state.services.workspace.list_provider_credentials().await?,
    }))
}

async fn workspace_provider_credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderCredentialRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_provider_credentials().await?))
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
    Ok(Json(state.services.workspace.update_tool(&tool_id, record).await?))
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
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.create_automation(record).await?))
}

async fn update_automation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(automation_id): Path<String>,
    Json(record): Json<AutomationRecord>,
) -> Result<Json<AutomationRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", record.project_id.as_deref()).await?;
    Ok(Json(state.services.workspace.update_automation(&automation_id, record).await?))
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
        .filter(|record| current_user.role_ids.iter().any(|role_id| role_id == &record.id))
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
    Json(record): Json<UserRecordSummary>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.create_user(record).await?))
}

async fn update_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(record): Json<UserRecordSummary>,
) -> Result<Json<UserRecordSummary>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.update_user(&user_id, record).await?))
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
    Ok(Json(state.services.workspace.update_role(&role_id, record).await?))
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
    Ok(Json(state.services.workspace.create_permission(record).await?))
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
    Ok(Json(state.services.workspace.update_menu(&menu_id, record).await?))
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
            description: format!("{} {} {}", record.actor_type, record.actor_id, record.outcome),
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
    let session =
        ensure_authorized_session_with_request_id(&state, &headers, "runtime.read", project_id, &request_id)
            .await?;
    let idempotency_scope = idempotency_key(&headers)
        .map(|key| idempotency_scope(&session, "runtime.create_session", &input.conversation_id, &key));
    if let Some(scope) = idempotency_scope.as_deref() {
        if let Some(response) = load_idempotent_response(&state, scope, &request_id)? {
            return Ok(response);
        }
    }

    let detail = state.services.runtime_session.create_session(input).await?;
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
    Ok(Json(state.services.runtime_session.get_session(&session_id).await?))
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
    ensure_authorized_session_with_request_id(&state, &headers, "runtime.read", project_id.as_deref(), &request_id)
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
    let normalized = normalize_runtime_permission_mode_label(&input.permission_mode).ok_or_else(|| {
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
    let detail = state.services.runtime_session.get_session(session_id).await?;
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

    use axum::{body::{to_bytes, Body}, http::{Method, Request}};
    use octopus_core::{
        ApiErrorEnvelope, CreateRuntimeSessionInput, LoginRequest, LoginResponse,
        ResolveRuntimeApprovalInput, RuntimeEventEnvelope, RuntimeSessionDetail,
        RuntimeRunSnapshot, SessionRecord, SubmitRuntimeTurnInput,
    };
    use octopus_infra::{build_infra_bundle, InfraBundle};
    use octopus_platform::{ObservationService, PlatformServices};
    use octopus_runtime_adapter::RuntimeAdapter;
    use tokio_stream::StreamExt;
    use tower::ServiceExt;

    use super::*;

    #[derive(Clone)]
    struct TestHarness {
        router: Router,
        infra: InfraBundle,
    }

    fn test_harness() -> TestHarness {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let preferences_path = root.join("shell-preferences.json");
        std::mem::forget(temp);
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let runtime = Arc::new(RuntimeAdapter::new(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
        ));
        let services = PlatformServices {
            workspace: infra.workspace.clone(),
            auth: infra.auth.clone(),
            app_registry: infra.app_registry.clone(),
            rbac: infra.rbac.clone(),
            runtime_session: runtime.clone(),
            runtime_execution: runtime,
            artifact: infra.artifact.clone(),
            inbox: infra.inbox.clone(),
            knowledge: infra.knowledge.clone(),
            observation: infra.observation.clone(),
        };
        let router = build_router(ServerState {
            services,
            host_auth_token: "desktop-test-token".into(),
            transport_security: "loopback".into(),
            idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
            host_state: octopus_core::default_host_state("0.1.0-test".into(), true),
            host_connections: octopus_core::default_connection_stubs(),
            host_preferences_path: preferences_path,
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
        });

        TestHarness { router, infra }
    }

    async fn decode_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json body")
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
                            password: "owner".into(),
                            workspace_id: None,
                        })
                        .expect("json"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json::<LoginResponse>(response).await.session
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

    async fn submit_turn(
        router: &Router,
        token: &str,
        session_id: &str,
        permission_mode: &str,
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
                    .body(Body::from(
                        serde_json::to_vec(&SubmitRuntimeTurnInput {
                            content: "hello".into(),
                            model_id: "claude-sonnet-4-5".into(),
                            permission_mode: permission_mode.into(),
                        })
                        .expect("json"),
                    ))
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
                    .uri(format!("/api/v1/runtime/sessions/{session_id}/events?after={after}"))
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
            .oneshot(
                request
                    .body(Body::empty())
                    .expect("request"),
            )
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
                            password: "owner".into(),
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
        assert!(preferences.left_sidebar_collapsed);
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
    async fn runtime_session_flow_supports_json_event_polling_and_observation_with_session_token() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session(&harness.router, &session.token, "Session", None).await;

        let run = submit_turn(&harness.router, &session.token, &created.summary.id, "ask", None).await;
        assert_eq!(run.status, "waiting_approval");

        let events_response = harness
            .router
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/runtime/sessions/{}/events?after=missing", created.summary.id))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(events_response.status(), StatusCode::OK);
        let events = decode_json::<Vec<RuntimeEventEnvelope>>(events_response).await;
        assert!(events.iter().any(|event| event.event_type == "runtime.approval.requested"));
        assert!(events.iter().any(|event| event.event_type == "runtime.run.updated"));

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
        assert!(trace_events.iter().any(|event| event.event_kind == "turn_submitted"));
        assert!(audit_records
            .iter()
            .any(|record| record.action == "runtime.submit_turn"));
    }

    #[tokio::test]
    async fn runtime_events_support_sse_and_polling_consistency_for_session_tokens() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session(&harness.router, &session.token, "SSE Session", None).await;
        let initial_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            "missing",
        )
        .await;
        let baseline_event = initial_events
            .last()
            .expect("baseline event")
            .id
            .clone();

        let sse_event = next_sse_event(&harness.router, &session.token, &created.summary.id, None, true).await;
        let polled_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            &baseline_event,
        )
        .await;

        assert!(polled_events.iter().any(|event| event.id == sse_event.id));
        assert!(polled_events.iter().any(|event| event.event_type == "runtime.approval.requested"));
    }

    #[tokio::test]
    async fn runtime_events_support_sse_backlog_replay_with_last_event_id() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created =
            create_runtime_session(&harness.router, &session.token, "Replay Session", None).await;
        submit_turn(&harness.router, &session.token, &created.summary.id, "ask", None).await;

        let initial_events = runtime_events_after(
            &harness.router,
            &session.token,
            &created.summary.id,
            "missing",
        )
        .await;
        let baseline_event = initial_events
            .first()
            .expect("baseline event")
            .id
            .clone();

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
        let sessions = decode_json::<Vec<octopus_core::RuntimeSessionSummary>>(sessions_response).await;
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
        submit_turn(&harness.router, &session.token, &approved_session.summary.id, "ask", None).await;
        let detail = runtime_session_detail(&harness.router, &session.token, &approved_session.summary.id).await;
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
        submit_turn(&harness.router, &session.token, &rejected_session.summary.id, "ask", None).await;
        let reject_detail =
            runtime_session_detail(&harness.router, &session.token, &rejected_session.summary.id).await;
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
}
