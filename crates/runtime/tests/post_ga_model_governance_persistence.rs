use std::path::{Path, PathBuf};

use octopus_execution::ExecutionAction;
use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    CreateTaskInput, ModelSelectionDecisionRecord, RuntimeError, Slice2Runtime,
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
            format!("binding-{project_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{project_id}"),
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
    base.join("post-ga-model-governance-persistence.sqlite")
}

#[tokio::test]
async fn record_model_selection_decision_persists_one_decision_per_run() {
    let tempdir = tempfile::tempdir().unwrap();
    let runtime = Slice2Runtime::open_at(&sample_db_path(tempdir.path()))
        .await
        .unwrap();
    seed_context(&runtime, "project-model-center", "Model Center Project").await;
    seed_governance(
        &runtime,
        "project-model-center",
        "capability-model-governance",
    )
    .await;

    let task = runtime
        .create_task(CreateTaskInput {
            workspace_id: "workspace-alpha".to_string(),
            project_id: "project-model-center".to_string(),
            source_kind: "task".to_string(),
            automation_id: None,
            title: "Record model choice".to_string(),
            instruction: "Emit a deterministic artifact".to_string(),
            action: ExecutionAction::EmitText {
                content: "model-governance".to_string(),
            },
            capability_id: "capability-model-governance".to_string(),
            estimated_cost: 1,
            idempotency_key: "task:model-governance".to_string(),
        })
        .await
        .unwrap();
    let report = runtime.start_task(&task.id).await.unwrap();

    let first = ModelSelectionDecisionRecord {
        id: "selection-1".to_string(),
        run_id: report.run.id.clone(),
        model_profile_id: Some("profile-default-reasoning".to_string()),
        requested_intent: "web_research".to_string(),
        decision_outcome: "selected".to_string(),
        selected_model_key: Some("openai:gpt-5.4".to_string()),
        selected_provider_id: Some("provider-openai".to_string()),
        required_feature_tags: vec![
            "supports_structured_output".to_string(),
            "supports_builtin_web_search".to_string(),
        ],
        missing_feature_tags: vec![],
        requires_approval: false,
        decision_reason: "best matching features within tenant policy".to_string(),
        created_at: "2026-03-30T10:00:00Z".to_string(),
    };

    let persisted = runtime
        .record_model_selection_decision(first.clone())
        .await
        .unwrap();
    assert_eq!(persisted, first);

    let duplicate_attempt = ModelSelectionDecisionRecord {
        id: "selection-2".to_string(),
        run_id: report.run.id.clone(),
        model_profile_id: Some("profile-default-reasoning".to_string()),
        requested_intent: "web_research".to_string(),
        decision_outcome: "approval_required".to_string(),
        selected_model_key: None,
        selected_provider_id: None,
        required_feature_tags: vec!["supports_structured_output".to_string()],
        missing_feature_tags: vec!["supports_builtin_web_search".to_string()],
        requires_approval: true,
        decision_reason: "preview model requires approval".to_string(),
        created_at: "2026-03-30T10:05:00Z".to_string(),
    };

    let duplicate = runtime
        .record_model_selection_decision(duplicate_attempt)
        .await
        .unwrap();
    assert_eq!(duplicate, first);

    assert_eq!(
        runtime
            .fetch_model_selection_decision_by_run(&report.run.id)
            .await
            .unwrap(),
        Some(first.clone())
    );
    assert_eq!(
        runtime
            .load_run_report(&report.run.id)
            .await
            .unwrap()
            .model_selection_decision,
        Some(first)
    );
}

#[tokio::test]
async fn record_model_selection_decision_rejects_missing_runs() {
    let tempdir = tempfile::tempdir().unwrap();
    let runtime = Slice2Runtime::open_at(&sample_db_path(tempdir.path()))
        .await
        .unwrap();

    let result = runtime
        .record_model_selection_decision(ModelSelectionDecisionRecord {
            id: "selection-missing".to_string(),
            run_id: "run-missing".to_string(),
            model_profile_id: None,
            requested_intent: "web_research".to_string(),
            decision_outcome: "denied".to_string(),
            selected_model_key: None,
            selected_provider_id: None,
            required_feature_tags: vec!["supports_structured_output".to_string()],
            missing_feature_tags: vec!["supports_builtin_web_search".to_string()],
            requires_approval: false,
            decision_reason: "run does not exist".to_string(),
            created_at: "2026-03-30T10:00:00Z".to_string(),
        })
        .await;

    assert!(matches!(result, Err(RuntimeError::RunNotFound(run_id)) if run_id == "run-missing"));
}
