use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateAutomationInput, CreateTaskInput, DispatchManualEventInput,
    McpServerRecord, Slice2Runtime,
};
use serde_json::json;

async fn seed_context(runtime: &Slice2Runtime, project_id: &str, project_name: &str) {
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            project_id,
            project_id,
            project_name,
        )
        .await
        .unwrap();
}

async fn seed_connector_governance(
    runtime: &Slice2Runtime,
    project_id: &str,
    capability_id: &str,
    risk_level: &str,
    soft_limit: i64,
    hard_limit: i64,
    trust_level: &str,
) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new_connector_backed(
            capability_id,
            capability_id,
            "desktop",
            "mcp_server",
            risk_level,
            risk_level == "high",
            trust_level,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{capability_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{capability_id}"),
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
            soft_limit,
            hard_limit,
        ))
        .await
        .unwrap();
}

async fn seed_mcp_server(
    runtime: &Slice2Runtime,
    capability_id: &str,
    namespace: &str,
    trust_level: &str,
) {
    runtime
        .upsert_mcp_server(McpServerRecord::new_fake(
            format!("server-{capability_id}"),
            capability_id,
            namespace,
            "desktop",
            trust_level,
            60,
        ))
        .await
        .unwrap();
}

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("slice5-runtime.sqlite")
}

#[tokio::test]
async fn connector_backed_task_executes_and_persists_invocation_artifact_and_released_lease() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-connector-success", "Connector Success").await;
    seed_connector_governance(
        &runtime,
        "project-connector-success",
        "capability-connector-success",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_mcp_server(
        &runtime,
        "capability-connector-success",
        "test.connector.success",
        "trusted",
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-connector-success".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Connector success".into(),
            instruction: "Call connector-backed capability".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "connector artifact"
                }),
            },
            capability_id: "capability-connector-success".into(),
            estimated_cost: 1,
            idempotency_key: "task-connector-success-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.artifacts[0].content.as_str(), "connector artifact");
    assert_eq!(
        report.artifacts[0].provenance_source.as_str(),
        "mcp_connector"
    );
    assert_eq!(
        report.artifacts[0].knowledge_gate_status.as_str(),
        "eligible"
    );

    let visible = runtime
        .list_capability_resolutions("workspace-alpha", "project-connector-success", 1)
        .await
        .unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(
        visible[0].descriptor.id.as_str(),
        "capability-connector-success"
    );
    assert_eq!(visible[0].execution_state.as_str(), "executable");

    let servers = runtime.list_mcp_servers().await.unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(
        servers[0].capability_id.as_str(),
        "capability-connector-success"
    );

    let invocations = runtime
        .list_mcp_invocations_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].status.as_str(), "succeeded");
    assert_eq!(invocations[0].gate_status.as_str(), "eligible");

    let leases = runtime
        .list_environment_leases_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].status.as_str(), "released");
}

#[tokio::test]
async fn connector_backed_manual_event_automation_waits_for_approval_and_resumes_once_approved() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-connector-approval", "Connector Approval").await;
    seed_connector_governance(
        &runtime,
        "project-connector-approval",
        "capability-connector-approval",
        "high",
        5,
        10,
        "trusted",
    )
    .await;
    seed_mcp_server(
        &runtime,
        "capability-connector-approval",
        "test.connector.approval",
        "trusted",
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-connector-approval".into(),
            title: "Connector approval".into(),
            instruction: "Manual event to connector-backed action".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "approval connector artifact"
                }),
            },
            capability_id: "capability-connector-approval".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let waiting = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-connector-approval-1".into(),
            payload: json!({"source": "slice5"}),
        })
        .await
        .unwrap();

    assert_eq!(waiting.run_report.run.status.as_str(), "waiting_approval");
    assert_eq!(waiting.delivery.status.as_str(), "delivering");
    assert!(runtime
        .list_mcp_invocations_by_run(waiting.run_report.run.id.as_str())
        .await
        .unwrap()
        .is_empty());

    let report = runtime
        .resolve_approval(
            waiting.run_report.approvals[0].id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "connector approved",
        )
        .await
        .unwrap();

    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(
        runtime
            .list_mcp_invocations_by_run(report.run.id.as_str())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn retryable_connector_failure_normalizes_into_run_retry_and_then_succeeds() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-connector-retry", "Connector Retry").await;
    seed_connector_governance(
        &runtime,
        "project-connector-retry",
        "capability-connector-retry",
        "low",
        5,
        10,
        "trusted",
    )
    .await;
    seed_mcp_server(
        &runtime,
        "capability-connector-retry",
        "test.connector.retry",
        "trusted",
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-connector-retry".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Connector retry".into(),
            instruction: "Fail once and recover".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "fail_once_then_emit_text".into(),
                arguments: json!({
                    "failure_message": "connector timeout",
                    "content": "recovered connector artifact"
                }),
            },
            capability_id: "capability-connector-retry".into(),
            estimated_cost: 1,
            idempotency_key: "task-connector-retry-1".into(),
        })
        .await
        .unwrap();

    let failed = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(failed.run.status.as_str(), "failed");
    assert!(failed.run.resume_token.is_some());
    assert!(failed.artifacts.is_empty());

    let failed_invocations = runtime
        .list_mcp_invocations_by_run(failed.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(failed_invocations.len(), 1);
    assert_eq!(failed_invocations[0].status.as_str(), "failed");
    assert!(failed_invocations[0].retryable);

    let recovered = runtime.retry_run(failed.run.id.as_str()).await.unwrap();
    assert_eq!(recovered.run.status.as_str(), "completed");
    assert_eq!(recovered.artifacts.len(), 1);
    assert_eq!(
        runtime
            .list_mcp_invocations_by_run(recovered.run.id.as_str())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn environment_lease_heartbeat_and_release_round_trip_through_runtime_api() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-lease-api", "Lease API").await;
    seed_connector_governance(
        &runtime,
        "project-lease-api",
        "capability-lease-api",
        "low",
        5,
        10,
        "trusted",
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-lease-api".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Lease request".into(),
            instruction: "Create run to anchor lease API".into(),
            action: ExecutionAction::EmitText {
                content: "lease anchor".into(),
            },
            capability_id: "capability-lease-api".into(),
            estimated_cost: 1,
            idempotency_key: "task-lease-api-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();

    let lease = runtime
        .request_environment_lease(
            report.run.id.as_str(),
            task.id.as_str(),
            "capability-lease-api",
            "mcp_tool_call",
            "ephemeral_restricted",
            30,
        )
        .await
        .unwrap();
    assert_eq!(lease.status.as_str(), "active");

    let heartbeat = runtime
        .heartbeat_environment_lease(lease.id.as_str(), 45)
        .await
        .unwrap();
    assert_eq!(heartbeat.status.as_str(), "active");
    assert!(heartbeat.expires_at >= heartbeat.heartbeat_at);

    let released = runtime
        .release_environment_lease(lease.id.as_str())
        .await
        .unwrap();
    assert_eq!(released.status.as_str(), "released");

    let leases = runtime
        .list_environment_leases_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].status.as_str(), "released");
}

#[tokio::test]
async fn stale_active_environment_lease_expires_when_runtime_reopens() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-lease-expiry", "Lease Expiry").await;
    seed_connector_governance(
        &runtime,
        "project-lease-expiry",
        "capability-lease-expiry",
        "low",
        5,
        10,
        "trusted",
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-lease-expiry".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Lease expiry".into(),
            instruction: "Create stale lease".into(),
            action: ExecutionAction::EmitText {
                content: "expiry anchor".into(),
            },
            capability_id: "capability-lease-expiry".into(),
            estimated_cost: 1,
            idempotency_key: "task-lease-expiry-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    let lease = runtime
        .request_environment_lease(
            report.run.id.as_str(),
            task.id.as_str(),
            "capability-lease-expiry",
            "mcp_tool_call",
            "ephemeral_restricted",
            30,
        )
        .await
        .unwrap();

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db_path)
        .foreign_keys(true);
    let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
    sqlx::query(
        r#"
        UPDATE environment_leases
        SET status = 'active', expires_at = '2026-03-26T10:00:00Z'
        WHERE id = ?1
        "#,
    )
    .bind(&lease.id)
    .execute(&pool)
    .await
    .unwrap();
    drop(pool);
    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let leases = reopened
        .list_environment_leases_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].status.as_str(), "expired");
}

#[tokio::test]
async fn low_trust_connector_output_persists_artifact_but_blocks_knowledge_candidate_capture() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(
        &runtime,
        "project-connector-low-trust",
        "Connector Low Trust",
    )
    .await;
    seed_connector_governance(
        &runtime,
        "project-connector-low-trust",
        "capability-connector-low-trust",
        "low",
        5,
        10,
        "external_untrusted",
    )
    .await;
    seed_mcp_server(
        &runtime,
        "capability-connector-low-trust",
        "test.connector.low-trust",
        "external_untrusted",
    )
    .await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-connector-low-trust",
            "Low Trust Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-connector-low-trust".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Low trust connector".into(),
            instruction: "Persist artifact but gate knowledge".into(),
            action: ExecutionAction::ConnectorCall {
                tool_name: "emit_text".into(),
                arguments: json!({
                    "content": "external low trust artifact"
                }),
            },
            capability_id: "capability-connector-low-trust".into(),
            estimated_cost: 1,
            idempotency_key: "task-connector-low-trust-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(
        report.artifacts[0].trust_level.as_str(),
        "external_untrusted"
    );
    assert_eq!(
        report.artifacts[0].knowledge_gate_status.as_str(),
        "blocked_low_trust"
    );
    assert!(report.knowledge_candidates.is_empty());

    let invocations = runtime
        .list_mcp_invocations_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].gate_status.as_str(), "blocked_low_trust");
}
