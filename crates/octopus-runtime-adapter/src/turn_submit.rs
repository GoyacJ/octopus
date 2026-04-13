use super::*;

pub(super) async fn submit_turn(
    adapter: &RuntimeAdapter,
    session_id: &str,
    input: SubmitRuntimeTurnInput,
) -> Result<RuntimeRunSnapshot, AppError> {
    agent_runtime_core::AgentRuntimeCore::submit_turn(adapter, session_id, input).await
}
