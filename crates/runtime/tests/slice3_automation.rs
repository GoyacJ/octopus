use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, CreateAutomationInput, DispatchManualEventInput, Slice2Runtime,
};
use serde_json::json;

use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
};

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

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("slice3-runtime.sqlite")
}

#[tokio::test]
async fn manual_event_automation_creates_deduped_delivery_task_and_automation_run() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-automation-allow", "Automation Allow").await;
    seed_governance(
        &runtime,
        "project-automation-allow",
        "capability-automation-allow",
        "low",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-allow".into(),
            title: "Automation allow".into(),
            instruction: "Dispatch manual event".into(),
            action: ExecutionAction::EmitText {
                content: "automation artifact".into(),
            },
            capability_id: "capability-automation-allow".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let report = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-allow-1".into(),
            payload: json!({
                "source": "test"
            }),
        })
        .await
        .unwrap();

    assert_eq!(report.automation.id, automation.id);
    assert_eq!(report.trigger.id, automation.trigger_id);
    assert_eq!(report.delivery.status.as_str(), "succeeded");
    assert_eq!(report.delivery.attempt_count, 1);
    assert_eq!(report.task.source_kind.as_str(), "automation");
    assert_eq!(
        report.task.automation_id.as_deref(),
        Some(automation.id.as_str())
    );
    assert_eq!(report.run_report.run.run_type.as_str(), "automation");
    assert_eq!(
        report.run_report.run.trigger_delivery_id.as_deref(),
        Some(report.delivery.id.as_str())
    );
    assert_eq!(
        report.run_report.run.automation_id.as_deref(),
        Some(automation.id.as_str())
    );
    assert_eq!(report.run_report.artifacts.len(), 1);
    assert!(report
        .run_report
        .traces
        .iter()
        .any(|trace| trace.stage.as_str() == "trigger_delivery"));

    let deliveries = runtime
        .list_trigger_deliveries_by_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
}

#[tokio::test]
async fn duplicate_delivery_key_reuses_existing_delivery_task_and_run() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-automation-dedupe", "Automation Dedupe").await;
    seed_governance(
        &runtime,
        "project-automation-dedupe",
        "capability-automation-dedupe",
        "low",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-dedupe".into(),
            title: "Automation dedupe".into(),
            instruction: "Dispatch manual event".into(),
            action: ExecutionAction::EmitText {
                content: "deduped artifact".into(),
            },
            capability_id: "capability-automation-dedupe".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let first = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-dedupe-1".into(),
            payload: json!({"source": "first"}),
        })
        .await
        .unwrap();
    let second = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-dedupe-1".into(),
            payload: json!({"source": "second"}),
        })
        .await
        .unwrap();

    assert_eq!(first.delivery.id, second.delivery.id);
    assert_eq!(first.task.id, second.task.id);
    assert_eq!(first.run_report.run.id, second.run_report.run.id);
    assert_eq!(second.run_report.artifacts.len(), 1);

    let deliveries = runtime
        .list_trigger_deliveries_by_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
}

#[tokio::test]
async fn high_risk_automation_waits_for_approval_and_marks_delivery_succeeded_after_approval() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(
        &runtime,
        "project-automation-approval",
        "Automation Approval",
    )
    .await;
    seed_governance(
        &runtime,
        "project-automation-approval",
        "capability-automation-approval",
        "high",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-approval".into(),
            title: "Automation approval".into(),
            instruction: "Dispatch manual event".into(),
            action: ExecutionAction::EmitText {
                content: "approval artifact".into(),
            },
            capability_id: "capability-automation-approval".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let waiting = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-approval-1".into(),
            payload: json!({"source": "approval"}),
        })
        .await
        .unwrap();

    assert_eq!(waiting.delivery.status.as_str(), "delivering");
    assert_eq!(waiting.run_report.run.status.as_str(), "waiting_approval");

    runtime
        .resolve_approval(
            waiting.run_report.approvals[0].id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "approved",
        )
        .await
        .unwrap();

    let deliveries = runtime
        .list_trigger_deliveries_by_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status.as_str(), "succeeded");
}

#[tokio::test]
async fn rejected_or_denied_automation_delivery_finishes_failed_without_artifact() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-automation-reject", "Automation Reject").await;
    seed_governance(
        &runtime,
        "project-automation-reject",
        "capability-automation-reject",
        "high",
        5,
        10,
    )
    .await;
    let reject_automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-reject".into(),
            title: "Automation reject".into(),
            instruction: "Reject approval".into(),
            action: ExecutionAction::EmitText {
                content: "should not run".into(),
            },
            capability_id: "capability-automation-reject".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();
    let waiting = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: reject_automation.trigger_id.clone(),
            dedupe_key: "delivery-reject-1".into(),
            payload: json!({"source": "reject"}),
        })
        .await
        .unwrap();
    let rejected = runtime
        .resolve_approval(
            waiting.run_report.approvals[0].id.as_str(),
            ApprovalDecision::Reject,
            "reviewer-alpha",
            "rejected",
        )
        .await
        .unwrap();
    assert!(rejected.artifacts.is_empty());

    let rejected_deliveries = runtime
        .list_trigger_deliveries_by_automation(reject_automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(rejected_deliveries[0].status.as_str(), "failed");

    seed_context(&runtime, "project-automation-deny", "Automation Deny").await;
    seed_governance(
        &runtime,
        "project-automation-deny",
        "capability-automation-deny",
        "low",
        0,
        0,
    )
    .await;
    let deny_automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-deny".into(),
            title: "Automation deny".into(),
            instruction: "Deny by policy".into(),
            action: ExecutionAction::EmitText {
                content: "should not run".into(),
            },
            capability_id: "capability-automation-deny".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();
    let denied = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: deny_automation.trigger_id.clone(),
            dedupe_key: "delivery-deny-1".into(),
            payload: json!({"source": "deny"}),
        })
        .await
        .unwrap();

    assert_eq!(denied.delivery.status.as_str(), "failed");
    assert_eq!(denied.run_report.run.status.as_str(), "blocked");
    assert!(denied.run_report.artifacts.is_empty());
}

#[tokio::test]
async fn failed_delivery_can_retry_without_creating_second_delivery_record() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-automation-retry", "Automation Retry").await;
    seed_governance(
        &runtime,
        "project-automation-retry",
        "capability-automation-retry",
        "low",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-retry".into(),
            title: "Automation retry".into(),
            instruction: "Retry after failure".into(),
            action: ExecutionAction::FailOnceThenEmitText {
                failure_message: "network_glitch".into(),
                content: "recovered".into(),
            },
            capability_id: "capability-automation-retry".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let failed = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-retry-1".into(),
            payload: json!({"source": "retry"}),
        })
        .await
        .unwrap();

    assert_eq!(failed.delivery.status.as_str(), "failed");
    assert_eq!(failed.run_report.run.status.as_str(), "failed");

    let recovered = runtime
        .retry_trigger_delivery(failed.delivery.id.as_str())
        .await
        .unwrap();

    assert_eq!(recovered.delivery.id, failed.delivery.id);
    assert_eq!(recovered.delivery.status.as_str(), "succeeded");
    assert_eq!(recovered.delivery.attempt_count, 2);
    assert_eq!(recovered.run_report.run.status.as_str(), "completed");
    assert_eq!(recovered.run_report.run.attempt_count, 2);
    assert_eq!(recovered.run_report.artifacts.len(), 1);

    let deliveries = runtime
        .list_trigger_deliveries_by_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
}

#[tokio::test]
async fn pending_delivery_survives_reopen_and_duplicate_dispatch() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-automation-reopen", "Automation Reopen").await;
    seed_governance(
        &runtime,
        "project-automation-reopen",
        "capability-automation-reopen",
        "high",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-reopen".into(),
            title: "Automation reopen".into(),
            instruction: "Wait for approval and reopen".into(),
            action: ExecutionAction::EmitText {
                content: "pending".into(),
            },
            capability_id: "capability-automation-reopen".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let waiting = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-reopen-1".into(),
            payload: json!({"source": "reopen"}),
        })
        .await
        .unwrap();
    assert_eq!(waiting.delivery.status.as_str(), "delivering");

    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let deliveries = reopened
        .list_trigger_deliveries_by_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status.as_str(), "delivering");

    let deduped = reopened
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-reopen-1".into(),
            payload: json!({"source": "reopen-again"}),
        })
        .await
        .unwrap();

    assert_eq!(deduped.delivery.id, waiting.delivery.id);
    assert_eq!(deduped.run_report.run.id, waiting.run_report.run.id);
    assert_eq!(deduped.run_report.run.status.as_str(), "waiting_approval");
}
