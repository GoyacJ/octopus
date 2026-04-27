#![cfg(feature = "stdio")]

use std::collections::{BTreeMap, BTreeSet};

use harness_contracts::{McpServerId, McpServerSource};
use harness_mcp::{
    McpClient, McpServerSpec, StdioEnv, StdioPolicy, StdioTransport, TransportChoice,
};
use serde_json::json;

#[tokio::test]
async fn stdio_transport_initializes_lists_and_calls_tool() {
    let script = r#"
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"0.1.0"}}}'
      ;;
    *'"method":"tools/list"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo input","inputSchema":{"type":"object"}}]}}'
      ;;
    *'"method":"tools/call"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"echo:hi"}],"isError":false}}'
      ;;
  esac
done
"#;
    let spec = McpServerSpec::new(
        McpServerId("stdio".into()),
        "stdio fixture",
        TransportChoice::Stdio {
            command: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            env: StdioEnv::default(),
            policy: StdioPolicy::default(),
        },
        McpServerSource::Workspace,
    );

    let connection = McpClient::new(std::sync::Arc::new(StdioTransport::new()))
        .connect(spec)
        .await
        .expect("stdio connects");

    let tools = connection.list_tools().await.expect("tools list");
    assert_eq!(tools[0].name, "echo");

    let result = connection
        .call_tool("echo", json!({ "text": "hi" }))
        .await
        .expect("tool call");
    assert_eq!(result, harness_mcp::McpToolResult::text("echo:hi"));

    connection.shutdown().await.expect("shutdown");
}

#[test]
fn stdio_env_resolver_denies_credentials_before_spawning() {
    let parent = BTreeMap::from([
        ("OPENAI_API_KEY".to_owned(), "secret".to_owned()),
        ("PATH".to_owned(), "/bin".to_owned()),
    ]);
    let env = StdioEnv::InheritWithDeny {
        deny: BTreeSet::from(["OPENAI_API_KEY".to_owned()]),
        extra: BTreeMap::from([("EXTRA".to_owned(), "1".to_owned())]),
    };

    let resolved = StdioTransport::resolve_env(&env, &parent);

    assert!(!resolved.contains_key("OPENAI_API_KEY"));
    assert_eq!(resolved.get("PATH").map(String::as_str), Some("/bin"));
    assert_eq!(resolved.get("EXTRA").map(String::as_str), Some("1"));
}
