use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateAutomationInput, CreateTaskInput, DispatchManualEventInput, Slice2Runtime,
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
    base.join("slice4-runtime.sqlite")
}

#[tokio::test]
async fn completed_task_creates_knowledge_candidate_and_lineage() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-knowledge-capture", "Knowledge Capture").await;
    seed_governance(
        &runtime,
        "project-knowledge-capture",
        "capability-knowledge-capture",
        "low",
        5,
        10,
    )
    .await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-capture",
            "Knowledge Capture Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-capture".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Capture knowledge".into(),
            instruction: "Emit a durable execution artifact".into(),
            action: ExecutionAction::EmitText {
                content: "Knowledge-worthy result".into(),
            },
            capability_id: "capability-knowledge-capture".into(),
            estimated_cost: 1,
            idempotency_key: "task-knowledge-capture-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();

    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.knowledge_candidates.len(), 1);
    assert_eq!(
        report.knowledge_candidates[0].source_run_id.as_str(),
        report.run.id.as_str()
    );
    assert_eq!(report.recalled_knowledge_assets.len(), 0);

    let lineage = runtime
        .list_knowledge_lineage_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(lineage.len(), 1);
    assert_eq!(lineage[0].relation_type.as_str(), "derived_from");
    assert!(report
        .audits
        .iter()
        .any(|audit| audit.event_type == "knowledge_candidate_created"));
}

#[tokio::test]
async fn promoted_knowledge_is_recalled_by_later_task_and_automation_runs() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-knowledge-recall", "Knowledge Recall").await;
    seed_governance(
        &runtime,
        "project-knowledge-recall",
        "capability-knowledge-recall",
        "low",
        5,
        10,
    )
    .await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-recall",
            "Knowledge Recall Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let source_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-recall".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Seed shared knowledge".into(),
            instruction: "Emit a reusable result".into(),
            action: ExecutionAction::EmitText {
                content: "Reusable shared knowledge".into(),
            },
            capability_id: "capability-knowledge-recall".into(),
            estimated_cost: 1,
            idempotency_key: "task-knowledge-recall-source".into(),
        })
        .await
        .unwrap();

    let source_report = runtime.start_task(source_task.id.as_str()).await.unwrap();
    let first_promotion = runtime
        .promote_knowledge_candidate(
            source_report.knowledge_candidates[0].id.as_str(),
            "workspace_admin:alice",
            "Promote validated knowledge",
        )
        .await
        .unwrap();
    let second_promotion = runtime
        .promote_knowledge_candidate(
            source_report.knowledge_candidates[0].id.as_str(),
            "workspace_admin:alice",
            "Promote validated knowledge again",
        )
        .await
        .unwrap();
    assert_eq!(first_promotion.asset.id, second_promotion.asset.id);

    let followup_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-recall".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Use shared knowledge".into(),
            instruction: "Emit another result".into(),
            action: ExecutionAction::EmitText {
                content: "Follow-up result".into(),
            },
            capability_id: "capability-knowledge-recall".into(),
            estimated_cost: 1,
            idempotency_key: "task-knowledge-recall-followup".into(),
        })
        .await
        .unwrap();

    let followup_report = runtime.start_task(followup_task.id.as_str()).await.unwrap();
    assert_eq!(followup_report.recalled_knowledge_assets.len(), 1);
    assert_eq!(
        followup_report.recalled_knowledge_assets[0].id,
        first_promotion.asset.id
    );

    let automation = runtime
        .create_automation(CreateAutomationInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-recall".into(),
            title: "Recall through automation".into(),
            instruction: "Dispatch manual event".into(),
            action: ExecutionAction::EmitText {
                content: "Automation follow-up".into(),
            },
            capability_id: "capability-knowledge-recall".into(),
            estimated_cost: 1,
        })
        .await
        .unwrap();

    let automation_report = runtime
        .dispatch_manual_event(DispatchManualEventInput {
            trigger_id: automation.trigger_id.clone(),
            dedupe_key: "delivery-knowledge-recall-1".into(),
            payload: json!({"source": "slice4"}),
        })
        .await
        .unwrap();
    assert_eq!(
        automation_report.run_report.recalled_knowledge_assets.len(),
        1
    );
    assert_eq!(
        automation_report.run_report.recalled_knowledge_assets[0].id,
        first_promotion.asset.id
    );
}

#[tokio::test]
async fn knowledge_capture_failure_does_not_block_run_and_retry_is_idempotent() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-knowledge-retry", "Knowledge Retry").await;
    seed_governance(
        &runtime,
        "project-knowledge-retry",
        "capability-knowledge-retry",
        "low",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-retry".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Retry knowledge capture".into(),
            instruction: "Emit a result before knowledge space exists".into(),
            action: ExecutionAction::EmitText {
                content: "Retryable knowledge result".into(),
            },
            capability_id: "capability-knowledge-retry".into(),
            estimated_cost: 1,
            idempotency_key: "task-knowledge-retry-1".into(),
        })
        .await
        .unwrap();

    let initial_report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(initial_report.run.status.as_str(), "completed");
    assert_eq!(initial_report.artifacts.len(), 1);
    assert_eq!(initial_report.knowledge_candidates.len(), 0);
    assert!(initial_report
        .audits
        .iter()
        .any(|audit| audit.event_type == "knowledge_capture_failed"));

    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-retry",
            "Knowledge Retry Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let first_retry = runtime
        .retry_knowledge_capture(initial_report.run.id.as_str())
        .await
        .unwrap();
    let second_retry = runtime
        .retry_knowledge_capture(initial_report.run.id.as_str())
        .await
        .unwrap();

    assert_eq!(first_retry.knowledge_candidates.len(), 1);
    assert_eq!(second_retry.knowledge_candidates.len(), 1);
    assert_eq!(
        first_retry.knowledge_candidates[0].id,
        second_retry.knowledge_candidates[0].id
    );
    assert!(second_retry
        .audits
        .iter()
        .any(|audit| audit.event_type == "knowledge_capture_retried"));
}
