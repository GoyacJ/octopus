use std::{collections::HashMap, fs, path::Path};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use jsonschema::JSONSchema;
use octopus_access_auth::RemoteAccessService;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    Slice1Runtime,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use walkdir::WalkDir;

use octopus_remote_hub::{app, AppState};

fn sample_db_path(base: &Path, name: &str) -> std::path::PathBuf {
    base.join(name)
}

async fn seed_governance(
    runtime: &Slice1Runtime,
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

fn compile_schema(relative_path: &str) -> JSONSchema {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let schemas_root = repo_root.join("schemas");
    let mut resources = HashMap::new();

    for entry in WalkDir::new(&schemas_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "json")
        })
    {
        let schema_json =
            serde_json::from_str::<Value>(&fs::read_to_string(entry.path()).unwrap()).unwrap();
        let id = schema_json
            .get("$id")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        resources.insert(id, schema_json);
    }

    let target = schemas_root.join(relative_path);
    let schema_json = serde_json::from_str::<Value>(&fs::read_to_string(target).unwrap()).unwrap();
    let mut options = JSONSchema::options();
    for (id, document) in resources {
        options.with_document(id, document);
    }
    options.compile(&schema_json).unwrap()
}

fn assert_valid(schema: &JSONSchema, value: &Value) {
    if let Err(errors) = schema.validate(value) {
        let messages = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!("schema validation failed: {messages:?}\nvalue={value}");
    }
}

async fn response_json(router: axum::Router, request: Request<Body>) -> Value {
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn login_access_token(router: axum::Router, workspace_id: &str) -> String {
    let response = router
        .oneshot(
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
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice::<Value>(&body).unwrap()["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn completed_run_surface_matches_minimum_contracts() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "completed.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-slice1",
            "project-slice1",
            "Project Slice 1",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-slice1", "capability-write-note", false).await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-slice1",
            "Project Slice 1 Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime.clone(), auth));
    let access_token = login_access_token(router.clone(), "workspace-alpha").await;
    let authorization = format!("Bearer {access_token}");
    let task_schema = compile_schema("runtime/task.schema.json");
    let run_detail_schema = compile_schema("runtime/run-detail.schema.json");
    let run_summary_schema = compile_schema("runtime/run-summary.schema.json");
    let artifact_schema = compile_schema("observe/artifact.schema.json");
    let knowledge_detail_schema = compile_schema("observe/knowledge-detail.schema.json");
    let project_knowledge_index_schema =
        compile_schema("observe/project-knowledge-index.schema.json");
    let capability_resolution_schema =
        compile_schema("governance/capability-resolution.schema.json");
    let hub_connection_schema = compile_schema("interop/hub-connection-status.schema.json");

    let created_task = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/tasks")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-slice1",
                    "title": "Write note",
                    "instruction": "Emit a deterministic artifact",
                    "action": {
                        "kind": "emit_text",
                        "content": "hello"
                    },
                    "capability_id": "capability-write-note",
                    "estimated_cost": 1,
                    "idempotency_key": "task-1"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&task_schema, &created_task);

    let task_id = created_task["id"].as_str().unwrap().to_string();
    let run_detail = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/tasks/{task_id}/start"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&run_detail_schema, &run_detail);

    let run_id = run_detail["run"]["id"].as_str().unwrap().to_string();
    let loaded_run_detail = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/runs/{run_id}"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&run_detail_schema, &loaded_run_detail);
    assert!(loaded_run_detail["policy_decisions"].is_array());

    let recent_runs = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/runs")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(recent_runs
        .as_array()
        .unwrap()
        .iter()
        .all(|item| run_summary_schema.is_valid(item)));
    assert_eq!(recent_runs.as_array().unwrap()[0]["id"], run_id);

    let artifacts = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/runs/{run_id}/artifacts"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(artifacts
        .as_array()
        .unwrap()
        .iter()
        .all(|item| artifact_schema.is_valid(item)));

    let knowledge_detail = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/runs/{run_id}/knowledge"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&knowledge_detail_schema, &knowledge_detail);

    let project_knowledge = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/knowledge")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&project_knowledge_index_schema, &project_knowledge);
    assert_eq!(project_knowledge["knowledge_space"]["project_id"], "project-slice1");

    let capabilities = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/capabilities?estimated_cost=1")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(capabilities
        .as_array()
        .unwrap()
        .iter()
        .all(|item| capability_resolution_schema.is_valid(item)));

    let connection = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/hub/connection")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&hub_connection_schema, &connection);

    let events_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/events?workspace_id=workspace-alpha&run_id={run_id}"
                ))
                .header("authorization", authorization.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(events_response.status(), StatusCode::OK);
    let events_body = events_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let text = String::from_utf8(events_body.to_vec()).unwrap();
    assert!(text.contains("event: hub.connection.updated"));
    assert!(text.contains("event: run.updated"));
}

#[tokio::test]
async fn capability_resolution_surface_tracks_estimated_cost() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "capability-resolution.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-slice1",
            "project-slice1",
            "Project Slice 1",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-slice1", "capability-write-note", false).await;

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime, auth));
    let access_token = login_access_token(router.clone(), "workspace-alpha").await;
    let authorization = format!("Bearer {access_token}");
    let capability_resolution_schema =
        compile_schema("governance/capability-resolution.schema.json");

    let executable = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/capabilities?estimated_cost=1")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&capability_resolution_schema, &executable[0]);
    assert_eq!(executable[0]["execution_state"], "executable");
    assert_eq!(executable[0]["reason_code"], "within_budget");

    let approval_required = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/capabilities?estimated_cost=7")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(approval_required[0]["execution_state"], "approval_required");
    assert_eq!(
        approval_required[0]["reason_code"],
        "budget_soft_limit_exceeded"
    );

    let denied = response_json(
        router,
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/capabilities?estimated_cost=11")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(denied[0]["execution_state"], "denied");
    assert_eq!(denied[0]["reason_code"], "budget_hard_limit_exceeded");
}

#[tokio::test]
async fn approval_and_knowledge_promotion_surface_round_trip() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "approval.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
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
    seed_governance(&runtime, "project-approval", "capability-approval", true).await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-approval",
            "Approval Project Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime.clone(), auth));
    let access_token = login_access_token(router.clone(), "workspace-alpha").await;
    let authorization = format!("Bearer {access_token}");
    let run_detail_schema = compile_schema("runtime/run-detail.schema.json");
    let approval_request_schema = compile_schema("governance/approval-request.schema.json");
    let inbox_item_schema = compile_schema("observe/inbox-item.schema.json");
    let notification_schema = compile_schema("observe/notification.schema.json");
    let knowledge_detail_schema = compile_schema("observe/knowledge-detail.schema.json");

    let created_task = response_json(
        router.clone(),
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
                    "idempotency_key": "task-approval"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;

    let run_detail = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/tasks/{}/start",
                created_task["id"].as_str().unwrap()
            ))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&run_detail_schema, &run_detail);
    assert_eq!(run_detail["run"]["status"], "waiting_approval");

    let approval_id = run_detail["approvals"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let run_id = run_detail["run"]["id"].as_str().unwrap().to_string();

    let approval_detail = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/approvals/{approval_id}"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&approval_request_schema, &approval_detail);
    assert_eq!(approval_detail["approval_type"], "execution");
    assert_eq!(approval_detail["target_ref"], format!("run:{run_id}"));

    let inbox = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/inbox")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(inbox
        .as_array()
        .unwrap()
        .iter()
        .all(|item| inbox_item_schema.is_valid(item)));

    let notifications = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/notifications")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(notifications
        .as_array()
        .unwrap()
        .iter()
        .all(|item| notification_schema.is_valid(item)));

    let resolved = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/approvals/{approval_id}/resolve"))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
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
    assert_valid(&run_detail_schema, &resolved);
    assert_eq!(resolved["run"]["status"], "completed");

    let knowledge_before = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/runs/{run_id}/knowledge"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&knowledge_detail_schema, &knowledge_before);

    let candidate_id = knowledge_before["candidates"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let promotion_approval = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/knowledge/candidates/{candidate_id}/request-promotion"
            ))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
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
    assert_valid(&approval_request_schema, &promotion_approval);
    assert_eq!(promotion_approval["approval_type"], "knowledge_promotion");
    assert_eq!(
        promotion_approval["target_ref"],
        format!("knowledge_candidate:{candidate_id}")
    );

    let promotion_approval_id = promotion_approval["id"].as_str().unwrap().to_string();

    let resolved_promotion = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/approvals/{promotion_approval_id}/resolve"))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "approval_id": promotion_approval_id,
                    "decision": "approve",
                    "actor_ref": "workspace_admin:alice",
                    "note": "promote"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&run_detail_schema, &resolved_promotion);

    let promoted = response_json(
        router,
        Request::builder()
            .uri(format!("/api/runs/{run_id}/knowledge"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&knowledge_detail_schema, &promoted);
    assert_eq!(promoted["assets"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn automation_manager_surface_round_trip_matches_minimum_contracts() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "automation-surface.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-automation",
            "project-automation",
            "Automation Project",
        )
        .await
        .unwrap();
    seed_governance(
        &runtime,
        "project-automation",
        "capability-automation",
        false,
    )
    .await;

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime.clone(), auth));
    let access_token = login_access_token(router.clone(), "workspace-alpha").await;
    let authorization = format!("Bearer {access_token}");
    let create_response_schema = compile_schema("runtime/create-automation-response.schema.json");
    let summary_schema = compile_schema("runtime/automation-summary.schema.json");
    let detail_schema = compile_schema("runtime/automation-detail.schema.json");

    let manual_created = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-automation",
                    "title": "Manual automation",
                    "instruction": "Dispatch on demand",
                    "action": {
                        "kind": "emit_text",
                        "content": "manual artifact"
                    },
                    "capability_id": "capability-automation",
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
    assert_valid(&create_response_schema, &manual_created);

    let cron_created = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-automation",
                    "title": "Cron automation",
                    "instruction": "Fire from schedule",
                    "action": {
                        "kind": "emit_text",
                        "content": "cron artifact"
                    },
                    "capability_id": "capability-automation",
                    "estimated_cost": 1,
                    "trigger": {
                        "trigger_type": "cron",
                        "config": {
                            "schedule": "0 * * * * * *",
                            "timezone": "UTC",
                            "next_fire_at": "2026-03-27T10:00:00Z"
                        }
                    }
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&create_response_schema, &cron_created);

    let webhook_created = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-automation",
                    "title": "Webhook automation",
                    "instruction": "Accept webhook",
                    "action": {
                        "kind": "emit_text",
                        "content": "webhook artifact"
                    },
                    "capability_id": "capability-automation",
                    "estimated_cost": 1,
                    "trigger": {
                        "trigger_type": "webhook",
                        "config": {
                            "ingress_mode": "shared_secret_header",
                            "secret_header_name": "X-Octopus-Trigger-Secret",
                            "secret_hint": "hook",
                            "secret_plaintext": null
                        }
                    }
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&create_response_schema, &webhook_created);
    assert!(webhook_created["webhook_secret"].as_str().is_some());

    let mcp_created = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-automation",
                    "title": "MCP automation",
                    "instruction": "React to MCP event",
                    "action": {
                        "kind": "emit_text",
                        "content": "mcp artifact"
                    },
                    "capability_id": "capability-automation",
                    "estimated_cost": 1,
                    "trigger": {
                        "trigger_type": "mcp_event",
                        "config": {
                            "server_id": "server-automation",
                            "event_name": "connector.output.ready",
                            "event_pattern": null
                        }
                    }
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&create_response_schema, &mcp_created);

    let manual_automation_id = manual_created["automation"]["id"].as_str().unwrap();
    let manual_trigger_id = manual_created["trigger"]["id"].as_str().unwrap();

    let listed = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 4);
    assert!(listed.iter().all(|item| summary_schema.is_valid(item)));

    let manual_detail = response_json(
        router.clone(),
        Request::builder()
            .uri(format!("/api/automations/{manual_automation_id}"))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &manual_detail);
    assert_eq!(manual_detail["automation"]["status"], "active");

    let already_active = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/automations/{manual_automation_id}/activate"))
                .header("content-type", "application/json")
                .header("authorization", authorization.as_str())
                .body(Body::from(
                    json!({
                        "automation_id": manual_automation_id,
                        "action": "activate"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(already_active.status(), StatusCode::BAD_REQUEST);

    let paused = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/automations/{manual_automation_id}/pause"))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "automation_id": manual_automation_id,
                    "action": "pause"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &paused);
    assert_eq!(paused["automation"]["status"], "paused");

    let reactivated = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/automations/{manual_automation_id}/activate"))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "automation_id": manual_automation_id,
                    "action": "activate"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &reactivated);
    assert_eq!(reactivated["automation"]["status"], "active");

    let dispatched = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/triggers/{manual_trigger_id}/manual-dispatch"))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "trigger_id": manual_trigger_id,
                    "dedupe_key": "manual-dispatch-1",
                    "payload": {
                        "source": "test"
                    }
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &dispatched);
    assert_eq!(dispatched["recent_deliveries"][0]["status"], "succeeded");
    assert_eq!(dispatched["last_run_summary"]["status"], "completed");

    let failing_created = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/workspaces/workspace-alpha/projects/project-automation/automations")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-automation",
                    "title": "Recovering automation",
                    "instruction": "Fail once then recover",
                    "action": {
                        "kind": "fail_once_then_emit_text",
                        "failure_message": "boom",
                        "content": "recovered artifact"
                    },
                    "capability_id": "capability-automation",
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
    assert_valid(&create_response_schema, &failing_created);

    let failing_trigger_id = failing_created["trigger"]["id"].as_str().unwrap();
    let dispatched_failed = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/triggers/{failing_trigger_id}/manual-dispatch"
            ))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "trigger_id": failing_trigger_id,
                    "dedupe_key": "manual-dispatch-fail-1",
                    "payload": {
                        "source": "test"
                    }
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &dispatched_failed);
    assert_eq!(
        dispatched_failed["recent_deliveries"][0]["status"],
        "failed"
    );

    let failed_delivery_id = dispatched_failed["recent_deliveries"][0]["id"]
        .as_str()
        .unwrap();
    let retried = response_json(
        router,
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/trigger-deliveries/{failed_delivery_id}/retry"
            ))
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "delivery_id": failed_delivery_id
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_valid(&detail_schema, &retried);
    assert_eq!(retried["recent_deliveries"][0]["id"], failed_delivery_id);
    assert_eq!(retried["recent_deliveries"][0]["status"], "succeeded");
    assert_eq!(retried["last_run_summary"]["status"], "completed");
}

#[tokio::test]
async fn project_scoped_recent_runs_are_ordered_and_empty_when_no_runs_exist() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path(), "recent-runs.sqlite");
    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-slice1",
            "project-slice1",
            "Project Slice 1",
        )
        .await
        .unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-empty",
            "project-empty",
            "Project Empty",
        )
        .await
        .unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-other",
            "project-other",
            "Project Other",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-slice1", "capability-write-note", false).await;
    seed_governance(&runtime, "project-other", "capability-write-note", false).await;

    let auth = RemoteAccessService::open_at(&db_path).await.unwrap();
    let router = app(AppState::new(runtime.clone(), auth));
    let access_token = login_access_token(router.clone(), "workspace-alpha").await;
    let authorization = format!("Bearer {access_token}");
    let run_summary_schema = compile_schema("runtime/run-summary.schema.json");

    let first_task = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/tasks")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-slice1",
                    "title": "First project run",
                    "instruction": "Emit first artifact",
                    "action": {
                        "kind": "emit_text",
                        "content": "first"
                    },
                    "capability_id": "capability-write-note",
                    "estimated_cost": 1,
                    "idempotency_key": "task-project-1"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    let first_run = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/tasks/{}/start", first_task["id"].as_str().unwrap()))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let second_task = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/tasks")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-slice1",
                    "title": "Second project run",
                    "instruction": "Emit second artifact",
                    "action": {
                        "kind": "emit_text",
                        "content": "second"
                    },
                    "capability_id": "capability-write-note",
                    "estimated_cost": 1,
                    "idempotency_key": "task-project-2"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    let second_run = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/tasks/{}/start", second_task["id"].as_str().unwrap()))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    let other_task = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/tasks")
            .header("content-type", "application/json")
            .header("authorization", authorization.as_str())
            .body(Body::from(
                json!({
                    "workspace_id": "workspace-alpha",
                    "project_id": "project-other",
                    "title": "Other project run",
                    "instruction": "Emit other artifact",
                    "action": {
                        "kind": "emit_text",
                        "content": "other"
                    },
                    "capability_id": "capability-write-note",
                    "estimated_cost": 1,
                    "idempotency_key": "task-project-other"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    let _other_run = response_json(
        router.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/tasks/{}/start", other_task["id"].as_str().unwrap()))
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    let recent_runs = response_json(
        router.clone(),
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-slice1/runs")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let recent_runs = recent_runs.as_array().unwrap();
    assert_eq!(recent_runs.len(), 2);
    assert!(recent_runs.iter().all(|item| run_summary_schema.is_valid(item)));
    assert_eq!(recent_runs[0]["id"], second_run["run"]["id"]);
    assert_eq!(recent_runs[1]["id"], first_run["run"]["id"]);

    let empty_runs = response_json(
        router,
        Request::builder()
            .uri("/api/workspaces/workspace-alpha/projects/project-empty/runs")
            .header("authorization", authorization.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(empty_runs, json!([]));
}
