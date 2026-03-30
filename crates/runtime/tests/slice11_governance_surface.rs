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
    base.join("slice11-runtime.sqlite")
}

async fn seed_completed_run_with_candidate(
    runtime: &Slice2Runtime,
    project_id: &str,
    capability_id: &str,
) -> (String, String) {
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            project_id,
            "Governance Surface Knowledge",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: project_id.into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Seed candidate".into(),
            instruction: "Emit a durable result".into(),
            action: ExecutionAction::EmitText {
                content: "governed knowledge".into(),
            },
            capability_id: capability_id.into(),
            estimated_cost: 1,
            idempotency_key: format!("task-{project_id}-candidate"),
        })
        .await
        .unwrap();

    let report = runtime.start_task(task.id.as_str()).await.unwrap();
    (
        report.run.id.clone(),
        report.knowledge_candidates[0].id.clone(),
    )
}

#[tokio::test]
async fn request_knowledge_promotion_is_idempotent_and_creates_pending_records() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice11", "Slice 11 Project").await;
    seed_governance(
        &runtime,
        "project-slice11",
        "capability-governance",
        "low",
        5,
        10,
    )
    .await;

    let (_, candidate_id) =
        seed_completed_run_with_candidate(&runtime, "project-slice11", "capability-governance")
            .await;

    let first = runtime
        .request_knowledge_promotion(
            candidate_id.as_str(),
            "workspace_admin:alice",
            "needs approval",
        )
        .await
        .unwrap();
    let second = runtime
        .request_knowledge_promotion(
            candidate_id.as_str(),
            "workspace_admin:alice",
            "still needs approval",
        )
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.approval_type.as_str(), "knowledge_promotion");
    assert_eq!(
        first.target_ref.as_str(),
        format!("knowledge_candidate:{candidate_id}")
    );
    assert_eq!(first.status.as_str(), "pending");

    let report = runtime
        .fetch_approval_request(first.id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        report.target_ref.as_str(),
        format!("knowledge_candidate:{candidate_id}")
    );

    let inbox_items = runtime
        .list_inbox_items_by_workspace("workspace-alpha")
        .await
        .unwrap();
    assert!(inbox_items
        .iter()
        .any(|item| item.approval_request_id == first.id
            && item.target_ref == format!("knowledge_candidate:{candidate_id}")));

    let notifications = runtime
        .list_notifications_by_workspace("workspace-alpha")
        .await
        .unwrap();
    assert!(notifications
        .iter()
        .any(|item| item.approval_request_id == first.id
            && item.target_ref == format!("knowledge_candidate:{candidate_id}")));
}

#[tokio::test]
async fn approving_knowledge_promotion_creates_asset_without_mutating_run_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice11-approve", "Slice 11 Approve").await;
    seed_governance(
        &runtime,
        "project-slice11-approve",
        "capability-governance",
        "low",
        5,
        10,
    )
    .await;

    let (run_id, candidate_id) = seed_completed_run_with_candidate(
        &runtime,
        "project-slice11-approve",
        "capability-governance",
    )
    .await;
    let approval = runtime
        .request_knowledge_promotion(
            candidate_id.as_str(),
            "workspace_admin:alice",
            "ready for promotion",
        )
        .await
        .unwrap();

    let resolved = runtime
        .resolve_approval(
            approval.id.as_str(),
            ApprovalDecision::Approve,
            "workspace_admin:alice",
            "approved promotion",
        )
        .await
        .unwrap();

    let candidate = runtime
        .list_knowledge_candidates_by_run(run_id.as_str())
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == candidate_id)
        .unwrap();
    assert_eq!(candidate.status.as_str(), "verified_shared");
    assert_eq!(resolved.run.status.as_str(), "completed");
    assert!(runtime
        .list_knowledge_assets_by_run(run_id.as_str())
        .await
        .unwrap()
        .iter()
        .any(|asset| asset.source_candidate_id == candidate_id));
}

#[tokio::test]
async fn rejecting_knowledge_promotion_keeps_candidate_unpromoted() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-slice11-reject", "Slice 11 Reject").await;
    seed_governance(
        &runtime,
        "project-slice11-reject",
        "capability-governance",
        "low",
        5,
        10,
    )
    .await;

    let (run_id, candidate_id) = seed_completed_run_with_candidate(
        &runtime,
        "project-slice11-reject",
        "capability-governance",
    )
    .await;
    let approval = runtime
        .request_knowledge_promotion(
            candidate_id.as_str(),
            "workspace_admin:alice",
            "review please",
        )
        .await
        .unwrap();

    let resolved = runtime
        .resolve_approval(
            approval.id.as_str(),
            ApprovalDecision::Reject,
            "workspace_admin:alice",
            "not yet",
        )
        .await
        .unwrap();

    let candidate = runtime
        .list_knowledge_candidates_by_run(run_id.as_str())
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == candidate_id)
        .unwrap();
    assert_eq!(candidate.status.as_str(), "candidate");
    assert_eq!(resolved.run.status.as_str(), "completed");
    assert!(runtime
        .list_knowledge_assets_by_run(run_id.as_str())
        .await
        .unwrap()
        .is_empty());
}
