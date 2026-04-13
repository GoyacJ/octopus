use super::*;

pub(crate) fn apply_team_runtime_projection(
    detail: &mut RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    now: u64,
) {
    let workflow_run_id = format!("workflow-{}", detail.run.id);
    let mailbox_ref = format!("mailbox-{}", detail.run.id);
    let (subruns, worker_dispatch) = subrun_orchestrator::build_subrun_projection(
        team,
        &detail.run,
        &workflow_run_id,
        &mailbox_ref,
        now,
    );
    let handoffs = handoff_runtime::build_handoff_projection(&detail.run, &subruns, &mailbox_ref, now);
    let mailbox = mailbox_runtime::build_mailbox_summary(
        &mailbox_ref,
        &detail.run.status,
        handoffs.len() as u64,
        now,
    );
    let (workflow, workflow_detail) = workflow_runtime::build_workflow_projection(
        team,
        &detail.run,
        &workflow_run_id,
        subruns.len() as u64,
        now,
    );
    let background = background_runtime::build_background_summary(
        &detail.run,
        &workflow_run_id,
        workflow.background_capable,
        now,
    );

    detail.subrun_count = subruns.len() as u64;
    detail.subruns = subruns;
    detail.handoffs = handoffs.clone();
    detail.workflow = Some(workflow.clone());
    detail.pending_mailbox = Some(mailbox.clone());
    detail.background_run = Some(background.clone());

    detail.run.workflow_run = Some(workflow_run_id);
    detail.run.workflow_run_detail = Some(workflow_detail);
    detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
    detail.run.handoff_ref = handoffs.first().map(|handoff| handoff.handoff_ref.clone());
    detail.run.background_state = Some(background.status.clone());
    detail.run.worker_dispatch = Some(worker_dispatch);
}
