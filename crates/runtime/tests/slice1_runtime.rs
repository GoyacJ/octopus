use std::path::Path;

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateTaskInput, Slice1Runtime,
};

fn sample_db_path(base: &Path) -> std::path::PathBuf {
    base.join("slice1.sqlite")
}

async fn seed_governance(runtime: &Slice1Runtime, project_id: &str) {
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            "capability-slice1",
            "capability-slice1",
            "low",
            false,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{project_id}"),
            "capability-slice1",
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{project_id}"),
            "capability-slice1",
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

#[tokio::test]
async fn persists_completed_run_and_reloads_after_reopen() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-slice1",
            "project-slice1",
            "Project Slice 1",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-slice1").await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-slice1".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Write a deterministic note".into(),
            instruction: "Emit a single execution artifact".into(),
            action: ExecutionAction::EmitText {
                content: "Slice 1 says hello".into(),
            },
            capability_id: "capability-slice1".into(),
            estimated_cost: 1,
            idempotency_key: "task-success-1".into(),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(report.run.status.as_str(), "completed");
    assert_eq!(report.artifacts.len(), 1);
    assert!(report
        .audits
        .iter()
        .any(|audit| audit.event_type.as_str() == "run_completed"));
    assert!(report
        .traces
        .iter()
        .any(|trace| trace.stage.as_str() == "execution_action"));

    drop(runtime);

    let reopened = Slice1Runtime::open_at(&db_path).await.unwrap();
    let persisted_run = reopened
        .fetch_run(report.run.id.as_str())
        .await
        .unwrap()
        .unwrap();
    let persisted_artifacts = reopened
        .list_artifacts_by_run(report.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(persisted_run.status.as_str(), "completed");
    assert_eq!(persisted_artifacts.len(), 1);
}

#[tokio::test]
async fn failed_run_can_retry_and_then_succeed() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-retry",
            "project-retry",
            "Retry Project",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-retry").await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-retry".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Retry once".into(),
            instruction: "Fail once then emit text".into(),
            action: ExecutionAction::FailOnceThenEmitText {
                failure_message: "network_glitch".into(),
                content: "Recovered artifact".into(),
            },
            capability_id: "capability-slice1".into(),
            estimated_cost: 1,
            idempotency_key: "task-retry-1".into(),
        })
        .await
        .unwrap();

    let first_attempt = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(first_attempt.run.status.as_str(), "failed");
    assert!(first_attempt.artifacts.is_empty());

    let second_attempt = runtime
        .retry_run(first_attempt.run.id.as_str())
        .await
        .unwrap();
    assert_eq!(second_attempt.run.status.as_str(), "completed");
    assert_eq!(second_attempt.run.attempt_count, 2);
    assert_eq!(second_attempt.artifacts.len(), 1);
}

#[tokio::test]
async fn failed_run_can_be_explicitly_terminated() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-terminate",
            "project-terminate",
            "Terminate Project",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-terminate").await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-terminate".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Terminate after failure".into(),
            instruction: "Always fail and then terminate".into(),
            action: ExecutionAction::AlwaysFail {
                message: "irrecoverable".into(),
            },
            capability_id: "capability-slice1".into(),
            estimated_cost: 1,
            idempotency_key: "task-terminate-1".into(),
        })
        .await
        .unwrap();

    let failed = runtime.start_task(task.id.as_str()).await.unwrap();
    assert_eq!(failed.run.status.as_str(), "failed");

    let terminated = runtime
        .terminate_run(failed.run.id.as_str(), "operator_stopped")
        .await
        .unwrap();
    assert_eq!(terminated.run.status.as_str(), "terminated");
}

#[tokio::test]
async fn idempotency_deduplicates_task_and_run_creation() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice1Runtime::open_at(&db_path).await.unwrap();
    runtime
        .ensure_project_context(
            "workspace-alpha",
            "workspace-alpha",
            "Workspace Alpha",
            "project-idempotent",
            "project-idempotent",
            "Idempotent Project",
        )
        .await
        .unwrap();
    seed_governance(&runtime, "project-idempotent").await;

    let first_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-idempotent".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Deduplicate intake".into(),
            instruction: "Use one task and one run".into(),
            action: ExecutionAction::EmitText {
                content: "Stable artifact".into(),
            },
            capability_id: "capability-slice1".into(),
            estimated_cost: 1,
            idempotency_key: "task-idempotent-1".into(),
        })
        .await
        .unwrap();

    let second_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-idempotent".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Deduplicate intake".into(),
            instruction: "Use one task and one run".into(),
            action: ExecutionAction::EmitText {
                content: "Stable artifact".into(),
            },
            capability_id: "capability-slice1".into(),
            estimated_cost: 1,
            idempotency_key: "task-idempotent-1".into(),
        })
        .await
        .unwrap();

    assert_eq!(first_task.id, second_task.id);

    let first_run = runtime.start_task(first_task.id.as_str()).await.unwrap();
    let second_run = runtime.start_task(first_task.id.as_str()).await.unwrap();
    assert_eq!(first_run.run.id, second_run.run.id);
}
