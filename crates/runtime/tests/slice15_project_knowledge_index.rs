use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateTaskInput, KnowledgeSummaryRecord, Slice2Runtime,
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
    base.join("slice15-runtime.sqlite")
}

#[tokio::test]
async fn project_knowledge_index_is_project_scoped_sorted_latest_first_and_traceable() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-knowledge-index", "Knowledge Index").await;
    seed_context(&runtime, "project-knowledge-other", "Knowledge Other").await;
    seed_governance(
        &runtime,
        "project-knowledge-index",
        "capability-knowledge-index",
        "low",
        5,
        10,
    )
    .await;
    seed_governance(
        &runtime,
        "project-knowledge-other",
        "capability-knowledge-other",
        "low",
        5,
        10,
    )
    .await;

    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-index",
            "Knowledge Index Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-other",
            "Knowledge Other Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let source_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-index".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Seed shared knowledge".into(),
            instruction: "Emit a reusable result".into(),
            action: ExecutionAction::EmitText {
                content: "Reusable shared knowledge".into(),
            },
            capability_id: "capability-knowledge-index".into(),
            estimated_cost: 1,
            idempotency_key: "task-project-knowledge-source".into(),
        })
        .await
        .unwrap();
    let source_report = runtime.start_task(source_task.id.as_str()).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let promotion = runtime
        .promote_knowledge_candidate(
            source_report.knowledge_candidates[0].id.as_str(),
            "workspace_admin:alice",
            "Promote for project knowledge index",
        )
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let later_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-index".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Capture another candidate".into(),
            instruction: "Emit another result".into(),
            action: ExecutionAction::EmitText {
                content: "Another knowledge candidate".into(),
            },
            capability_id: "capability-knowledge-index".into(),
            estimated_cost: 1,
            idempotency_key: "task-project-knowledge-later".into(),
        })
        .await
        .unwrap();
    let later_report = runtime.start_task(later_task.id.as_str()).await.unwrap();

    let other_task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".into(),
            project_id: "project-knowledge-other".into(),
            source_kind: "manual".into(),
            automation_id: None,
            title: "Other project candidate".into(),
            instruction: "Emit isolated result".into(),
            action: ExecutionAction::EmitText {
                content: "Other project knowledge".into(),
            },
            capability_id: "capability-knowledge-other".into(),
            estimated_cost: 1,
            idempotency_key: "task-project-knowledge-other".into(),
        })
        .await
        .unwrap();
    let other_report = runtime.start_task(other_task.id.as_str()).await.unwrap();

    let index = runtime
        .get_project_knowledge_index("workspace-alpha", "project-knowledge-index")
        .await
        .unwrap();

    assert_eq!(
        index.knowledge_space.project_id.as_deref(),
        Some("project-knowledge-index")
    );
    assert_eq!(index.entries.len(), 3);

    match &index.entries[0] {
        KnowledgeSummaryRecord::Candidate(candidate) => {
            assert_eq!(candidate.id, later_report.knowledge_candidates[0].id);
            assert_eq!(candidate.source_run_id, later_report.run.id);
        }
        other => panic!("expected latest candidate entry, got {other:?}"),
    }

    match &index.entries[1] {
        KnowledgeSummaryRecord::Asset(asset) => {
            assert_eq!(asset.id, promotion.asset.id);
            assert_eq!(
                asset.source_candidate_id,
                source_report.knowledge_candidates[0].id
            );
        }
        other => panic!("expected promoted asset entry, got {other:?}"),
    }

    match &index.entries[2] {
        KnowledgeSummaryRecord::Candidate(candidate) => {
            assert_eq!(candidate.id, source_report.knowledge_candidates[0].id);
            assert_eq!(candidate.status, "verified_shared");
        }
        other => panic!("expected promoted candidate entry, got {other:?}"),
    }

    let leaked_other_project = index.entries.iter().any(|entry| match entry {
        KnowledgeSummaryRecord::Candidate(candidate) => {
            candidate.id == other_report.knowledge_candidates[0].id
        }
        KnowledgeSummaryRecord::Asset(asset) => {
            asset.source_candidate_id == other_report.knowledge_candidates[0].id
        }
    });
    assert!(!leaked_other_project);
}

#[tokio::test]
async fn project_knowledge_index_returns_empty_entries_for_an_empty_project_space() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-knowledge-empty", "Knowledge Empty").await;
    runtime
        .ensure_project_knowledge_space(
            "workspace-alpha",
            "project-knowledge-empty",
            "Knowledge Empty Space",
            "workspace_admin:alice",
        )
        .await
        .unwrap();

    let index = runtime
        .get_project_knowledge_index("workspace-alpha", "project-knowledge-empty")
        .await
        .unwrap();

    assert_eq!(index.entries, Vec::<KnowledgeSummaryRecord>::new());
    assert_eq!(
        index.knowledge_space.project_id.as_deref(),
        Some("project-knowledge-empty")
    );
}
