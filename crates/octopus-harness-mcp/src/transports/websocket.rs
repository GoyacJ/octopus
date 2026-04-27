use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{HeaderName, HeaderValue},
        Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use crate::{
    call_tool_request, decode_list_tools, decode_tool_result, initialize_request,
    initialized_notification, list_tools_request, notification_change, response_key,
    JsonRpcNotification, JsonRpcPeer, JsonRpcRequest, JsonRpcResponse, ListChangedEvent, McpChange,
    McpConnection, McpError, McpServerSpec, McpToolDescriptor, McpToolResult, McpTransport,
    TransportChoice,
};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WsWriter = futures::stream::SplitSink<WsStream, Message>;
type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<JsonRpcResponse, McpError>>>>>;

#[derive(Default)]
pub struct WebsocketTransport;

impl WebsocketTransport {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl McpTransport for WebsocketTransport {
    fn transport_id(&self) -> &'static str {
        "websocket"
    }

    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        let TransportChoice::WebSocket { url, headers } = spec.transport.clone() else {
            return Err(McpError::Unsupported(
                "WebsocketTransport requires TransportChoice::WebSocket".into(),
            ));
        };

        let mut request = url
            .into_client_request()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        for (key, value) in headers {
            let name = HeaderName::try_from(key.as_str())
                .map_err(|error| McpError::Transport(error.to_string()))?;
            let value = HeaderValue::try_from(value.as_str())
                .map_err(|error| McpError::Transport(error.to_string()))?;
            request.headers_mut().insert(name, value);
        }

        let (socket, _) = connect_async(request)
            .await
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let (writer, reader) = socket.split();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (changes, _) = broadcast::channel(64);
        spawn_reader(reader, Arc::clone(&pending), changes.clone());

        let connection = Arc::new(WebsocketConnection {
            connection_id: format!("websocket:{}", spec.server_id.0),
            writer: Arc::new(Mutex::new(writer)),
            pending,
            changes,
            timeout: spec.timeouts.call_default,
            peer: JsonRpcPeer::new(),
        });
        connection
            .send(initialize_request(&connection.peer))
            .await?;
        connection
            .send_notification(initialized_notification())
            .await?;
        Ok(connection)
    }
}

pub struct WebsocketConnection {
    connection_id: String,
    writer: Arc<Mutex<WsWriter>>,
    pending: PendingMap,
    changes: broadcast::Sender<McpChange>,
    timeout: std::time::Duration,
    peer: JsonRpcPeer,
}

impl WebsocketConnection {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let key = response_key(&request.id);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(key.clone(), sender);

        let payload = serde_json::to_string(&request)
            .map_err(|error| McpError::InvalidResponse(error.to_string()))?;
        if let Err(error) = self.writer.lock().await.send(Message::Text(payload)).await {
            self.pending.lock().await.remove(&key);
            return Err(McpError::Transport(error.to_string()));
        }

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::Connection(
                "websocket response channel closed".into(),
            )),
            Err(_) => {
                self.pending.lock().await.remove(&key);
                Err(McpError::Connection(format!(
                    "websocket request timed out: {}",
                    request.method
                )))
            }
        }
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<(), McpError> {
        let payload = serde_json::to_string(&notification)
            .map_err(|error| McpError::InvalidResponse(error.to_string()))?;
        self.writer
            .lock()
            .await
            .send(Message::Text(payload))
            .await
            .map_err(|error| McpError::Transport(error.to_string()))
    }
}

#[async_trait]
impl McpConnection for WebsocketConnection {
    fn connection_id(&self) -> &str {
        &self.connection_id
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        decode_list_tools(self.send(list_tools_request(&self.peer)).await?)
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError> {
        decode_tool_result(self.send(call_tool_request(&self.peer, name, args)).await?)
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, McpError> {
        let stream = BroadcastStream::new(self.changes.subscribe())
            .filter_map(|event| async move { event.ok() });
        Ok(Box::pin(stream))
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        self.send_notification(JsonRpcNotification::new("shutdown", None))
            .await
    }
}

fn spawn_reader(
    mut reader: futures::stream::SplitStream<WsStream>,
    pending: PendingMap,
    changes: broadcast::Sender<McpChange>,
) {
    tokio::spawn(async move {
        while let Some(message) = reader.next().await {
            let text = match message {
                Ok(Message::Text(text)) => text,
                Ok(Message::Binary(bytes)) => match String::from_utf8(bytes) {
                    Ok(text) => text,
                    Err(error) => {
                        notify_all(&pending, McpError::InvalidResponse(error.to_string())).await;
                        break;
                    }
                },
                Ok(Message::Close(_)) => break,
                Ok(_) => continue,
                Err(error) => {
                    notify_all(&pending, McpError::Transport(error.to_string())).await;
                    break;
                }
            };

            let value = match serde_json::from_str::<Value>(&text) {
                Ok(value) => value,
                Err(error) => {
                    notify_all(&pending, McpError::InvalidResponse(error.to_string())).await;
                    break;
                }
            };

            if let Some(method) = value.get("method").and_then(Value::as_str) {
                if let Some(change) = notification_change(method) {
                    let _ = changes.send(change);
                }
                continue;
            }

            let response = match serde_json::from_value::<JsonRpcResponse>(value) {
                Ok(response) => response,
                Err(error) => {
                    notify_all(&pending, McpError::InvalidResponse(error.to_string())).await;
                    break;
                }
            };
            let key = response_key(&response.id);
            if let Some(sender) = pending.lock().await.remove(&key) {
                let _ = sender.send(Ok(response));
            }
        }
        notify_all(&pending, McpError::Connection("websocket closed".into())).await;
    });
}

async fn notify_all(pending: &PendingMap, error: McpError) {
    let senders = pending
        .lock()
        .await
        .drain()
        .map(|(_, sender)| sender)
        .collect::<Vec<_>>();
    for sender in senders {
        let _ = sender.send(Err(error.clone()));
    }
}
