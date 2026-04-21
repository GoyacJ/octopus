use async_trait::async_trait;
use octopus_sdk_contracts::{SubagentError, SubagentOutput, SubagentSpec};

#[async_trait]
pub trait TaskFn: Send + Sync {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError>;
}
