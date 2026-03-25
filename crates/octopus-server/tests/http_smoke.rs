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
