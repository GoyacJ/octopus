use async_trait::async_trait;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

pub struct WebSearchTool {
    spec: ToolSpec,
}

impl WebSearchTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "web_search".into(),
                description: "[STUB · W6] Search the web for current information through a provider injected in W6.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": { "type": "string" },
                        "count": { "type": "integer", "minimum": 1 }
                    }
                }),
                category: ToolCategory::Network,
            },
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::NotYetImplemented {
            crate_name: "octopus-sdk-tools::web_search",
            week: "W6",
        })
    }
}
