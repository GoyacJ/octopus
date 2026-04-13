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
