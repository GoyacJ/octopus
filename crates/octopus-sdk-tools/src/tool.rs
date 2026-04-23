use async_trait::async_trait;

use crate::{
    ToolContext, ToolDisplayDescriptor, ToolError, ToolOutputFormat, ToolResult, ToolSpec,
};

#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn version(&self) -> Option<&str> {
        None
    }
    fn output_format(&self) -> ToolOutputFormat {
        ToolOutputFormat::Concise
    }
    fn display_descriptor(&self) -> Option<ToolDisplayDescriptor> {
        None
    }
    fn validate(&self, _input: &serde_json::Value) -> Result<(), ToolError> {
        Ok(())
    }
    fn is_concurrency_safe(&self, input: &serde_json::Value) -> bool;
    async fn execute(
        &self,
        ctx: ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult, ToolError>;
}
