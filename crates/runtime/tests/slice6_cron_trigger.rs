use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateAutomationInput, CreateTriggerInput, Slice2Runtime, TriggerSpec,
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
    base.join("slice6-cron-runtime.sqlite")
}

fn cron_automation_input(
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

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

#[tokio::test]
async fn due_cron_trigger_fires_once_and_duplicate_tick_is_noop_after_schedule_advances() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-cron-due", "Cron Due").await;
    seed_governance(
        &runtime,
        "project-cron-due",
        "capability-cron-due",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-due",
                "cron due",
                "capability-cron-due",
                ExecutionAction::EmitText {
                    content: "cron artifact".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let first = runtime
        .tick_due_triggers("2026-03-27T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].automation.id, created.automation.id);
    assert_eq!(first[0].delivery.status.as_str(), "succeeded");

    let trigger = runtime
        .fetch_trigger(created.automation.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        trigger.spec,
        TriggerSpec::Cron { ref config }
            if parse_timestamp(config.next_fire_at.as_str())
                > parse_timestamp("2026-03-27T10:00:00Z")
    ));

    let second = runtime
        .tick_due_triggers("2026-03-27T10:00:00Z")
        .await
        .unwrap();
    assert!(second.is_empty());
}

#[tokio::test]
async fn overdue_cron_trigger_catches_up_one_window_per_tick() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-cron-catchup", "Cron Catchup").await;
    seed_governance(
        &runtime,
        "project-cron-catchup",
        "capability-cron-catchup",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-catchup",
                "cron catchup",
                "capability-cron-catchup",
                ExecutionAction::EmitText {
                    content: "catchup artifact".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let first = runtime
        .tick_due_triggers("2026-03-27T10:05:00Z")
        .await
        .unwrap();
    assert_eq!(first.len(), 1);

    let trigger_after_first = runtime
        .fetch_trigger(created.automation.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        trigger_after_first.spec,
        TriggerSpec::Cron { ref config }
            if parse_timestamp(config.next_fire_at.as_str())
                == parse_timestamp("2026-03-27T10:01:00+00:00")
    ));

    let second = runtime
        .tick_due_triggers("2026-03-27T10:05:00Z")
        .await
        .unwrap();
    assert_eq!(second.len(), 1);
    assert_ne!(first[0].delivery.id, second[0].delivery.id);
}

#[tokio::test]
async fn due_cron_trigger_is_recovered_after_runtime_reopen() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-cron-reopen", "Cron Reopen").await;
    seed_governance(
        &runtime,
        "project-cron-reopen",
        "capability-cron-reopen",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-reopen",
                "cron reopen",
                "capability-cron-reopen",
                ExecutionAction::EmitText {
                    content: "reopen artifact".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();
    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let reports = reopened
        .tick_due_triggers("2026-03-27T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].automation.id, created.automation.id);
    assert_eq!(reports[0].delivery.status.as_str(), "succeeded");
}

#[tokio::test]
async fn high_risk_cron_trigger_waits_for_approval_and_then_completes() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-cron-approval", "Cron Approval").await;
    seed_governance(
        &runtime,
        "project-cron-approval",
        "capability-cron-approval",
        "high",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-approval",
                "cron approval",
                "capability-cron-approval",
                ExecutionAction::EmitText {
                    content: "approval artifact".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let waiting = runtime
        .tick_due_triggers("2026-03-27T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].delivery.status.as_str(), "delivering");
    assert_eq!(
        waiting[0].run_report.run.status.as_str(),
        "waiting_approval"
    );

    runtime
        .resolve_approval(
            waiting[0].run_report.approvals[0].id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "approved cron trigger",
        )
        .await
        .unwrap();

    let deliveries = runtime
        .list_trigger_deliveries_by_automation(created.automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status.as_str(), "succeeded");
}

#[tokio::test]
async fn cron_trigger_propagates_policy_denials_and_retryable_failures() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-cron-deny", "Cron Deny").await;
    seed_governance(
        &runtime,
        "project-cron-deny",
        "capability-cron-deny",
        "low",
        0,
        0,
    )
    .await;

    let _denied = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-deny",
                "cron deny",
                "capability-cron-deny",
                ExecutionAction::EmitText {
                    content: "denied".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let denied_reports = runtime
        .tick_due_triggers("2026-03-27T10:00:00Z")
        .await
        .unwrap();
    assert_eq!(denied_reports.len(), 1);
    assert_eq!(denied_reports[0].delivery.status.as_str(), "failed");
    assert_eq!(denied_reports[0].run_report.run.status.as_str(), "blocked");

    seed_context(&runtime, "project-cron-retry", "Cron Retry").await;
    seed_governance(
        &runtime,
        "project-cron-retry",
        "capability-cron-retry",
        "low",
        5,
        10,
    )
    .await;

    let retryable = runtime
        .create_automation_with_trigger(
            cron_automation_input(
                "project-cron-retry",
                "cron retry",
                "capability-cron-retry",
                ExecutionAction::FailOnceThenEmitText {
                    failure_message: "flaky_network".into(),
                    content: "retry artifact".into(),
                },
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T11:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let failed = runtime
        .tick_due_triggers("2026-03-27T11:00:00Z")
        .await
        .unwrap();
    let failed_retry = failed
        .iter()
        .find(|report| report.automation.id == retryable.automation.id)
        .unwrap();
    assert_eq!(failed_retry.delivery.status.as_str(), "failed");
    assert_eq!(failed_retry.run_report.run.status.as_str(), "failed");

    let recovered = runtime
        .retry_trigger_delivery(failed_retry.delivery.id.as_str())
        .await
        .unwrap();
    assert_eq!(recovered.automation.id, retryable.automation.id);
    assert_eq!(recovered.delivery.status.as_str(), "succeeded");
    assert_eq!(recovered.run_report.run.status.as_str(), "completed");
}
