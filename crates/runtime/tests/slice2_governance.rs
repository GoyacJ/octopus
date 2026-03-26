use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateTaskInput, Slice2Runtime,
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
    base.join("slice2-runtime.sqlite")
}

#[tokio::test]
async fn low_risk_allowed_task_executes_without_approval() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-allow", "Allow Project").await;
    seed_governance(&runtime, "project-allow", "capability-allow", "low", 5, 10).await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-allow".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Allowed task".into(),
            instruction: "Run immediately".into(),
            action: ExecutionAction::EmitText {
                content: "allowed".into(),
            },
            capability_id: "capability-allow".into(),
            estimated_cost: 1,
            idempotency_key: "task-allow-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.approvals.len(), 0);
    assert_eq!(report.inbox_items.len(), 0);
    assert_eq!(report.notifications.len(), 0);
    assert_eq!(report.policy_decisions.len(), 1);
    assert_eq!(report.policy_decisions[0].decision.as_str(), "allow");
}

#[tokio::test]
async fn high_risk_task_waits_for_approval_and_persists_pending_records() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-approval", "Approval Project").await;
    seed_governance(
        &runtime,
        "project-approval",
        "capability-high-risk",
        "high",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-approval".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "High risk task".into(),
            instruction: "Pause for approval".into(),
            action: ExecutionAction::EmitText {
                content: "needs approval".into(),
            },
            capability_id: "capability-high-risk".into(),
            estimated_cost: 1,
            idempotency_key: "task-approval-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "waiting_approval");
    assert!(report.run.approval_request_id.is_some());
    assert_eq!(report.artifacts.len(), 0);
    assert_eq!(report.approvals.len(), 1);
    assert_eq!(report.approvals[0].status.as_str(), "pending");
    assert_eq!(report.inbox_items.len(), 1);
    assert_eq!(report.inbox_items[0].status.as_str(), "open");
    assert_eq!(report.notifications.len(), 1);
    assert_eq!(report.policy_decisions.len(), 1);
    assert_eq!(
        report.policy_decisions[0].decision.as_str(),
        "require_approval"
    );
}

#[tokio::test]
async fn approved_request_resumes_waiting_run_and_completes() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-approve", "Approve Project").await;
    seed_governance(
        &runtime,
        "project-approve",
        "capability-approve",
        "high",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-approve".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Approve and resume".into(),
            instruction: "Needs approval, then run".into(),
            action: ExecutionAction::EmitText {
                content: "approved".into(),
            },
            capability_id: "capability-approve".into(),
            estimated_cost: 1,
            idempotency_key: "task-approve-1".into(),
        })
        .await
        .unwrap();

    let waiting = runtime.start_task(task.id.as_str()).await.unwrap();
    let approval_id = waiting.approvals[0].id.clone();

    let report = runtime
        .resolve_approval(
            approval_id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "looks good",
        )
        .await
        .unwrap();

    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.run.id, waiting.run.id);
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.approvals.len(), 1);
    assert_eq!(report.approvals[0].status.as_str(), "approved");
    assert_eq!(report.inbox_items.len(), 1);
    assert_eq!(report.inbox_items[0].status.as_str(), "resolved");
    assert_eq!(report.notifications.len(), 1);
    assert_eq!(report.policy_decisions.len(), 2);
}

#[tokio::test]
async fn rejected_request_blocks_run_without_artifact() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-reject", "Reject Project").await;
    seed_governance(
        &runtime,
        "project-reject",
        "capability-reject",
        "high",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-reject".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Reject and block".into(),
            instruction: "Needs approval, then block".into(),
            action: ExecutionAction::EmitText {
                content: "should never execute".into(),
            },
            capability_id: "capability-reject".into(),
            estimated_cost: 1,
            idempotency_key: "task-reject-1".into(),
        })
        .await
        .unwrap();

    let waiting = runtime.start_task(task.id.as_str()).await.unwrap();
    let approval_id = waiting.approvals[0].id.clone();

    let report = runtime
        .resolve_approval(
            approval_id.as_str(),
            ApprovalDecision::Reject,
            "reviewer-alpha",
            "not allowed",
        )
        .await
        .unwrap();

    assert_eq!(report.run.status.as_str(), "blocked");
    assert_eq!(report.artifacts.len(), 0);
    assert_eq!(report.approvals[0].status.as_str(), "rejected");
    assert_eq!(report.inbox_items[0].status.as_str(), "resolved");
}

#[tokio::test]
async fn hard_limit_denied_task_does_not_execute_and_logs_policy_decision() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-deny", "Deny Project").await;
    seed_governance(&runtime, "project-deny", "capability-deny", "low", 5, 10).await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-deny".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Denied task".into(),
            instruction: "Should be denied before execution".into(),
            action: ExecutionAction::EmitText {
                content: "denied".into(),
            },
            capability_id: "capability-deny".into(),
            estimated_cost: 11,
            idempotency_key: "task-deny-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "blocked");
    assert_eq!(report.artifacts.len(), 0);
    assert_eq!(report.approvals.len(), 0);
    assert_eq!(report.inbox_items.len(), 0);
    assert_eq!(report.notifications.len(), 0);
    assert_eq!(report.policy_decisions.len(), 1);
    assert_eq!(report.policy_decisions[0].decision.as_str(), "deny");
}

#[tokio::test]
async fn pending_approval_survives_reopen_without_duplicate_records() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-reopen", "Reopen Project").await;
    seed_governance(
        &runtime,
        "project-reopen",
        "capability-reopen",
        "high",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-reopen".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Reopen pending approval".into(),
            instruction: "Pause and stay pending".into(),
            action: ExecutionAction::EmitText {
                content: "pending".into(),
            },
            capability_id: "capability-reopen".into(),
            estimated_cost: 1,
            idempotency_key: "task-reopen-1".into(),
        })
        .await
        .unwrap();

    let waiting = runtime.start_task(task.id.as_str()).await.unwrap();
    drop(runtime);

    let reopened = Slice2Runtime::open_at(&db_path).await.unwrap();
    let report = reopened
        .load_run_report(waiting.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(report.run.status.as_str(), "waiting_approval");
    assert_eq!(report.approvals.len(), 1);
    assert_eq!(report.inbox_items.len(), 1);
    assert_eq!(report.notifications.len(), 1);

    let second = reopened.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(second.run.id, waiting.run.id);
    assert_eq!(second.approvals.len(), 1);
    assert_eq!(second.inbox_items.len(), 1);
    assert_eq!(second.notifications.len(), 1);
}

#[tokio::test]
async fn repeated_approval_resolution_is_idempotent_and_does_not_duplicate_records() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-repeat", "Repeat Project").await;
    seed_governance(
        &runtime,
        "project-repeat",
        "capability-repeat",
        "high",
        5,
        10,
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-repeat".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Repeat approval".into(),
            instruction: "Approval should be idempotent".into(),
            action: ExecutionAction::EmitText {
                content: "repeat".into(),
            },
            capability_id: "capability-repeat".into(),
            estimated_cost: 1,
            idempotency_key: "task-repeat-1".into(),
        })
        .await
        .unwrap();

    let waiting = runtime.start_task(task.id.as_str()).await.unwrap();
    let approval_id = waiting.approvals[0].id.clone();

    let first = runtime
        .resolve_approval(
            approval_id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "first decision",
        )
        .await
        .unwrap();
    let second = runtime
        .resolve_approval(
            approval_id.as_str(),
            ApprovalDecision::Approve,
            "reviewer-alpha",
            "same decision",
        )
        .await
        .unwrap();

    assert_eq!(first.run.id, second.run.id);
    assert_eq!(second.approvals.len(), 1);
    assert_eq!(second.inbox_items.len(), 1);
    assert_eq!(second.notifications.len(), 1);
}
