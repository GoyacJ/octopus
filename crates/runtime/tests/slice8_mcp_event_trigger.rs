use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateAutomationInput, CreateTriggerInput, DispatchMcpEventInput, McpServerRecord,
    RuntimeError, Slice2Runtime,
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

async fn seed_governance(
    runtime: &Slice2Runtime,
    project_id: &str,
    capability_id: &str,
    risk_level: &str,
    soft_limit: i64,
    hard_limit: i64,
) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            capability_id,
            capability_id,
            risk_level,
            risk_level == "high",
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
    server_id: &str,
    capability_id: &str,
    namespace: &str,
    trust_level: &str,
) {
    runtime
        .upsert_mcp_server(McpServerRecord::new_fake(
            server_id,
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
    base.join("slice8-mcp-event-runtime.sqlite")
}

fn automation_input(
    project_id: &str,
    title: &str,
    capability_id: &str,
    action: ExecutionAction,
) -> CreateAutomationInput {
    CreateAutomationInput {
        workspace_id: "workspace-alpha".into(),
        project_id: project_id.into(),
        title: title.into(),
        instruction: format!("Run {title}"),
        action,
        capability_id: capability_id.into(),
        estimated_cost: 1,
    }
}

#[tokio::test]
async fn known_mcp_event_server_dispatches_and_duplicate_dedupe_key_reuses_delivery() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-mcp-happy", "MCP Happy").await;
    seed_governance(
        &runtime,
        "project-mcp-happy",
        "capability-mcp-happy",
        "low",
        5,
        10,
    )
    .await;
    seed_mcp_server(
        &runtime,
        "server-mcp-happy",
        "capability-mcp-happy",
        "test.mcp.happy",
        "trusted",
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-mcp-happy",
                "mcp happy",
                "capability-mcp-happy",
                ExecutionAction::EmitText {
                    content: "mcp event artifact".into(),
                },
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-mcp-happy".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();

    let first = runtime
        .dispatch_mcp_event(DispatchMcpEventInput {
            trigger_id: created.automation.trigger_id.clone(),
            server_id: "server-mcp-happy".into(),
            event_name: "connector.output.ready".into(),
            dedupe_key: "event-1".into(),
            payload: json!({"kind": "ready"}),
        })
        .await
        .unwrap();
    assert_eq!(first.delivery.status.as_str(), "succeeded");

    let second = runtime
        .dispatch_mcp_event(DispatchMcpEventInput {
            trigger_id: created.automation.trigger_id.clone(),
            server_id: "server-mcp-happy".into(),
            event_name: "connector.output.ready".into(),
            dedupe_key: "event-1".into(),
            payload: json!({"kind": "ready-duplicate"}),
        })
        .await
        .unwrap();
    assert_eq!(first.delivery.id, second.delivery.id);
    assert_eq!(first.run_report.run.id, second.run_report.run.id);
}

#[tokio::test]
async fn unknown_mcp_event_server_is_rejected() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-mcp-missing", "MCP Missing").await;
    seed_governance(
        &runtime,
        "project-mcp-missing",
        "capability-mcp-missing",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-mcp-missing",
                "mcp missing",
                "capability-mcp-missing",
                ExecutionAction::EmitText {
                    content: "should not run".into(),
                },
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-missing".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();

    let error = runtime
        .dispatch_mcp_event(DispatchMcpEventInput {
            trigger_id: created.automation.trigger_id,
            server_id: "server-missing".into(),
            event_name: "connector.output.ready".into(),
            dedupe_key: "event-1".into(),
            payload: json!({}),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::McpServerNotFound(_)));
}

#[tokio::test]
async fn mcp_event_selector_mismatch_is_rejected() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-mcp-mismatch", "MCP Mismatch").await;
    seed_governance(
        &runtime,
        "project-mcp-mismatch",
        "capability-mcp-mismatch",
        "low",
        5,
        10,
    )
    .await;
    seed_mcp_server(
        &runtime,
        "server-mcp-mismatch",
        "capability-mcp-mismatch",
        "test.mcp.mismatch",
        "trusted",
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-mcp-mismatch",
                "mcp mismatch",
                "capability-mcp-mismatch",
                ExecutionAction::EmitText {
                    content: "should not run".into(),
                },
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-mcp-mismatch".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();

    let error = runtime
        .dispatch_mcp_event(DispatchMcpEventInput {
            trigger_id: created.automation.trigger_id,
            server_id: "server-mcp-mismatch".into(),
            event_name: "connector.output.failed".into(),
            dedupe_key: "event-1".into(),
            payload: json!({}),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::McpEventMismatch { .. }));
}

#[tokio::test]
async fn mcp_event_trigger_preserves_low_trust_artifact_gate() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-mcp-low-trust", "MCP Low Trust Event").await;
    seed_connector_governance(
        &runtime,
        "project-mcp-low-trust",
        "capability-mcp-low-trust",
        "low",
        5,
        10,
        "external_untrusted",
    )
    .await;
    seed_mcp_server(
        &runtime,
        "server-mcp-low-trust",
        "capability-mcp-low-trust",
        "test.mcp.low-trust",
        "external_untrusted",
    )
    .await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-mcp-low-trust",
            "MCP Event Low Trust Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let created = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-mcp-low-trust",
                "mcp low trust",
                "capability-mcp-low-trust",
                ExecutionAction::ConnectorCall {
                    tool_name: "emit_text".into(),
                    arguments: json!({
                        "content": "external low trust artifact"
                    }),
                },
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-mcp-low-trust".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();

    let report = runtime
        .dispatch_mcp_event(DispatchMcpEventInput {
            trigger_id: created.automation.trigger_id,
            server_id: "server-mcp-low-trust".into(),
            event_name: "connector.output.ready".into(),
            dedupe_key: "event-low-trust-1".into(),
            payload: json!({"kind": "ready"}),
        })
        .await
        .unwrap();

    assert_eq!(report.delivery.status.as_str(), "succeeded");
    assert_eq!(report.run_report.run.status.as_str(), "completed");
    assert_eq!(report.run_report.artifacts.len(), 1);
    assert_eq!(
        report.run_report.artifacts[0].trust_level.as_str(),
        "external_untrusted"
    );
    assert_eq!(
        report.run_report.artifacts[0]
            .knowledge_gate_status
            .as_str(),
        "blocked_low_trust"
    );
    assert!(report.run_report.knowledge_candidates.is_empty());
}
