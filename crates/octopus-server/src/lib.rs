mod dto_mapping;
mod handlers;
mod routes;
mod workspace_runtime;

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
    timestamp_now, AccessAuditListResponse, AccessAuditQuery, AccessCapabilityBundle,
    AccessExperienceResponse, AccessExperienceSnapshot, AccessExperienceSummary,
    AccessMemberSummary, AccessRolePreset, AccessRoleRecord, AccessRoleTemplate,
    AccessSectionGrant, AccessSessionRecord, AccessUserPresetUpdateRequest, AccessUserRecord,
    AccessUserUpsertRequest, AgentRecord, ApiErrorDetail, ApiErrorEnvelope, AppError, AuditRecord,
    AuthorizationRequest, AuthorizationSnapshot, CapabilityAssetDisablePatch,
    ChangeCurrentUserPasswordRequest, ChangeCurrentUserPasswordResponse, ClientAppRecord,
    ConnectionProfile, CopyWorkspaceSkillToManagedInput, CreateHostWorkspaceConnectionInput,
    CreateMenuPolicyRequest, CreateNotificationInput, CreateProjectRequest,
    CreateWorkspaceResourceFolderInput, CreateWorkspaceResourceInput, CreateWorkspaceSkillInput,
    DataPolicyRecord, DataPolicyUpsertRequest, DesktopBackendConnection, FeatureDefinition,
    HealthcheckBackendStatus, HealthcheckStatus, HostReleaseSummary, HostState, HostUpdateStatus,
    HostWorkspaceConnectionRecord, ImportWorkspaceAgentBundleInput,
    ImportWorkspaceAgentBundlePreview, ImportWorkspaceAgentBundlePreviewInput,
    ImportWorkspaceAgentBundleResult, ImportWorkspaceSkillArchiveInput,
    ImportWorkspaceSkillFolderInput, KnowledgeRecord, LoginRequest, MenuDefinition, MenuGateResult,
    MenuPolicyRecord, MenuPolicyUpsertRequest, ModelCatalogSnapshot, NotificationFilter,
    NotificationListResponse, NotificationRecord, NotificationUnreadSummary, OrgUnitRecord,
    OrgUnitUpsertRequest, PermissionDefinition, PetConversationBinding, PetPresenceState,
    PetWorkspaceSnapshot, PositionRecord, PositionUpsertRequest, ProjectAgentLinkInput,
    ProjectAgentLinkRecord, ProjectDashboardSnapshot, ProjectRecord, ProjectTeamLinkInput,
    ProjectTeamLinkRecord, PromoteWorkspaceResourceInput, ProtectedResourceDescriptor,
    ProtectedResourceMetadataUpsertRequest, ProviderCredentialRecord,
    RegisterBootstrapAdminRequest, ResolveRuntimeApprovalInput, ResourceActionGrant,
    ResourcePolicyRecord, ResourcePolicyUpsertRequest, RoleBindingRecord, RoleBindingUpsertRequest,
    RoleUpsertRequest, RuntimeConfigPatch, RuntimeConfigValidationResult,
    RuntimeConfiguredModelCredentialRecord, RuntimeConfiguredModelCredentialUpsertInput,
    RuntimeConfiguredModelProbeInput, RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig,
    SavePetPresenceInput, SessionRecord, ShellBootstrap, ShellPreferences, SubmitRuntimeTurnInput,
    TeamRecord, ToolRecord, UpdateCurrentUserProfileRequest, UpdateProjectRequest,
    UpdateWorkspaceResourceInput, UpdateWorkspaceSkillFileInput, UpdateWorkspaceSkillInput,
    UpsertAgentInput, UpsertTeamInput, UpsertWorkspaceMcpServerInput, UserGroupRecord,
    UserGroupUpsertRequest, UserOrgAssignmentRecord, UserOrgAssignmentUpsertRequest,
    UserRecordSummary, WorkspaceActivityRecord, WorkspaceDirectoryBrowserResponse,
    WorkspaceMcpServerDocument, WorkspaceMetricRecord, WorkspaceOverviewSnapshot,
    WorkspaceResourceChildrenRecord, WorkspaceResourceContentDocument,
    WorkspaceResourceImportInput, WorkspaceResourceRecord, WorkspaceSkillDocument,
    WorkspaceSkillFileDocument, WorkspaceSkillTreeDocument, WorkspaceSummary,
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
    pub auth_rate_limits: Arc<Mutex<HashMap<String, AuthRateLimitState>>>,
    pub host_state: HostState,
    pub host_connections: Vec<ConnectionProfile>,
    pub host_preferences_path: PathBuf,
    pub host_workspace_connections_path: PathBuf,
    pub host_default_preferences: ShellPreferences,
    pub backend_connection: DesktopBackendConnection,
}

#[derive(Clone, Debug, Default)]
pub struct AuthRateLimitState {
    failed_attempts: Vec<u64>,
    locked_until: Option<u64>,
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
            AppError::Auth(message) if message.contains("authorization stale") => {
                (StatusCode::FORBIDDEN, "AUTHORIZATION_STALE", false)
            }
            AppError::Auth(message)
                if message.contains("access denied")
                    || message.contains("no matching role permission")
                    || message.contains("workspace scope mismatch")
                    || message.contains("resource access denied")
                    || message.contains("data policy denied")
                    || message.contains("data policy allow missing")
                    || message.contains("resource allow missing") =>
            {
                (StatusCode::FORBIDDEN, "PERMISSION_DENIED", false)
            }
            AppError::Auth(message)
                if message.contains("too many failed attempts")
                    || message.contains("authentication temporarily locked") =>
            {
                (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", false)
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
    authorize_request(
        state,
        session,
        &AuthorizationRequest {
            subject_id: session.user_id.clone(),
            capability: capability.into(),
            project_id: project_id.map(str::to_string),
            resource_type: None,
            resource_id: None,
            resource_subtype: None,
            tags: Vec::new(),
            classification: None,
            owner_subject_type: None,
            owner_subject_id: None,
        },
        request_id,
    )
    .await
}

async fn ensure_authorized_request(
    state: &ServerState,
    headers: &HeaderMap,
    authorization_request: &AuthorizationRequest,
) -> Result<SessionRecord, ApiError> {
    let request_id = request_id(headers);
    let session = authenticate_session_with_request_id(state, headers, &request_id).await?;
    authorize_request(state, &session, authorization_request, &request_id).await?;
    Ok(session)
}

async fn authorize_request(
    state: &ServerState,
    session: &SessionRecord,
    authorization_request: &AuthorizationRequest,
    request_id: &str,
) -> Result<(), ApiError> {
    let mut decision = state
        .services
        .authorization
        .authorize_request(session, authorization_request)
        .await?;
    if decision.allowed {
        if let Some(project_id) = authorization_request.project_id.as_deref() {
            if let Some(reason) = evaluate_project_authorization_denial(
                state,
                session,
                authorization_request,
                project_id,
            )
            .await?
            {
                decision.allowed = false;
                decision.reason = Some(reason);
            }
        }
    }
    if is_sensitive_capability(&authorization_request.capability) {
        let resource_type = authorization_request
            .resource_type
            .as_deref()
            .unwrap_or("authorization");
        let resource =
            audit_resource_label(resource_type, authorization_request.resource_id.as_deref());
        let outcome = if decision.allowed {
            "allowed".to_string()
        } else {
            format!(
                "denied:{}",
                decision
                    .reason
                    .clone()
                    .unwrap_or_else(|| "access denied".into())
            )
        };
        append_session_audit(
            state,
            session,
            &authorization_request.capability,
            &resource,
            &outcome,
            authorization_request.project_id.clone(),
        )
        .await?;
    }
    if !decision.allowed {
        return Err(ApiError::new(
            AppError::auth(decision.reason.unwrap_or_else(|| "access denied".into())),
            request_id,
        ));
    }
    Ok(())
}

async fn evaluate_project_authorization_denial(
    state: &ServerState,
    session: &SessionRecord,
    authorization_request: &AuthorizationRequest,
    project_id: &str,
) -> Result<Option<String>, ApiError> {
    let project = state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))?;

    if !project
        .member_user_ids
        .iter()
        .any(|user_id| user_id == &session.user_id)
    {
        return Ok(Some("project membership is required".into()));
    }

    let Some(module) = project_module_for_request(authorization_request) else {
        return Ok(None);
    };
    let workspace = state.services.workspace.workspace_summary().await?;
    if resolve_project_module_permission(&workspace, &project, module) == "deny" {
        return Ok(Some(format!(
            "project module {module} is not available for this project"
        )));
    }
    Ok(None)
}

fn project_module_for_request(
    authorization_request: &AuthorizationRequest,
) -> Option<&'static str> {
    if authorization_request.capability.starts_with("agent.")
        || authorization_request.capability.starts_with("team.")
    {
        return Some("agents");
    }
    if authorization_request.capability.starts_with("resource.") {
        return Some("resources");
    }
    if authorization_request.capability.starts_with("knowledge.") {
        return Some("knowledge");
    }
    if authorization_request.capability.starts_with("tool.") {
        return Some("tools");
    }

    match authorization_request.resource_type.as_deref() {
        Some("agent") | Some("team") => Some("agents"),
        Some("resource") => Some("resources"),
        Some("knowledge") => Some("knowledge"),
        Some(resource_type) if resource_type.starts_with("tool.") => Some("tools"),
        _ => None,
    }
}

fn resolve_project_module_permission<'a>(
    workspace: &'a WorkspaceSummary,
    project: &'a ProjectRecord,
    module: &'a str,
) -> &'a str {
    let default_value = match module {
        "agents" => workspace.project_default_permissions.agents.as_str(),
        "resources" => workspace.project_default_permissions.resources.as_str(),
        "tools" => workspace.project_default_permissions.tools.as_str(),
        "knowledge" => workspace.project_default_permissions.knowledge.as_str(),
        _ => "allow",
    };
    let override_value = match module {
        "agents" => project.permission_overrides.agents.as_str(),
        "resources" => project.permission_overrides.resources.as_str(),
        "tools" => project.permission_overrides.tools.as_str(),
        "knowledge" => project.permission_overrides.knowledge.as_str(),
        _ => "inherit",
    };
    if override_value == "inherit" {
        default_value
    } else {
        override_value
    }
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
        if let Some(permission_mode) = input.permission_mode.as_deref() {
            if permission_mode.trim().is_empty() {
                return Err(ApiError::new(
                    AppError::invalid_input("permission mode must not be empty"),
                    request_id,
                ));
            }
        }
    }
    Ok(session)
}

fn normalize_runtime_submit_input(input: &mut SubmitRuntimeTurnInput) -> Result<(), ApiError> {
    input.permission_mode = input
        .permission_mode
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(permission_mode) = input.permission_mode.as_deref() {
        let normalized =
            normalize_runtime_permission_mode_label(permission_mode).ok_or_else(|| {
                ApiError::from(AppError::invalid_input(format!(
                    "unsupported permission mode: {permission_mode}"
                )))
            })?;
        input.permission_mode = Some(normalized.to_string());
    }
    input.recall_mode = input
        .recall_mode
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(recall_mode) = input.recall_mode.as_deref() {
        if !matches!(recall_mode, "default" | "skip") {
            return Err(ApiError::from(AppError::invalid_input(format!(
                "unsupported recall mode: {recall_mode}"
            ))));
        }
    }
    input.ignored_memory_ids = input
        .ignored_memory_ids
        .drain(..)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    input.memory_intent = input
        .memory_intent
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
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

const AUTH_RATE_LIMIT_WINDOW_SECONDS: u64 = 10 * 60;
const AUTH_RATE_LIMIT_MAX_FAILURES: usize = 5;
const AUTH_RATE_LIMIT_LOCK_SECONDS: u64 = 15 * 60;

async fn workspace_id_for_audit(state: &ServerState) -> Result<String, ApiError> {
    Ok(state.services.workspace.workspace_summary().await?.id)
}

fn audit_resource_label(resource_type: &str, resource_id: Option<&str>) -> String {
    resource_id
        .map(|id| format!("{resource_type}:{id}"))
        .unwrap_or_else(|| resource_type.to_string())
}

async fn append_audit_event(
    state: &ServerState,
    workspace_id: &str,
    project_id: Option<String>,
    actor_type: &str,
    actor_id: &str,
    action: &str,
    resource: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    state
        .services
        .observation
        .append_audit(AuditRecord {
            id: format!("audit-{}", Uuid::new_v4()),
            workspace_id: workspace_id.to_string(),
            project_id,
            actor_type: actor_type.to_string(),
            actor_id: actor_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            outcome: outcome.to_string(),
            created_at: timestamp_now(),
        })
        .await?;
    Ok(())
}

async fn append_session_audit(
    state: &ServerState,
    session: &SessionRecord,
    action: &str,
    resource: &str,
    outcome: &str,
    project_id: Option<String>,
) -> Result<(), ApiError> {
    append_audit_event(
        state,
        &session.workspace_id,
        project_id,
        "user",
        &session.user_id,
        action,
        resource,
        outcome,
    )
    .await
}

fn auth_source_fingerprint(headers: &HeaderMap) -> String {
    [
        "x-forwarded-for",
        "x-real-ip",
        "cf-connecting-ip",
        "user-agent",
    ]
    .iter()
    .find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
    .unwrap_or_else(|| "unknown".into())
}

fn auth_rate_limit_key(workspace_id: &str, username: &str, headers: &HeaderMap) -> String {
    format!(
        "{workspace_id}:{}:{}",
        username.trim().to_lowercase(),
        auth_source_fingerprint(headers)
    )
}

fn check_auth_rate_limit(state: &ServerState, key: &str) -> Result<Option<u64>, ApiError> {
    let now = timestamp_now();
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    let Some(entry) = rate_limits.get_mut(key) else {
        return Ok(None);
    };
    entry
        .failed_attempts
        .retain(|attempt| now.saturating_sub(*attempt) <= AUTH_RATE_LIMIT_WINDOW_SECONDS);
    if let Some(locked_until) = entry.locked_until {
        if locked_until > now {
            return Ok(Some(locked_until));
        }
        entry.locked_until = None;
        entry.failed_attempts.clear();
    }
    Ok(None)
}

fn record_auth_failure(state: &ServerState, key: &str) -> Result<Option<u64>, ApiError> {
    let now = timestamp_now();
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    let entry = rate_limits.entry(key.to_string()).or_default();
    entry
        .failed_attempts
        .retain(|attempt| now.saturating_sub(*attempt) <= AUTH_RATE_LIMIT_WINDOW_SECONDS);
    entry.failed_attempts.push(now);
    if entry.failed_attempts.len() >= AUTH_RATE_LIMIT_MAX_FAILURES {
        let locked_until = now + AUTH_RATE_LIMIT_LOCK_SECONDS;
        entry.locked_until = Some(locked_until);
        entry.failed_attempts.clear();
        return Ok(Some(locked_until));
    }
    Ok(None)
}

fn clear_auth_failures(state: &ServerState, key: &str) -> Result<bool, ApiError> {
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    Ok(rate_limits.remove(key).is_some())
}

fn is_sensitive_capability(capability: &str) -> bool {
    matches!(
        capability.rsplit('.').next(),
        Some("run")
            | Some("invoke")
            | Some("publish")
            | Some("delete")
            | Some("grant")
            | Some("export")
            | Some("retrieve")
            | Some("bind-credential")
    )
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
