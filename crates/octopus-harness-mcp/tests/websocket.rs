#![cfg(feature = "websocket")]

use std::collections::BTreeMap;

use futures::{SinkExt, StreamExt};
use harness_contracts::{McpServerId, McpServerSource};
use harness_mcp::{McpChange, McpClient, McpServerSpec, TransportChoice, WebsocketTransport};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[tokio::test]
async fn websocket_transport_handles_requests_and_list_changed_notifications() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut socket = accept_async(stream).await.expect("websocket accept");
        while let Some(message) = socket.next().await {
            let text = message.expect("message").into_text().expect("text");
            let value: Value = serde_json::from_str(&text).expect("json");
            match value.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    socket
                        .send(Message::Text(
                            json!({
                                "jsonrpc": "2.0",
                                "id": value["id"].clone(),
                                "result": {
                                    "protocolVersion": "2025-03-26",
                                    "capabilities": { "tools": {} },
                                    "serverInfo": { "name": "fixture", "version": "0.1.0" }
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .expect("send initialize");
                }
                Some("tools/list") => {
                    socket
                        .send(Message::Text(json!({
                            "jsonrpc": "2.0",
                            "id": value["id"].clone(),
                            "result": {
                                "tools": [
                                    { "name": "lookup", "description": "Lookup", "inputSchema": { "type": "object" } }
                                ]
                            }
                        }).to_string()))
                        .await
                        .expect("send tools list");
                    socket
                        .send(Message::Text(
                            json!({
                                "jsonrpc": "2.0",
                                "method": "notifications/tools/list_changed"
                            })
                            .to_string(),
                        ))
                        .await
                        .expect("send list changed");
                }
                Some("tools/call") => {
                    socket
                        .send(Message::Text(
                            json!({
                                "jsonrpc": "2.0",
                                "id": value["id"].clone(),
                                "result": {
                                    "content": [{ "type": "text", "text": "looked up" }],
                                    "isError": false
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .expect("send tool result");
                }
                _ => {}
            }
        }
    });

    let spec = McpServerSpec::new(
        McpServerId("ws".into()),
        "websocket fixture",
        TransportChoice::WebSocket {
            url: format!("ws://{addr}"),
            headers: BTreeMap::default(),
        },
        McpServerSource::Workspace,
    );

    let connection = McpClient::new(std::sync::Arc::new(WebsocketTransport::new()))
        .connect(spec)
        .await
        .expect("websocket connects");
    let mut changes = connection.subscribe_changes().await.expect("changes");

    let tools = connection.list_tools().await.expect("tools list");
    assert_eq!(tools[0].name, "lookup");
    assert_eq!(changes.next().await, Some(McpChange::ToolsListChanged));

    let result = connection
        .call_tool("lookup", json!({ "id": 1 }))
        .await
        .expect("tool call");
    assert_eq!(result, harness_mcp::McpToolResult::text("looked up"));
}
