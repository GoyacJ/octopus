//! Runtime-layer primitives for resume and waiting flows.

use octopus_application::RunEnvelope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeDescriptor {
    pub run_id: String,
    pub resume_token: String,
    pub idempotency_key: String,
}

pub fn can_resume(run: &RunEnvelope, descriptor: &ResumeDescriptor) -> bool {
    !run.run_id.is_empty()
        && run.run_id == descriptor.run_id
        && !descriptor.resume_token.is_empty()
        && !descriptor.idempotency_key.is_empty()
}

#[cfg(test)]
mod tests {
    use octopus_application::RunEnvelope;
    use octopus_domain::{AgentId, AgentRef, RunStatus};
    use octopus_shared::WorkspaceId;

    use super::{can_resume, ResumeDescriptor};

    #[test]
    fn rejects_empty_resume_inputs() {
        let run = RunEnvelope {
            run_id: "run-1".to_owned(),
            agent: AgentRef {
                workspace_id: WorkspaceId("workspace-1".to_owned()),
                agent_id: AgentId("agent-1".to_owned()),
            },
            status: RunStatus::WaitingInput,
        };

        let descriptor = ResumeDescriptor {
            run_id: "run-1".to_owned(),
            resume_token: String::new(),
            idempotency_key: "idempotency-key".to_owned(),
        };

        assert!(!can_resume(&run, &descriptor));
    }
}
