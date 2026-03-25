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

