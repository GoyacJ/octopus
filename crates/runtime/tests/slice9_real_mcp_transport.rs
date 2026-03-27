use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateAutomationInput, CreateTaskInput, DispatchManualEventInput,
    McpCredentialRecord, McpServerRecord, Slice2Runtime,
};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle, time::sleep};

async fn seed_context(runtime: &Slice2Runtime, project_id: &str, project_name: &str) {
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            project_id,
            project_id,
            project_name,
        )
        .await
        .unwrap();
}

async fn seed_connector_governance(
    runtime: &Slice2Runtime,
    project_id: &str,
    capability_id: &str,
    risk_level: &str,
    soft_limit: i64,
    hard_limit: i64,
    trust_level: &str,
) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new_connector_backed(
            capability_id,
            capability_id,
            "desktop",
            "mcp_server",
            risk_level,
            risk_level == "high",
            trust_level,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{capability_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{capability_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_budget_policy(BudgetPolicyRecord::project_scope(
            format!("budget-{project_id}"),
            "workspace-alpha",
            project_id,
            soft_limit,
            hard_limit,
        ))
        .await
        .unwrap();
}

async fn seed_bearer_credential(runtime: &Slice2Runtime, credential_id: &str, token: &str) {
    runtime
        .upsert_mcp_credential(
            McpCredentialRecord::bearer_token(credential_id, "test.connector"),
            token,
        )
        .await
        .unwrap();
}

async fn seed_http_mcp_server(
    runtime: &Slice2Runtime,
    capability_id: &str,
    endpoint: &str,
    trust_level: &str,
    credential_ref: Option<&str>,
    timeout_ms: i64,
) {
    runtime
        .upsert_mcp_server(McpServerRecord::new_http(
            format!("server-{capability_id}"),
            capability_id,
            "test.connector",
            "desktop",
            trust_level,
            60,
            endpoint,
            timeout_ms,
            credential_ref,
        ))
        .await
        .unwrap();
}

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("slice9-runtime.sqlite")
}

#[derive(Clone)]
struct MockMcpState {
    scenario: MockMcpScenario,
    requests: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
enum MockMcpScenario {
    EchoSuccess {
        expected_token: String,
        response_text: String,
    },
    FailOnceThenSuccess {
        expected_token: String,
        failure_message: String,
        success_text: String,
        seen: Arc<Mutex<u32>>,
    },
    InvalidJsonRpc {
        expected_token: String,
    },
    SlowSuccess {
        expected_token: String,
        response_text: String,
        delay_ms: u64,
    },
}

struct MockMcpServer {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl MockMcpServer {
    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn requests(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockMcpServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
        }
    }
}

async fn spawn_mock_mcp_server(scenario: MockMcpScenario) -> MockMcpServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = MockMcpState {
        scenario,
        requests: requests.clone(),
    };
    let app = Router::new()
        .route("/mcp", post(handle_mock_mcp))
        .with_state(state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let join_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    MockMcpServer {
        endpoint,
        requests,
        shutdown_tx: Some(shutdown_tx),
        join_handle: Some(join_handle),
    }
}

async fn handle_mock_mcp(
    State(state): State<MockMcpState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    state.requests.lock().unwrap().push(body.clone());

    match state.scenario.clone() {
        MockMcpScenario::EchoSuccess {
            expected_token,
            response_text,
        } => {
            if !has_expected_auth_header(&headers, &expected_token) {
                return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
            }
            Json(json!({
                "jsonrpc": "2.0",
                "id": body.get("id").cloned().unwrap_or(json!(1)),
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": response_text,
                        }
                    ]
                }
            }))
            .into_response()
        }
        MockMcpScenario::FailOnceThenSuccess {
            expected_token,
            failure_message,
            success_text,
            seen,
        } => {
            if !has_expected_auth_header(&headers, &expected_token) {
                return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
            }
            let mut seen = seen.lock().unwrap();
            *seen += 1;
            if *seen == 1 {
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": body.get("id").cloned().unwrap_or(json!(1)),
                    "error": {
                        "code": -32001,
                        "message": failure_message,
                        "data": {
                            "retryable": true,
                        }
                    }
                }))
                .into_response()
            } else {
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": body.get("id").cloned().unwrap_or(json!(1)),
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": success_text,
                            }
                        ]
                    }
                }))
                .into_response()
            }
        }
        MockMcpScenario::InvalidJsonRpc { expected_token } => {
            if !has_expected_auth_header(&headers, &expected_token) {
                return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
            }
            Json(json!({
                "jsonrpc": "2.0",
                "id": body.get("id").cloned().unwrap_or(json!(1)),
                "result": {
                    "unexpected": "shape"
                }
            }))
            .into_response()
        }
        MockMcpScenario::SlowSuccess {
            expected_token,
            response_text,
            delay_ms,
        } => {
            if !has_expected_auth_header(&headers, &expected_token) {
                return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
            }
            sleep(Duration::from_millis(delay_ms)).await;
            Json(json!({
                "jsonrpc": "2.0",
                "id": body.get("id").cloned().unwrap_or(json!(1)),
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": response_text,
                        }
                    ]
                }
            }))
            .into_response()
        }
    }
}

fn has_expected_auth_header(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == format!("Bearer {expected_token}"))
        .unwrap_or(false)
}

#[tokio::test]
async fn real_http_connector_executes_and_records_invocation_artifact_health_and_lease() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let mock_server = spawn_mock_mcp_server(MockMcpScenario::EchoSuccess {
        expected_token: "token-success".into(),
        response_text: "connector artifact".into(),
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice9-success", "Slice 9 Success").await;
    seed_connector_governance(
        &runtime,
        "project-slice9-success",
        "capability-slice9-success",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-success", "token-success").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-success",
        mock_server.endpoint(),
        "trusted",
        Some("credential-slice9-success"),
        1_000,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-success".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Slice 9 real MCP success".into(),
            instruction: "Call real HTTP MCP connector".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "connector artifact"
                }),
            },
            capability_id: "capability-slice9-success".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-success-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.artifacts[0].content.as_str(), "connector artifact");
    assert_eq!(
        report.artifacts[0].provenance_source.as_str(),
        "mcp_connector"
    );

    let invocations = runtime
        .list_mcp_invocations_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].status.as_str(), "succeeded");

    let leases = runtime
        .list_environment_leases_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].status.as_str(), "released");

    let servers = runtime.list_mcp_servers().await.unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].health_status.as_str(), "healthy");

    let credentials = runtime.list_mcp_credentials().await.unwrap();
    assert_eq!(credentials.len(), 1);
    assert!(credentials[0].secret_present);

    let requests = mock_server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["method"], "tools/call");
    assert_eq!(requests[0]["params"]["name"], "emit_text");
    assert_eq!(
        requests[0]["params"]["arguments"]["content"],
        "connector artifact"
    );
}

#[tokio::test]
async fn real_http_connector_waits_for_approval_before_invocation_then_executes_after_approval() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let mock_server = spawn_mock_mcp_server(MockMcpScenario::EchoSuccess {
        expected_token: "token-approval".into(),
        response_text: "approval connector artifact".into(),
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice9-approval", "Slice 9 Approval").await;
    seed_connector_governance(
        &runtime,
        "project-slice9-approval",
        "capability-slice9-approval",
        "high",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-approval", "token-approval").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-approval",
        mock_server.endpoint(),
        "trusted",
        Some("credential-slice9-approval"),
        1_000,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-approval".into(),
            title: "Slice 9 approval".into(),
            instruction: "Approval before real MCP call".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "approval connector artifact"
                }),
            },
            capability_id: "capability-slice9-approval".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let waiting = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-slice9-approval-1".into(),
            payload: json!({"source": "slice9"}),
        })
        .await
        .unwrap();

    assert_eq!(waiting.run_report.run.status.as_str(), "waiting_approval");
    assert!(runtime
        .list_mcp_invocations_by_run(waiting.run_report.run.id.as_str())
        .await
        .unwrap()
        .is_empty());
    assert!(mock_server.requests().is_empty());

    let report = runtime
        .resolve_approval(
            waiting.run_report.approvals[0].id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "approved for slice 9",
        )
        .await
        .unwrap();

    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(
        runtime
            .list_mcp_invocations_by_run(report.run.id.as_str())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn missing_credential_reference_fails_non_retryable_and_records_invocation() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(
        &runtime,
        "project-slice9-missing-credential",
        "Slice 9 Missing Credential",
    )
    .await;
    seed_connector_governance(
        &runtime,
        "project-slice9-missing-credential",
        "capability-slice9-missing-credential",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-missing-credential",
        "http://127.0.0.1:9/mcp",
        "trusted",
        Some("credential-does-not-exist"),
        1_000,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-missing-credential".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Missing credential".into(),
            instruction: "Should normalize missing credential".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "never sent"
                }),
            },
            capability_id: "capability-slice9-missing-credential".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-missing-credential-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "failed");
    assert!(report.run.resume_token.is_none());
    assert!(report
        .run
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("credential"));

    let invocations = runtime
        .list_mcp_invocations_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].status.as_str(), "failed");
    assert!(!invocations[0].retryable);

    let leases = runtime
        .list_environment_leases_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].status.as_str(), "released");
}

#[tokio::test]
async fn unauthorized_http_response_fails_non_retryable_and_degrades_server_health() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let mock_server = spawn_mock_mcp_server(MockMcpScenario::EchoSuccess {
        expected_token: "expected-token".into(),
        response_text: "not returned".into(),
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(
        &runtime,
        "project-slice9-unauthorized",
        "Slice 9 Unauthorized",
    )
    .await;
    seed_connector_governance(
        &runtime,
        "project-slice9-unauthorized",
        "capability-slice9-unauthorized",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-unauthorized", "wrong-token").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-unauthorized",
        mock_server.endpoint(),
        "trusted",
        Some("credential-slice9-unauthorized"),
        1_000,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-unauthorized".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Unauthorized credential".into(),
            instruction: "Wrong credential should fail".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "never sent"
                }),
            },
            capability_id: "capability-slice9-unauthorized".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-unauthorized-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "failed");
    assert!(report.run.resume_token.is_none());

    let invocations = runtime
        .list_mcp_invocations_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].status.as_str(), "failed");
    assert!(!invocations[0].retryable);

    let servers = runtime.list_mcp_servers().await.unwrap();
    assert_eq!(servers[0].health_status.as_str(), "degraded");
}

#[tokio::test]
async fn endpoint_unreachable_and_timeout_failures_are_retryable() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let unavailable_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unavailable_endpoint = format!("http://{}/mcp", unavailable_listener.local_addr().unwrap());
    drop(unavailable_listener);

    let slow_server = spawn_mock_mcp_server(MockMcpScenario::SlowSuccess {
        expected_token: "slow-token".into(),
        response_text: "too late".into(),
        delay_ms: 250,
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(
        &runtime,
        "project-slice9-unreachable",
        "Slice 9 Unreachable",
    )
    .await;
    seed_connector_governance(
        &runtime,
        "project-slice9-unreachable",
        "capability-slice9-unreachable",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(
        &runtime,
        "credential-slice9-unreachable",
        "unreachable-token",
    )
    .await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-unreachable",
        &unavailable_endpoint,
        "trusted",
        Some("credential-slice9-unreachable"),
        100,
    )
    .await;

    let unreachable_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-unreachable".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Endpoint unreachable".into(),
            instruction: "Connection refused should be retryable".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "never sent"
                }),
            },
            capability_id: "capability-slice9-unreachable".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-unreachable-1".into(),
        })
        .await
        .unwrap();

    let unreachable = runtime
        .start_task(unreachable_task.id.as_str())
        .await
        .unwrap();
    assert_eq!(unreachable.run.status.as_str(), "failed");
    assert!(unreachable.run.resume_token.is_some());
    assert!(unreachable
        .run
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("transport"));

    seed_context(&runtime, "project-slice9-timeout", "Slice 9 Timeout").await;
    seed_connector_governance(
        &runtime,
        "project-slice9-timeout",
        "capability-slice9-timeout",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-timeout", "slow-token").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-timeout",
        slow_server.endpoint(),
        "trusted",
        Some("credential-slice9-timeout"),
        50,
    )
    .await;

    let timeout_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-timeout".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Endpoint timeout".into(),
            instruction: "Timeout should be retryable".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "too slow"
                }),
            },
            capability_id: "capability-slice9-timeout".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-timeout-1".into(),
        })
        .await
        .unwrap();

    let timeout = runtime.start_task(timeout_task.id.as_str()).await.unwrap();
    assert_eq!(timeout.run.status.as_str(), "failed");
    assert!(timeout.run.resume_token.is_some());

    let timeout_invocations = runtime
        .list_mcp_invocations_by_run(timeout.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(timeout_invocations.len(), 1);
    assert!(timeout_invocations[0].retryable);
}

#[tokio::test]
async fn retryable_jsonrpc_error_can_retry_after_runtime_reopen() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let mock_server = spawn_mock_mcp_server(MockMcpScenario::FailOnceThenSuccess {
        expected_token: "retry-token".into(),
        failure_message: "temporary upstream failure".into(),
        success_text: "recovered connector artifact".into(),
        seen: Arc::new(Mutex::new(0)),
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice9-retry", "Slice 9 Retry").await;
    seed_connector_governance(
        &runtime,
        "project-slice9-retry",
        "capability-slice9-retry",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-retry", "retry-token").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-retry",
        mock_server.endpoint(),
        "trusted",
        Some("credential-slice9-retry"),
        1_000,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-retry".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Retry after reopen".into(),
            instruction: "Retryable JSON-RPC error should recover".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "recovered connector artifact"
                }),
            },
            capability_id: "capability-slice9-retry".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-retry-1".into(),
        })
        .await
        .unwrap();

    let failed = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(failed.run.status.as_str(), "failed");
    assert!(failed.run.resume_token.is_some());
    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let recovered = reopened.retry_run(failed.run.id.as_str()).await.unwrap();
    assert_eq!(recovered.run.status.as_str(), "completed");
    assert_eq!(recovered.artifacts.len(), 1);
    assert_eq!(
        recovered.artifacts[0].content.as_str(),
        "recovered connector artifact"
    );
    assert_eq!(
        reopened
            .list_mcp_invocations_by_run(recovered.run.id.as_str())
            .await
            .unwrap()
            .len(),
        2
    );
    let leases = reopened
        .list_environment_leases_by_run(recovered.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 2);
    assert!(leases.iter().all(|lease| lease.status == "released"));
}

#[tokio::test]
async fn invalid_jsonrpc_response_is_normalized_and_low_trust_output_stays_gated() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let invalid_server = spawn_mock_mcp_server(MockMcpScenario::InvalidJsonRpc {
        expected_token: "invalid-token".into(),
    })
    .await;
    let low_trust_server = spawn_mock_mcp_server(MockMcpScenario::EchoSuccess {
        expected_token: "low-trust-token".into(),
        response_text: "external low trust artifact".into(),
    })
    .await;

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(
        &runtime,
        "project-slice9-invalid-response",
        "Slice 9 Invalid Response",
    )
    .await;
    seed_connector_governance(
        &runtime,
        "project-slice9-invalid-response",
        "capability-slice9-invalid-response",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_bearer_credential(&runtime, "credential-slice9-invalid", "invalid-token").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-invalid-response",
        invalid_server.endpoint(),
        "trusted",
        Some("credential-slice9-invalid"),
        1_000,
    )
    .await;

    let invalid_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-invalid-response".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Invalid JSON-RPC".into(),
            instruction: "Invalid response should normalize".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "never accepted"
                }),
            },
            capability_id: "capability-slice9-invalid-response".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-invalid-response-1".into(),
        })
        .await
        .unwrap();

    let invalid = runtime.start_task(invalid_task.id.as_str()).await.unwrap();
    assert_eq!(invalid.run.status.as_str(), "failed");
    assert!(invalid.run.resume_token.is_none());

    seed_context(&runtime, "project-slice9-low-trust", "Slice 9 Low Trust").await;
    seed_connector_governance(
        &runtime,
        "project-slice9-low-trust",
        "capability-slice9-low-trust",
        "low",
        5,
        10,
        "external_untrusted",
    )
    .await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-slice9-low-trust",
            "Low Trust Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();
    seed_bearer_credential(&runtime, "credential-slice9-low-trust", "low-trust-token").await;
    seed_http_mcp_server(
        &runtime,
        "capability-slice9-low-trust",
        low_trust_server.endpoint(),
        "external_untrusted",
        Some("credential-slice9-low-trust"),
        1_000,
    )
    .await;

    let low_trust_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice9-low-trust".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Low trust HTTP connector".into(),
            instruction: "Artifact should persist but knowledge should stay gated".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "external low trust artifact"
                }),
            },
            capability_id: "capability-slice9-low-trust".into(),
            estimated_cost: 1,
            idempotency_key: "task-slice9-low-trust-1".into(),
        })
        .await
        .unwrap();

    let low_trust = runtime
        .start_task(low_trust_task.id.as_str())
        .await
        .unwrap();
    assert_eq!(low_trust.run.status.as_str(), "completed");
    assert_eq!(low_trust.artifacts.len(), 1);
    assert_eq!(
        low_trust.artifacts[0].knowledge_gate_status.as_str(),
        "blocked_low_trust"
    );
    assert!(low_trust.knowledge_candidates.is_empty());
}
