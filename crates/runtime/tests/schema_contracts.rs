use std::{collections::HashMap, fs, path::PathBuf};

use serde_json::{json, Value};
use walkdir::WalkDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn schema_root() -> PathBuf {
    repo_root().join("schemas")
}

fn schema_path(relative: &str) -> PathBuf {
    schema_root().join(relative)
}

fn load_json(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn compiled_schema(relative: &str) -> jsonschema::JSONSchema {
    let mut resources = HashMap::new();
    for entry in WalkDir::new(schema_root()) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file()
            || !entry.path().extension().is_some_and(|ext| ext == "json")
        {
            continue;
        }
        let value = load_json(&entry.path().to_path_buf());
        let id = value
            .get("$id")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        resources.insert(id, value);
    }

    let schema = load_json(&schema_path(relative));
    let mut options = jsonschema::JSONSchema::options();
    for (id, document) in resources {
        options.with_document(id, document);
    }
    options.compile(&schema).unwrap()
}

#[test]
fn all_schema_files_parse_as_json() {
    let mut count = 0;
    for entry in WalkDir::new(schema_root()) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file()
            || !entry.path().extension().is_some_and(|ext| ext == "json")
        {
            continue;
        }
        let _: Value = load_json(&entry.path().to_path_buf());
        count += 1;
    }

    assert!(count >= 40);
}

#[test]
fn refined_slice1_examples_validate() {
    let project_context_schema = compiled_schema("context/project-context.schema.json");
    let workspace_schema = compiled_schema("context/workspace.schema.json");
    let project_schema = compiled_schema("context/project.schema.json");
    let knowledge_space_schema = compiled_schema("context/knowledge-space.schema.json");
    let task_schema = compiled_schema("runtime/task.schema.json");
    let task_create_command_schema = compiled_schema("runtime/task-create-command.schema.json");
    let run_schema = compiled_schema("runtime/run.schema.json");
    let run_summary_schema = compiled_schema("runtime/run-summary.schema.json");
    let run_detail_schema = compiled_schema("runtime/run-detail.schema.json");
    let automation_schema = compiled_schema("runtime/automation.schema.json");
    let trigger_schema = compiled_schema("runtime/trigger.schema.json");
    let trigger_delivery_schema = compiled_schema("runtime/trigger-delivery.schema.json");
    let environment_lease_status_schema =
        compiled_schema("runtime/environment-lease-status.schema.json");
    let environment_lease_schema = compiled_schema("runtime/environment-lease.schema.json");
    let approval_schema = compiled_schema("governance/approval-request.schema.json");
    let approval_resolve_command_schema =
        compiled_schema("governance/approval-resolve-command.schema.json");
    let capability_descriptor_schema =
        compiled_schema("governance/capability-descriptor.schema.json");
    let capability_binding_schema = compiled_schema("governance/capability-binding.schema.json");
    let capability_grant_schema = compiled_schema("governance/capability-grant.schema.json");
    let budget_policy_schema = compiled_schema("governance/budget-policy.schema.json");
    let capability_visibility_schema =
        compiled_schema("governance/capability-visibility.schema.json");
    let artifact_schema = compiled_schema("observe/artifact.schema.json");
    let artifact_summary_schema = compiled_schema("observe/artifact-summary.schema.json");
    let audit_schema = compiled_schema("observe/audit-record.schema.json");
    let trace_schema = compiled_schema("observe/trace-record.schema.json");
    let inbox_schema = compiled_schema("observe/inbox-item.schema.json");
    let notification_schema = compiled_schema("observe/notification.schema.json");
    let policy_decision_schema = compiled_schema("observe/policy-decision-log.schema.json");
    let knowledge_candidate_status_schema =
        compiled_schema("observe/knowledge-candidate-status.schema.json");
    let knowledge_asset_status_schema =
        compiled_schema("observe/knowledge-asset-status.schema.json");
    let knowledge_candidate_schema = compiled_schema("observe/knowledge-candidate.schema.json");
    let knowledge_asset_schema = compiled_schema("observe/knowledge-asset.schema.json");
    let knowledge_lineage_schema = compiled_schema("observe/knowledge-lineage-record.schema.json");
    let knowledge_summary_schema = compiled_schema("observe/knowledge-summary.schema.json");
    let knowledge_detail_schema = compiled_schema("observe/knowledge-detail.schema.json");
    let knowledge_promote_command_schema =
        compiled_schema("observe/knowledge-promote-command.schema.json");
    let hub_connection_status_schema =
        compiled_schema("interop/hub-connection-status.schema.json");
    let hub_event_schema = compiled_schema("interop/hub-event.schema.json");

    assert!(workspace_schema.is_valid(&json!({
        "id": "workspace-alpha",
        "slug": "workspace-alpha",
        "display_name": "Workspace Alpha",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(project_context_schema.is_valid(&json!({
        "workspace": {
            "id": "workspace-alpha",
            "slug": "workspace-alpha",
            "display_name": "Workspace Alpha",
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:00Z"
        },
        "project": {
            "id": "project-slice1",
            "workspace_id": "workspace-alpha",
            "slug": "project-slice1",
            "display_name": "Project Slice 1",
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:00Z"
        }
    })));
    assert!(project_schema.is_valid(&json!({
        "id": "project-slice1",
        "workspace_id": "workspace-alpha",
        "slug": "project-slice1",
        "display_name": "Project Slice 1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(knowledge_space_schema.is_valid(&json!({
        "id": "knowledge-space-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "owner_ref": "workspace_admin:alice",
        "display_name": "Project Slice 1 Knowledge",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(task_create_command_schema.is_valid(&json!({
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
    })));
    assert!(task_schema.is_valid(&json!({
        "id": "task-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "source_kind": "manual",
        "automation_id": null,
        "title": "Write note",
        "instruction": "Emit a deterministic artifact",
        "action": {
            "kind": "emit_text",
            "content": "hello"
        },
        "capability_id": "capability-write-note",
        "estimated_cost": 1,
        "idempotency_key": "task-1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(task_schema.is_valid(&json!({
        "id": "task-connector-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "source_kind": "manual",
        "automation_id": null,
        "title": "Run connector tool",
        "instruction": "Invoke MCP tool through capability runtime",
        "action": {
            "kind": "connector_call",
            "tool_name": "emit_text",
            "arguments": {
                "content": "hello from connector"
            }
        },
        "capability_id": "capability-write-note-connector",
        "estimated_cost": 1,
        "idempotency_key": "task-connector-1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(run_schema.is_valid(&json!({
        "id": "run-1",
        "task_id": "task-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "automation_id": null,
        "trigger_delivery_id": null,
        "run_type": "task",
        "status": "completed",
        "approval_request_id": null,
        "idempotency_key": "run-task-1",
        "attempt_count": 1,
        "max_attempts": 2,
        "checkpoint_seq": 3,
        "resume_token": null,
        "last_error": null,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:01Z",
        "started_at": "2026-03-26T10:00:00Z",
        "completed_at": "2026-03-26T10:00:01Z",
        "terminated_at": null
    })));
    assert!(run_summary_schema.is_valid(&json!({
        "id": "run-1",
        "task_id": "task-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "title": "Write note",
        "run_type": "task",
        "status": "completed",
        "approval_request_id": null,
        "attempt_count": 1,
        "max_attempts": 2,
        "last_error": null,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:01Z",
        "started_at": "2026-03-26T10:00:00Z",
        "completed_at": "2026-03-26T10:00:01Z",
        "terminated_at": null
    })));
    assert!(automation_schema.is_valid(&json!({
        "id": "automation-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "trigger_id": "trigger-1",
        "title": "Automation note",
        "instruction": "Run from manual event",
        "action": {
            "kind": "emit_text",
            "content": "hello from automation"
        },
        "capability_id": "capability-write-note",
        "estimated_cost": 1,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(automation_schema.is_valid(&json!({
        "id": "automation-connector-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "trigger_id": "trigger-connector-1",
        "title": "Connector automation",
        "instruction": "Dispatch MCP-backed manual event automation",
        "action": {
            "kind": "connector_call",
            "tool_name": "emit_text",
            "arguments": {
                "content": "hello from connector automation"
            }
        },
        "capability_id": "capability-write-note-connector",
        "estimated_cost": 1,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(trigger_schema.is_valid(&json!({
        "id": "trigger-1",
        "automation_id": "automation-1",
        "trigger_type": "manual_event",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(trigger_delivery_schema.is_valid(&json!({
        "id": "delivery-1",
        "trigger_id": "trigger-1",
        "run_id": "run-automation-1",
        "status": "succeeded",
        "dedupe_key": "delivery:1",
        "payload": {
            "source": "test"
        },
        "attempt_count": 1,
        "last_error": null,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:01Z"
    })));
    assert!(environment_lease_status_schema.is_valid(&json!("requested")));
    assert!(environment_lease_status_schema.is_valid(&json!("granted")));
    assert!(environment_lease_status_schema.is_valid(&json!("active")));
    assert!(environment_lease_status_schema.is_valid(&json!("released")));
    assert!(environment_lease_status_schema.is_valid(&json!("expired")));
    assert!(environment_lease_status_schema.is_valid(&json!("revoked")));
    assert!(environment_lease_schema.is_valid(&json!({
        "id": "lease-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "capability_id": "capability-write-note-connector",
        "environment_type": "mcp_tool_call",
        "sandbox_tier": "ephemeral_restricted",
        "status": "active",
        "heartbeat_at": "2026-03-26T10:00:01Z",
        "expires_at": "2026-03-26T10:01:01Z",
        "resume_token": "lease:lease-1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:01Z"
    })));
    assert!(capability_descriptor_schema.is_valid(&json!({
        "id": "capability-write-note",
        "slug": "capability-write-note",
        "kind": "connector_backed",
        "source": "mcp_server",
        "platform": "desktop",
        "risk_level": "low",
        "requires_approval": false,
        "input_schema_uri": "https://octopus.local/schemas/runtime/task.schema.json",
        "output_schema_uri": "https://octopus.local/schemas/observe/artifact.schema.json",
        "fallback_capability_id": "capability-write-note-local",
        "trust_level": "external_untrusted",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(capability_binding_schema.is_valid(&json!({
        "id": "binding-1",
        "capability_id": "capability-write-note",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "scope_ref": "workspace:workspace-alpha/project:project-slice1",
        "binding_status": "active",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(capability_grant_schema.is_valid(&json!({
        "id": "grant-1",
        "capability_id": "capability-write-note",
        "subject_ref": "workspace:workspace-alpha/project:project-slice1",
        "grant_status": "active",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(budget_policy_schema.is_valid(&json!({
        "id": "budget-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "soft_cost_limit": 5,
        "hard_cost_limit": 10,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(approval_schema.is_valid(&json!({
        "id": "approval-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "approval_type": "execution",
        "status": "pending",
        "reason": "risk_level_high",
        "dedupe_key": "approval:run-1",
        "decided_by": null,
        "decision_note": null,
        "decided_at": null,
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(approval_resolve_command_schema.is_valid(&json!({
        "approval_id": "approval-1",
        "decision": "approve",
        "actor_ref": "reviewer-alpha",
        "note": "connector approved"
    })));
    assert!(capability_visibility_schema.is_valid(&json!({
        "descriptor": {
            "id": "capability-write-note",
            "slug": "capability-write-note",
            "kind": "connector_backed",
            "source": "mcp_server",
            "platform": "desktop",
            "risk_level": "low",
            "requires_approval": false,
            "input_schema_uri": "https://octopus.local/schemas/runtime/task.schema.json",
            "output_schema_uri": "https://octopus.local/schemas/observe/artifact.schema.json",
            "fallback_capability_id": "capability-write-note-local",
            "trust_level": "trusted",
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:00Z"
        },
        "scope_ref": "workspace:workspace-alpha/project:project-slice1",
        "visibility": "visible",
        "reason_code": "project_scope_grant_active",
        "explanation": "Capability is visible because the project scope has an active binding and grant."
    })));
    assert!(artifact_schema.is_valid(&json!({
        "id": "artifact-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "artifact_type": "execution_output",
        "content": "hello",
        "provenance_source": "mcp_connector",
        "source_descriptor_id": "capability-write-note",
        "source_invocation_id": "invocation-1",
        "trust_level": "external_untrusted",
        "knowledge_gate_status": "blocked_low_trust",
        "created_at": "2026-03-26T10:00:01Z",
        "updated_at": "2026-03-26T10:00:01Z"
    })));
    assert!(artifact_summary_schema.is_valid(&json!({
        "id": "artifact-1",
        "run_id": "run-1",
        "task_id": "task-1",
        "artifact_type": "execution_output",
        "provenance_source": "mcp_connector",
        "trust_level": "external_untrusted",
        "knowledge_gate_status": "blocked_low_trust",
        "created_at": "2026-03-26T10:00:01Z"
    })));
    assert!(audit_schema.is_valid(&json!({
        "id": "audit-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "event_type": "run_completed",
        "message": "Run completed successfully",
        "created_at": "2026-03-26T10:00:01Z"
    })));
    assert!(trace_schema.is_valid(&json!({
        "id": "trace-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "stage": "trigger_delivery",
        "attempt": 1,
        "message": "Trigger delivery started",
        "created_at": "2026-03-26T10:00:01Z"
    })));
    assert!(audit_schema.is_valid(&json!({
        "id": "audit-knowledge-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "event_type": "knowledge_candidate_created",
        "message": "Knowledge candidate captured from execution artifact",
        "created_at": "2026-03-26T10:00:02Z"
    })));
    assert!(trace_schema.is_valid(&json!({
        "id": "trace-knowledge-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "stage": "knowledge_capture",
        "attempt": 1,
        "message": "Knowledge candidate captured from execution artifact",
        "created_at": "2026-03-26T10:00:02Z"
    })));
    assert!(inbox_schema.is_valid(&json!({
        "id": "inbox-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "approval_request_id": "approval-1",
        "item_type": "approval_request",
        "status": "open",
        "dedupe_key": "inbox:approval-1",
        "title": "Approval required",
        "message": "A run needs approval before execution",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z",
        "resolved_at": null
    })));
    assert!(notification_schema.is_valid(&json!({
        "id": "notification-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "approval_request_id": "approval-1",
        "status": "delivered",
        "dedupe_key": "notification:approval-1",
        "title": "Approval required",
        "message": "A run is waiting for approval",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(policy_decision_schema.is_valid(&json!({
        "id": "decision-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "capability_id": "capability-write-note",
        "decision": "allow",
        "reason": "within_budget",
        "estimated_cost": 1,
        "approval_request_id": null,
        "created_at": "2026-03-26T10:00:00Z"
    })));
    assert!(knowledge_candidate_status_schema.is_valid(&json!("candidate")));
    assert!(knowledge_asset_status_schema.is_valid(&json!("verified_shared")));
    assert!(knowledge_candidate_schema.is_valid(&json!({
        "id": "candidate-1",
        "knowledge_space_id": "knowledge-space-1",
        "source_run_id": "run-1",
        "source_task_id": "task-1",
        "source_artifact_id": "artifact-1",
        "capability_id": "capability-write-note",
        "status": "candidate",
        "content": "hello",
        "provenance_source": "builtin",
        "source_trust_level": "trusted",
        "dedupe_key": "knowledge_candidate:artifact-1",
        "created_at": "2026-03-26T10:00:01Z",
        "updated_at": "2026-03-26T10:00:01Z"
    })));
    assert!(knowledge_asset_schema.is_valid(&json!({
        "id": "asset-1",
        "knowledge_space_id": "knowledge-space-1",
        "source_candidate_id": "candidate-1",
        "capability_id": "capability-write-note",
        "status": "verified_shared",
        "content": "hello",
        "trust_level": "verified",
        "created_at": "2026-03-26T10:00:02Z",
        "updated_at": "2026-03-26T10:00:02Z"
    })));
    assert!(knowledge_lineage_schema.is_valid(&json!({
        "id": "lineage-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "source_ref": "artifact:artifact-1",
        "target_ref": "knowledge_candidate:candidate-1",
        "relation_type": "derived_from",
        "created_at": "2026-03-26T10:00:01Z"
    })));
    assert!(knowledge_summary_schema.is_valid(&json!({
        "kind": "candidate",
        "id": "candidate-1",
        "knowledge_space_id": "knowledge-space-1",
        "capability_id": "capability-write-note",
        "status": "candidate",
        "source_run_id": "run-1",
        "source_artifact_id": "artifact-1",
        "source_candidate_id": null,
        "provenance_source": "builtin",
        "trust_level": "trusted",
        "created_at": "2026-03-26T10:00:01Z"
    })));
    assert!(knowledge_detail_schema.is_valid(&json!({
        "knowledge_space": {
            "id": "knowledge-space-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "owner_ref": "workspace_admin:alice",
            "display_name": "Project Slice 1 Knowledge",
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:00Z"
        },
        "candidates": [{
            "id": "candidate-1",
            "knowledge_space_id": "knowledge-space-1",
            "source_run_id": "run-1",
            "source_task_id": "task-1",
            "source_artifact_id": "artifact-1",
            "capability_id": "capability-write-note",
            "status": "candidate",
            "content": "hello",
            "provenance_source": "builtin",
            "source_trust_level": "trusted",
            "dedupe_key": "knowledge_candidate:artifact-1",
            "created_at": "2026-03-26T10:00:01Z",
            "updated_at": "2026-03-26T10:00:01Z"
        }],
        "assets": [{
            "id": "asset-1",
            "knowledge_space_id": "knowledge-space-1",
            "source_candidate_id": "candidate-1",
            "capability_id": "capability-write-note",
            "status": "verified_shared",
            "content": "hello",
            "trust_level": "verified",
            "created_at": "2026-03-26T10:00:02Z",
            "updated_at": "2026-03-26T10:00:02Z"
        }],
        "lineage": [{
            "id": "lineage-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "source_ref": "artifact:artifact-1",
            "target_ref": "knowledge_candidate:candidate-1",
            "relation_type": "derived_from",
            "created_at": "2026-03-26T10:00:01Z"
        }]
    })));
    assert!(knowledge_promote_command_schema.is_valid(&json!({
        "candidate_id": "candidate-1",
        "actor_ref": "workspace_admin:alice",
        "note": "Promote trusted note"
    })));
    assert!(hub_connection_status_schema.is_valid(&json!({
        "mode": "local",
        "state": "connected",
        "active_server_count": 1,
        "healthy_server_count": 1,
        "servers": [{
            "id": "server-capability-write-note",
            "capability_id": "capability-write-note",
            "namespace": "test.connector.success",
            "platform": "desktop",
            "trust_level": "trusted",
            "health_status": "healthy",
            "lease_ttl_seconds": 60,
            "last_checked_at": "2026-03-26T10:00:00Z"
        }],
        "last_refreshed_at": "2026-03-26T10:00:01Z"
    })));
    assert!(hub_event_schema.is_valid(&json!({
        "event_type": "hub.connection.updated",
        "sequence": 1,
        "occurred_at": "2026-03-26T10:00:01Z",
        "payload": {
            "mode": "local",
            "state": "connected",
            "active_server_count": 1,
            "healthy_server_count": 1,
            "servers": [{
                "id": "server-capability-write-note",
                "capability_id": "capability-write-note",
                "namespace": "test.connector.success",
                "platform": "desktop",
                "trust_level": "trusted",
                "health_status": "healthy",
                "lease_ttl_seconds": 60,
                "last_checked_at": "2026-03-26T10:00:00Z"
            }],
            "last_refreshed_at": "2026-03-26T10:00:01Z"
        }
    })));
    assert!(run_detail_schema.is_valid(&json!({
        "run": {
            "id": "run-1",
            "task_id": "task-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "automation_id": null,
            "trigger_delivery_id": null,
            "run_type": "task",
            "status": "completed",
            "approval_request_id": null,
            "idempotency_key": "run-task-1",
            "attempt_count": 1,
            "max_attempts": 2,
            "checkpoint_seq": 3,
            "resume_token": null,
            "last_error": null,
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:01Z",
            "started_at": "2026-03-26T10:00:00Z",
            "completed_at": "2026-03-26T10:00:01Z",
            "terminated_at": null
        },
        "task": {
            "id": "task-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "source_kind": "manual",
            "automation_id": null,
            "title": "Write note",
            "instruction": "Emit a deterministic artifact",
            "action": {
                "kind": "emit_text",
                "content": "hello"
            },
            "capability_id": "capability-write-note",
            "estimated_cost": 1,
            "idempotency_key": "task-1",
            "created_at": "2026-03-26T10:00:00Z",
            "updated_at": "2026-03-26T10:00:00Z"
        },
        "artifacts": [{
            "id": "artifact-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "artifact_type": "execution_output",
            "content": "hello",
            "provenance_source": "builtin",
            "source_descriptor_id": "capability-write-note",
            "source_invocation_id": null,
            "trust_level": "trusted",
            "knowledge_gate_status": "eligible",
            "created_at": "2026-03-26T10:00:01Z",
            "updated_at": "2026-03-26T10:00:01Z"
        }],
        "audits": [{
            "id": "audit-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "event_type": "run_completed",
            "message": "Run completed successfully",
            "created_at": "2026-03-26T10:00:01Z"
        }],
        "traces": [{
            "id": "trace-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "stage": "execution_action",
            "attempt": 1,
            "message": "Execution completed",
            "created_at": "2026-03-26T10:00:01Z"
        }],
        "approvals": [],
        "inbox_items": [],
        "notifications": [],
        "policy_decisions": [{
            "id": "decision-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "capability_id": "capability-write-note",
            "decision": "allow",
            "reason": "within_budget",
            "estimated_cost": 1,
            "approval_request_id": null,
            "created_at": "2026-03-26T10:00:00Z"
        }],
        "knowledge_candidates": [{
            "id": "candidate-1",
            "knowledge_space_id": "knowledge-space-1",
            "source_run_id": "run-1",
            "source_task_id": "task-1",
            "source_artifact_id": "artifact-1",
            "capability_id": "capability-write-note",
            "status": "candidate",
            "content": "hello",
            "provenance_source": "builtin",
            "source_trust_level": "trusted",
            "dedupe_key": "knowledge_candidate:artifact-1",
            "created_at": "2026-03-26T10:00:01Z",
            "updated_at": "2026-03-26T10:00:01Z"
        }],
        "knowledge_assets": [{
            "id": "asset-1",
            "knowledge_space_id": "knowledge-space-1",
            "source_candidate_id": "candidate-1",
            "capability_id": "capability-write-note",
            "status": "verified_shared",
            "content": "hello",
            "trust_level": "verified",
            "created_at": "2026-03-26T10:00:02Z",
            "updated_at": "2026-03-26T10:00:02Z"
        }],
        "knowledge_lineage": [{
            "id": "lineage-1",
            "workspace_id": "workspace-alpha",
            "project_id": "project-slice1",
            "run_id": "run-1",
            "task_id": "task-1",
            "source_ref": "artifact:artifact-1",
            "target_ref": "knowledge_candidate:candidate-1",
            "relation_type": "derived_from",
            "created_at": "2026-03-26T10:00:01Z"
        }]
    })));
}
