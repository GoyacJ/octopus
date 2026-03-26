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

    assert!(count >= 28);
}

#[test]
fn refined_slice1_examples_validate() {
    let workspace_schema = compiled_schema("context/workspace.schema.json");
    let project_schema = compiled_schema("context/project.schema.json");
    let task_schema = compiled_schema("runtime/task.schema.json");
    let run_schema = compiled_schema("runtime/run.schema.json");
    let approval_schema = compiled_schema("governance/approval-request.schema.json");
    let capability_descriptor_schema =
        compiled_schema("governance/capability-descriptor.schema.json");
    let capability_binding_schema = compiled_schema("governance/capability-binding.schema.json");
    let capability_grant_schema = compiled_schema("governance/capability-grant.schema.json");
    let budget_policy_schema = compiled_schema("governance/budget-policy.schema.json");
    let artifact_schema = compiled_schema("observe/artifact.schema.json");
    let audit_schema = compiled_schema("observe/audit-record.schema.json");
    let trace_schema = compiled_schema("observe/trace-record.schema.json");
    let inbox_schema = compiled_schema("observe/inbox-item.schema.json");
    let notification_schema = compiled_schema("observe/notification.schema.json");
    let policy_decision_schema = compiled_schema("observe/policy-decision-log.schema.json");

    assert!(workspace_schema.is_valid(&json!({
        "id": "workspace-alpha",
        "slug": "workspace-alpha",
        "display_name": "Workspace Alpha",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(project_schema.is_valid(&json!({
        "id": "project-slice1",
        "workspace_id": "workspace-alpha",
        "slug": "project-slice1",
        "display_name": "Project Slice 1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(task_schema.is_valid(&json!({
        "id": "task-1",
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
        "idempotency_key": "task-1",
        "created_at": "2026-03-26T10:00:00Z",
        "updated_at": "2026-03-26T10:00:00Z"
    })));
    assert!(run_schema.is_valid(&json!({
        "id": "run-1",
        "task_id": "task-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
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
    assert!(capability_descriptor_schema.is_valid(&json!({
        "id": "capability-write-note",
        "slug": "capability-write-note",
        "risk_level": "low",
        "requires_approval": false,
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
    assert!(artifact_schema.is_valid(&json!({
        "id": "artifact-1",
        "workspace_id": "workspace-alpha",
        "project_id": "project-slice1",
        "run_id": "run-1",
        "task_id": "task-1",
        "artifact_type": "execution_output",
        "content": "hello",
        "created_at": "2026-03-26T10:00:01Z",
        "updated_at": "2026-03-26T10:00:01Z"
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
        "stage": "execution_action",
        "attempt": 1,
        "message": "Execution action succeeded",
        "created_at": "2026-03-26T10:00:01Z"
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
}
