use super::*;

pub(crate) fn build_subrun_projection(
    team: &actor_manifest::CompiledTeamManifest,
    run: &RuntimeRunSnapshot,
    workflow_run_id: &str,
    mailbox_ref: &str,
    now: u64,
) -> (Vec<RuntimeSubrunSummary>, RuntimeWorkerDispatchSummary) {
    let subruns = worker_runtime::worker_actor_refs(team)
        .into_iter()
        .enumerate()
        .map(|(index, actor_ref)| RuntimeSubrunSummary {
            run_id: format!("{}-subrun-{}", run.id, index + 1),
            parent_run_id: Some(run.id.clone()),
            actor_ref: actor_ref.clone(),
            label: worker_runtime::worker_label(&actor_ref),
            status: run.status.clone(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some(format!("team-dispatch-{}", index + 1)),
            workflow_run_id: Some(workflow_run_id.to_string()),
            mailbox_ref: Some(mailbox_ref.to_string()),
            handoff_ref: Some(format!("handoff-{}-{}", run.id, index + 1)),
            started_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();

    let total_subruns = subruns.len() as u64;
    let dispatch = RuntimeWorkerDispatchSummary {
        total_subruns,
        active_subruns: if run.status == "running" { total_subruns } else { 0 },
        completed_subruns: if run.status == "completed" {
            total_subruns
        } else {
            0
        },
        failed_subruns: if run.status == "failed" { total_subruns } else { 0 },
    };

    (subruns, dispatch)
}
