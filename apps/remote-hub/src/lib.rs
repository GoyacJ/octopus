use std::convert::Infallible;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use futures_util::stream;
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, ApprovalRequestRecord, ArtifactRecord, AuditRecord,
    CapabilityDescriptorRecord, CreateTaskInput, InboxItemRecord, KnowledgeAssetRecord,
    KnowledgeCandidateRecord, KnowledgeLineageRecord, KnowledgeSpaceRecord, NotificationRecord,
    PolicyDecisionLogRecord, ProjectContext, RunExecutionReport, RunRecord, RuntimeError,
    Slice1Runtime, TaskRecord, TraceRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AppState {
    runtime: Slice1Runtime,
}

impl AppState {
    pub fn new(runtime: Slice1Runtime) -> Self {
        Self { runtime }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/context",
            get(get_project_context),
        )
        .route("/api/tasks", post(create_task))
        .route("/api/tasks/{task_id}/start", post(start_task))
        .route("/api/runs/{run_id}", get(get_run_detail))
        .route("/api/runs/{run_id}/artifacts", get(list_artifacts))
        .route("/api/runs/{run_id}/knowledge", get(get_knowledge_detail))
        .route("/api/approvals/{approval_id}/resolve", post(resolve_approval))
        .route("/api/workspaces/{workspace_id}/inbox", get(list_inbox_items))
        .route(
            "/api/workspaces/{workspace_id}/notifications",
            get(list_notifications),
        )
        .route(
            "/api/workspaces/{workspace_id}/projects/{project_id}/capabilities",
            get(list_capability_visibility),
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
    Runtime(#[from] RuntimeError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Runtime(error) => match error {
                RuntimeError::TaskNotFound(message)
                | RuntimeError::RunNotFound(message)
                | RuntimeError::ApprovalRequestNotFound(message)
                | RuntimeError::KnowledgeCandidateNotFound(message)
                | RuntimeError::AutomationNotFound(message)
                | RuntimeError::TriggerNotFound(message)
                | RuntimeError::TriggerDeliveryNotFound(message) => {
                    (StatusCode::NOT_FOUND, message)
                }
                RuntimeError::InvalidRunTransition { run_id, from, to } => (
                    StatusCode::BAD_REQUEST,
                    format!("invalid run transition for `{run_id}`: `{from}` -> `{to}`"),
                ),
                RuntimeError::InvalidTriggerType {
                    trigger_id,
                    trigger_type,
                } => (
                    StatusCode::BAD_REQUEST,
                    format!("trigger `{trigger_id}` has unsupported type `{trigger_type}`"),
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
struct SurfaceApprovalResolveCommand {
    approval_id: String,
    decision: String,
    actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceKnowledgePromoteCommand {
    candidate_id: String,
    actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    workspace_id: Option<String>,
    run_id: Option<String>,
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
struct CapabilityVisibilityResponse {
    descriptor: CapabilityDescriptorRecord,
    scope_ref: String,
    visibility: String,
    reason_code: String,
    explanation: String,
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
    active_server_count: usize,
    healthy_server_count: usize,
    servers: Vec<HubConnectionServerSummary>,
    last_refreshed_at: String,
}

async fn get_project_context(
    State(state): State<AppState>,
    Path((workspace_id, project_id)): Path<(String, String)>,
) -> AppResult<ProjectContext> {
    Ok(Json(
        state
            .runtime
            .fetch_project_context(&workspace_id, &project_id)
            .await?,
    ))
}

async fn create_task(
    State(state): State<AppState>,
    Json(command): Json<SurfaceTaskCreateCommand>,
) -> AppResult<TaskRecord> {
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

async fn start_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> AppResult<RunDetailResponse> {
    let report = state.runtime.start_task(&task_id).await?;
    Ok(Json(build_run_detail_response(&state.runtime, report).await?))
}

async fn get_run_detail(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> AppResult<RunDetailResponse> {
    let report = state.runtime.load_run_report(&run_id).await?;
    Ok(Json(build_run_detail_response(&state.runtime, report).await?))
}

async fn list_artifacts(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> AppResult<Vec<ArtifactRecord>> {
    Ok(Json(state.runtime.list_artifacts_by_run(&run_id).await?))
}

async fn get_knowledge_detail(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> AppResult<KnowledgeDetailResponse> {
    Ok(Json(build_knowledge_detail_response(&state.runtime, &run_id).await?))
}

async fn resolve_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    Json(command): Json<SurfaceApprovalResolveCommand>,
) -> AppResult<RunDetailResponse> {
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

    let report = state
        .runtime
        .resolve_approval(&approval_id, decision, &command.actor_ref, &command.note)
        .await?;
    Ok(Json(build_run_detail_response(&state.runtime, report).await?))
}

async fn list_inbox_items(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> AppResult<Vec<InboxItemRecord>> {
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
) -> AppResult<Vec<NotificationRecord>> {
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
) -> AppResult<Vec<CapabilityVisibilityResponse>> {
    let descriptors = state
        .runtime
        .list_visible_capabilities(&workspace_id, &project_id)
        .await?;
    Ok(Json(
        descriptors
            .into_iter()
            .map(|descriptor| CapabilityVisibilityResponse {
                descriptor,
                scope_ref: format!("project:{project_id}"),
                visibility: "visible".to_string(),
                reason_code: "project_scope_grant_active".to_string(),
                explanation: format!(
                    "Visible because the project-scoped capability grant is active for `{project_id}`."
                ),
            })
            .collect(),
    ))
}

async fn promote_knowledge(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
    Json(command): Json<SurfaceKnowledgePromoteCommand>,
) -> AppResult<KnowledgeDetailResponse> {
    if candidate_id != command.candidate_id {
        return Err(AppError::BadRequest(
            "candidate_id path/body mismatch".to_string(),
        ));
    }

    let report = state
        .runtime
        .promote_knowledge_candidate(&candidate_id, &command.actor_ref, &command.note)
        .await?;
    let run_id = report.candidate.source_run_id;
    Ok(Json(build_knowledge_detail_response(&state.runtime, &run_id).await?))
}

async fn get_hub_connection_status(
    State(state): State<AppState>,
) -> AppResult<HubConnectionStatusResponse> {
    Ok(Json(build_hub_connection_status(&state.runtime).await?))
}

async fn stream_events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let mut payloads = Vec::new();
    let mut sequence = 1_u64;

    let connection = build_hub_connection_status(&state.runtime).await?;
    payloads.push(event_json(
        "hub.connection.updated",
        sequence,
        json!(connection),
    ));
    sequence += 1;

    if let Some(workspace_id) = query.workspace_id.as_deref() {
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
    let knowledge_lineage = runtime.list_knowledge_lineage_by_run(&report.run.id).await?;

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

async fn build_hub_connection_status(
    runtime: &Slice1Runtime,
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

fn event_json(event_type: &str, sequence: u64, payload: Value) -> Value {
    json!({
        "event_type": event_type,
        "sequence": sequence,
        "occurred_at": current_timestamp(),
        "payload": payload
    })
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
