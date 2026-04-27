#![cfg(feature = "sse")]

use std::{collections::BTreeMap, convert::Infallible, net::SocketAddr};

use futures::StreamExt;
use harness_contracts::{McpServerId, McpServerSource};
use harness_mcp::{
    McpChange, McpClient, McpClientAuth, McpServerSpec, SseTransport, TransportChoice,
};
use serde_json::{json, Value};
use tokio::{
    net::TcpListener,
    sync::{broadcast, oneshot},
};
use tokio_stream::wrappers::BroadcastStream;

#[tokio::test]
async fn sse_transport_posts_requests_and_receives_streamed_responses() {
    let (addr, shutdown) = spawn_sse_fixture().await;
    let mut headers = BTreeMap::new();
    headers.insert("x-mcp-client".to_owned(), "octopus".to_owned());
    let mut spec = McpServerSpec::new(
        McpServerId("sse".into()),
        "sse fixture",
        TransportChoice::Sse {
            url: format!("http://{addr}/mcp"),
            headers,
        },
        McpServerSource::Workspace,
    );
    spec.auth = McpClientAuth::Bearer("token".into());

    let connection = McpClient::new(std::sync::Arc::new(SseTransport::new()))
        .connect(spec)
        .await
        .expect("sse connects");
    let mut changes = connection.subscribe_changes().await.expect("changes");

    let tools = connection.list_tools().await.expect("tools list");
    assert_eq!(tools[0].name, "sse_search");
    assert_eq!(changes.next().await, Some(McpChange::ToolsListChanged));

    let result = connection
        .call_tool("sse_search", json!({ "q": "mcp" }))
        .await
        .expect("tool call");
    assert_eq!(result, harness_mcp::McpToolResult::text("sse-found"));

    connection.shutdown().await.expect("shutdown");
    let _ = shutdown.send(());
}

async fn spawn_sse_fixture() -> (SocketAddr, oneshot::Sender<()>) {
    use axum::{
        body::Bytes,
        extract::State,
        http::{HeaderMap, StatusCode},
        response::{sse::Event, Sse},
        routing::{get, post},
        Router,
    };

    #[derive(Clone)]
    struct AppState {
        events: broadcast::Sender<String>,
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some("Bearer token")
            && headers
                .get("x-mcp-client")
                .and_then(|value| value.to_str().ok())
                == Some("octopus")
    }

    async fn wait_for_event_subscriber(state: &AppState) {
        for _ in 0..20 {
            if state.events.receiver_count() > 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    async fn send_event(state: &AppState, data: String) {
        for _ in 0..20 {
            wait_for_event_subscriber(state).await;
            if state.events.send(data.clone()).is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("send sse event");
    }

    async fn rpc(
        State(state): State<AppState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<StatusCode, StatusCode> {
        if !authorized(&headers) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let request: Value = serde_json::from_slice(&body).expect("request json");
        let response = match request.get("method").and_then(Value::as_str) {
            Some("initialize") => json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "result": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "fixture", "version": "0.1.0" }
                }
            }),
            Some("tools/list") => {
                send_event(
                    &state,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/tools/list_changed"
                    })
                    .to_string(),
                )
                .await;
                json!({
                    "jsonrpc": "2.0",
                    "id": request["id"].clone(),
                    "result": {
                        "tools": [
                            { "name": "sse_search", "description": "SSE search", "inputSchema": { "type": "object" } }
                        ]
                    }
                })
            }
            Some("tools/call") => json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "result": { "content": [{ "type": "text", "text": "sse-found" }], "isError": false }
            }),
            Some("notifications/initialized") | Some("shutdown") => {
                return Ok(StatusCode::ACCEPTED)
            }
            other => json!({
                "jsonrpc": "2.0",
                "id": request["id"].clone(),
                "error": { "code": -32601, "message": format!("unknown method: {other:?}") }
            }),
        };
        send_event(&state, response.to_string()).await;
        Ok(StatusCode::ACCEPTED)
    }

    async fn real_stream(
        State(state): State<AppState>,
        headers: HeaderMap,
    ) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
        if !authorized(&headers) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let stream = BroadcastStream::new(state.events.subscribe())
            .filter_map(|data| async move { data.ok() })
            .map(|data| Ok(Event::default().data(data)));
        Ok(Sse::new(stream))
    }

    let (sender, _) = broadcast::channel::<String>(32);
    let state = AppState { events: sender };
    let app = Router::new()
        .route("/mcp", post(rpc))
        .route("/mcp/events", get(real_stream))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("serve");
    });
    wait_for_listener(addr).await;
    (addr, shutdown_tx)
}

async fn wait_for_listener(addr: SocketAddr) {
    for _ in 0..20 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
