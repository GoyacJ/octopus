use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateAutomationInput, CreateTriggerInput, DispatchManualEventInput, Slice2Runtime,
    TriggerSpec,
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

async fn seed_governance(runtime: &Slice2Runtime, project_id: &str, capability_id: &str) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            capability_id,
            capability_id,
            "low",
            false,
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
            5,
            10,
        ))
        .await
        .unwrap();
}

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("trigger-foundation-runtime.sqlite")
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
async fn trigger_substrate_persists_all_ga_trigger_variants_and_keeps_manual_event_compatibility() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-trigger-foundation", "Trigger Foundation").await;
    seed_governance(
        &runtime,
        "project-trigger-foundation",
        "capability-trigger-foundation",
    )
    .await;

    let manual = runtime
        .create_automation(automation_input(
            "project-trigger-foundation",
            "manual compatibility",
            "capability-trigger-foundation",
        ))
        .await
        .unwrap();
    let manual_trigger = runtime
        .fetch_trigger(manual.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        manual_trigger.spec,
        TriggerSpec::ManualEvent { .. }
    ));

    let manual_report = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: manual.trigger_id.clone(),
            dedupe_key: "delivery-manual-foundation-1".into(),
            payload: json!({"source": "trigger_foundation"}),
        })
        .await
        .unwrap();
    assert_eq!(manual_report.delivery.status.as_str(), "succeeded");

    let cron = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-trigger-foundation",
                "cron foundation",
                "capability-trigger-foundation",
            ),
            CreateTriggerInput::Cron {
                schedule: "0 * * * * * *".into(),
                timezone: "UTC".into(),
                next_fire_at: "2026-03-27T12:00:00Z".into(),
            },
        )
        .await
        .unwrap();
    assert!(cron.webhook_secret.is_none());
    let cron_trigger = runtime
        .fetch_trigger(cron.automation.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        cron_trigger.spec,
        TriggerSpec::Cron { ref config }
            if config.schedule == "0 * * * * * *"
                && config.timezone == "UTC"
                && config.next_fire_at == "2026-03-27T12:00:00Z"
    ));

    let webhook = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-trigger-foundation",
                "webhook foundation",
                "capability-trigger-foundation",
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
    assert!(webhook.webhook_secret.is_some());
    let webhook_trigger = runtime
        .fetch_trigger(webhook.automation.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        webhook_trigger.spec,
        TriggerSpec::Webhook { ref config }
            if config.ingress_mode == "shared_secret_header"
                && config.secret_header_name == "X-Octopus-Trigger-Secret"
                && config.secret_hint.as_deref() == Some("hook")
                && config.secret_present
    ));

    let mcp_event = runtime
        .create_automation_with_trigger(
            automation_input(
                "project-trigger-foundation",
                "mcp foundation",
                "capability-trigger-foundation",
            ),
            CreateTriggerInput::McpEvent {
                server_id: "server-foundation".into(),
                event_name: Some("connector.output.ready".into()),
                event_pattern: None,
            },
        )
        .await
        .unwrap();
    assert!(mcp_event.webhook_secret.is_none());
    let mcp_trigger = runtime
        .fetch_trigger(mcp_event.automation.trigger_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        mcp_trigger.spec,
        TriggerSpec::McpEvent { ref config }
            if config.server_id == "server-foundation"
                && config.event_name.as_deref() == Some("connector.output.ready")
                && config.event_pattern.is_none()
    ));
}
