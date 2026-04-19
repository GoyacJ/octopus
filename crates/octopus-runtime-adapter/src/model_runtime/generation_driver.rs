use async_trait::async_trait;
use octopus_core::{
    AppError, ResolvedExecutionTarget, ResolvedRequestPolicy, RuntimeExecutionClass,
    RuntimeExecutionProfile,
};

use super::ModelExecutionResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationModelDriverCapability {
    pub prompt: bool,
}

impl GenerationModelDriverCapability {
    pub const fn execution_profile(self) -> RuntimeExecutionProfile {
        let execution_class = if self.prompt {
            RuntimeExecutionClass::SingleShotGeneration
        } else {
            RuntimeExecutionClass::Unsupported
        };

        RuntimeExecutionProfile {
            execution_class,
            tool_loop: false,
            upstream_streaming: false,
        }
    }
}

#[async_trait]
pub trait GenerationModelDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
    fn capability(&self) -> GenerationModelDriverCapability;

    fn execution_profile(&self) -> RuntimeExecutionProfile {
        self.capability().execution_profile()
    }

    async fn execute_prompt(
        &self,
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError>;
}
