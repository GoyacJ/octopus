use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use octopus_access_auth::RemoteAccessService;
use octopus_runtime::Slice1Runtime;
use serde_json::{json, Value};
use tower::ServiceExt;

use octopus_remote_hub::{app, ensure_dev_seed_context, AppState};

async fn response(router: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let raw_body = String::from_utf8_lossy(&body).to_string();
    let json = serde_json::from_slice(&body).unwrap_or_else(|_| json!({ "raw": raw_body }));
    (status, json)
}

#[tokio::test]
async fn dev_seed_makes_an_empty_database_loginable_and_project_listable() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = tempdir.path().join("remote-hub-dev-seed.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();

    ensure_dev_seed_context(&runtime).await.unwrap();

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime, auth));

    let (login_status, login_body) = response(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "email": "admin@octopus.local",
                    "password": "octopus-bootstrap-password"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;

    assert_eq!(login_status, StatusCode::OK, "body={login_body}");
    let access_token = login_body["access_token"].as_str().unwrap().to_string();
    assert_eq!(login_body["session"]["workspace_id"], "workspace-alpha");

    let (projects_status, projects_body) = response(
        router,
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects")
            .header("authorization", format!("Bearer {access_token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(projects_status, StatusCode::OK, "body={projects_body}");
    let project_ids = projects_body
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(project_ids, vec!["project-remote-demo"]);
}
