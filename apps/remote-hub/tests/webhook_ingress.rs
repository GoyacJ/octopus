use std::path::Path;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use octopus_access_auth::RemoteAccessService;
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateAutomationInput, CreateTriggerInput, Slice1Runtime,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use octopus_remote_hub::{app, AppState};

fn sample_db_path(base: &Path, name: &str) -> std::path::PathBuf {
    base.join(name)
}

async fn seed_governance(runtime: &Slice1Runtime, project_id: &str, capability_id: &str) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            capability_id,
            capability_id,
            "low",
            false,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{project_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{project_id}"),
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
            5,
            10,
        ))
        .await
        .unwrap();
}

async fn response_status_and_json(router: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn webhook_route_enforces_headers_and_dedupes_duplicate_ingress() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "webhook.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-webhook",
            "project-webhook",
            "Webhook Project",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-webhook", "capability-webhook").await;

    let created = runtime
        .create_automation_with_trigger(
            CreateAutomationInput {
                workspace_id: "workspace-alpha".into(),
                project_id: "project-webhook".into(),
                title: "Webhook ingress".into(),
                instruction: "Run from webhook".into(),
                action: ExecutionAction::EmitText {
                    content: "webhook artifact".into(),
                },
                capability_id: "capability-webhook".into(),
                estimated_cost: 1,
            },
            CreateTriggerInput::Webhook {
                ingress_mode: "shared_secret_header".into(),
                secret_header_name: "X-Octopus-Trigger-Secret".into(),
                secret_hint: Some("hook".into()),
                secret_plaintext: None,
            },
        )
        .await
        .unwrap();

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime.clone(), auth));
    let trigger_id = created.automation.trigger_id;
    let secret = created.webhook_secret.unwrap();

    let (missing_idempotency_status, _) = response_status_and_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{trigger_id}/webhook"))
            .header("content-type", "application/json")
            .header("X-Octopus-Trigger-Secret", secret.as_str())
            .body(Body::from(json!({"source": "missing-idempotency"}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(missing_idempotency_status, StatusCode::BAD_REQUEST);

    let (missing_secret_status, _) = response_status_and_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{trigger_id}/webhook"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "event-1")
            .body(Body::from(json!({"source": "missing-secret"}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(missing_secret_status, StatusCode::BAD_REQUEST);

    let (invalid_secret_status, _) = response_status_and_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{trigger_id}/webhook"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "event-2")
            .header("X-Octopus-Trigger-Secret", "wrong-secret")
            .body(Body::from(json!({"source": "invalid-secret"}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(invalid_secret_status, StatusCode::UNAUTHORIZED);

    let (accepted_status, accepted_body) = response_status_and_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{trigger_id}/webhook"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "event-3")
            .header("X-Octopus-Trigger-Secret", secret.as_str())
            .body(Body::from(json!({"source": "ok"}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(accepted_status, StatusCode::OK);

    let (duplicate_status, duplicate_body) = response_status_and_json(
        router,
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{trigger_id}/webhook"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "event-3")
            .header("X-Octopus-Trigger-Secret", secret.as_str())
            .body(Body::from(json!({"source": "duplicate"}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(duplicate_status, StatusCode::OK);
    assert_eq!(accepted_body["delivery_id"], duplicate_body["delivery_id"]);
    assert_eq!(accepted_body["run_id"], duplicate_body["run_id"]);
}
