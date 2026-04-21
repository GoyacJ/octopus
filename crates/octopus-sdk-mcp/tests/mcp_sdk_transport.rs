use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_contracts::ContentBlock;
use octopus_sdk_mcp::{McpClient, McpError, McpTool, McpToolResult, SdkTransport, ToolDirectory};
use serde_json::json;

struct DummyDirectory;

#[async_trait]
impl ToolDirectory for DummyDirectory {
    fn list_tools(&self) -> Vec<McpTool> {
        vec![McpTool {
            name: "sleep".into(),
            description: "Sleep".into(),
            input_schema: json!({ "type": "object" }),
        }]
    }

    async fn call_tool(
        &self,
        name: &str,
        _input: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        Ok(McpToolResult {
            content: vec![ContentBlock::Text {
                text: format!("{name}:ok"),
            }],
            is_error: false,
        })
    }
}

#[tokio::test]
async fn sdk_transport_round_trips_list_and_call() {
    let transport = SdkTransport::from_directory(Arc::new(DummyDirectory));
    let client = McpClient::new("sdk", Arc::new(transport));

    let tools = client
        .list_tools()
        .await
        .expect("list tools should succeed");
    let result = client
        .call_tool("sleep", json!({ "ms": 10 }))
        .await
        .expect("tool call should succeed");

    assert_eq!(tools[0].name, "sleep");
    assert_eq!(
        result.content,
        vec![ContentBlock::Text {
            text: "sleep:ok".into()
        }]
    );
}
