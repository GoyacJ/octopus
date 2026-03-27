use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateAutomationInput, CreateTriggerInput, DispatchWebhookEventInput,
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

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("slice7-webhook-runtime.sqlite")
}

fn webhook_automation_input(
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

async fn create_webhook_automation(
    runtime: &Slice2Runtime,
    project_id: &str,
    title: &str,
    capability_id: &str,
    action: ExecutionAction,
) -> (String, String) {
    let created = runtime
        .create_automation_with_trigger(
            webhook_automation_input(project_id, title, capability_id, action),
            CreateTriggerInput::Webhook {
                ingress_mode: "shared_secret_header".into(),
                secret_header_name: "X-Octopus-Trigger-Secret".into(),
                secret_hint: Some("hook".into()),
                secret_plaintext: None,
            },
        )
        .await
        .unwrap();
    (
        created.automation.trigger_id,
        created.webhook_secret.unwrap(),
    )
}

#[tokio::test]
async fn webhook_trigger_happy_path_and_duplicate_ingress_reuse_delivery() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-webhook-happy", "Webhook Happy").await;
    seed_governance(
        &runtime,
        "project-webhook-happy",
        "capability-webhook-happy",
        "low",
        5,
        10,
    )
    .await;

    let (trigger_id, secret) = create_webhook_automation(
        &runtime,
        "project-webhook-happy",
        "webhook happy",
        "capability-webhook-happy",
        ExecutionAction::EmitText {
            content: "webhook artifact".into(),
        },
    )
    .await;

    let first = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id: trigger_id.clone(),
            idempotency_key: "event-1".into(),
            secret: secret.clone(),
            payload: json!({"source": "webhook"}),
        })
        .await
        .unwrap();
    assert_eq!(first.delivery.status.as_str(), "succeeded");

    let second = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id,
            idempotency_key: "event-1".into(),
            secret,
            payload: json!({"source": "webhook-duplicate"}),
        })
        .await
        .unwrap();
    assert_eq!(first.delivery.id, second.delivery.id);
    assert_eq!(first.run_report.run.id, second.run_report.run.id);
}

#[tokio::test]
async fn webhook_trigger_rejects_invalid_secret_and_missing_idempotency_key() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-webhook-reject", "Webhook Reject").await;
    seed_governance(
        &runtime,
        "project-webhook-reject",
        "capability-webhook-reject",
        "low",
        5,
        10,
    )
    .await;

    let (trigger_id, secret) = create_webhook_automation(
        &runtime,
        "project-webhook-reject",
        "webhook reject",
        "capability-webhook-reject",
        ExecutionAction::EmitText {
            content: "webhook artifact".into(),
        },
    )
    .await;

    let missing_idempotency = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id: trigger_id.clone(),
            idempotency_key: "".into(),
            secret: secret.clone(),
            payload: json!({}),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        missing_idempotency,
        RuntimeError::MissingWebhookIdempotencyKey { .. }
    ));

    let invalid_secret = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id,
            idempotency_key: "event-2".into(),
            secret: "wrong-secret".into(),
            payload: json!({}),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        invalid_secret,
        RuntimeError::InvalidWebhookSecret { .. }
    ));
}

#[tokio::test]
async fn webhook_trigger_reopen_reuses_waiting_delivery_and_completes_after_approval() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-webhook-approval", "Webhook Approval").await;
    seed_governance(
        &runtime,
        "project-webhook-approval",
        "capability-webhook-approval",
        "high",
        5,
        10,
    )
    .await;

    let (trigger_id, secret) = create_webhook_automation(
        &runtime,
        "project-webhook-approval",
        "webhook approval",
        "capability-webhook-approval",
        ExecutionAction::EmitText {
            content: "approval artifact".into(),
        },
    )
    .await;

    let waiting = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id: trigger_id.clone(),
            idempotency_key: "event-approval-1".into(),
            secret: secret.clone(),
            payload: json!({"kind": "approval"}),
        })
        .await
        .unwrap();
    assert_eq!(waiting.delivery.status.as_str(), "delivering");
    assert_eq!(waiting.run_report.run.status.as_str(), "waiting_approval");
    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let deduped = reopened
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id,
            idempotency_key: "event-approval-1".into(),
            secret,
            payload: json!({"kind": "approval-duplicate"}),
        })
        .await
        .unwrap();
    assert_eq!(deduped.delivery.id, waiting.delivery.id);
    assert_eq!(deduped.run_report.run.id, waiting.run_report.run.id);

    reopened
        .resolve_approval(
            deduped.run_report.approvals[0].id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "approved webhook event",
        )
        .await
        .unwrap();

    let deliveries = reopened
        .list_trigger_deliveries_by_automation(deduped.automation.id.as_str())
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status.as_str(), "succeeded");
}

#[tokio::test]
async fn webhook_trigger_propagates_policy_denial() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());
    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();

    seed_context(&runtime, "project-webhook-deny", "Webhook Deny").await;
    seed_governance(
        &runtime,
        "project-webhook-deny",
        "capability-webhook-deny",
        "low",
        0,
        0,
    )
    .await;

    let (trigger_id, secret) = create_webhook_automation(
        &runtime,
        "project-webhook-deny",
        "webhook deny",
        "capability-webhook-deny",
        ExecutionAction::EmitText {
            content: "denied".into(),
        },
    )
    .await;

    let denied = runtime
        .dispatch_webhook_event(DispatchWebhookEventInput {
            trigger_id,
            idempotency_key: "event-deny-1".into(),
            secret,
            payload: json!({"kind": "deny"}),
        })
        .await
        .unwrap();
    assert_eq!(denied.delivery.status.as_str(), "failed");
    assert_eq!(denied.run_report.run.status.as_str(), "blocked");
    assert!(denied.run_report.artifacts.is_empty());
}
