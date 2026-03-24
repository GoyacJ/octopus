//! Application-layer orchestration inputs and outputs.

use octopus_domain::{AgentRef, RunStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEnvelope {
    pub run_id: String,
    pub agent: AgentRef,
    pub status: RunStatus,
}
