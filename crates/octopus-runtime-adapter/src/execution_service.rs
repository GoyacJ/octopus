use super::*;

#[cfg(test)]
pub(crate) fn approval_decision_status(decision: &str) -> Result<&'static str, AppError> {
    approval_flow::approval_decision_status(decision)
}

#[cfg(test)]
pub(crate) fn requires_approval(permission_mode: &str) -> Result<bool, AppError> {
    execution_target::requires_approval(permission_mode)
}

#[async_trait]
impl RuntimeExecutionService for RuntimeAdapter {
    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        turn_submit::submit_turn(self, session_id, input).await
    }

    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        approval_flow::resolve_approval(self, session_id, approval_id, input).await
    }

    async fn resolve_auth_challenge(
        &self,
        session_id: &str,
        challenge_id: &str,
        input: ResolveRuntimeAuthChallengeInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        approval_flow::resolve_auth_challenge(self, session_id, challenge_id, input).await
    }

    async fn resolve_memory_proposal(
        &self,
        session_id: &str,
        proposal_id: &str,
        input: ResolveRuntimeMemoryProposalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        approval_flow::resolve_memory_proposal(self, session_id, proposal_id, input).await
    }

    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Receiver<RuntimeEventEnvelope>, AppError> {
        Ok(self.session_sender(session_id)?.subscribe())
    }
}
