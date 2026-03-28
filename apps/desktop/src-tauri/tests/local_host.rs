use std::sync::Arc;

use octopus_desktop_host::{
    local_hub_transport_contract, normalize_tauri_invoke_command, CollectingEventEmitter,
    DesktopLocalHost, LocalHostConfig,
};
use serde_json::{json, Value};

fn command(name: &str) -> String {
    normalize_tauri_invoke_command(name)
}

async fn open_host(
) -> (
    tempfile::TempDir,
    DesktopLocalHost,
    Arc<CollectingEventEmitter>,
) {
    let tempdir = tempfile::tempdir().unwrap();
    let emitter = Arc::new(CollectingEventEmitter::default());
    let host = DesktopLocalHost::open(
        LocalHostConfig::new(tempdir.path().join("desktop-local-host.sqlite")),
        emitter.clone(),
    )
    .await
    .unwrap();
    (tempdir, host, emitter)
}

async fn invoke(host: &DesktopLocalHost, name: &str, payload: Value) -> Value {
    host.invoke_transport_command(&command(name), payload)
        .await
        .unwrap()
}

#[tokio::test]
async fn first_boot_seeds_demo_context_and_governed_defaults() {
    let (_tempdir, host, emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let context = invoke(
        &host,
        &contract.commands.get_project_context,
        json!({
            "workspaceId": "demo",
            "projectId": "demo"
        }),
    )
    .await;

    assert_eq!(context["workspace"]["id"], "demo");
    assert_eq!(context["project"]["id"], "demo");

    let executable = invoke(
        &host,
        &contract.commands.list_capability_visibility,
        json!({
            "workspaceId": "demo",
            "projectId": "demo",
            "estimatedCost": 1
        }),
    )
    .await;
    assert_eq!(executable[0]["execution_state"], "executable");

    let approval_required = invoke(
        &host,
        &contract.commands.list_capability_visibility,
        json!({
            "workspaceId": "demo",
            "projectId": "demo",
            "estimatedCost": 7
        }),
    )
    .await;
    assert_eq!(approval_required[0]["execution_state"], "approval_required");

    let denied = invoke(
        &host,
        &contract.commands.list_capability_visibility,
        json!({
            "workspaceId": "demo",
            "projectId": "demo",
            "estimatedCost": 11
        }),
    )
    .await;
    assert_eq!(denied[0]["execution_state"], "denied");

    let connection_status = invoke(&host, &contract.commands.get_connection_status, json!({})).await;
    assert_eq!(connection_status["mode"], "local");
    assert_eq!(connection_status["state"], "connected");
    assert_eq!(connection_status["auth_state"], "authenticated");

    let events = emitter.events_snapshot();
    assert!(!events.is_empty());
    assert_eq!(events[0].channel, contract.event_channel);
    assert_eq!(events[0].payload["event_type"], "hub.connection.updated");
}

#[tokio::test]
async fn task_happy_path_round_trips_through_transport_and_records_knowledge() {
    let (_tempdir, host, emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let task = invoke(
        &host,
        &contract.commands.create_task,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "Write local artifact",
            "instruction": "Emit deterministic output",
            "action": {
                "kind": "emit_text",
                "content": "local host artifact"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 1,
            "idempotency_key": "task-local-demo"
        }),
    )
    .await;

    let run_detail = invoke(
        &host,
        &contract.commands.start_task,
        json!({
            "taskId": task["id"].as_str().unwrap()
        }),
    )
    .await;

    assert_eq!(run_detail["run"]["status"], "completed");
    assert_eq!(run_detail["artifacts"][0]["content"], "local host artifact");
    assert_eq!(run_detail["knowledge_candidates"][0]["status"], "candidate");

    let knowledge_detail = invoke(
        &host,
        &contract.commands.get_knowledge_detail,
        json!({
            "runId": run_detail["run"]["id"].as_str().unwrap()
        }),
    )
    .await;

    assert_eq!(knowledge_detail["knowledge_space"]["workspace_id"], "demo");
    assert_eq!(knowledge_detail["candidates"][0]["content"], "local host artifact");

    let events = emitter.events_snapshot();
    assert!(events
        .iter()
        .any(|event| event.payload["event_type"] == "run.updated"));
}

#[tokio::test]
async fn list_runs_is_project_scoped_empty_until_execution_and_sorted_latest_first() {
    let (_tempdir, host, _emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let empty = invoke(
        &host,
        &contract.commands.list_runs,
        json!({
            "workspaceId": "demo",
            "projectId": "other-project"
        }),
    )
    .await;
    assert_eq!(empty, json!([]));

    let first_task = invoke(
        &host,
        &contract.commands.create_task,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "First workbench run",
            "instruction": "Emit first artifact",
            "action": {
                "kind": "emit_text",
                "content": "first"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 1,
            "idempotency_key": "task-local-runs-first"
        }),
    )
    .await;
    let first_run = invoke(
        &host,
        &contract.commands.start_task,
        json!({
            "taskId": first_task["id"].as_str().unwrap()
        }),
    )
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let second_task = invoke(
        &host,
        &contract.commands.create_task,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "Second workbench run",
            "instruction": "Emit second artifact",
            "action": {
                "kind": "emit_text",
                "content": "second"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 1,
            "idempotency_key": "task-local-runs-second"
        }),
    )
    .await;
    let second_run = invoke(
        &host,
        &contract.commands.start_task,
        json!({
            "taskId": second_task["id"].as_str().unwrap()
        }),
    )
    .await;

    let runs = invoke(
        &host,
        &contract.commands.list_runs,
        json!({
            "workspaceId": "demo",
            "projectId": "demo"
        }),
    )
    .await;

    let summaries = runs.as_array().unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0]["id"], second_run["run"]["id"]);
    assert_eq!(summaries[1]["id"], first_run["run"]["id"]);
    assert_eq!(summaries[0]["title"], "Second workbench run");
}

#[tokio::test]
async fn approval_wait_and_resume_flow_updates_run_and_workspace_governance_state() {
    let (_tempdir, host, emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let task = invoke(
        &host,
        &contract.commands.create_task,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "Approval task",
            "instruction": "Require approval via soft budget limit",
            "action": {
                "kind": "emit_text",
                "content": "approval artifact"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 7,
            "idempotency_key": "task-local-approval"
        }),
    )
    .await;

    let waiting = invoke(
        &host,
        &contract.commands.start_task,
        json!({
            "taskId": task["id"].as_str().unwrap()
        }),
    )
    .await;

    assert_eq!(waiting["run"]["status"], "waiting_approval");
    assert_eq!(waiting["approvals"][0]["status"], "pending");
    assert_eq!(waiting["inbox_items"][0]["status"], "open");
    assert_eq!(waiting["notifications"][0]["status"], "pending");

    let resumed = invoke(
        &host,
        &contract.commands.resolve_approval,
        json!({
            "approval_id": waiting["approvals"][0]["id"].as_str().unwrap(),
            "decision": "approve",
            "actor_ref": "workspace_admin:desktop_operator",
            "note": "approved locally"
        }),
    )
    .await;

    assert_eq!(resumed["run"]["status"], "completed");
    assert_eq!(resumed["approvals"][0]["status"], "approved");
    assert_eq!(resumed["inbox_items"][0]["status"], "resolved");

    let events = emitter.events_snapshot();
    assert!(events
        .iter()
        .any(|event| event.payload["event_type"] == "inbox.updated"));
    assert!(events
        .iter()
        .any(|event| event.payload["event_type"] == "notification.updated"));
}

#[tokio::test]
async fn automation_lifecycle_manual_dispatch_and_retry_are_exposed() {
    let (_tempdir, host, _emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let created = invoke(
        &host,
        &contract.commands.create_automation,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "Retry automation",
            "instruction": "Fail once then recover",
            "action": {
                "kind": "fail_once_then_emit_text",
                "failure_message": "network_glitch",
                "content": "recovered automation artifact"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 1,
            "trigger": {
                "trigger_type": "manual_event",
                "config": {}
            }
        }),
    )
    .await;

    let automation_id = created["automation"]["id"].as_str().unwrap();

    let paused = invoke(
        &host,
        &contract.commands.pause_automation,
        json!({
            "automation_id": automation_id,
            "action": "pause"
        }),
    )
    .await;
    assert_eq!(paused["automation"]["status"], "paused");

    let active = invoke(
        &host,
        &contract.commands.activate_automation,
        json!({
            "automation_id": automation_id,
            "action": "activate"
        }),
    )
    .await;
    assert_eq!(active["automation"]["status"], "active");

    let failed_delivery = invoke(
        &host,
        &contract.commands.manual_dispatch,
        json!({
            "trigger_id": created["trigger"]["id"].as_str().unwrap(),
            "dedupe_key": "manual-dispatch-retry",
            "payload": {
                "source": "test"
            }
        }),
    )
    .await;
    assert_eq!(failed_delivery["recent_deliveries"][0]["status"], "failed");

    let retried = invoke(
        &host,
        &contract.commands.retry_trigger_delivery,
        json!({
            "delivery_id": failed_delivery["recent_deliveries"][0]["id"].as_str().unwrap()
        }),
    )
    .await;
    assert_eq!(retried["recent_deliveries"][0]["status"], "succeeded");
    assert_eq!(retried["last_run_summary"]["status"], "completed");

    let archived = invoke(
        &host,
        &contract.commands.archive_automation,
        json!({
            "automation_id": automation_id,
            "action": "archive"
        }),
    )
    .await;
    assert_eq!(archived["automation"]["status"], "archived");
}

#[tokio::test]
async fn cron_tick_dispatches_due_local_automation_once() {
    let (_tempdir, host, _emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let created = invoke(
        &host,
        &contract.commands.create_automation,
        json!({
            "workspace_id": "demo",
            "project_id": "demo",
            "title": "Cron automation",
            "instruction": "Fire on tick",
            "action": {
                "kind": "emit_text",
                "content": "cron artifact"
            },
            "capability_id": "capability-local-demo",
            "estimated_cost": 1,
            "trigger": {
                "trigger_type": "cron",
                "config": {
                    "schedule": "0 * * * * * *",
                    "timezone": "UTC",
                    "next_fire_at": "2026-03-28T10:00:00Z"
                }
            }
        }),
    )
    .await;

    let first = host.tick_due_triggers("2026-03-28T10:00:00Z").await.unwrap();
    assert_eq!(first.len(), 1);

    let second = host.tick_due_triggers("2026-03-28T10:00:00Z").await.unwrap();
    assert!(second.is_empty());

    let detail = invoke(
        &host,
        &contract.commands.get_automation_detail,
        json!({
            "automationId": created["automation"]["id"].as_str().unwrap()
        }),
    )
    .await;
    assert_eq!(detail["recent_deliveries"][0]["status"], "succeeded");
    assert_eq!(detail["last_run_summary"]["status"], "completed");
}

#[tokio::test]
async fn unsupported_local_ingress_triggers_are_rejected() {
    let (_tempdir, host, _emitter) = open_host().await;
    let contract = local_hub_transport_contract();

    let webhook_error = host
        .invoke_transport_command(
            &command(&contract.commands.create_automation),
            json!({
                "workspace_id": "demo",
                "project_id": "demo",
                "title": "Webhook automation",
                "instruction": "Should be rejected",
                "action": {
                    "kind": "emit_text",
                    "content": "webhook"
                },
                "capability_id": "capability-local-demo",
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
            }),
        )
        .await
        .unwrap_err();
    assert!(webhook_error
        .to_string()
        .contains("Local host only supports manual_event and cron"));

    let mcp_event_error = host
        .invoke_transport_command(
            &command(&contract.commands.create_automation),
            json!({
                "workspace_id": "demo",
                "project_id": "demo",
                "title": "MCP automation",
                "instruction": "Should be rejected",
                "action": {
                    "kind": "emit_text",
                    "content": "mcp"
                },
                "capability_id": "capability-local-demo",
                "estimated_cost": 1,
                "trigger": {
                    "trigger_type": "mcp_event",
                    "config": {
                        "server_id": "server-local",
                        "event_name": "connector.output.ready",
                        "event_pattern": null
                    }
                }
            }),
        )
        .await
        .unwrap_err();
    assert!(mcp_event_error
        .to_string()
        .contains("Local host only supports manual_event and cron"));
}
