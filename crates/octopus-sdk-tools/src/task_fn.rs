use std::future::Future;

use async_trait::async_trait;
use octopus_sdk_contracts::{SessionId, SubagentError, SubagentOutput, SubagentSpec};

tokio::task_local! {
    static TASK_PARENT_SESSION_ID: SessionId;
}

#[async_trait]
pub trait TaskFn: Send + Sync {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError>;
}

pub async fn with_task_parent_session<F>(session_id: SessionId, future: F) -> F::Output
where
    F: Future,
{
    TASK_PARENT_SESSION_ID.scope(session_id, future).await
}

#[must_use]
pub fn current_task_parent_session() -> Option<SessionId> {
    TASK_PARENT_SESSION_ID.try_with(Clone::clone).ok()
}
