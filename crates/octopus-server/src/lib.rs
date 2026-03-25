use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use octopus_hub::{
    contracts::{contract_catalog, ContractCatalog},
    runtime::{
        ApprovalResolutionRequest, InMemoryRuntimeService, RunDetailResponse, RuntimeError,
        TaskSubmissionRequest,
    },
};
use serde::Serialize;

#[derive(Clone, Default)]
struct AppState {
    runtime: InMemoryRuntimeService,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

pub fn build_app() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/contracts", get(get_contracts))
        .route("/api/v1/runs/task", post(submit_task))
        .route("/api/v1/runs/{run_id}", get(get_run))
        .route("/api/v1/runs/{run_id}/resume", post(resume_run))
        .route("/api/v1/approvals/{approval_id}/resolve", post(resolve_approval))
        .with_state(AppState::default())
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}

async fn get_contracts() -> Result<Json<ContractCatalog>, (StatusCode, Json<ErrorResponse>)> {
    contract_catalog()
        .map(Json)
        .map_err(into_http_error)
}

async fn submit_task(
    State(state): State<AppState>,
    Json(request): Json<TaskSubmissionRequest>,
) -> (StatusCode, Json<RunDetailResponse>) {
    let response = state.runtime.submit_task(request);
    let status = if response.approval.is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    };

    (status, Json(response))
}

async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<RunDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .get_run(&run_id)
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: format!("run {run_id} not found"),
                }),
            )
        })
}

async fn resolve_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    Json(request): Json<ApprovalResolutionRequest>,
) -> Result<Json<RunDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .resolve_approval(&approval_id, request)
        .map(Json)
        .map_err(into_runtime_http_error)
}

async fn resume_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<RunDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .resume_run(&run_id)
        .map(Json)
        .map_err(into_runtime_http_error)
}

fn into_runtime_http_error(error: RuntimeError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match error {
        RuntimeError::NotFound { .. } => StatusCode::NOT_FOUND,
        RuntimeError::InvalidState { .. } => StatusCode::CONFLICT,
    };

    (
        status,
        Json(ErrorResponse {
            message: error.to_string(),
        }),
    )
}

fn into_http_error(error: impl ToString) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            message: error.to_string(),
        }),
    )
}
