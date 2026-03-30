use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateAutomationInput, CreateTriggerInput, DispatchManualEventInput, RuntimeError,
    Slice2Runtime, TriggerSpec,
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

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("ga-minimum-automation-surface.sqlite")
}

fn automation_input(project_id: &str, title: &str, capability_id: &str) -> CreateAutomationInput {
    CreateAutomationInput {
        workspace_id: "workspace-alpha".into(),
        project_id: project_id.into(),
        title: title.into(),
        instruction: format!("Run {title}"),
        action: ExecutionAction::EmitText {
            content: format!("artifact:{title}"),
        },
        capability_id: capability_id.into(),
        estimated_cost: 1,
    }
}

#[tokio::test]
async fn automation_list_and_detail_cover_all_supported_trigger_types() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-automation-surface", "Automation Surface").await;
    seed_governance(
        &runtime,
        "project-automation-surface",
        "capability-automation-surface",
        "low",
        5,
        10,
    )
    .await;

    let manual = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-automation-surface",
                "manual automation",
                "capability-automation-surface",
            ),
            CreateTriggerInput::ManualEvent,
        )
        .await
        .unwrap();
    let cron = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-automation-surface",
                "cron automation",
                "capability-automation-surface",
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();
    let webhook = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-automation-surface",
                "webhook automation",
                "capability-automation-surface",
            ),
            CreateTriggerInput::Webhook {
                ingress_mode: "shared_secret_header".into(),
                secret_header_name: "X-Octopus-Trigger-Secret".into(),
                secret_hint: Some("hook".into()),
                secret_plaintext: None,
            },
        )
        .await
        .unwrap();
    let mcp_event = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-automation-surface",
                "mcp automation",
                "capability-automation-surface",
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-automation-surface".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();

    assert!(webhook.webhook_secret.is_some());

    let summaries = runtime
        .list_automations("workspace-alpha", "project-automation-surface")
        .await
        .unwrap();
    assert_eq!(summaries.len(), 4);
    assert!(summaries
        .iter()
        .all(|summary| summary.automation.status == "active"));
    assert!(summaries
        .iter()
        .all(|summary| summary.recent_deliveries.is_empty()));
    assert!(summaries
        .iter()
        .all(|summary| summary.last_run_summary.is_none()));
    assert!(summaries
        .iter()
        .any(|summary| matches!(summary.trigger.spec, TriggerSpec::ManualEvent { .. })));
    assert!(summaries
        .iter()
        .any(|summary| matches!(summary.trigger.spec, TriggerSpec::Cron { .. })));
    assert!(summaries
        .iter()
        .any(|summary| matches!(summary.trigger.spec, TriggerSpec::Webhook { .. })));
    assert!(summaries
        .iter()
        .any(|summary| matches!(summary.trigger.spec, TriggerSpec::McpEvent { .. })));

    let webhook_detail = runtime
        .load_automation_detail(webhook.automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(webhook_detail.automation.id, webhook.automation.id);
    assert_eq!(webhook_detail.automation.status, "active");
    assert!(matches!(
        webhook_detail.trigger.spec,
        TriggerSpec::Webhook { ref config }
            if config.secret_present
                && config.secret_header_name == "X-Octopus-Trigger-Secret"
                && config.secret_hint.as_deref() == Some("hook")
    ));
    assert!(webhook_detail.recent_deliveries.is_empty());
    assert!(webhook_detail.last_run_summary.is_none());

    let manual_detail = runtime
        .load_automation_detail(manual.automation.id.as_str())
        .await
        .unwrap();
    assert!(matches!(
        manual_detail.trigger.spec,
        TriggerSpec::ManualEvent { .. }
    ));
    let cron_detail = runtime
        .load_automation_detail(cron.automation.id.as_str())
        .await
        .unwrap();
    assert!(matches!(cron_detail.trigger.spec, TriggerSpec::Cron { .. }));
    let mcp_detail = runtime
        .load_automation_detail(mcp_event.automation.id.as_str())
        .await
        .unwrap();
    assert!(matches!(
        mcp_detail.trigger.spec,
        TriggerSpec::McpEvent { .. }
    ));
}

#[tokio::test]
async fn automation_lifecycle_allows_only_minimum_surface_transitions() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(
        &runtime,
        "project-automation-lifecycle",
        "Automation Lifecycle",
    )
    .await;
    seed_governance(
        &runtime,
        "project-automation-lifecycle",
        "capability-automation-lifecycle",
        "low",
        5,
        10,
    )
    .await;

    let automation = runtime
        .create_automation(automation_input(
            "project-automation-lifecycle",
            "lifecycle automation",
            "capability-automation-lifecycle",
        ))
        .await
        .unwrap();
    assert_eq!(automation.status, "active");

    let invalid_activate = runtime
        .activate_automation(automation.id.as_str())
        .await
        .unwrap_err();
    assert!(matches!(
        invalid_activate,
        RuntimeError::InvalidAutomationLifecycleTransition { from, to, .. }
            if from == "active" && to == "active"
    ));

    let paused = runtime
        .pause_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(paused.status, "paused");

    let resumed = runtime
        .activate_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(resumed.status, "active");

    let archived = runtime
        .archive_automation(automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(archived.status, "archived");

    let invalid_resume = runtime
        .activate_automation(automation.id.as_str())
        .await
        .unwrap_err();
    assert!(matches!(
        invalid_resume,
        RuntimeError::InvalidAutomationLifecycleTransition { from, to, .. }
            if from == "archived" && to == "active"
    ));
}

#[tokio::test]
async fn automation_detail_derives_recent_deliveries_and_last_run_summary() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-automation-detail", "Automation Detail").await;
    seed_governance(
        &runtime,
        "project-automation-detail",
        "capability-automation-detail",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-automation-detail".into(),
            title: "detail automation".into(),
            instruction: "Derive recent execution".into(),
            action: ExecutionAction::FailOnceThenEmitText {
                failure_message: "network_glitch".into(),
                content: "recovered".into(),
            },
            capability_id: "capability-automation-detail".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let failed = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: created.trigger_id.clone(),
            dedupe_key: "detail-delivery-1".into(),
            payload: json!({"source": "detail"}),
        })
        .await
        .unwrap();
    let recovered = runtime
        .retry_trigger_delivery(failed.delivery.id.as_str())
        .await
        .unwrap();

    let detail = runtime
        .load_automation_detail(created.id.as_str())
        .await
        .unwrap();
    assert_eq!(detail.automation.id, created.id);
    assert_eq!(detail.recent_deliveries.len(), 1);
    assert_eq!(detail.recent_deliveries[0].id, failed.delivery.id);
    assert_eq!(detail.recent_deliveries[0].status, "succeeded");
    assert_eq!(detail.recent_deliveries[0].attempt_count, 2);
    let last_run_summary = detail.last_run_summary.expect("last run summary");
    assert_eq!(last_run_summary.id, recovered.run_report.run.id);
    assert_eq!(last_run_summary.status, "completed");
    assert_eq!(last_run_summary.run_type, "automation");
    assert_eq!(last_run_summary.title, created.title);
}

#[tokio::test]
async fn manual_dispatch_rejects_non_manual_triggers() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(
        &runtime,
        "project-automation-manual-guard",
        "Automation Manual Guard",
    )
    .await;
    seed_governance(
        &runtime,
        "project-automation-manual-guard",
        "capability-automation-manual-guard",
        "low",
        5,
        10,
    )
    .await;

    let created = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-automation-manual-guard",
                "cron-only automation",
                "capability-automation-manual-guard",
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();

    let error = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: created.automation.trigger_id.clone(),
            dedupe_key: "cron-manual-guard".into(),
            payload: json!({}),
        })
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        RuntimeError::InvalidTriggerType { trigger_type, .. } if trigger_type == "cron"
    ));
}
