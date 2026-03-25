use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use octopus_hub::{
    contracts::{contract_catalog, ContractCatalog},
    runtime::{
        ApprovalResolutionRequest, AutomationCreateRequest, AutomationListResponse,
        InMemoryRuntimeService, KnowledgeAssetListResponse, KnowledgeCandidateCreateRequest,
        KnowledgeCandidateResponse, KnowledgePromotionRequest, KnowledgePromotionResponse,
        KnowledgeSpaceCreateRequest, KnowledgeSpaceDetailResponse, KnowledgeSpaceListResponse,
        McpEventDeliveryRequest, McpEventDeliveryResponse, RunDetailResponse, RuntimeError,
        TaskSubmissionRequest, TriggerDeliveryRequest, TriggerDeliveryResponse,
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
        .route("/api/v1/automations", get(list_automations).post(create_automation))
        .route("/api/v1/knowledge/spaces", get(list_knowledge_spaces).post(create_knowledge_space))
        .route(
            "/api/v1/knowledge/spaces/{space_id}/assets",
            get(list_knowledge_assets),
        )
        .route(
            "/api/v1/knowledge/candidates/from-run",
            post(create_candidate_from_run),
        )
        .route(
            "/api/v1/knowledge/candidates/{candidate_id}/promote",
            post(promote_candidate),
        )
        .route("/api/v1/mcp/events/deliver", post(deliver_mcp_event))
        .route("/api/v1/runs/task", post(submit_task))
        .route("/api/v1/runs/{run_id}", get(get_run))
        .route("/api/v1/runs/{run_id}/resume", post(resume_run))
        .route("/api/v1/triggers/deliver", post(deliver_trigger))
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

async fn list_automations(State(state): State<AppState>) -> Json<AutomationListResponse> {
    Json(AutomationListResponse {
        items: state.runtime.list_automations(),
    })
}

async fn create_automation(
    State(state): State<AppState>,
    Json(request): Json<AutomationCreateRequest>,
) -> Result<
    (StatusCode, Json<octopus_hub::runtime::AutomationDetailResponse>),
    (StatusCode, Json<ErrorResponse>),
> {
    state
        .runtime
        .create_automation(request)
        .map(|response| (StatusCode::CREATED, Json(response)))
        .map_err(into_runtime_http_error)
}

async fn list_knowledge_spaces(State(state): State<AppState>) -> Json<KnowledgeSpaceListResponse> {
    Json(KnowledgeSpaceListResponse {
        items: state.runtime.list_knowledge_spaces(),
    })
}

async fn create_knowledge_space(
    State(state): State<AppState>,
    Json(request): Json<KnowledgeSpaceCreateRequest>,
) -> Result<(StatusCode, Json<KnowledgeSpaceDetailResponse>), (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .create_knowledge_space(request)
        .map(|response| (StatusCode::CREATED, Json(response)))
        .map_err(into_runtime_http_error)
}

async fn list_knowledge_assets(
    State(state): State<AppState>,
    Path(space_id): Path<String>,
) -> Result<Json<KnowledgeAssetListResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .list_knowledge_assets(&space_id)
        .map(Json)
        .map_err(into_runtime_http_error)
}

async fn create_candidate_from_run(
    State(state): State<AppState>,
    Json(request): Json<KnowledgeCandidateCreateRequest>,
) -> Result<(StatusCode, Json<KnowledgeCandidateResponse>), (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .create_candidate_from_run(request)
        .map(|candidate| {
            (
                StatusCode::CREATED,
                Json(KnowledgeCandidateResponse { candidate }),
            )
        })
        .map_err(into_runtime_http_error)
}

async fn promote_candidate(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
    Json(request): Json<KnowledgePromotionRequest>,
) -> Result<Json<KnowledgePromotionResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .runtime
        .promote_candidate(&candidate_id, request)
        .map(Json)
        .map_err(into_runtime_http_error)
}

async fn deliver_mcp_event(
    State(state): State<AppState>,
    Json(request): Json<McpEventDeliveryRequest>,
) -> Result<(StatusCode, Json<McpEventDeliveryResponse>), (StatusCode, Json<ErrorResponse>)> {
    let response = state
        .runtime
        .deliver_mcp_event(request)
        .map_err(into_runtime_http_error)?;
    let status = if response
        .items
        .iter()
        .any(|entry| entry.run.as_ref().and_then(|detail| detail.approval.as_ref()).is_some())
    {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };

    Ok((status, Json(response)))
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

async fn deliver_trigger(
    State(state): State<AppState>,
    Json(request): Json<TriggerDeliveryRequest>,
) -> Result<(StatusCode, Json<TriggerDeliveryResponse>), (StatusCode, Json<ErrorResponse>)> {
    let response = state.runtime.deliver_trigger(request).map_err(into_runtime_http_error)?;
    let status = if response
        .run
        .as_ref()
        .and_then(|detail| detail.approval.as_ref())
        .is_some()
    {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };

    Ok((status, Json(response)))
}

fn into_runtime_http_error(error: RuntimeError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match error {
        RuntimeError::NotFound { .. } => StatusCode::NOT_FOUND,
        RuntimeError::InvalidState { .. } => StatusCode::CONFLICT,
        RuntimeError::InvalidDecision { .. } => StatusCode::BAD_REQUEST,
        RuntimeError::InvalidRequest { .. } => StatusCode::BAD_REQUEST,
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
