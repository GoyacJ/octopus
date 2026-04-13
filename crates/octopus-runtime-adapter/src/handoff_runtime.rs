use super::*;

pub(crate) fn build_handoff_projection(
    run: &RuntimeRunSnapshot,
    subruns: &[RuntimeSubrunSummary],
    mailbox_ref: &str,
    now: u64,
) -> Vec<RuntimeHandoffSummary> {
    subruns
        .iter()
        .map(|subrun| RuntimeHandoffSummary {
            handoff_ref: subrun
                .handoff_ref
                .clone()
                .unwrap_or_else(|| format!("handoff-{}", subrun.run_id)),
            mailbox_ref: mailbox_ref.to_string(),
            sender_actor_ref: run.actor_ref.clone(),
            receiver_actor_ref: subrun.actor_ref.clone(),
            state: if run.status == "completed" {
                "delivered".into()
            } else {
                "pending".into()
            },
            artifact_refs: vec![format!("artifact-{}", subrun.run_id)],
            updated_at: now,
        })
        .collect()
}
