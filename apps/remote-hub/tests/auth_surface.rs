use std::path::Path;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use octopus_access_auth::{RemoteAccessConfig, RemoteAccessService};
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    Slice1Runtime,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use octopus_remote_hub::{app, AppState};

fn sample_db_path(base: &Path, name: &str) -> std::path::PathBuf {
    base.join(name)
}

async fn seed_governance(
    runtime: &Slice1Runtime,
    workspace_id: &str,
    project_id: &str,
    capability_id: &str,
    requires_approval: bool,
) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            capability_id,
            capability_id,
            "low",
            requires_approval,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{project_id}"),
            capability_id,
            workspace_id,
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{project_id}"),
            capability_id,
            workspace_id,
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_budget_policy(BudgetPolicyRecord::project_scope(
            format!("budget-{project_id}"),
            workspace_id,
            project_id,
            5,
            10,
        ))
        .await
        .unwrap();
}

async fn response(router: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let raw_body = String::from_utf8_lossy(&body).to_string();
    let json = serde_json::from_slice(&body).unwrap_or_else(|_| json!({ "raw": raw_body }));
    (status, json)
}

struct TestHarness {
    _tempdir: tempfile::TempDir,
    router: axum::Router,
}

impl TestHarness {
    async fn seeded() -> Self {
        Self::seeded_with_auth_config(RemoteAccessConfig::default()).await
    }

    async fn seeded_with_auth_config(config: RemoteAccessConfig) -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let db_path = sample_db_path(tempdir.path(), "auth.sqlite");
        let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();

        runtime
            .ensure_project_context(
                "workspace-alpha",
                "workspace-alpha",
                "Workspace Alpha",
                "project-auth",
                "project-auth",
                "Auth Project",
            )
            .await
            .unwrap();
        runtime
            .ensure_project_context(
                "workspace-alpha",
                "workspace-alpha",
                "Workspace Alpha",
                "project-approval",
                "project-approval",
                "Approval Project",
            )
            .await
            .unwrap();
        runtime
            .ensure_project_context(
                "workspace-bravo",
                "workspace-bravo",
                "Workspace Bravo",
                "project-bravo",
                "project-bravo",
                "Bravo Project",
            )
            .await
            .unwrap();
        seed_governance(
            &runtime,
            "workspace-alpha",
            "project-auth",
            "capability-write-note",
            false,
        )
        .await;
        seed_governance(
            &runtime,
            "workspace-alpha",
            "project-approval",
            "capability-approval",
            true,
        )
        .await;
        runtime
            .ensure_project_knowledge_space(
                "workspace-alpha",
                "project-approval",
                "Approval Project Knowledge",
                "workspace_admin:bootstrap_admin",
            )
            .await
            .unwrap();

        let auth = RemoteAccessService::open_at_with_config(&db_path, config)
            .await
            .unwrap();
        Self {
            _tempdir: tempdir,
            router: app(AppState::new(runtime, auth)),
        }
    }

    async fn response(&self, request: Request<Body>) -> (StatusCode, Value) {
        response(self.router.clone(), request).await
    }

    async fn login(&self, workspace_id: &str) -> (String, Value) {
        let (status, body) = self
            .response(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "workspace_id": workspace_id,
                            "email": "admin@octopus.local",
                            "password": "octopus-bootstrap-password"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        let access_token = body["access_token"].as_str().unwrap().to_string();
        (format!("Bearer {access_token}"), body)
    }
}

#[tokio::test]
async fn hub_connection_reports_auth_required_without_session() {
    let harness = TestHarness::seeded().await;
    let (status, body) = harness
        .response(
            Request::builder()
                .uri("/api/hub/connection")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(body["mode"], "remote");
    assert_eq!(body["auth_state"], "auth_required");
}

#[tokio::test]
async fn protected_routes_require_authentication() {
    let harness = TestHarness::seeded().await;
    let (status, body) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-auth",
                        "title": "Write note",
                        "instruction": "Emit a deterministic artifact",
                        "action": {
                            "kind": "emit_text",
                            "content": "hello"
                        },
                        "capability_id": "capability-write-note",
                        "estimated_cost": 1,
                        "idempotency_key": "task-auth-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error_code"], "auth_required");
}

#[tokio::test]
async fn project_list_route_requires_authentication_and_enforces_workspace_membership() {
    let harness = TestHarness::seeded().await;

    let (unauthorized_status, unauthorized_body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(unauthorized_status, StatusCode::UNAUTHORIZED);
    assert_eq!(unauthorized_body["error_code"], "auth_required");

    let (alpha_authorization, _) = harness.login("workspace-alpha").await;
    let (ok_status, ok_body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/projects")
                .header("authorization", alpha_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(ok_status, StatusCode::OK, "body={ok_body}");
    let project_ids = ok_body
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(project_ids, vec!["project-approval", "project-auth"]);

    let (forbidden_status, forbidden_body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-bravo/projects")
                .header("authorization", alpha_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(forbidden_status, StatusCode::FORBIDDEN, "body={forbidden_body}");
    assert_eq!(forbidden_body["error_code"], "workspace_forbidden");
}

#[tokio::test]
async fn automation_routes_require_authentication_and_enforce_workspace_membership() {
    let harness = TestHarness::seeded().await;

    let (list_status, list_body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/projects/project-auth/automations")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(list_status, StatusCode::UNAUTHORIZED, "body={list_body}");
    assert_eq!(list_body["error_code"], "auth_required");

    let (alpha_authorization, _) = harness.login("workspace-alpha").await;
    let (create_status, created) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces/workspace-alpha/projects/project-auth/automations")
                .header("content-type", "application/json")
                .header("authorization", alpha_authorization.as_str())
                .body(Body::from(
                    json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-auth",
                        "title": "Manual automation",
                        "instruction": "Dispatch on demand",
                        "action": {
                            "kind": "emit_text",
                            "content": "hello"
                        },
                        "capability_id": "capability-write-note",
                        "estimated_cost": 1,
                        "trigger": {
                            "trigger_type": "manual_event",
                            "config": {}
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(create_status, StatusCode::OK, "body={created}");

    let automation_id = created["automation"]["id"].as_str().unwrap();
    let (bravo_authorization, _) = harness.login("workspace-bravo").await;
    let (detail_status, detail_body) = harness
        .response(
            Request::builder()
                .uri(format!("/api/automations/{automation_id}"))
                .header("authorization", bravo_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(detail_status, StatusCode::FORBIDDEN, "body={detail_body}");
    assert_eq!(detail_body["error_code"], "workspace_forbidden");
    assert_eq!(detail_body["auth_state"], "authenticated");
}

#[tokio::test]
async fn governance_routes_require_authentication_and_enforce_workspace_membership() {
    let harness = TestHarness::seeded().await;

    let (alpha_authorization, _) = harness.login("workspace-alpha").await;
    let (create_status, created_task) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .header("authorization", alpha_authorization.as_str())
                .body(Body::from(
                    json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-approval",
                        "title": "Sensitive task",
                        "instruction": "Emit a deterministic artifact",
                        "action": {
                            "kind": "emit_text",
                            "content": "restricted"
                        },
                        "capability_id": "capability-approval",
                        "estimated_cost": 1,
                        "idempotency_key": "task-approval-auth"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(create_status, StatusCode::OK, "body={created_task}");

    let task_id = created_task["id"].as_str().unwrap();
    let (start_status, run_detail) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{task_id}/start"))
                .header("authorization", alpha_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(start_status, StatusCode::OK, "body={run_detail}");

    let approval_id = run_detail["approvals"][0].get("id").and_then(Value::as_str).unwrap();
    let run_id = run_detail["run"]["id"].as_str().unwrap();

    let (approval_unauth_status, approval_unauth_body) = harness
        .response(
            Request::builder()
                .uri(format!("/api/approvals/{approval_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(approval_unauth_status, StatusCode::UNAUTHORIZED);
    assert_eq!(approval_unauth_body["error_code"], "auth_required");

    let (resolve_status, resolved_run) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri(format!("/api/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .header("authorization", alpha_authorization.as_str())
                .body(Body::from(
                    json!({
                        "approval_id": approval_id,
                        "decision": "approve",
                        "actor_ref": "workspace_admin:alice",
                        "note": "approved"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(resolve_status, StatusCode::OK, "body={resolved_run}");

    let knowledge_detail = {
        let (status, body) = harness
            .response(
                Request::builder()
                    .uri(format!("/api/runs/{run_id}/knowledge"))
                    .header("authorization", alpha_authorization.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "body={body}");
        body
    };
    let candidate_id = knowledge_detail["candidates"][0]
        .get("id")
        .and_then(Value::as_str)
        .unwrap();

    let (bravo_authorization, _) = harness.login("workspace-bravo").await;
    let (approval_forbidden_status, approval_forbidden_body) = harness
        .response(
            Request::builder()
                .uri(format!("/api/approvals/{approval_id}"))
                .header("authorization", bravo_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(approval_forbidden_status, StatusCode::FORBIDDEN);
    assert_eq!(approval_forbidden_body["error_code"], "workspace_forbidden");

    let (promotion_forbidden_status, promotion_forbidden_body) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/knowledge/candidates/{candidate_id}/request-promotion"
                ))
                .header("content-type", "application/json")
                .header("authorization", bravo_authorization.as_str())
                .body(Body::from(
                    json!({
                        "candidate_id": candidate_id,
                        "actor_ref": "workspace_admin:alice",
                        "note": "promote"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(promotion_forbidden_status, StatusCode::FORBIDDEN);
    assert_eq!(promotion_forbidden_body["error_code"], "workspace_forbidden");

    let (project_knowledge_forbidden_status, project_knowledge_forbidden_body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/projects/project-approval/knowledge")
                .header("authorization", bravo_authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(project_knowledge_forbidden_status, StatusCode::FORBIDDEN);
    assert_eq!(
        project_knowledge_forbidden_body["error_code"],
        "workspace_forbidden"
    );
}

#[tokio::test]
async fn bootstrap_login_returns_a_remote_session() {
    let harness = TestHarness::seeded().await;
    let (status, body) = harness
        .response(
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

    assert_eq!(status, StatusCode::OK, "body={body}");
    assert!(body["access_token"].as_str().is_some());
    assert_eq!(body["session"]["workspace_id"], "workspace-alpha");
    assert_eq!(body["session"]["actor_ref"], "workspace_admin:bootstrap_admin");
}

#[tokio::test]
async fn expired_tokens_report_token_expired_separately_from_disconnect() {
    let harness = TestHarness::seeded_with_auth_config(RemoteAccessConfig {
        session_ttl_seconds: -60,
        ..RemoteAccessConfig::default()
    })
    .await;
    let (authorization, _) = harness.login("workspace-alpha").await;

    let (session_status, session_body) = harness
        .response(
            Request::builder()
                .uri("/api/auth/session")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(session_status, StatusCode::UNAUTHORIZED, "body={session_body}");
    assert_eq!(session_body["error_code"], "token_expired");
    assert_eq!(session_body["auth_state"], "token_expired");

    let (connection_status, connection_body) = harness
        .response(
            Request::builder()
                .uri("/api/hub/connection")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(connection_status, StatusCode::OK, "body={connection_body}");
    assert_eq!(connection_body["state"], "connected");
    assert_eq!(connection_body["auth_state"], "token_expired");
}

#[tokio::test]
async fn workspace_membership_is_enforced_after_login() {
    let harness = TestHarness::seeded().await;
    let (authorization, _) = harness.login("workspace-alpha").await;

    let (status, body) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-bravo/inbox")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "body={body}");
    assert_eq!(body["error_code"], "workspace_forbidden");
    assert_eq!(body["auth_state"], "authenticated");
}

#[tokio::test]
async fn logout_revokes_the_current_session() {
    let harness = TestHarness::seeded().await;
    let (authorization, _) = harness.login("workspace-alpha").await;

    let (logout_status, logout_body) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(logout_status, StatusCode::NO_CONTENT, "body={logout_body}");

    let (session_status, session_body) = harness
        .response(
            Request::builder()
                .uri("/api/auth/session")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(session_status, StatusCode::UNAUTHORIZED, "body={session_body}");
    assert_eq!(session_body["error_code"], "auth_required");

    let (connection_status, connection_body) = harness
        .response(
            Request::builder()
                .uri("/api/hub/connection")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(connection_status, StatusCode::OK, "body={connection_body}");
    assert_eq!(connection_body["auth_state"], "auth_required");
}

#[tokio::test]
async fn authenticated_session_can_access_task_approval_and_knowledge_routes() {
    let harness = TestHarness::seeded().await;
    let (authorization, session_body) = harness.login("workspace-alpha").await;

    let (session_status, current_session) = harness
        .response(
            Request::builder()
                .uri("/api/auth/session")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(session_status, StatusCode::OK, "body={current_session}");
    assert_eq!(current_session["session_id"], session_body["session"]["session_id"]);

    let (create_status, created_task) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .header("authorization", authorization.as_str())
                .body(Body::from(
                    json!({
                        "workspace_id": "workspace-alpha",
                        "project_id": "project-approval",
                        "title": "Sensitive task",
                        "instruction": "Emit a deterministic artifact",
                        "action": {
                            "kind": "emit_text",
                            "content": "restricted"
                        },
                        "capability_id": "capability-approval",
                        "estimated_cost": 1,
                        "idempotency_key": "task-auth-approval"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(create_status, StatusCode::OK, "body={created_task}");

    let task_id = created_task["id"].as_str().unwrap();
    let (start_status, run_detail) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{task_id}/start"))
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(start_status, StatusCode::OK, "body={run_detail}");
    assert_eq!(run_detail["run"]["status"], "waiting_approval");

    let run_id = run_detail["run"]["id"].as_str().unwrap();
    let approval_id = run_detail["approvals"][0]["id"].as_str().unwrap();

    let (loaded_run_status, loaded_run) = harness
        .response(
            Request::builder()
                .uri(format!("/api/runs/{run_id}"))
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(loaded_run_status, StatusCode::OK, "body={loaded_run}");
    assert_eq!(loaded_run["run"]["id"], run_id);

    let (approval_status, approved_run) = harness
        .response(
            Request::builder()
                .method("POST")
                .uri(format!("/api/approvals/{approval_id}/resolve"))
                .header("content-type", "application/json")
                .header("authorization", authorization.as_str())
                .body(Body::from(
                    json!({
                        "approval_id": approval_id,
                        "decision": "approve",
                        "actor_ref": "workspace_admin:bootstrap_admin",
                        "note": "approved by auth surface test"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(approval_status, StatusCode::OK, "body={approved_run}");
    assert_eq!(approved_run["run"]["status"], "completed");

    let (knowledge_status, knowledge_detail) = harness
        .response(
            Request::builder()
                .uri(format!("/api/runs/{run_id}/knowledge"))
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(knowledge_status, StatusCode::OK, "body={knowledge_detail}");
    assert_eq!(knowledge_detail["knowledge_space"]["project_id"], "project-approval");

    let (project_knowledge_status, project_knowledge) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/projects/project-approval/knowledge")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        project_knowledge_status,
        StatusCode::OK,
        "body={project_knowledge}"
    );
    assert_eq!(project_knowledge["knowledge_space"]["project_id"], "project-approval");

    let (inbox_status, inbox_items) = harness
        .response(
            Request::builder()
                .uri("/api/workspaces/workspace-alpha/inbox")
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(inbox_status, StatusCode::OK, "body={inbox_items}");
    assert!(inbox_items.as_array().is_some());
}
