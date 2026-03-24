//! Domain-layer types and lifecycle primitives for the Phase 3 MVP slice.

use std::{fmt, str::FromStr};

use octopus_shared::WorkspaceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRef {
    pub workspace_id: WorkspaceId,
    pub agent_id: AgentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    WaitingInput,
    WaitingApproval,
    WaitingTrigger,
    Suspended,
    Resuming,
    Completed,
    Failed,
    Canceled,
    Handoff,
}

impl RunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingInput => "waiting_input",
            Self::WaitingApproval => "waiting_approval",
            Self::WaitingTrigger => "waiting_trigger",
            Self::Suspended => "suspended",
            Self::Resuming => "resuming",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::Handoff => "handoff",
        }
    }
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "waiting_input" => Ok(Self::WaitingInput),
            "waiting_approval" => Ok(Self::WaitingApproval),
            "waiting_trigger" => Ok(Self::WaitingTrigger),
            "suspended" => Ok(Self::Suspended),
            "resuming" => Ok(Self::Resuming),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "canceled" => Ok(Self::Canceled),
            "handoff" => Ok(Self::Handoff),
            _ => Err(format!("unknown run status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    AskUser,
    Approval,
}

impl InteractionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AskUser => "ask_user",
            Self::Approval => "approval",
        }
    }
}

impl fmt::Display for InteractionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InteractionKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ask_user" => Ok(Self::AskUser),
            "approval" => Ok(Self::Approval),
            _ => Err(format!("unknown interaction kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionResponseType {
    SingleSelect,
    MultiSelect,
    Text,
    Approval,
}

impl InteractionResponseType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleSelect => "single_select",
            Self::MultiSelect => "multi_select",
            Self::Text => "text",
            Self::Approval => "approval",
        }
    }
}

impl fmt::Display for InteractionResponseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InteractionResponseType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "single_select" => Ok(Self::SingleSelect),
            "multi_select" => Ok(Self::MultiSelect),
            "text" => Ok(Self::Text),
            "approval" => Ok(Self::Approval),
            _ => Err(format!("unknown interaction response type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxItemStatus {
    Pending,
    Resolved,
}

impl InboxItemStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
        }
    }
}

impl fmt::Display for InboxItemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InboxItemStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "resolved" => Ok(Self::Resolved),
            _ => Err(format!("unknown inbox item status: {value}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractionKind, InteractionResponseType, RunStatus};

    #[test]
    fn exposes_runtime_status_labels() {
        assert_eq!(RunStatus::WaitingApproval.as_str(), "waiting_approval");
        assert_eq!(RunStatus::Resuming.as_str(), "resuming");
        assert_eq!(RunStatus::Handoff.as_str(), "handoff");
    }

    #[test]
    fn parses_phase_three_domain_enums() {
        assert_eq!(
            "ask_user".parse::<InteractionKind>().unwrap(),
            InteractionKind::AskUser
        );
        assert_eq!(
            "approval".parse::<InteractionResponseType>().unwrap(),
            InteractionResponseType::Approval
        );
    }
}
