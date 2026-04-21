use std::collections::HashMap;

use async_trait::async_trait;
use octopus_sdk_mcp::{
    JsonRpcRequest, JsonRpcResponse, McpError, McpServerManager, McpServerSpec, McpServerTransport,
    McpTransport,
};
use serde_json::json;

struct MockTransport;

#[async_trait]
impl McpTransport for MockTransport {
    async fn call(&self, _req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        Ok(JsonRpcResponse::success(
            json!(1),
            json!({ "protocolVersion": "2025-03-26" }),
        ))
    }
}

#[tokio::test]
async fn stdio_process_cleanup_on_drop() {
    let marker = format!("lifecycle-{}", std::process::id());

    for index in 0..10 {
        let manager = McpServerManager::new();
        manager
            .spawn(McpServerSpec {
                server_id: format!("server-{index}"),
                transport: McpServerTransport::Stdio {
                    command: env!("CARGO_BIN_EXE_mcp-echo-server").into(),
                    args: vec!["--marker".into(), marker.clone()],
                    env: HashMap::new(),
                    transport: std::sync::Arc::new(MockTransport),
                },
            })
            .await
            .expect("spawn should succeed");
        drop(manager);
    }

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ps -eo pid,args | grep -c '[m]cp-echo-server --marker {marker}' || true"
        ))
        .output()
        .expect("ps should run");
    let count = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .expect("count should parse");

    assert_eq!(count, 0);
}
