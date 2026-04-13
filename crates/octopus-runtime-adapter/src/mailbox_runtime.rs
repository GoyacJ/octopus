use super::*;

pub(crate) fn build_mailbox_summary(
    mailbox_ref: &str,
    status: &str,
    total_messages: u64,
    now: u64,
) -> RuntimeMailboxSummary {
    RuntimeMailboxSummary {
        mailbox_ref: mailbox_ref.to_string(),
        channel: "team-mailbox".into(),
        status: status.to_string(),
        pending_count: if status == "completed" { 0 } else { total_messages },
        total_messages,
        updated_at: now,
    }
}
