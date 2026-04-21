use std::collections::HashMap;

use octopus_sdk_mcp::{HttpTransport, McpClient};
use serde_json::json;
use wiremock::{
    matchers::{body_partial_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn http_transport_posts_jsonrpc_and_keeps_auth_header() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(header("authorization", "Bearer token"))
        .and(body_partial_json(json!({ "method": "initialize" })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/initialize_response.json"),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(header("authorization", "Bearer token"))
        .and(body_partial_json(json!({ "method": "tools/list" })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/tools_list_response.json"),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(header("authorization", "Bearer token"))
        .and(body_partial_json(json!({ "method": "tools/call" })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/tools_call_response.json"),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let mut headers = HashMap::new();
    headers.insert("Authorization".into(), "Bearer token".into());

    let transport = HttpTransport::new(server.uri(), headers).expect("http transport should build");
    let client = McpClient::new("http", std::sync::Arc::new(transport));

    let tools = client
        .list_tools()
        .await
        .expect("list tools should succeed");
    let result = client
        .call_tool("grep", json!({ "pattern": "foo" }))
        .await
        .expect("tool call should succeed");

    assert_eq!(tools[0].name, "grep");
    assert!(!result.is_error);
}
