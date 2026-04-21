use async_trait::async_trait;

use crate::{ToolContext, ToolError, ToolResult, ToolSpec};

#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn is_concurrency_safe(&self, input: &serde_json::Value) -> bool;
    async fn execute(
        &self,
        ctx: ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult, ToolError>;
}
