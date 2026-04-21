use std::{sync::Arc, time::Instant};

use octopus_sdk_mcp::{McpClient, SdkTransport};
use octopus_sdk_tools::{builtin::SleepTool, ToolRegistry};

#[tokio::test]
async fn mcp_sdk_shim_roundtrip_reaches_builtin_sleep() {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(SleepTool::new()))
        .expect("sleep tool should register");

    let transport = SdkTransport::from_directory(registry.as_directory());
    let client = McpClient::new("sdk-shim", Arc::new(transport));

    let tools = client
        .list_tools()
        .await
        .expect("list tools should succeed");
    let started = Instant::now();
    let result = client
        .call_tool("sleep", serde_json::json!({ "ms": 10 }))
        .await
        .expect("sleep tool call should succeed");

    assert_eq!(tools[0].name, "sleep");
    assert!(!result.is_error);
    assert!(started.elapsed().as_millis() >= 10);
}
