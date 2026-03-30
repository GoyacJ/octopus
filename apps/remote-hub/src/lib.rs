use std::convert::Infallible;

use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use futures_util::stream;
use octopus_access_auth::{AccessAuthError, HubLoginResponse, HubSession, RemoteAccessService};
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, ApprovalRequestRecord, ArtifactRecord, AuditRecord, AutomationDetailRecord,
    AutomationRecord, AutomationSummaryRecord, CapabilityResolutionRecord, CreateAutomationInput,
    CreateTaskInput, CreateTriggerInput, DispatchManualEventInput, DispatchWebhookEventInput,
    InboxItemRecord, KnowledgeAssetRecord, KnowledgeCandidateRecord, KnowledgeLineageRecord,
    KnowledgeSpaceRecord, NotificationRecord, PolicyDecisionLogRecord, ProjectContext,
    ProjectRecord,
    ProjectKnowledgeIndexRecord, RunExecutionReport, RunRecord, RunSummaryRecord, RuntimeError,
    Slice1Runtime, TaskRecord, TraceRecord, TriggerDeliveryRecord, TriggerRecord, TriggerSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

mod dev_seed;

pub use dev_seed::ensure_dev_seed_context;

#[derive(Clone)]
pub struct AppState {
    runtime: Slice1Runtime,
    auth: RemoteAccessService,
}

impl AppState {
    pub fn new(runtime: Slice1Runtime, auth: RemoteAccessService) -> Self {
        Self { runtime, auth }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", get(get_current_session))
        .route("/api/auth/logout", post(logout_session))
        .route("/api/workspaces/{workspace_id}/projects", get(list_projects))
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/context",
            get(get_project_context),
        )
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/knowledge",
            get(get_project_knowledge),
        )
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/automations",
            get(list_automations).post(create_automation),
        )
        .route(
            "/api/automations/{automation_id}",
            get(get_automation_detail),
        )
        .route(
            "/api/automations/{automation_id}/activate",
            post(activate_automation),
        )
        .route(
            "/api/automations/{automation_id}/pause",
            post(pause_automation),
        )
        .route(
            "/api/automations/{automation_id}/archive",
            post(archive_automation),
        )
        .route(
            "/api/triggers/{trigger_id}/manual-dispatch",
            post(dispatch_manual_trigger),
        )
        .route(
            "/api/trigger-deliveries/{delivery_id}/retry",
            post(retry_trigger_delivery),
        )
        .route("/api/tasks", post(create_task))
        .route("/api/tasks/{task_id}/start", post(start_task))
        .route("/api/triggers/{trigger_id}/webhook", post(dispatch_webhook))
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/runs",
            get(list_runs),
        )
        .route("/api/runs/{run_id}", get(get_run_detail))
        .route("/api/runs/{run_id}/retry", post(retry_run))
        .route("/api/runs/{run_id}/terminate", post(terminate_run))
        .route("/api/runs/{run_id}/artifacts", get(list_artifacts))
        .route("/api/runs/{run_id}/knowledge", get(get_knowledge_detail))
        .route("/api/approvals/{approval_id}", get(get_approval_request))
        .route(
            "/api/approvals/{approval_id}/resolve",
            post(resolve_approval),
        )
        .route(
            "/api/workspaces/{workspace_id}/inbox",
            get(list_inbox_items),
        )
        .route(
            "/api/workspaces/{workspace_id}/notifications",
            get(list_notifications),
        )
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/capabilities",
            get(list_capability_visibility),
        )
        .route(
            "/api/knowledge/candidates/{candidate_id}/request-promotion",
            post(request_knowledge_promotion),
        )
        .route(
            "/api/knowledge/candidates/{candidate_id}/promote",
            post(promote_knowledge),
        )
        .route("/api/hub/connection", get(get_hub_connection_status))
        .route("/api/events", get(stream_events))
        .with_state(state)
}

#[derive(Debug, Error)]
enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Auth(#[from] AccessAuthError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Auth(error) => return auth_error_response(error),
            Self::Runtime(error) => match error {
                RuntimeError::TaskNotFound(message)
                | RuntimeError::RunNotFound(message)
                | RuntimeError::ApprovalRequestNotFound(message)
                | RuntimeError::KnowledgeCandidateNotFound(message)
                | RuntimeError::KnowledgeSpaceNotFound(message)
                | RuntimeError::AutomationNotFound(message)
                | RuntimeError::TriggerNotFound(message)
                | RuntimeError::TriggerDeliveryNotFound(message) => {
                    (StatusCode::NOT_FOUND, message)
                }
                RuntimeError::InvalidRunTransition { run_id, from, to } => (
                    StatusCode::BAD_REQUEST,
                    format!("invalid run transition for `{run_id}`: `{from}` -> `{to}`"),
                ),
                RuntimeError::InvalidAutomationLifecycleTransition {
                    automation_id,
                    from,
                    to,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "automation `{automation_id}` cannot transition from `{from}` to `{to}`"
                    ),
                ),
                RuntimeError::InvalidTriggerType {
                    trigger_id,
                    trigger_type,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!("trigger `{trigger_id}` has unsupported type `{trigger_type}`"),
                ),
                RuntimeError::MissingWebhookIdempotencyKey { trigger_id } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "webhook event for trigger `{trigger_id}` requires a non-empty idempotency key"
                    ),
                ),
                RuntimeError::InvalidWebhookSecret { trigger_id } => (
                    StatusCode::UNAUTHORIZED,
                    format!("webhook event for trigger `{trigger_id}` has invalid secret"),
                ),
                RuntimeError::McpServerNotFound(server_id) => {
                    (StatusCode::NOT_FOUND, format!("mcp server `{server_id}` not found"))
                }
                RuntimeError::McpEventMismatch {
                    trigger_id,
                    event_name,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!("mcp event `{event_name}` does not match trigger `{trigger_id}` selector"),
                ),
                RuntimeError::InvalidTriggerDeliveryTransition {
                    delivery_id,
                    from,
                    to,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "trigger delivery `{delivery_id}` cannot transition from `{from}` to `{to}`"
                    ),
                ),
                RuntimeError::InvalidKnowledgeCandidateState {
                    candidate_id,
                    status,
                    expected,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "knowledge candidate `{candidate_id}` has invalid status `{status}`; expected `{expected}`"
                    ),
                ),
                RuntimeError::InvalidApprovalType {
                    approval_id,
                    approval_type,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "approval `{approval_id}` has unsupported type `{approval_type}`"
                    ),
                ),
                RuntimeError::InvalidApprovalTargetRef {
                    approval_id,
                    target_ref,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "approval `{approval_id}` has invalid target_ref `{target_ref}`"
                    ),
                ),
                other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
            },
        };

        (
            status,
            Json(json!({
                "error": message
            })),
        )
            .into_response()
    }
}

type AppResult<T> = Result<Json<T>, AppError>;

#[derive(Debug, Deserialize)]
struct SurfaceTaskCreateCommand {
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: ExecutionAction,
    capability_id: String,
    estimated_cost: i64,
    idempotency_key: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceCreateAutomationCommand {
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: ExecutionAction,
    capability_id: String,
    estimated_cost: i64,
    trigger: SurfaceCreateTriggerInput,
}

#[derive(Debug, Deserialize)]
struct SurfaceManualEventTriggerConfig {}

#[derive(Debug, Deserialize)]
struct SurfaceCronTriggerConfig {
    schedule: String,
    timezone: String,
    next_fire_at: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceWebhookTriggerConfig {
    ingress_mode: String,
    secret_header_name: String,
    secret_hint: Option<String>,
    secret_plaintext: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CapabilityResolutionQuery {
    estimated_cost: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SurfaceMcpEventTriggerConfig {
    server_id: String,
    event_name: Option<String>,
    event_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "trigger_type", content = "config", rename_all = "snake_case")]
enum SurfaceCreateTriggerInput {
    ManualEvent(SurfaceManualEventTriggerConfig),
    Cron(SurfaceCronTriggerConfig),
    Webhook(SurfaceWebhookTriggerConfig),
    McpEvent(SurfaceMcpEventTriggerConfig),
}

impl SurfaceCreateTriggerInput {
    fn into_runtime(self) -> CreateTriggerInput {
        match self {
            Self::ManualEvent(_) => CreateTriggerInput::ManualEvent,
            Self::Cron(config) => CreateTriggerInput::Cron {
                schedule: config.schedule,
                timezone: config.timezone,
                next_fire_at: config.next_fire_at,
            },
            Self::Webhook(config) => CreateTriggerInput::Webhook {
                ingress_mode: config.ingress_mode,
                secret_header_name: config.secret_header_name,
                secret_hint: config.secret_hint,
                secret_plaintext: config.secret_plaintext,
            },
            Self::McpEvent(config) => CreateTriggerInput::McpEvent {
                server_id: config.server_id,
                event_name: config.event_name,
                event_pattern: config.event_pattern,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct SurfaceCreateAutomationResponse {
    automation: AutomationRecord,
    trigger: TriggerRecord,
    webhook_secret: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceLoginCommand {
    workspace_id: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceAutomationLifecycleCommand {
    automation_id: String,
    action: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceApprovalResolveCommand {
    approval_id: String,
    decision: String,
    #[serde(rename = "actor_ref")]
    _actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRequestKnowledgePromotionCommand {
    candidate_id: String,
    #[serde(rename = "actor_ref")]
    _actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceKnowledgePromoteCommand {
    candidate_id: String,
    #[serde(rename = "actor_ref")]
    _actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceTriggerDeliveryRetryCommand {
    delivery_id: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRunRetryCommand {
    run_id: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRunTerminateCommand {
    run_id: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    workspace_id: Option<String>,
    run_id: Option<String>,
    access_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct RunSummaryResponse {
    id: String,
    task_id: String,
    workspace_id: String,
    project_id: String,
    title: String,
    run_type: String,
    status: String,
    approval_request_id: Option<String>,
    attempt_count: i64,
    max_attempts: i64,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    terminated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct RunDetailResponse {
    run: RunRecord,
    task: TaskRecord,
    artifacts: Vec<ArtifactRecord>,
    audits: Vec<AuditRecord>,
    traces: Vec<TraceRecord>,
    approvals: Vec<ApprovalRequestRecord>,
    inbox_items: Vec<InboxItemRecord>,
    notifications: Vec<NotificationRecord>,
    policy_decisions: Vec<PolicyDecisionLogRecord>,
    knowledge_candidates: Vec<KnowledgeCandidateRecord>,
    knowledge_assets: Vec<KnowledgeAssetRecord>,
    knowledge_lineage: Vec<KnowledgeLineageRecord>,
}

#[derive(Debug, Serialize)]
struct KnowledgeDetailResponse {
    knowledge_space: KnowledgeSpaceRecord,
    candidates: Vec<KnowledgeCandidateRecord>,
    assets: Vec<KnowledgeAssetRecord>,
    lineage: Vec<KnowledgeLineageRecord>,
}

#[derive(Debug, Serialize)]
struct HubConnectionServerSummary {
    id: String,
    capability_id: String,
    namespace: String,
    platform: String,
    trust_level: String,
    health_status: String,
    lease_ttl_seconds: i64,
    last_checked_at: String,
}

#[derive(Debug, Serialize)]
struct HubConnectionStatusResponse {
    mode: String,
    state: String,
    auth_state: String,
    active_server_count: usize,
    healthy_server_count: usize,
    servers: Vec<HubConnectionServerSummary>,
    last_refreshed_at: String,
}

#[derive(Debug, Serialize)]
struct WebhookDispatchResponse {
    trigger_id: String,
    delivery_id: String,
    run_id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct AuthErrorResponse {
    error: String,
    error_code: String,
    auth_state: String,
}

async fn login(
    State(state): State<AppState>,
    Json(command): Json<SurfaceLoginCommand>,
) -> Result<Json<HubLoginResponse>, AppError> {
    Ok(Json(
        state
            .auth
            .login(&command.workspace_id, &command.email, &command.password)
            .await?,
    ))
}

async fn get_current_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HubSession>, AppError> {
    let session = require_session(&state, &headers).await?;
    Ok(Json(session))
}

async fn logout_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    state.auth.logout(&session.session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_project_context(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> AppResult<ProjectContext> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .fetch_project_context(&workspace_id, &project_id)
            .await?,
    ))
}

async fn list_projects(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Vec<ProjectRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(state.runtime.list_projects(&workspace_id).await?))
}

async fn get_project_knowledge(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> AppResult<ProjectKnowledgeIndexRecord> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .get_project_knowledge_index(&workspace_id, &project_id)
            .await?,
    ))
}

async fn list_automations(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> AppResult<Vec<AutomationSummaryRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .list_automations(&workspace_id, &project_id)
            .await?,
    ))
}

async fn list_runs(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> AppResult<Vec<RunSummaryRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .list_runs(&workspace_id, &project_id)
            .await?,
    ))
}

async fn create_automation(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(command): Json<SurfaceCreateAutomationCommand>,
) -> AppResult<SurfaceCreateAutomationResponse> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;
    if workspace_id != command.workspace_id {
        return Err(AppError::BadRequest(
            "workspace_id path/body mismatch".to_string(),
        ));
    }
    if project_id != command.project_id {
        return Err(AppError::BadRequest(
            "project_id path/body mismatch".to_string(),
        ));
    }

    let report = state
        .runtime
        .create_automation_with_trigger(
            CreateAutomationInput {
                workspace_id: command.workspace_id,
                project_id: command.project_id,
                title: command.title,
                instruction: command.instruction,
                action: command.action,
                capability_id: command.capability_id,
                estimated_cost: command.estimated_cost,
            },
            command.trigger.into_runtime(),
        )
        .await?;

    Ok(Json(SurfaceCreateAutomationResponse {
        automation: report.automation,
        trigger: report.trigger,
        webhook_secret: report.webhook_secret,
    }))
}

async fn get_automation_detail(
    State(state): State<AppState>,
    Path(automation_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<AutomationDetailRecord> {
    let session = require_session(&state, &headers).await?;
    load_authorized_automation(&state, &session, &automation_id).await?;
    Ok(Json(
        state.runtime.load_automation_detail(&automation_id).await?,
    ))
}

async fn activate_automation(
    State(state): State<AppState>,
    Path(automation_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceAutomationLifecycleCommand>,
) -> AppResult<AutomationDetailRecord> {
    transition_automation(state, headers, automation_id, command, "activate").await
}

async fn pause_automation(
    State(state): State<AppState>,
    Path(automation_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceAutomationLifecycleCommand>,
) -> AppResult<AutomationDetailRecord> {
    transition_automation(state, headers, automation_id, command, "pause").await
}

async fn archive_automation(
    State(state): State<AppState>,
    Path(automation_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceAutomationLifecycleCommand>,
) -> AppResult<AutomationDetailRecord> {
    transition_automation(state, headers, automation_id, command, "archive").await
}

async fn create_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<SurfaceTaskCreateCommand>,
) -> AppResult<TaskRecord> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &command.workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .create_task(CreateTaskInput {
                workspace_id: command.workspace_id,
                project_id: command.project_id,
                source_kind: "manual".to_string(),
                automation_id: None,
                title: command.title,
                instruction: command.instruction,
                action: command.action,
                capability_id: command.capability_id,
                estimated_cost: command.estimated_cost,
                idempotency_key: command.idempotency_key,
            })
            .await?,
    ))
}

async fn dispatch_manual_trigger(
    State(state): State<AppState>,
    Path(trigger_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<DispatchManualEventInput>,
) -> AppResult<AutomationDetailRecord> {
    let session = require_session(&state, &headers).await?;
    let (_, automation) = load_authorized_trigger(&state, &session, &trigger_id).await?;
    if trigger_id != command.trigger_id {
        return Err(AppError::BadRequest(
            "trigger_id path/body mismatch".to_string(),
        ));
    }

    state.runtime.dispatch_manual_event(command).await?;
    Ok(Json(
        state.runtime.load_automation_detail(&automation.id).await?,
    ))
}

async fn dispatch_webhook(
    State(state): State<AppState>,
    Path(trigger_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> AppResult<WebhookDispatchResponse> {
    let trigger = state
        .runtime
        .fetch_trigger(trigger_id.as_str())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("trigger `{trigger_id}` not found")))?;
    let TriggerSpec::Webhook { config } = trigger.spec else {
        return Err(AppError::BadRequest(format!(
            "trigger `{trigger_id}` is not a webhook trigger"
        )));
    };

    let idempotency_key = required_header(&headers, "Idempotency-Key")?;
    let secret = required_header(&headers, config.secret_header_name.as_str())?;
    let report = state
        .runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id: trigger_id.clone(),
            idempotency_key,
            secret,
            payload,
        })
        .await?;

    Ok(Json(WebhookDispatchResponse {
        trigger_id,
        delivery_id: report.delivery.id,
        run_id: report.run_report.run.id,
        status: report.delivery.status,
    }))
}

async fn retry_trigger_delivery(
    State(state): State<AppState>,
    Path(delivery_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceTriggerDeliveryRetryCommand>,
) -> AppResult<AutomationDetailRecord> {
    let session = require_session(&state, &headers).await?;
    let (_, _, automation) =
        load_authorized_trigger_delivery(&state, &session, &delivery_id).await?;
    if delivery_id != command.delivery_id {
        return Err(AppError::BadRequest(
            "delivery_id path/body mismatch".to_string(),
        ));
    }

    state.runtime.retry_trigger_delivery(&delivery_id).await?;
    Ok(Json(
        state.runtime.load_automation_detail(&automation.id).await?,
    ))
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, AppError> {
    headers
        .get(name)
        .ok_or_else(|| AppError::BadRequest(format!("missing required header `{name}`")))?
        .to_str()
        .map(|value| value.to_string())
        .map_err(|_| AppError::BadRequest(format!("invalid header `{name}`")))
}

async fn start_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<RunDetailResponse> {
    let session = require_session(&state, &headers).await?;
    let task = state.runtime.fetch_task(&task_id).await?;
    state
        .auth
        .ensure_workspace_access(&session, &task.workspace_id)
        .await?;

    let report = state.runtime.start_task(&task_id).await?;
    Ok(Json(
        build_run_detail_response(&state.runtime, report).await?,
    ))
}

async fn get_run_detail(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<RunDetailResponse> {
    let session = require_session(&state, &headers).await?;
    let report = state.runtime.load_run_report(&run_id).await?;
    state
        .auth
        .ensure_workspace_access(&session, &report.run.workspace_id)
        .await?;
    Ok(Json(
        build_run_detail_response(&state.runtime, report).await?,
    ))
}

async fn retry_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceRunRetryCommand>,
) -> AppResult<RunDetailResponse> {
    let session = require_session(&state, &headers).await?;
    if run_id != command.run_id {
        return Err(AppError::BadRequest("run_id path/body mismatch".to_string()));
    }

    load_authorized_run(&state, &session, &run_id).await?;
    let report = state.runtime.retry_run(&run_id).await?;
    Ok(Json(
        build_run_detail_response(&state.runtime, report).await?,
    ))
}

async fn terminate_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceRunTerminateCommand>,
) -> AppResult<RunDetailResponse> {
    let session = require_session(&state, &headers).await?;
    if run_id != command.run_id {
        return Err(AppError::BadRequest("run_id path/body mismatch".to_string()));
    }

    load_authorized_run(&state, &session, &run_id).await?;
    let report = state.runtime.terminate_run(&run_id, &command.reason).await?;
    Ok(Json(
        build_run_detail_response(&state.runtime, report).await?,
    ))
}

async fn list_artifacts(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Vec<ArtifactRecord>> {
    let session = require_session(&state, &headers).await?;
    let report = state.runtime.load_run_report(&run_id).await?;
    state
        .auth
        .ensure_workspace_access(&session, &report.run.workspace_id)
        .await?;

    Ok(Json(state.runtime.list_artifacts_by_run(&run_id).await?))
}

async fn get_knowledge_detail(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<KnowledgeDetailResponse> {
    let session = require_session(&state, &headers).await?;
    let report = state.runtime.load_run_report(&run_id).await?;
    state
        .auth
        .ensure_workspace_access(&session, &report.run.workspace_id)
        .await?;

    Ok(Json(
        build_knowledge_detail_response(&state.runtime, &run_id).await?,
    ))
}

async fn get_approval_request(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<ApprovalRequestRecord> {
    let session = require_session(&state, &headers).await?;
    let approval = load_authorized_approval(&state, &session, &approval_id).await?;
    Ok(Json(approval))
}

async fn resolve_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceApprovalResolveCommand>,
) -> AppResult<RunDetailResponse> {
    let session = require_session(&state, &headers).await?;
    if approval_id != command.approval_id {
        return Err(AppError::BadRequest(
            "approval_id path/body mismatch".to_string(),
        ));
    }

    let decision = match command.decision.as_str() {
        "approve" => ApprovalDecision::Approve,
        "reject" => ApprovalDecision::Reject,
        "expire" => ApprovalDecision::Expire,
        "cancel" => ApprovalDecision::Cancel,
        other => {
            return Err(AppError::BadRequest(format!(
                "unsupported approval decision `{other}`"
            )))
        }
    };

    load_authorized_approval(&state, &session, &approval_id).await?;

    let report = state
        .runtime
        .resolve_approval(&approval_id, decision, &session.actor_ref, &command.note)
        .await?;
    Ok(Json(
        build_run_detail_response(&state.runtime, report).await?,
    ))
}

async fn list_inbox_items(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Vec<InboxItemRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .list_inbox_items_by_workspace(&workspace_id)
            .await?,
    ))
}

async fn list_notifications(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Vec<NotificationRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    Ok(Json(
        state
            .runtime
            .list_notifications_by_workspace(&workspace_id)
            .await?,
    ))
}

async fn list_capability_visibility(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
    Query(query): Query<CapabilityResolutionQuery>,
    headers: HeaderMap,
) -> AppResult<Vec<CapabilityResolutionRecord>> {
    let session = require_session(&state, &headers).await?;
    state
        .auth
        .ensure_workspace_access(&session, &workspace_id)
        .await?;

    let estimated_cost = query.estimated_cost.unwrap_or(1);
    if estimated_cost < 0 {
        return Err(AppError::BadRequest(
            "estimated_cost must be greater than or equal to 0".to_string(),
        ));
    }

    Ok(Json(
        state
            .runtime
            .list_capability_resolutions(&workspace_id, &project_id, estimated_cost)
            .await?,
    ))
}

async fn request_knowledge_promotion(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceRequestKnowledgePromotionCommand>,
) -> AppResult<ApprovalRequestRecord> {
    let session = require_session(&state, &headers).await?;
    if candidate_id != command.candidate_id {
        return Err(AppError::BadRequest(
            "candidate_id path/body mismatch".to_string(),
        ));
    }

    load_authorized_candidate(&state, &session, &candidate_id).await?;
    Ok(Json(
        state
            .runtime
            .request_knowledge_promotion(&candidate_id, &session.actor_ref, &command.note)
            .await?,
    ))
}

async fn promote_knowledge(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
    headers: HeaderMap,
    Json(command): Json<SurfaceKnowledgePromoteCommand>,
) -> AppResult<KnowledgeDetailResponse> {
    let session = require_session(&state, &headers).await?;
    if candidate_id != command.candidate_id {
        return Err(AppError::BadRequest(
            "candidate_id path/body mismatch".to_string(),
        ));
    }

    let _candidate = load_authorized_candidate(&state, &session, &candidate_id).await?;

    let report = state
        .runtime
        .promote_knowledge_candidate(&candidate_id, &session.actor_ref, &command.note)
        .await?;
    let run_id = report.candidate.source_run_id;
    Ok(Json(
        build_knowledge_detail_response(&state.runtime, &run_id).await?,
    ))
}

async fn get_hub_connection_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<HubConnectionStatusResponse> {
    let auth_state =
        resolve_auth_state(&state, auth_header_value(&headers).map(str::to_owned)).await;
    Ok(Json(
        build_hub_connection_status(&state.runtime, auth_state.as_str()).await?,
    ))
}

async fn stream_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let mut payloads = Vec::new();
    let mut sequence = 1_u64;
    let auth_state =
        resolve_auth_state(&state, event_token(&headers, query.access_token.as_deref())).await;

    let connection = build_hub_connection_status(&state.runtime, auth_state.as_str()).await?;
    payloads.push(event_json(
        "hub.connection.updated",
        sequence,
        json!(connection),
    ));
    sequence += 1;

    let session = if query.workspace_id.is_some() || query.run_id.is_some() {
        Some(require_session_with_token(&state, &headers, query.access_token.as_deref()).await?)
    } else {
        None
    };

    if let Some(workspace_id) = query.workspace_id.as_deref() {
        let session = session
            .as_ref()
            .ok_or(AccessAuthError::MissingBearerToken)
            .map_err(AppError::from)?;
        state
            .auth
            .ensure_workspace_access(session, workspace_id)
            .await?;

        let inbox_items = state
            .runtime
            .list_inbox_items_by_workspace(workspace_id)
            .await?;
        payloads.push(event_json("inbox.updated", sequence, json!(inbox_items)));
        sequence += 1;

        let notifications = state
            .runtime
            .list_notifications_by_workspace(workspace_id)
            .await?;
        payloads.push(event_json(
            "notification.updated",
            sequence,
            json!(notifications),
        ));
        sequence += 1;
    }

    if let Some(run_id) = query.run_id.as_deref() {
        let report = state.runtime.load_run_report(run_id).await?;
        let session = session
            .as_ref()
            .ok_or(AccessAuthError::MissingBearerToken)
            .map_err(AppError::from)?;
        state
            .auth
            .ensure_workspace_access(session, &report.run.workspace_id)
            .await?;
        let task = state.runtime.fetch_task(&report.run.task_id).await?;
        payloads.push(event_json(
            "run.updated",
            sequence,
            json!(RunSummaryResponse {
                id: report.run.id,
                task_id: report.run.task_id,
                workspace_id: report.run.workspace_id,
                project_id: report.run.project_id,
                title: task.title,
                run_type: report.run.run_type,
                status: report.run.status,
                approval_request_id: report.run.approval_request_id,
                attempt_count: report.run.attempt_count,
                max_attempts: report.run.max_attempts,
                last_error: report.run.last_error,
                created_at: report.run.created_at,
                updated_at: report.run.updated_at,
                started_at: report.run.started_at,
                completed_at: report.run.completed_at,
                terminated_at: report.run.terminated_at,
            }),
        ));
    }

    let stream = stream::iter(payloads.into_iter().map(|payload| {
        Ok(Event::default()
            .event(payload["event_type"].as_str().unwrap_or("hub.event"))
            .data(payload.to_string()))
    }));

    Ok(Sse::new(stream))
}

async fn build_run_detail_response(
    runtime: &Slice1Runtime,
    report: RunExecutionReport,
) -> Result<RunDetailResponse, AppError> {
    let task = runtime.fetch_task(&report.run.task_id).await?;
    let knowledge_assets = runtime.list_knowledge_assets_by_run(&report.run.id).await?;
    let knowledge_lineage = runtime
        .list_knowledge_lineage_by_run(&report.run.id)
        .await?;

    Ok(RunDetailResponse {
        run: report.run,
        task,
        artifacts: report.artifacts,
        audits: report.audits,
        traces: report.traces,
        approvals: report.approvals,
        inbox_items: report.inbox_items,
        notifications: report.notifications,
        policy_decisions: report.policy_decisions,
        knowledge_candidates: report.knowledge_candidates,
        knowledge_assets,
        knowledge_lineage,
    })
}

async fn build_knowledge_detail_response(
    runtime: &Slice1Runtime,
    run_id: &str,
) -> Result<KnowledgeDetailResponse, AppError> {
    let report = runtime.load_run_report(run_id).await?;
    let knowledge_space = runtime
        .fetch_project_knowledge_space(&report.run.workspace_id, &report.run.project_id)
        .await?
        .ok_or_else(|| AppError::NotFound("project knowledge space not found".to_string()))?;

    Ok(KnowledgeDetailResponse {
        knowledge_space,
        candidates: runtime.list_knowledge_candidates_by_run(run_id).await?,
        assets: runtime.list_knowledge_assets_by_run(run_id).await?,
        lineage: runtime.list_knowledge_lineage_by_run(run_id).await?,
    })
}

async fn transition_automation(
    state: AppState,
    headers: HeaderMap,
    automation_id: String,
    command: SurfaceAutomationLifecycleCommand,
    expected_action: &str,
) -> AppResult<AutomationDetailRecord> {
    let session = require_session(&state, &headers).await?;
    let automation = load_authorized_automation(&state, &session, &automation_id).await?;
    if automation_id != command.automation_id {
        return Err(AppError::BadRequest(
            "automation_id path/body mismatch".to_string(),
        ));
    }
    if command.action != expected_action {
        return Err(AppError::BadRequest(format!(
            "action/body mismatch: expected `{expected_action}`"
        )));
    }

    match expected_action {
        "activate" => {
            state.runtime.activate_automation(&automation.id).await?;
        }
        "pause" => {
            state.runtime.pause_automation(&automation.id).await?;
        }
        "archive" => {
            state.runtime.archive_automation(&automation.id).await?;
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "unsupported lifecycle action `{expected_action}`"
            )))
        }
    }

    Ok(Json(
        state.runtime.load_automation_detail(&automation.id).await?,
    ))
}

async fn build_hub_connection_status(
    runtime: &Slice1Runtime,
    auth_state: &str,
) -> Result<HubConnectionStatusResponse, AppError> {
    let servers = runtime.list_mcp_servers().await?;
    let healthy_server_count = servers
        .iter()
        .filter(|server| server.health_status == "healthy")
        .count();
    let state = if servers.is_empty() || healthy_server_count == servers.len() {
        "connected"
    } else if healthy_server_count > 0 {
        "degraded"
    } else {
        "disconnected"
    };
    let last_refreshed_at = servers
        .iter()
        .map(|server| server.last_checked_at.as_str())
        .max()
        .map(str::to_owned)
        .unwrap_or_else(current_timestamp);

    Ok(HubConnectionStatusResponse {
        mode: "remote".to_string(),
        state: state.to_string(),
        auth_state: auth_state.to_string(),
        active_server_count: servers.len(),
        healthy_server_count,
        servers: servers
            .into_iter()
            .map(|server| HubConnectionServerSummary {
                id: server.id,
                capability_id: server.capability_id,
                namespace: server.namespace,
                platform: server.platform,
                trust_level: server.trust_level,
                health_status: server.health_status,
                lease_ttl_seconds: server.lease_ttl_seconds,
                last_checked_at: server.last_checked_at,
            })
            .collect(),
        last_refreshed_at,
    })
}

async fn load_authorized_automation(
    state: &AppState,
    session: &HubSession,
    automation_id: &str,
) -> Result<AutomationRecord, AppError> {
    let automation = state
        .runtime
        .fetch_automation(automation_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("automation `{automation_id}` not found")))?;
    state
        .auth
        .ensure_workspace_access(session, &automation.workspace_id)
        .await?;
    Ok(automation)
}

async fn load_authorized_approval(
    state: &AppState,
    session: &HubSession,
    approval_id: &str,
) -> Result<ApprovalRequestRecord, AppError> {
    let approval = state
        .runtime
        .fetch_approval_request(approval_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("approval `{approval_id}` not found")))?;
    state
        .auth
        .ensure_workspace_access(session, &approval.workspace_id)
        .await?;
    Ok(approval)
}

async fn load_authorized_run(
    state: &AppState,
    session: &HubSession,
    run_id: &str,
) -> Result<RunRecord, AppError> {
    let run = state
        .runtime
        .fetch_run(run_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("run `{run_id}` not found")))?;
    state
        .auth
        .ensure_workspace_access(session, &run.workspace_id)
        .await?;
    Ok(run)
}

async fn load_authorized_candidate(
    state: &AppState,
    session: &HubSession,
    candidate_id: &str,
) -> Result<KnowledgeCandidateRecord, AppError> {
    let candidate = state
        .runtime
        .fetch_knowledge_candidate(candidate_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("candidate `{candidate_id}` not found")))?;
    let report = state
        .runtime
        .load_run_report(&candidate.source_run_id)
        .await?;
    state
        .auth
        .ensure_workspace_access(session, &report.run.workspace_id)
        .await?;
    Ok(candidate)
}

async fn load_authorized_trigger(
    state: &AppState,
    session: &HubSession,
    trigger_id: &str,
) -> Result<(TriggerRecord, AutomationRecord), AppError> {
    let trigger = state
        .runtime
        .fetch_trigger(trigger_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("trigger `{trigger_id}` not found")))?;
    let automation = load_authorized_automation(state, session, &trigger.automation_id).await?;
    Ok((trigger, automation))
}

async fn load_authorized_trigger_delivery(
    state: &AppState,
    session: &HubSession,
    delivery_id: &str,
) -> Result<(TriggerDeliveryRecord, TriggerRecord, AutomationRecord), AppError> {
    let delivery = state
        .runtime
        .fetch_trigger_delivery(delivery_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("trigger delivery `{delivery_id}` not found")))?;
    let (trigger, automation) =
        load_authorized_trigger(state, session, &delivery.trigger_id).await?;
    Ok((delivery, trigger, automation))
}

fn event_json(event_type: &str, sequence: u64, payload: Value) -> Value {
    json!({
        "event_type": event_type,
        "sequence": sequence,
        "occurred_at": current_timestamp(),
        "payload": payload
    })
}

async fn require_session(state: &AppState, headers: &HeaderMap) -> Result<HubSession, AppError> {
    require_session_with_token(state, headers, None).await
}

async fn require_session_with_token(
    state: &AppState,
    headers: &HeaderMap,
    access_token: Option<&str>,
) -> Result<HubSession, AppError> {
    let authorization = event_token(headers, access_token)
        .ok_or(AccessAuthError::MissingBearerToken)
        .map_err(AppError::from)?;
    Ok(state.auth.authenticate_token(&authorization).await?)
}

fn auth_header_value(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
}

fn event_token<'a>(headers: &'a HeaderMap, access_token: Option<&'a str>) -> Option<String> {
    auth_header_value(headers)
        .map(str::to_owned)
        .or_else(|| access_token.map(|token| format!("Bearer {token}")))
}

async fn resolve_auth_state(state: &AppState, authorization: Option<String>) -> String {
    let Some(authorization) = authorization else {
        return "auth_required".to_string();
    };

    match state.auth.authenticate_token(&authorization).await {
        Ok(_) => "authenticated".to_string(),
        Err(AccessAuthError::TokenExpired) => "token_expired".to_string(),
        Err(_) => "auth_required".to_string(),
    }
}

fn auth_error_response(error: AccessAuthError) -> Response {
    match error {
        AccessAuthError::MissingBearerToken
        | AccessAuthError::InvalidCredentials
        | AccessAuthError::InvalidToken => (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse {
                error: "authentication required".to_string(),
                error_code: "auth_required".to_string(),
                auth_state: "auth_required".to_string(),
            }),
        )
            .into_response(),
        AccessAuthError::TokenExpired => (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse {
                error: "session token expired".to_string(),
                error_code: "token_expired".to_string(),
                auth_state: "token_expired".to_string(),
            }),
        )
            .into_response(),
        AccessAuthError::WorkspaceForbidden(workspace_id) => (
            StatusCode::FORBIDDEN,
            Json(AuthErrorResponse {
                error: format!("workspace `{workspace_id}` is not available for this session"),
                error_code: "workspace_forbidden".to_string(),
                auth_state: "authenticated".to_string(),
            }),
        )
            .into_response(),
        AccessAuthError::WorkspaceNotFound(workspace_id) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("workspace `{workspace_id}` not found")
            })),
        )
            .into_response(),
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": other.to_string() })),
        )
            .into_response(),
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
