mod dto_mapping;
mod handlers;
mod routes;
mod workspace_runtime;

#[cfg(test)]
mod split_module_tests;

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
    release_repo: Option<String>,
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

pub use routes::build_router;

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
