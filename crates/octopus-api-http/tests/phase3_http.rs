use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use octopus_infra_sqlite::SqlitePhase3Store;
use octopus_runtime::Phase3Service;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn serves_phase3_run_inbox_resume_and_audit_flow() {
    let store = SqlitePhase3Store::connect("sqlite::memory:").await.unwrap();
    let service = Phase3Service::new(store);
    let app = octopus_api_http::build_router(service);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workspaceId": "workspace-1",
                        "agentId": "agent-1",
                        "input": "Need user confirmation for the migration plan",
                        "interactionType": "ask_user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::ACCEPTED);

    let created: Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let run_id = created["id"].as_str().unwrap().to_owned();

    let inbox_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/inbox/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(inbox_response.status(), StatusCode::OK);
    let inbox: Value = serde_json::from_slice(
        &to_bytes(inbox_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let pending = &inbox["items"][0];

    let resume_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/runs/{run_id}/resume"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "inboxItemId": pending["id"],
                        "resumeToken": pending["resumeToken"],
                        "idempotencyKey": "resume-http-1",
                        "response": {
                            "type": "text",
                            "text": "Proceed with the updated scope",
                            "goalChanged": true
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resume_response.status(), StatusCode::ACCEPTED);

    let timeline_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/runs/{run_id}/timeline"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let timeline: Value = serde_json::from_slice(
        &to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(timeline["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["type"] == "run.completed"));

    let audit_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/audit/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let audit: Value = serde_json::from_slice(
        &to_bytes(audit_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["action"] == "run.resume.accepted"));
}

#[tokio::test]
async fn serves_phase3_approval_rejection_flow() {
    let store = SqlitePhase3Store::connect("sqlite::memory:").await.unwrap();
    let service = Phase3Service::new(store);
    let app = octopus_api_http::build_router(service);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workspaceId": "workspace-1",
                        "agentId": "agent-1",
                        "input": "Approve the production deployment",
                        "interactionType": "approval"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::ACCEPTED);

    let created: Value = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let run_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["status"], "waiting_approval");

    let inbox_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/inbox/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(inbox_response.status(), StatusCode::OK);
    let inbox: Value = serde_json::from_slice(
        &to_bytes(inbox_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let pending = &inbox["items"][0];
    assert_eq!(pending["kind"], "approval");
    assert_eq!(pending["responseType"], "approval");

    let resume_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/runs/{run_id}/resume"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "inboxItemId": pending["id"],
                        "resumeToken": pending["resumeToken"],
                        "idempotencyKey": "approval-http-1",
                        "response": {
                            "type": "approval",
                            "approved": false,
                            "text": "Risk changed while waiting",
                            "goalChanged": true
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resume_response.status(), StatusCode::ACCEPTED);
    let resumed: Value = serde_json::from_slice(
        &to_bytes(resume_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(resumed["status"], "failed");

    let timeline_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/runs/{run_id}/timeline"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let timeline: Value = serde_json::from_slice(
        &to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(timeline["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["type"] == "run.failed"));

    let audit_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/audit/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let audit: Value = serde_json::from_slice(
        &to_bytes(audit_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let audit_items = audit["items"].as_array().unwrap();
    assert!(audit_items
        .iter()
        .any(|event| event["action"] == "run.resume.accepted"));
    assert!(audit_items
        .iter()
        .any(|event| event["action"] == "approval.rejected"));
}
