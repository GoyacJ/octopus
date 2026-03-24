//! HTTP transport for the Phase 3 control-plane MVP slice.

use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use octopus_application::{
    AuditEventRecord, CreateRunInput, InboxItemRecord, Phase3Store, ResumeResult, ResumeRunInput,
    RunRecord, TimelineEventRecord,
};
use octopus_runtime::Phase3Service;
use serde::Serialize;

pub fn build_router<S>(service: Phase3Service<S>) -> Router
where
    S: Phase3Store + 'static,
{
    Router::new()
        .route("/api/v1/runs", get(list_runs::<S>).post(create_run::<S>))
        .route("/api/v1/runs/{run_id}", get(get_run::<S>))
        .route("/api/v1/runs/{run_id}/timeline", get(get_run_timeline::<S>))
        .route("/api/v1/runs/{run_id}/resume", post(resume_run::<S>))
        .route("/api/v1/inbox/items", get(list_inbox_items::<S>))
        .route("/api/v1/audit/events", get(list_audit_events::<S>))
        .with_state(service)
}

pub async fn serve<S>(service: Phase3Service<S>, addr: SocketAddr) -> Result<()>
where
    S: Phase3Store + 'static,
{
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, build_router(service)).await?;
    Ok(())
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemsResponse<T> {
    items: Vec<T>,
}

async fn create_run<S>(
    State(service): State<Phase3Service<S>>,
    Json(input): Json<CreateRunInput>,
) -> Result<(StatusCode, Json<RunRecord>), ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .create_run(input)
        .await
        .map(|run| (StatusCode::ACCEPTED, Json(run)))
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

async fn list_runs<S>(
    State(service): State<Phase3Service<S>>,
) -> Result<Json<ItemsResponse<RunRecord>>, ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .list_runs()
        .await
        .map(|items| Json(ItemsResponse { items }))
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

async fn get_run<S>(
    Path(run_id): Path<String>,
    State(service): State<Phase3Service<S>>,
) -> Result<Json<RunRecord>, ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .get_run(&run_id)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("run {run_id} not found")))
}

async fn get_run_timeline<S>(
    Path(run_id): Path<String>,
    State(service): State<Phase3Service<S>>,
) -> Result<Json<ItemsResponse<TimelineEventRecord>>, ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .list_run_timeline(&run_id)
        .await
        .map(|items| Json(ItemsResponse { items }))
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

async fn list_inbox_items<S>(
    State(service): State<Phase3Service<S>>,
) -> Result<Json<ItemsResponse<InboxItemRecord>>, ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .list_inbox_items()
        .await
        .map(|items| Json(ItemsResponse { items }))
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

async fn list_audit_events<S>(
    State(service): State<Phase3Service<S>>,
) -> Result<Json<ItemsResponse<AuditEventRecord>>, ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .list_audit_events()
        .await
        .map(|items| Json(ItemsResponse { items }))
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

async fn resume_run<S>(
    Path(run_id): Path<String>,
    State(service): State<Phase3Service<S>>,
    Json(input): Json<ResumeRunInput>,
) -> Result<(StatusCode, Json<ResumeResult>), ApiError>
where
    S: Phase3Store + 'static,
{
    service
        .resume_run(&run_id, input)
        .await
        .map(|result| (StatusCode::ACCEPTED, Json(result)))
        .map_err(|error| {
            if error.to_string().contains("not found") {
                ApiError::not_found(error.to_string())
            } else {
                ApiError::bad_request(error.to_string())
            }
        })
}
