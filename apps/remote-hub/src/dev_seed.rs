use octopus_runtime::{
    BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord, CapabilityGrantRecord,
    RuntimeError, Slice1Runtime,
};

const DEV_WORKSPACE_ID: &str = "workspace-alpha";
const DEV_WORKSPACE_SLUG: &str = "workspace-alpha";
const DEV_WORKSPACE_NAME: &str = "Workspace Alpha";
const DEV_PROJECT_ID: &str = "project-remote-demo";
const DEV_PROJECT_SLUG: &str = "project-remote-demo";
const DEV_PROJECT_NAME: &str = "Remote Demo Project";
const DEV_KNOWLEDGE_SPACE_NAME: &str = "Remote Demo Project Knowledge";
const DEV_KNOWLEDGE_OWNER_REF: &str = "workspace_admin:bootstrap_admin";
const DEV_CAPABILITY_ID: &str = "capability-remote-dev-demo";
const DEV_BUDGET_SOFT_LIMIT: i64 = 5;
const DEV_BUDGET_HARD_LIMIT: i64 = 10;

pub async fn ensure_dev_seed_context(runtime: &Slice1Runtime) -> Result<(), RuntimeError> {
    let existing_projects = runtime.list_projects(DEV_WORKSPACE_ID).await?;
    if !existing_projects.is_empty() {
        return Ok(());
    }

    runtime
        .ensure_project_context(
            DEV_WORKSPACE_ID,
            DEV_WORKSPACE_SLUG,
            DEV_WORKSPACE_NAME,
            DEV_PROJECT_ID,
            DEV_PROJECT_SLUG,
            DEV_PROJECT_NAME,
        )
        .await?;
    runtime
        .ensure_project_knowledge_space(
            DEV_WORKSPACE_ID,
            DEV_PROJECT_ID,
            DEV_KNOWLEDGE_SPACE_NAME,
            DEV_KNOWLEDGE_OWNER_REF,
        )
        .await?;
    runtime
        .upsert_capability_descriptor(CapabilityDescriptorRecord::new(
            DEV_CAPABILITY_ID,
            DEV_CAPABILITY_ID,
            "low",
            false,
        ))
        .await?;
    runtime
        .upsert_capability_binding(CapabilityBindingRecord::project_scope(
            format!("binding-{DEV_PROJECT_ID}"),
            DEV_CAPABILITY_ID,
            DEV_WORKSPACE_ID,
            DEV_PROJECT_ID,
        ))
        .await?;
    runtime
        .upsert_capability_grant(CapabilityGrantRecord::project_scope(
            format!("grant-{DEV_PROJECT_ID}"),
            DEV_CAPABILITY_ID,
            DEV_WORKSPACE_ID,
            DEV_PROJECT_ID,
        ))
        .await?;
    runtime
        .upsert_budget_policy(BudgetPolicyRecord::project_scope(
            format!("budget-{DEV_PROJECT_ID}"),
            DEV_WORKSPACE_ID,
            DEV_PROJECT_ID,
            DEV_BUDGET_SOFT_LIMIT,
            DEV_BUDGET_HARD_LIMIT,
        ))
        .await?;

    Ok(())
}
