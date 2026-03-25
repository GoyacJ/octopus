use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use octopus_hub::runtime::{ApprovalResolutionRequest, TaskSubmissionRequest};
use octopus_server::build_app;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok() {
    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn task_flow_waits_for_approval_and_resumes() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs/task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&TaskSubmissionRequest {
                        workspace_id: "workspace-alpha".into(),
                        project_id: "project-alpha".into(),
                        title: "Review remote hub policy".into(),
                        description: Some("Need approval before artifact generation".into()),
                        requested_by: "operator-1".into(),
                        requires_approval: true,
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(create_response.status(), StatusCode::ACCEPTED);

    let create_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(create_response.into_body(), usize::MAX).await.unwrap())
            .expect("response should deserialize");
    let run_id = create_payload["run"]["id"]
        .as_str()
        .expect("run id should exist")
        .to_owned();
    let approval_id = create_payload["approval"]["id"]
        .as_str()
        .expect("approval id should exist")
        .to_owned();

    let approval_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ApprovalResolutionRequest {
                        decision: "approved".into(),
                        reviewed_by: "reviewer-1".into(),
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(approval_response.status(), StatusCode::OK);

    let resume_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/runs/{run_id}/resume"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(resume_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_approval_decision_returns_bad_request() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs/task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&TaskSubmissionRequest {
                        workspace_id: "workspace-alpha".into(),
                        project_id: "project-alpha".into(),
                        title: "Review remote hub policy".into(),
                        description: Some("Need approval before artifact generation".into()),
                        requested_by: "operator-1".into(),
                        requires_approval: true,
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let create_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(create_response.into_body(), usize::MAX).await.unwrap())
            .expect("response should deserialize");
    let approval_id = create_payload["approval"]["id"]
        .as_str()
        .expect("approval id should exist")
        .to_owned();

    let invalid_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "decision": "later",
                        "reviewed_by": "reviewer-1",
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn missing_resources_return_not_found() {
    let app = build_app();

    let missing_run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/runs/run-missing")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(missing_run_response.status(), StatusCode::NOT_FOUND);

    let missing_approval_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/approvals/approval-missing/resolve")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ApprovalResolutionRequest {
                        decision: "approved".into(),
                        reviewed_by: "reviewer-1".into(),
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(missing_approval_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn repeated_approval_resolution_keeps_the_first_decision_visible() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs/task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&TaskSubmissionRequest {
                        workspace_id: "workspace-alpha".into(),
                        project_id: "project-alpha".into(),
                        title: "Review remote hub policy".into(),
                        description: Some("Need approval before artifact generation".into()),
                        requested_by: "operator-1".into(),
                        requires_approval: true,
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let create_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(create_response.into_body(), usize::MAX).await.unwrap())
            .expect("response should deserialize");
    let run_id = create_payload["run"]["id"]
        .as_str()
        .expect("run id should exist")
        .to_owned();
    let approval_id = create_payload["approval"]["id"]
        .as_str()
        .expect("approval id should exist")
        .to_owned();

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ApprovalResolutionRequest {
                        decision: "approved".into(),
                        reviewed_by: "reviewer-1".into(),
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(first_response.status(), StatusCode::OK);

    let second_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ApprovalResolutionRequest {
                        decision: "rejected".into(),
                        reviewed_by: "reviewer-2".into(),
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(second_response.status(), StatusCode::CONFLICT);

    let get_run_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runs/{run_id}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(get_run_response.status(), StatusCode::OK);

    let run_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(get_run_response.into_body(), usize::MAX).await.unwrap())
            .expect("response should deserialize");

    assert_eq!(run_payload["run"]["status"], "paused");
    assert_eq!(run_payload["approval"]["state"], "approved");
    assert_eq!(run_payload["approval"]["reviewed_by"], "reviewer-1");
}

#[tokio::test]
async fn rejected_runs_return_conflict_on_resume_and_do_not_expose_a_checkpoint() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs/task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&TaskSubmissionRequest {
                        workspace_id: "workspace-alpha".into(),
                        project_id: "project-alpha".into(),
                        title: "Review remote hub policy".into(),
                        description: Some("Need approval before artifact generation".into()),
                        requested_by: "operator-1".into(),
                        requires_approval: true,
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let create_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(create_response.into_body(), usize::MAX).await.unwrap())
            .expect("response should deserialize");
    let run_id = create_payload["run"]["id"]
        .as_str()
        .expect("run id should exist")
        .to_owned();
    let approval_id = create_payload["approval"]["id"]
        .as_str()
        .expect("approval id should exist")
        .to_owned();

    let rejection_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ApprovalResolutionRequest {
                        decision: "rejected".into(),
                        reviewed_by: "reviewer-1".into(),
                    })
                    .expect("payload should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(rejection_response.status(), StatusCode::OK);

    let rejection_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(rejection_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");

    assert_eq!(rejection_payload["run"]["status"], "terminated");
    assert_eq!(rejection_payload["run"]["checkpoint_token"], serde_json::Value::Null);

    let resume_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/runs/{run_id}/resume"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(resume_response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn automation_endpoints_create_list_and_manually_deliver() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/automations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-alpha",
                        "name": "Manual drift detector",
                        "trigger_source": "manual_event",
                        "requested_by": "operator-1",
                        "requires_approval": true
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    let trigger_id = create_payload["trigger"]["id"]
        .as_str()
        .expect("trigger id should exist")
        .to_owned();

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/automations")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(list_response.status(), StatusCode::OK);

    let list_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(list_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");

    assert_eq!(list_payload["items"][0]["automation"]["workspace_id"], "workspace-alpha");

    let delivery_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/triggers/deliver")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "trigger_id": trigger_id,
                        "dedupe_key": "manual-event-001",
                        "requested_by": "operator-1",
                        "title": "Investigate configuration drift",
                        "description": "Needs review before artifact generation"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(delivery_response.status(), StatusCode::ACCEPTED);

    let delivery_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(delivery_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");

    assert_eq!(delivery_payload["delivery"]["state"], "succeeded");
    assert_eq!(delivery_payload["run"]["run"]["run_type"], "watch");
    assert_eq!(
        delivery_payload["run"]["inbox_item"]["workspace_id"],
        "workspace-alpha"
    );
}

#[tokio::test]
async fn repeated_trigger_delivery_returns_the_existing_run_without_duplication() {
    let app = build_app();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/automations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-alpha",
                        "name": "Nightly workspace scan",
                        "trigger_source": "cron",
                        "requested_by": "operator-1",
                        "requires_approval": false
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    let trigger_id = create_payload["trigger"]["id"]
        .as_str()
        .expect("trigger id should exist")
        .to_owned();

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/triggers/deliver")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "trigger_id": trigger_id,
                        "dedupe_key": "cron-2026-03-26T00:00",
                        "requested_by": "operator-1",
                        "description": "Scan the workspace"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/triggers/deliver")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "trigger_id": trigger_id,
                        "dedupe_key": "cron-2026-03-26T00:00",
                        "requested_by": "operator-1",
                        "description": "Scan the workspace"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(first_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    let second_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(second_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");

    assert_eq!(first_payload["delivery"]["id"], second_payload["delivery"]["id"]);
    assert_eq!(first_payload["run"]["run"]["id"], second_payload["run"]["run"]["id"]);
}

#[tokio::test]
async fn knowledge_routes_create_candidates_and_promote_assets() {
    let app = build_app();

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs/task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-alpha",
                        "title": "Summarize workspace health",
                        "description": "Artifact body for the workspace summary",
                        "requested_by": "operator-1",
                        "requires_approval": false
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(run_response.status(), StatusCode::CREATED);

    let run_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(run_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    let run_id = run_payload["run"]["id"]
        .as_str()
        .expect("run id should exist")
        .to_owned();

    let spaces_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/knowledge/spaces")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(spaces_response.status(), StatusCode::OK);

    let spaces_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(spaces_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    assert_eq!(spaces_payload["items"][0]["space"]["id"], "knowledge-space-alpha");

    let candidate_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/knowledge/candidates/from-run")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "run_id": run_id,
                        "knowledge_space_id": "knowledge-space-alpha",
                        "created_by": "operator-1"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(candidate_response.status(), StatusCode::CREATED);

    let candidate_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(candidate_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    let candidate_id = candidate_payload["candidate"]["id"]
        .as_str()
        .expect("candidate id should exist")
        .to_owned();

    let assets_before = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/knowledge/spaces/knowledge-space-alpha/assets")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let assets_before_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(assets_before.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    assert_eq!(assets_before_payload["items"], serde_json::json!([]));

    let promote_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/knowledge/candidates/{candidate_id}/promote"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "promoted_by": "owner-1"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(promote_response.status(), StatusCode::OK);

    let promote_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(promote_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    assert_eq!(promote_payload["asset"]["status"], "verified_shared");

    let assets_after = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/knowledge/spaces/knowledge-space-alpha/assets")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    let assets_after_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(assets_after.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");
    assert_eq!(assets_after_payload["items"][0]["status"], "verified_shared");
}

#[tokio::test]
async fn mcp_event_delivery_requires_binding_and_matches_registered_automation() {
    let app = build_app();

    let invalid_create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/automations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-alpha",
                        "name": "Broken MCP watcher",
                        "trigger_source": "mcp_event",
                        "requested_by": "operator-1",
                        "requires_approval": false
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(invalid_create_response.status(), StatusCode::BAD_REQUEST);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/automations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-alpha",
                        "name": "Confluence sync",
                        "trigger_source": "mcp_event",
                        "requested_by": "operator-1",
                        "requires_approval": false,
                        "mcp_binding": {
                            "server_name": "confluence",
                            "event_name": "page.updated"
                        }
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let mcp_delivery_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mcp/events/deliver")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "server_name": "confluence",
                        "event_name": "page.updated",
                        "dedupe_key": "evt-001",
                        "requested_by": "operator-1",
                        "title": "Confluence page updated",
                        "description": "Remote page update"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(mcp_delivery_response.status(), StatusCode::OK);

    let mcp_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(mcp_delivery_response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should deserialize");

    assert_eq!(mcp_payload["items"][0]["delivery"]["source_type"], "mcp_event");
    assert_eq!(mcp_payload["items"][0]["run"]["run"]["run_type"], "watch");
}
