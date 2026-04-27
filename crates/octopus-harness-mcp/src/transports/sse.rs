use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    call_tool_request, decode_list_tools, decode_tool_result, initialize_request,
    initialized_notification, list_tools_request, notification_change, response_key,
    JsonRpcNotification, JsonRpcPeer, JsonRpcRequest, JsonRpcResponse, ListChangedEvent, McpChange,
    McpClientAuth, McpConnection, McpError, McpServerSpec, McpToolDescriptor, McpToolResult,
    McpTransport, TransportChoice,
};

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<JsonRpcResponse, McpError>>>>>;

#[derive(Default)]
pub struct SseTransport;

impl SseTransport {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    fn transport_id(&self) -> &'static str {
        "sse"
    }

    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        let TransportChoice::Sse { url, headers } = spec.transport.clone() else {
            return Err(McpError::Unsupported(
                "SseTransport requires TransportChoice::Sse".into(),
            ));
        };

        let default_headers = header_map(headers.clone(), &spec.auth)?;
        let event_headers = event_header_map(headers, &spec.auth)?;
        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .timeout(spec.timeouts.call_default)
            .build()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let event_client = reqwest_eventsource_client::Client::builder()
            .default_headers(event_headers)
            .timeout(spec.timeouts.call_default)
            .build()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (changes, _) = broadcast::channel(64);
        let events_url = format!("{}/events", url.trim_end_matches('/'));
        spawn_event_reader(
            event_client,
            events_url,
            Arc::clone(&pending),
            changes.clone(),
        )
        .await?;

        let connection = Arc::new(SseConnection {
            connection_id: format!("sse:{}", spec.server_id.0),
            endpoint: url,
            client,
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

pub struct SseConnection {
    connection_id: String,
    endpoint: String,
    client: reqwest::Client,
    pending: PendingMap,
    changes: broadcast::Sender<McpChange>,
    timeout: std::time::Duration,
    peer: JsonRpcPeer,
}

impl SseConnection {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let key = response_key(&request.id);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(key.clone(), sender);

        if let Err(error) = self.post_json(&request).await {
            self.pending.lock().await.remove(&key);
            return Err(error);
        }

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::Connection("sse response channel closed".into())),
            Err(_) => {
                self.pending.lock().await.remove(&key);
                Err(McpError::Connection(format!(
                    "sse request timed out: {}",
                    request.method
                )))
            }
        }
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<(), McpError> {
        self.post_json(&notification).await
    }

    async fn post_json<T>(&self, body: &T) -> Result<(), McpError>
    where
        T: serde::Serialize + ?Sized,
    {
        self.client
            .post(&self.endpoint)
            .json(body)
            .send()
            .await
            .map_err(|error| McpError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl McpConnection for SseConnection {
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

async fn spawn_event_reader(
    client: reqwest_eventsource_client::Client,
    events_url: String,
    pending: PendingMap,
    changes: broadcast::Sender<McpChange>,
) -> Result<(), McpError> {
    let mut stream = EventSource::new(client.get(events_url))
        .map_err(|error| McpError::Transport(error.to_string()))?;

    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            let data = match event {
                Ok(Event::Open) => continue,
                Ok(Event::Message(message)) => message.data,
                Err(error) => {
                    notify_all(&pending, McpError::Transport(error.to_string())).await;
                    break;
                }
            };
            handle_sse_data(&data, &pending, &changes).await;
        }
        notify_all(&pending, McpError::Connection("sse stream closed".into())).await;
    });

    Ok(())
}

async fn handle_sse_data(data: &str, pending: &PendingMap, changes: &broadcast::Sender<McpChange>) {
    let value = match serde_json::from_str::<Value>(data) {
        Ok(value) => value,
        Err(_) => return,
    };

    if let Some(method) = value.get("method").and_then(Value::as_str) {
        if let Some(change) = notification_change(method) {
            let _ = changes.send(change);
        }
        return;
    }

    let response = match serde_json::from_value::<JsonRpcResponse>(value) {
        Ok(response) => response,
        Err(error) => {
            notify_all(pending, McpError::InvalidResponse(error.to_string())).await;
            return;
        }
    };
    let key = response_key(&response.id);
    if let Some(sender) = pending.lock().await.remove(&key) {
        let _ = sender.send(Ok(response));
    }
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

fn header_map(
    headers: std::collections::BTreeMap<String, String>,
    auth: &McpClientAuth,
) -> Result<HeaderMap, McpError> {
    let mut default_headers = HeaderMap::new();
    for (key, value) in headers {
        let name = HeaderName::try_from(key.as_str())
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let value = HeaderValue::try_from(value.as_str())
            .map_err(|error| McpError::Transport(error.to_string()))?;
        default_headers.insert(name, value);
    }
    if let McpClientAuth::Bearer(token) = auth {
        let value = HeaderValue::try_from(format!("Bearer {token}").as_str())
            .map_err(|error| McpError::Transport(error.to_string()))?;
        default_headers.insert(AUTHORIZATION, value);
    } else if !matches!(auth, McpClientAuth::None) {
        return Err(McpError::Unsupported(
            "sse transport only supports bearer auth in M4-T13".into(),
        ));
    }
    Ok(default_headers)
}

fn event_header_map(
    headers: std::collections::BTreeMap<String, String>,
    auth: &McpClientAuth,
) -> Result<reqwest_eventsource_client::header::HeaderMap, McpError> {
    let mut default_headers = reqwest_eventsource_client::header::HeaderMap::new();
    for (key, value) in headers {
        let name = reqwest_eventsource_client::header::HeaderName::try_from(key.as_str())
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let value = reqwest_eventsource_client::header::HeaderValue::try_from(value.as_str())
            .map_err(|error| McpError::Transport(error.to_string()))?;
        default_headers.insert(name, value);
    }
    default_headers.insert(
        reqwest_eventsource_client::header::ACCEPT,
        reqwest_eventsource_client::header::HeaderValue::from_static("text/event-stream"),
    );
    if let McpClientAuth::Bearer(token) = auth {
        let value =
            reqwest_eventsource_client::header::HeaderValue::try_from(format!("Bearer {token}"))
                .map_err(|error| McpError::Transport(error.to_string()))?;
        default_headers.insert(reqwest_eventsource_client::header::AUTHORIZATION, value);
    } else if !matches!(auth, McpClientAuth::None) {
        return Err(McpError::Unsupported(
            "sse transport only supports bearer auth in M4-T13".into(),
        ));
    }
    Ok(default_headers)
}
