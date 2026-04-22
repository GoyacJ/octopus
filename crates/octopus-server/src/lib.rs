mod auth_rate_limit;
mod audit_support;
mod dto_mapping;
mod handlers;
mod http_support;
mod routes;
mod runtime_support;
mod session_auth;
#[cfg(test)]
pub(crate) mod test_runtime_sdk;
#[cfg(test)]
mod lib_tests;
mod workspace_runtime;

use std::{
    collections::HashMap,
    env, fs,
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
    ProjectAgentLinkRecord, ProjectRecord, ProjectTeamLinkInput, ProjectTeamLinkRecord,
    PromoteWorkspaceResourceInput, ProtectedResourceDescriptor,
    ProtectedResourceMetadataUpsertRequest, ProviderCredentialRecord,
    RegisterBootstrapAdminRequest, ResolveRuntimeApprovalInput, ResourceActionGrant,
    ResourcePolicyRecord, ResourcePolicyUpsertRequest, RoleBindingRecord, RoleBindingUpsertRequest,
    RoleUpsertRequest, RunRuntimeGenerationInput, RuntimeConfigPatch,
    RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, SavePetPresenceInput, SessionRecord,
    ShellBootstrap, ShellPreferences, SubmitRuntimeTurnInput, TeamRecord, ToolRecord,
    UpdateCurrentUserProfileRequest, UpdateProjectRequest, UpdateWorkspaceResourceInput,
    UpdateWorkspaceSkillFileInput, UpdateWorkspaceSkillInput, UpsertAgentInput, UpsertTeamInput,
    UpsertWorkspaceMcpServerInput, UserGroupRecord, UserGroupUpsertRequest,
    UserOrgAssignmentRecord, UserOrgAssignmentUpsertRequest, UserRecordSummary,
    WorkspaceActivityRecord, WorkspaceDirectoryBrowserResponse, WorkspaceMcpServerDocument,
    WorkspaceMetricRecord, WorkspaceOverviewSnapshot, WorkspaceResourceChildrenRecord,
    WorkspaceResourceContentDocument, WorkspaceResourceImportInput, WorkspaceResourceRecord,
    WorkspaceSkillDocument, WorkspaceSkillFileDocument, WorkspaceSkillTreeDocument,
    WorkspaceSummary,
};
use octopus_persistence::Database;
use octopus_platform::PlatformServices;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct ServerState {
    pub services: PlatformServices,
    pub host_notifications_db: Database,
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

const HEADER_REQUEST_ID: &str = "x-request-id";
const HEADER_WORKSPACE_ID: &str = "x-workspace-id";
const HEADER_IDEMPOTENCY_KEY: &str = "idempotency-key";
const HEADER_LAST_EVENT_ID: &str = "last-event-id";


pub(crate) use auth_rate_limit::*;
pub(crate) use audit_support::*;
pub(crate) use http_support::*;
pub(crate) use runtime_support::*;
pub(crate) use session_auth::*;
pub use routes::build_router;
