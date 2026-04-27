#![cfg(feature = "http")]

use std::collections::BTreeMap;

use harness_contracts::{McpServerId, McpServerSource};
use harness_mcp::{HttpTransport, McpClient, McpClientAuth, McpServerSpec, TransportChoice};
use serde_json::json;
use wiremock::{
    matchers::{body_partial_json, header, method},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn http_transport_posts_jsonrpc_with_headers_and_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("x-mcp-client", "octopus"))
        .and(header("authorization", "Bearer token"))
        .and(body_partial_json(json!({ "method": "initialize" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "fixture", "version": "0.1.0" }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            json!({ "method": "notifications/initialized" }),
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "tools/list" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    { "name": "search", "description": "Search docs", "inputSchema": { "type": "object" } }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "tools/call" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": { "content": [{ "type": "text", "text": "found" }], "isError": false }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let mut headers = BTreeMap::new();
    headers.insert("x-mcp-client".to_owned(), "octopus".to_owned());
    let mut spec = McpServerSpec::new(
        McpServerId("http".into()),
        "http fixture",
        TransportChoice::Http {
            url: server.uri(),
            headers,
        },
        McpServerSource::Workspace,
    );
    spec.auth = McpClientAuth::Bearer("token".into());

    let connection = McpClient::new(std::sync::Arc::new(HttpTransport::new()))
        .connect(spec)
        .await
        .expect("http connects");

    let tools = connection.list_tools().await.expect("tools list");
    assert_eq!(tools[0].name, "search");

    let result = connection
        .call_tool("search", json!({ "q": "mcp" }))
        .await
        .expect("tool call");
    assert_eq!(result, harness_mcp::McpToolResult::text("found"));
}
