use std::future::Future;

use async_trait::async_trait;
use octopus_sdk_contracts::{SessionId, SubagentError, SubagentOutput, SubagentSpec, ToolCallId};

tokio::task_local! {
    static TASK_PARENT_CONTEXT: TaskParentContext;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskParentContext {
    pub session_id: SessionId,
    pub tool_call_id: Option<ToolCallId>,
}

#[async_trait]
pub trait TaskFn: Send + Sync {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError>;
}

pub async fn with_task_parent_context<F>(
    session_id: SessionId,
    tool_call_id: Option<ToolCallId>,
    future: F,
) -> F::Output
where
    F: Future,
{
    TASK_PARENT_CONTEXT
        .scope(
            TaskParentContext {
                session_id,
                tool_call_id,
            },
            future,
        )
        .await
}

pub async fn with_task_parent_session<F>(session_id: SessionId, future: F) -> F::Output
where
    F: Future,
{
    with_task_parent_context(session_id, None, future).await
}

#[must_use]
pub fn current_task_parent_context() -> Option<TaskParentContext> {
    TASK_PARENT_CONTEXT.try_with(Clone::clone).ok()
}

#[must_use]
pub fn current_task_parent_session() -> Option<SessionId> {
    current_task_parent_context().map(|context| context.session_id)
}
