#![allow(clippy::large_futures)]

use super::*;

pub(crate) fn approval_decision_status(decision: &str) -> Result<&'static str, AppError> {
    match decision {
        "approve" => Ok("approved"),
        "reject" => Ok("rejected"),
        _ => Err(AppError::invalid_input(
            "approval decision must be approve or reject",
        )),
    }
}

pub(crate) fn memory_proposal_decision_status(decision: &str) -> Result<&'static str, AppError> {
    memory_runtime::memory_proposal_state_from_decision(decision)
}

pub(crate) fn auth_challenge_resolution_status(resolution: &str) -> Result<&'static str, AppError> {
    match resolution {
        "resolved" => Ok("resolved"),
        "failed" => Ok("failed"),
        "cancelled" => Ok("cancelled"),
        _ => Err(AppError::invalid_input(
            "auth challenge resolution must be resolved, failed, or cancelled",
        )),
    }
}

pub(super) async fn resolve_approval(
    adapter: &RuntimeAdapter,
    session_id: &str,
    approval_id: &str,
    input: ResolveRuntimeApprovalInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    agent_runtime_core::AgentRuntimeCore::resume_after_approval(
        adapter,
        session_id,
        approval_id,
        input,
    )
    .await
}

pub(super) async fn resolve_auth_challenge(
    adapter: &RuntimeAdapter,
    session_id: &str,
    challenge_id: &str,
    input: ResolveRuntimeAuthChallengeInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    agent_runtime_core::AgentRuntimeCore::resolve_auth_challenge(
        adapter,
        session_id,
        challenge_id,
        input,
    )
    .await
}

pub(super) async fn resolve_memory_proposal(
    adapter: &RuntimeAdapter,
    session_id: &str,
    proposal_id: &str,
    input: ResolveRuntimeMemoryProposalInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    agent_runtime_core::AgentRuntimeCore::resolve_memory_proposal(
        adapter,
        session_id,
        proposal_id,
        input,
    )
    .await
}
