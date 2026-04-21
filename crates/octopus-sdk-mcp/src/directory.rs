use async_trait::async_trait;

use crate::{McpError, McpTool, McpToolResult};

#[async_trait]
pub trait ToolDirectory: Send + Sync {
    fn list_tools(&self) -> Vec<McpTool>;

    async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<McpToolResult, McpError>;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use octopus_sdk_contracts::ContentBlock;
    use serde_json::json;

    use super::ToolDirectory;
    use crate::{McpError, McpTool, McpToolResult};

    struct DummyDirectory;

    #[async_trait]
    impl ToolDirectory for DummyDirectory {
        fn list_tools(&self) -> Vec<McpTool> {
            vec![McpTool {
                name: "grep".into(),
                description: "Search files".into(),
                input_schema: json!({ "type": "object" }),
            }]
        }

        async fn call_tool(
            &self,
            _name: &str,
            _input: serde_json::Value,
        ) -> Result<McpToolResult, McpError> {
            Ok(McpToolResult {
                content: vec![ContentBlock::Text { text: "ok".into() }],
                is_error: false,
            })
        }
    }

    #[test]
    fn tool_directory_is_object_safe() {
        let directory: Arc<dyn ToolDirectory> = Arc::new(DummyDirectory);
        assert_eq!(directory.list_tools().len(), 1);
    }
}
