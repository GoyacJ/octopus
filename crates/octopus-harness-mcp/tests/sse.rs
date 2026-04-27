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
    sync::{mpsc, oneshot},
};
use tokio_stream::wrappers::ReceiverStream;

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
        events: mpsc::Sender<String>,
        receiver: std::sync::Arc<tokio::sync::Mutex<Option<mpsc::Receiver<String>>>>,
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
                state
                    .events
                    .send(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/tools/list_changed"
                        })
                        .to_string(),
                    )
                    .await
                    .expect("send notification");
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
        state
            .events
            .send(response.to_string())
            .await
            .expect("send response");
        Ok(StatusCode::ACCEPTED)
    }

    async fn real_stream(
        State(state): State<AppState>,
        headers: HeaderMap,
    ) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
        if !authorized(&headers) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let receiver = state
            .receiver
            .lock()
            .await
            .take()
            .expect("single sse stream");
        let stream = ReceiverStream::new(receiver).map(|data| Ok(Event::default().data(data)));
        Ok(Sse::new(stream))
    }

    let (sender, receiver) = mpsc::channel::<String>(32);
    let state = AppState {
        events: sender,
        receiver: std::sync::Arc::new(tokio::sync::Mutex::new(Some(receiver))),
    };
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
    (addr, shutdown_tx)
}
