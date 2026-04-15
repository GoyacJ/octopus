use super::*;

fn continuation_state(workflow_status: &str, background_capable: bool) -> String {
    if !background_capable {
        return "disabled".into();
    }
    match workflow_status {
        "waiting_approval" | "auth-required" => "paused".into(),
        "completed" => "completed".into(),
        "failed" => "failed".into(),
        "queued" => "queued".into(),
        _ => "running".into(),
    }
}

pub(crate) fn build_background_summary(
    run_id: &str,
    workflow_run_id: &str,
    workflow_status: &str,
    background_capable: bool,
    blocking: Option<RuntimeWorkflowBlockingSummary>,
    now: u64,
) -> RuntimeBackgroundRunSummary {
    RuntimeBackgroundRunSummary {
        run_id: run_id.to_string(),
        workflow_run_id: Some(workflow_run_id.to_string()),
        status: workflow_status.to_string(),
        background_capable,
        continuation_state: continuation_state(workflow_status, background_capable),
        blocking,
        updated_at: now,
    }
}
