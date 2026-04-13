use super::*;

pub(crate) fn build_workflow_projection(
    team: &actor_manifest::CompiledTeamManifest,
    run: &RuntimeRunSnapshot,
    workflow_run_id: &str,
    subrun_count: u64,
    now: u64,
) -> (RuntimeWorkflowSummary, RuntimeWorkflowRunDetail) {
    let total_steps = subrun_count + 1;
    let completed_steps = if run.status == "completed" {
        total_steps
    } else if subrun_count > 0 {
        1
    } else {
        0
    };
    let current_step_id = if run.status == "completed" {
        Some("workflow-complete".into())
    } else {
        Some("leader-plan".into())
    };
    let current_step_label = if run.status == "completed" {
        Some("Workflow complete".into())
    } else {
        Some("Leader plan".into())
    };
    let summary = RuntimeWorkflowSummary {
        workflow_run_id: workflow_run_id.to_string(),
        label: format!("{} workflow", team.record.name),
        status: run.status.clone(),
        total_steps,
        completed_steps,
        current_step_id: current_step_id.clone(),
        current_step_label: current_step_label.clone(),
        background_capable: team.record.workflow_affordance.background_capable,
        updated_at: now,
    };
    let detail = RuntimeWorkflowRunDetail {
        workflow_run_id: workflow_run_id.to_string(),
        status: run.status.clone(),
        current_step_id,
        current_step_label,
        total_steps,
        completed_steps,
        background_capable: team.record.workflow_affordance.background_capable,
    };
    (summary, detail)
}
