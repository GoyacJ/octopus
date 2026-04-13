use super::*;

pub(crate) fn build_background_summary(
    run: &RuntimeRunSnapshot,
    workflow_run_id: &str,
    background_capable: bool,
    now: u64,
) -> RuntimeBackgroundRunSummary {
    RuntimeBackgroundRunSummary {
        run_id: run.id.clone(),
        workflow_run_id: Some(workflow_run_id.to_string()),
        status: run.status.clone(),
        background_capable,
        updated_at: now,
    }
}
