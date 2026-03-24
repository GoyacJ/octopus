//! Domain-layer types and lifecycle primitives.

use octopus_shared::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRef {
    pub workspace_id: WorkspaceId,
    pub agent_id: AgentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    WaitingInput,
    WaitingApproval,
    WaitingTrigger,
    Completed,
    Failed,
    Canceled,
}

impl RunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingInput => "waiting_input",
            Self::WaitingApproval => "waiting_approval",
            Self::WaitingTrigger => "waiting_trigger",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RunStatus;

    #[test]
    fn exposes_runtime_status_labels() {
        assert_eq!(RunStatus::WaitingApproval.as_str(), "waiting_approval");
    }
}
