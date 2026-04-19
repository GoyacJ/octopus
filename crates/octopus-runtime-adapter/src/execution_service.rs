#![allow(clippy::large_futures)]

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
    async fn run_generation(
        &self,
        input: RunRuntimeGenerationInput,
        user_id: &str,
    ) -> Result<RuntimeGenerationResult, AppError> {
        let (resolved_target, configured_model) =
            self.resolve_generation_execution(input.project_id.as_deref(), user_id, &input)?;
        let reservation_id = format!("generation-{}", Uuid::new_v4());
        self.reserve_configured_model_budget(
            &reservation_id,
            &configured_model,
            crate::model_budget::BUDGET_TRAFFIC_CLASS_INTERACTIVE_TURN,
            timestamp_now(),
        )?;
        let response = match self
            .execute_resolved_prompt(
                &resolved_target,
                &input.content,
                input.system_prompt.as_deref(),
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.release_configured_model_budget_reservation(&reservation_id, timestamp_now())?;
                return Err(error);
            }
        };
        let consumed_tokens = match self.resolve_consumed_tokens(&configured_model, &response) {
            Ok(consumed_tokens) => consumed_tokens,
            Err(error) => {
                self.release_configured_model_budget_reservation(&reservation_id, timestamp_now())?;
                return Err(error);
            }
        };
        let now = timestamp_now();
        self.state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: input.project_id.clone(),
                run_id: None,
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                metric: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("turns")
                    .into(),
                amount: response.total_tokens.map(i64::from).unwrap_or(1),
                unit: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("count")
                    .into(),
                created_at: now,
            })
            .await?;
        self.settle_configured_model_budget_reservation(
            &reservation_id,
            &resolved_target.configured_model_id,
            consumed_tokens.unwrap_or(0),
            now,
        )?;

        Ok(RuntimeGenerationResult {
            configured_model_id: resolved_target.configured_model_id,
            configured_model_name: configured_model.name,
            content: response.content,
            request_id: response.request_id,
            consumed_tokens,
        })
    }

    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        agent_runtime_core::AgentRuntimeCore::submit_turn(self, session_id, input).await
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

    async fn cancel_subrun(
        &self,
        session_id: &str,
        subrun_id: &str,
        input: CancelRuntimeSubrunInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        agent_runtime_core::AgentRuntimeCore::cancel_subrun(self, session_id, subrun_id, input)
            .await
    }

    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Receiver<RuntimeEventEnvelope>, AppError> {
        Ok(self.session_sender(session_id)?.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use super::{approval_decision_status, requires_approval};

    #[test]
    fn normalizes_approval_decisions_and_permission_helpers() {
        assert_eq!(
            approval_decision_status("approve").expect("approve"),
            "approved"
        );
        assert!(!requires_approval("workspace-write").expect("workspace-write"));
        assert!(!requires_approval("danger-full-access").expect("danger-full-access"));
        assert!(approval_decision_status("nope").is_err());
    }
}
