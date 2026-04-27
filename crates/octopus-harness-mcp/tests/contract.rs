use std::{
    collections::{BTreeMap, VecDeque},
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::StreamExt;
use harness_contracts::{McpServerId, McpServerSource};
use harness_mcp::{
    InProcessTransport, ListChangedEvent, McpChange, McpClient, McpConnection, McpError,
    McpServerSpec, McpToolDescriptor, McpToolResult, TransportChoice,
};
use serde_json::{json, Value};

#[cfg(feature = "http")]
use harness_mcp::HttpTransport;
#[cfg(feature = "sse")]
use harness_mcp::SseTransport;
#[cfg(feature = "websocket")]
use harness_mcp::WebsocketTransport;
#[cfg(feature = "stdio")]
use harness_mcp::{StdioEnv, StdioPolicy, StdioTransport};

#[tokio::test]
#[cfg(feature = "in-process")]
async fn in_process_transport_satisfies_client_contract() {
    let connection = MockConnection {
        tools: vec![tool("contract_tool")],
        results: Mutex::new(VecDeque::from([McpToolResult::text("contract-ok")])),
    };
    let spec = McpServerSpec::new(
        McpServerId("contract_in_process".into()),
        "contract in-process",
        TransportChoice::InProcess,
        McpServerSource::Workspace,
    );
    let connection = McpClient::new(Arc::new(InProcessTransport::from_connection(Arc::new(
        connection,
    ))))
    .connect(spec)
    .await
    .expect("connect");

    assert_contract(connection, true).await;
}

#[tokio::test]
#[cfg(feature = "stdio")]
async fn stdio_transport_satisfies_client_contract() {
    let script = r#"
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"0.1.0"}}}'
      ;;
    *'"method":"tools/list"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"contract_tool","description":"Contract tool","inputSchema":{"type":"object"}}]}}'
      ;;
    *'"method":"tools/call"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"contract-ok"}],"isError":false}}'
      ;;
  esac
done
"#;
    let spec = McpServerSpec::new(
        McpServerId("contract_stdio".into()),
        "contract stdio",
        TransportChoice::Stdio {
            command: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            env: StdioEnv::default(),
            policy: StdioPolicy::default(),
        },
        McpServerSource::Workspace,
    );
    let connection = McpClient::new(Arc::new(StdioTransport::new()))
        .connect(spec)
        .await
        .expect("connect");

    assert_contract(connection, false).await;
}

#[tokio::test]
#[cfg(feature = "http")]
async fn http_transport_satisfies_client_contract() {
    use wiremock::{
        matchers::{body_partial_json, method},
        Mock, MockServer, ResponseTemplate,
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
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
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            json!({ "method": "notifications/initialized" }),
        ))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "tools/list" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    { "name": "contract_tool", "description": "Contract tool", "inputSchema": { "type": "object" } }
                ]
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "tools/call" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": { "content": [{ "type": "text", "text": "contract-ok" }], "isError": false }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "method": "shutdown" })))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    let spec = McpServerSpec::new(
        McpServerId("contract_http".into()),
        "contract http",
        TransportChoice::Http {
            url: server.uri(),
            headers: BTreeMap::default(),
        },
        McpServerSource::Workspace,
    );
    let connection = McpClient::new(Arc::new(HttpTransport::new()))
        .connect(spec)
        .await
        .expect("connect");

    assert_contract(connection, false).await;
}

#[tokio::test]
#[cfg(feature = "websocket")]
async fn websocket_transport_satisfies_client_contract() {
    use futures::SinkExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut socket = accept_async(stream).await.expect("accept websocket");
        while let Some(message) = socket.next().await {
            let value: Value =
                serde_json::from_str(&message.expect("message").into_text().expect("text"))
                    .expect("json");
            let method = value.get("method").and_then(Value::as_str);
            let response = match method {
                Some("initialize") => Some(json!({
                    "jsonrpc": "2.0",
                    "id": value["id"].clone(),
                    "result": {
                        "protocolVersion": "2025-03-26",
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "fixture", "version": "0.1.0" }
                    }
                })),
                Some("tools/list") => Some(json!({
                    "jsonrpc": "2.0",
                    "id": value["id"].clone(),
                    "result": {
                        "tools": [
                            { "name": "contract_tool", "description": "Contract tool", "inputSchema": { "type": "object" } }
                        ]
                    }
                })),
                Some("tools/call") => Some(json!({
                    "jsonrpc": "2.0",
                    "id": value["id"].clone(),
                    "result": { "content": [{ "type": "text", "text": "contract-ok" }], "isError": false }
                })),
                _ => None,
            };
            if let Some(response) = response {
                socket
                    .send(Message::Text(response.to_string()))
                    .await
                    .expect("send response");
            }
            if method == Some("tools/list") {
                socket
                    .send(Message::Text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/tools/list_changed"
                        })
                        .to_string(),
                    ))
                    .await
                    .expect("send notification");
            }
        }
    });

    let spec = McpServerSpec::new(
        McpServerId("contract_websocket".into()),
        "contract websocket",
        TransportChoice::WebSocket {
            url: format!("ws://{addr}"),
            headers: BTreeMap::default(),
        },
        McpServerSource::Workspace,
    );
    let connection = McpClient::new(Arc::new(WebsocketTransport::new()))
        .connect(spec)
        .await
        .expect("connect");

    assert_contract(connection, true).await;
}

#[tokio::test]
#[cfg(feature = "sse")]
async fn sse_transport_satisfies_client_contract() {
    let (addr, shutdown) = spawn_sse_contract_fixture().await;
    let spec = McpServerSpec::new(
        McpServerId("contract_sse".into()),
        "contract sse",
        TransportChoice::Sse {
            url: format!("http://{addr}/mcp"),
            headers: BTreeMap::default(),
        },
        McpServerSource::Workspace,
    );
    let connection = McpClient::new(Arc::new(SseTransport::new()))
        .connect(spec)
        .await
        .expect("connect");

    assert_contract(connection, true).await;
    let _ = shutdown.send(());
}

async fn assert_contract(connection: Arc<dyn McpConnection>, expect_change: bool) {
    let mut changes = if expect_change {
        Some(connection.subscribe_changes().await.expect("changes"))
    } else {
        None
    };
    let tools = connection.list_tools().await.expect("tools/list");
    assert_eq!(tools[0].name, "contract_tool");
    if let Some(changes) = changes.as_mut() {
        assert_eq!(changes.next().await, Some(McpChange::ToolsListChanged));
    }
    let result = connection
        .call_tool("contract_tool", json!({ "input": true }))
        .await
        .expect("tools/call");
    assert_eq!(result, McpToolResult::text("contract-ok"));
    connection.shutdown().await.expect("shutdown");
}

fn tool(name: &str) -> McpToolDescriptor {
    McpToolDescriptor {
        name: name.to_owned(),
        description: Some("Contract tool".to_owned()),
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        meta: BTreeMap::default(),
    }
}

struct MockConnection {
    tools: Vec<McpToolDescriptor>,
    results: Mutex<VecDeque<McpToolResult>>,
}

#[async_trait]
impl McpConnection for MockConnection {
    fn connection_id(&self) -> &'static str {
        "contract-in-process"
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        Ok(self.tools.clone())
    }

    async fn call_tool(&self, _name: &str, _args: Value) -> Result<McpToolResult, McpError> {
        self.results
            .lock()
            .expect("results")
            .pop_front()
            .ok_or_else(|| McpError::Protocol("missing result".to_owned()))
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, McpError> {
        Ok(Box::pin(futures::stream::iter([
            McpChange::ToolsListChanged,
        ])))
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }
}

#[cfg(feature = "sse")]
async fn spawn_sse_contract_fixture() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    use axum::{
        body::Bytes,
        extract::State,
        response::{sse::Event, Sse},
        routing::{get, post},
        Router,
    };
    use tokio::{
        net::{TcpListener, TcpStream},
        sync::{broadcast, oneshot},
    };
    use tokio_stream::wrappers::BroadcastStream;

    #[derive(Clone)]
    struct AppState {
        events: broadcast::Sender<String>,
    }

    async fn wait_for_event_subscriber(state: &AppState) {
        for _ in 0..20 {
            if state.events.receiver_count() > 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    async fn rpc(State(state): State<AppState>, body: Bytes) -> axum::http::StatusCode {
        let request: Value = serde_json::from_slice(&body).expect("json");
        wait_for_event_subscriber(&state).await;
        let response = match request.get("method").and_then(Value::as_str) {
            Some("initialize") => Some(json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "result": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "fixture", "version": "0.1.0" }
                }
            })),
            Some("tools/list") => {
                state
                    .events
                    .send(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/tools/list_changed"
                        })
                        .to_string(),
                    )
                    .expect("send notification");
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "result": {
                        "tools": [
                            { "name": "contract_tool", "description": "Contract tool", "inputSchema": { "type": "object" } }
                        ]
                    }
                }))
            }
            Some("tools/call") => Some(json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "result": { "content": [{ "type": "text", "text": "contract-ok" }], "isError": false }
            })),
            _ => None,
        };
        if let Some(response) = response {
            state
                .events
                .send(response.to_string())
                .expect("send response");
        }
        axum::http::StatusCode::ACCEPTED
    }

    async fn events(
        State(state): State<AppState>,
    ) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
        Sse::new(
            BroadcastStream::new(state.events.subscribe())
                .filter_map(|data| async move { data.ok() })
                .map(|data| Ok(Event::default().data(data))),
        )
    }

    let (sender, _) = broadcast::channel::<String>(32);
    let state = AppState { events: sender };
    let app = Router::new()
        .route("/mcp", post(rpc))
        .route("/mcp/events", get(events))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("serve");
    });
    for _ in 0..20 {
        if TcpStream::connect(addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    (addr, shutdown_tx)
}
