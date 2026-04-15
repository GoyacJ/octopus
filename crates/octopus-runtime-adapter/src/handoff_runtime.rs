use super::*;

pub(crate) fn handoff_state_for_subrun(
    subruns: &[RuntimeSubrunSummary],
    primary_run_blocked: bool,
    subrun_status: &str,
) -> &'static str {
    let all_outputs_consumed = !primary_run_blocked
        && !subruns.is_empty()
        && subruns.iter().all(|subrun| subrun.status == "completed");

    match subrun_status {
        "completed" if all_outputs_consumed => "acknowledged",
        "completed" => "delivered",
        "cancelled" => "cancelled",
        "failed" => "failed",
        _ => "queued",
    }
}

pub(crate) fn handoff_actor_refs<'a>(
    parent_actor_ref: &'a str,
    worker_actor_ref: &'a str,
    handoff_state: &str,
) -> (&'a str, &'a str) {
    match handoff_state {
        "queued" => (parent_actor_ref, worker_actor_ref),
        _ => (worker_actor_ref, parent_actor_ref),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_subrun(run_id: &str, actor_ref: &str, status: &str, updated_at: u64) -> RuntimeSubrunSummary {
        RuntimeSubrunSummary {
            run_id: run_id.into(),
            parent_run_id: Some("parent-run".into()),
            actor_ref: actor_ref.into(),
            label: actor_ref.into(),
            status: status.into(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some(format!("tool-{run_id}")),
            workflow_run_id: Some("workflow-test".into()),
            mailbox_ref: Some("mailbox-test".into()),
            handoff_ref: Some(format!("handoff-{run_id}")),
            started_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn completed_handoffs_become_acknowledged_after_parent_consumes_all_outputs() {
        let subruns = vec![
            test_subrun("run-a", "agent:worker-a", "completed", 10),
            test_subrun("run-b", "agent:worker-b", "completed", 20),
        ];

        let state = handoff_state_for_subrun(&subruns, false, "completed");
        assert_eq!(state, "acknowledged");
    }

    #[test]
    fn mailbox_pending_count_keeps_delivered_handoffs_unacked_until_parent_consumes_them() {
        let handoffs = vec![
            RuntimeHandoffSummary {
                handoff_ref: "handoff-a".into(),
                mailbox_ref: "mailbox-test".into(),
                sender_actor_ref: "agent:worker-a".into(),
                receiver_actor_ref: "agent:leader".into(),
                state: "delivered".into(),
                artifact_refs: Vec::new(),
                updated_at: 10,
            },
            RuntimeHandoffSummary {
                handoff_ref: "handoff-b".into(),
                mailbox_ref: "mailbox-test".into(),
                sender_actor_ref: "agent:leader".into(),
                receiver_actor_ref: "agent:worker-b".into(),
                state: "queued".into(),
                artifact_refs: Vec::new(),
                updated_at: 20,
            },
        ];

        let summary = mailbox_runtime::summarize_handoffs("mailbox-test", "leader-hub", &handoffs, 30);
        assert_eq!(summary.status, "pending");
        assert_eq!(summary.pending_count, 2);
        assert_eq!(summary.total_messages, 2);
    }
}
