use std::time::Duration;

use async_stream::stream;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use octopus_core::{
    AppError, ClientAppRecord, HealthcheckBackendStatus, HealthcheckStatus, LoginRequest,
    ResolveRuntimeApprovalInput, SessionRecord, SubmitRuntimeTurnInput,
};
use octopus_platform::PlatformServices;
use serde::Deserialize;

#[derive(Clone)]
pub struct ServerState {
    pub services: PlatformServices,
    pub host_auth_token: String,
}

#[derive(Debug)]
struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            AppError::Auth(_) => StatusCode::UNAUTHORIZED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(serde_json::json!({
            "error": self.0.to_string(),
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after: Option<String>,
}

pub fn build_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(healthcheck))
        .route("/api/v1/system/health", get(healthcheck))
        .route("/api/v1/system/bootstrap", get(system_bootstrap))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/apps", get(list_apps).post(register_app))
        .route("/api/v1/workspace", get(workspace))
        .route("/api/v1/projects", get(projects))
        .route("/api/v1/inbox", get(inbox))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/knowledge", get(knowledge))
        .route("/api/v1/audit", get(audit))
        .nest("/api/v1/runtime", runtime_routes())
        .nest("/runtime", runtime_routes())
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
    State(_state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    Ok(Json(HealthcheckStatus {
        status: "ok".into(),
        host: "tauri".into(),
        mode: "local".into(),
        cargo_workspace: true,
        backend: HealthcheckBackendStatus {
            state: "ready".into(),
            transport: "http".into(),
        },
    }))
}

async fn system_bootstrap(
    State(state): State<ServerState>,
) -> Result<Json<octopus_core::SystemBootstrapStatus>, ApiError> {
    Ok(Json(state.services.workspace.system_bootstrap().await?))
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

async fn projects(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<octopus_core::ProjectRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "workspace.read", None).await?;
    Ok(Json(state.services.workspace.list_projects().await?))
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
) -> Result<Json<octopus_core::RuntimeSessionDetail>, ApiError> {
    let project_id = normalize_project_scope(&input.project_id);
    ensure_authorized_session(&state, &headers, "runtime.read", project_id).await?;
    Ok(Json(state.services.runtime_session.create_session(input).await?))
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
    Json(input): Json<SubmitRuntimeTurnInput>,
) -> Result<Json<octopus_core::RuntimeRunSnapshot>, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_runtime_submit(&state, &headers, Some(&input), project_id.as_deref()).await?;
    Ok(Json(
        state
            .services
            .runtime_execution
            .submit_turn(&session_id, input)
            .await?,
    ))
}

async fn resolve_runtime_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((session_id, approval_id)): Path<(String, String)>,
    Json(input): Json<ResolveRuntimeApprovalInput>,
) -> Result<Json<octopus_core::RuntimeRunSnapshot>, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(
        &state,
        &headers,
        "runtime.resolve_approval",
        project_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .services
            .runtime_execution
            .resolve_approval(&session_id, &approval_id, input)
            .await?,
    ))
}

async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session(&state, &headers, "runtime.read", project_id.as_deref()).await?;

    if query.after.is_some() || !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, query.after.as_deref())
            .await?;
        return Ok(Json(events).into_response());
    }

    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream = stream! {
        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok::<Event, std::convert::Infallible>(
                            Event::default().id(event.id.clone()).data(data)
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
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response())
}

async fn authenticate_session(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionRecord, ApiError> {
    let token = extract_bearer(headers).ok_or_else(|| ApiError(AppError::auth("missing bearer token")))?;
    state
        .services
        .auth
        .lookup_session(&token)
        .await?
        .ok_or_else(|| ApiError(AppError::auth("invalid bearer token")))
}

async fn ensure_authorized_session(
    state: &ServerState,
    headers: &HeaderMap,
    capability: &str,
    project_id: Option<&str>,
) -> Result<SessionRecord, ApiError> {
    let session = authenticate_session(state, headers).await?;
    authorize_session(state, &session, capability, project_id).await?;
    Ok(session)
}

async fn authorize_session(
    state: &ServerState,
    session: &SessionRecord,
    capability: &str,
    project_id: Option<&str>,
) -> Result<(), ApiError> {
    let decision = state
        .services
        .rbac
        .authorize(session, capability, project_id)
        .await?;
    if !decision.allowed {
        return Err(ApiError(AppError::auth(
            decision.reason.unwrap_or_else(|| "access denied".into()),
        )));
    }
    Ok(())
}

async fn ensure_runtime_submit(
    state: &ServerState,
    headers: &HeaderMap,
    input: Option<&SubmitRuntimeTurnInput>,
    project_id: Option<&str>,
) -> Result<(), ApiError> {
    ensure_authorized_session(state, headers, "runtime.submit_turn", project_id).await?;
    if let Some(input) = input {
        if input.permission_mode.is_empty() {
            return Err(ApiError(AppError::invalid_input(
                "permission mode is required",
            )));
        }
    }
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
        CreateRuntimeSessionInput, LoginRequest, LoginResponse, ResolveRuntimeApprovalInput,
        RuntimeEventEnvelope, RuntimeSessionDetail, RuntimeRunSnapshot, SessionRecord,
        SubmitRuntimeTurnInput,
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
    ) -> RuntimeSessionDetail {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/runtime/sessions")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
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
    ) -> RuntimeRunSnapshot {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/runtime/sessions/{session_id}/turns"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
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
                    .uri(format!("/runtime/sessions/{session_id}"))
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
                    .uri(format!("/runtime/sessions/{session_id}/events?after={after}"))
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
    ) -> RuntimeEventEnvelope {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/runtime/sessions/{session_id}/events"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::ACCEPT, "text/event-stream")
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
        submit_turn(router, token, session_id, "ask").await;

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

        let runtime_response = harness
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/runtime/sessions")
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
    }

    #[tokio::test]
    async fn runtime_session_flow_supports_json_event_polling_and_observation_with_session_token() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;
        let created = create_runtime_session(&harness.router, &session.token, "Session").await;

        let run = submit_turn(&harness.router, &session.token, &created.summary.id, "ask").await;
        assert_eq!(run.status, "waiting_approval");

        let events_response = harness
            .router
            .oneshot(
                Request::builder()
                    .uri(format!("/runtime/sessions/{}/events?after=missing", created.summary.id))
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(events_response.status(), StatusCode::OK);
        let events = decode_json::<Vec<RuntimeEventEnvelope>>(events_response).await;
        assert!(events.iter().any(|event| event.kind == "approval_requested"));
        assert!(events.iter().any(|event| event.kind == "run_updated"));

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
        let created = create_runtime_session(&harness.router, &session.token, "SSE Session").await;
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

        let sse_event = next_sse_event(&harness.router, &session.token, &created.summary.id).await;
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
            .any(|event| event.kind == "approval_requested"));
    }

    #[tokio::test]
    async fn approval_resolution_updates_run_status_and_observation_records() {
        let harness = test_harness();
        let session = login_owner_session(&harness.router, "octopus-desktop").await;

        let approved_session =
            create_runtime_session(&harness.router, &session.token, "Approve Session").await;
        submit_turn(&harness.router, &session.token, &approved_session.summary.id, "ask").await;
        let detail = runtime_session_detail(&harness.router, &session.token, &approved_session.summary.id).await;
        let approval = detail.pending_approval.expect("pending approval");
        let approve_response = harness
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "/runtime/sessions/{}/approvals/{}",
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
            .find(|event| event.kind == "approval_resolved")
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
            create_runtime_session(&harness.router, &session.token, "Reject Session").await;
        submit_turn(&harness.router, &session.token, &rejected_session.summary.id, "ask").await;
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
                        "/runtime/sessions/{}/approvals/{}",
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
            .find(|event| event.kind == "approval_resolved")
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
