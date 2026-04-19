use async_trait::async_trait;
use octopus_core::{
    AppError, ResolvedExecutionTarget, ResolvedRequestPolicy, RuntimeExecutionClass,
    RuntimeExecutionProfile,
};

use super::{RuntimeConversationExecution, RuntimeConversationRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationModelDriverCapability {
    pub tool_loop: bool,
    pub upstream_streaming: bool,
}

impl ConversationModelDriverCapability {
    pub const fn execution_profile(self) -> RuntimeExecutionProfile {
        let execution_class = if self.tool_loop && self.upstream_streaming {
            RuntimeExecutionClass::AgentConversation
        } else {
            RuntimeExecutionClass::Unsupported
        };

        RuntimeExecutionProfile {
            execution_class,
            tool_loop: self.tool_loop,
            upstream_streaming: self.upstream_streaming,
        }
    }
}

#[async_trait]
pub trait ConversationModelDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
    fn capability(&self) -> ConversationModelDriverCapability;

    fn execution_profile(&self) -> RuntimeExecutionProfile {
        self.capability().execution_profile()
    }

    async fn execute_conversation(
        &self,
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError>;
}
