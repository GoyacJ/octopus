use std::path::{Path, PathBuf};

use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    Slice2Runtime,
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

async fn seed_bound_capability(
    runtime: &Slice2Runtime,
    project_id: &str,
    capability_id: &str,
    risk_level: &str,
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
}

async fn seed_grant(runtime: &Slice2Runtime, project_id: &str, capability_id: &str) {
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{capability_id}"),
            capability_id,
            "workspace-alpha",
            project_id,
        ))
        .await
        .unwrap();
}

async fn seed_budget(runtime: &Slice2Runtime, project_id: &str, soft_limit: i64, hard_limit: i64) {
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

async fn single_resolution(
    runtime: &Slice2Runtime,
    project_id: &str,
    estimated_cost: i64,
) -> octopus_runtime::CapabilityResolutionRecord {
    let resolutions = runtime
        .list_capability_resolutions("workspace-alpha", project_id, estimated_cost)
        .await
        .unwrap();
    assert_eq!(resolutions.len(), 1);
    resolutions.into_iter().next().unwrap()
}

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("slice12-runtime.sqlite")
}

#[tokio::test]
async fn capability_resolution_denies_missing_grant() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-missing-grant", "Missing Grant").await;
    seed_bound_capability(
        &runtime,
        "project-missing-grant",
        "capability-missing-grant",
        "low",
    )
    .await;
    seed_budget(&runtime, "project-missing-grant", 5, 10).await;

    let resolution = single_resolution(&runtime, "project-missing-grant", 1).await;
    assert_eq!(resolution.execution_state.as_str(), "denied");
    assert_eq!(resolution.reason_code.as_str(), "capability_not_granted");
}

#[tokio::test]
async fn capability_resolution_denies_missing_budget_policy() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-missing-budget", "Missing Budget").await;
    seed_bound_capability(
        &runtime,
        "project-missing-budget",
        "capability-missing-budget",
        "low",
    )
    .await;
    seed_grant(
        &runtime,
        "project-missing-budget",
        "capability-missing-budget",
    )
    .await;

    let resolution = single_resolution(&runtime, "project-missing-budget", 1).await;
    assert_eq!(resolution.execution_state.as_str(), "denied");
    assert_eq!(resolution.reason_code.as_str(), "budget_policy_missing");
}

#[tokio::test]
async fn capability_resolution_allows_within_budget() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-within-budget", "Within Budget").await;
    seed_bound_capability(
        &runtime,
        "project-within-budget",
        "capability-within-budget",
        "low",
    )
    .await;
    seed_grant(
        &runtime,
        "project-within-budget",
        "capability-within-budget",
    )
    .await;
    seed_budget(&runtime, "project-within-budget", 5, 10).await;

    let resolution = single_resolution(&runtime, "project-within-budget", 1).await;
    assert_eq!(resolution.execution_state.as_str(), "executable");
    assert_eq!(resolution.reason_code.as_str(), "within_budget");
}

#[tokio::test]
async fn capability_resolution_requires_approval_above_soft_limit() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-soft-limit", "Soft Limit").await;
    seed_bound_capability(
        &runtime,
        "project-soft-limit",
        "capability-soft-limit",
        "low",
    )
    .await;
    seed_grant(&runtime, "project-soft-limit", "capability-soft-limit").await;
    seed_budget(&runtime, "project-soft-limit", 5, 10).await;

    let resolution = single_resolution(&runtime, "project-soft-limit", 7).await;
    assert_eq!(resolution.execution_state.as_str(), "approval_required");
    assert_eq!(
        resolution.reason_code.as_str(),
        "budget_soft_limit_exceeded"
    );
}

#[tokio::test]
async fn capability_resolution_denies_above_hard_limit() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-hard-limit", "Hard Limit").await;
    seed_bound_capability(
        &runtime,
        "project-hard-limit",
        "capability-hard-limit",
        "low",
    )
    .await;
    seed_grant(&runtime, "project-hard-limit", "capability-hard-limit").await;
    seed_budget(&runtime, "project-hard-limit", 5, 10).await;

    let resolution = single_resolution(&runtime, "project-hard-limit", 11).await;
    assert_eq!(resolution.execution_state.as_str(), "denied");
    assert_eq!(
        resolution.reason_code.as_str(),
        "budget_hard_limit_exceeded"
    );
}

#[tokio::test]
async fn capability_resolution_requires_approval_for_high_risk_capabilities() {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = sample_db_path(tempdir.path());

    let runtime = Slice2Runtime::open_at(&db_path).await.unwrap();
    seed_context(&runtime, "project-high-risk", "High Risk").await;
    seed_bound_capability(
        &runtime,
        "project-high-risk",
        "capability-high-risk",
        "high",
    )
    .await;
    seed_grant(&runtime, "project-high-risk", "capability-high-risk").await;
    seed_budget(&runtime, "project-high-risk", 5, 10).await;

    let resolution = single_resolution(&runtime, "project-high-risk", 1).await;
    assert_eq!(resolution.execution_state.as_str(), "approval_required");
    assert_eq!(resolution.reason_code.as_str(), "risk_level_high");
}
