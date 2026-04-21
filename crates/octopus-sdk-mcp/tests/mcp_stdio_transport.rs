use octopus_sdk_mcp::{McpClient, StdioTransport};
use serde_json::json;

#[tokio::test]
async fn stdio_transport_round_trips_initialize_list_and_call() {
    let transport = StdioTransport::spawn(
        env!("CARGO_BIN_EXE_mcp-echo-server"),
        std::iter::empty::<String>(),
        std::collections::HashMap::new(),
    )
    .expect("stdio transport should spawn");
    let client = McpClient::new("echo", std::sync::Arc::new(transport));

    let init = client
        .initialize()
        .await
        .expect("initialize should succeed");
    let tools = client
        .list_tools()
        .await
        .expect("list tools should succeed");
    let result = client
        .call_tool("echo", json!({ "message": "hello" }))
        .await
        .expect("tool call should succeed");

    assert_eq!(init.protocol_version, "2025-03-26");
    assert_eq!(tools[0].name, "echo");
    assert_eq!(result.content.len(), 1);
}
