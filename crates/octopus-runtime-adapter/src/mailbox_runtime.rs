use super::*;

pub(crate) fn mailbox_channel(team: &actor_manifest::CompiledTeamManifest) -> String {
    team.record.mailbox_policy.mode.clone()
}

pub(crate) fn build_mailbox_summary(
    mailbox_ref: &str,
    channel: &str,
    status: &str,
    total_messages: u64,
    now: u64,
) -> RuntimeMailboxSummary {
    RuntimeMailboxSummary {
        mailbox_ref: mailbox_ref.to_string(),
        channel: channel.to_string(),
        status: status.to_string(),
        pending_count: if status == "completed" {
            0
        } else {
            total_messages
        },
        total_messages,
        updated_at: now,
    }
}

pub(crate) fn summarize_handoffs(
    mailbox_ref: &str,
    channel: &str,
    handoffs: &[RuntimeHandoffSummary],
    now: u64,
) -> RuntimeMailboxSummary {
    let has_terminal_failure = handoffs
        .iter()
        .any(|handoff| matches!(handoff.state.as_str(), "failed" | "cancelled"));
    let pending_count = handoffs
        .iter()
        .filter(|handoff| {
            !matches!(
                handoff.state.as_str(),
                "acknowledged" | "failed" | "cancelled"
            )
        })
        .count() as u64;
    let status = if has_terminal_failure {
        "failed"
    } else if pending_count == 0 {
        "completed"
    } else {
        "pending"
    };

    RuntimeMailboxSummary {
        mailbox_ref: mailbox_ref.to_string(),
        channel: channel.to_string(),
        status: status.to_string(),
        pending_count: if has_terminal_failure {
            0
        } else {
            pending_count
        },
        total_messages: handoffs.len() as u64,
        updated_at: now,
    }
}
