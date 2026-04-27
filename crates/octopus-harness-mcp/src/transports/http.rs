use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde_json::Value;

use crate::{
    call_tool_request, decode_list_tools, decode_tool_result, initialize_request,
    initialized_notification, list_tools_request, JsonRpcNotification, JsonRpcPeer, JsonRpcRequest,
    JsonRpcResponse, McpClientAuth, McpConnection, McpError, McpServerSpec, McpToolDescriptor,
    McpToolResult, McpTransport, TransportChoice,
};

#[derive(Default)]
pub struct HttpTransport;

impl HttpTransport {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    fn transport_id(&self) -> &'static str {
        "http"
    }

    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        let TransportChoice::Http { url, headers } = spec.transport.clone() else {
            return Err(McpError::Unsupported(
                "HttpTransport requires TransportChoice::Http".into(),
            ));
        };

        let mut default_headers = HeaderMap::new();
        for (key, value) in headers {
            let name = HeaderName::try_from(key.as_str())
                .map_err(|error| McpError::Transport(error.to_string()))?;
            let value = HeaderValue::try_from(value.as_str())
                .map_err(|error| McpError::Transport(error.to_string()))?;
            default_headers.insert(name, value);
        }
        if let McpClientAuth::Bearer(token) = &spec.auth {
            let value = HeaderValue::try_from(format!("Bearer {token}").as_str())
                .map_err(|error| McpError::Transport(error.to_string()))?;
            default_headers.insert(AUTHORIZATION, value);
        } else if !matches!(spec.auth, McpClientAuth::None) {
            return Err(McpError::Unsupported(
                "http transport only supports bearer auth in M4-T12".into(),
            ));
        }

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .timeout(spec.timeouts.call_default)
            .build()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let connection = Arc::new(HttpConnection {
            connection_id: format!("http:{}", spec.server_id.0),
            endpoint: url,
            client,
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

pub struct HttpConnection {
    connection_id: String,
    endpoint: String,
    client: reqwest::Client,
    peer: JsonRpcPeer,
}

impl HttpConnection {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|error| McpError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| McpError::Transport(error.to_string()))?
            .json::<JsonRpcResponse>()
            .await
            .map_err(|error| McpError::InvalidResponse(error.to_string()))
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<(), McpError> {
        self.client
            .post(&self.endpoint)
            .json(&notification)
            .send()
            .await
            .map_err(|error| McpError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl McpConnection for HttpConnection {
    fn connection_id(&self) -> &str {
        &self.connection_id
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        decode_list_tools(self.send(list_tools_request(&self.peer)).await?)
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError> {
        decode_tool_result(self.send(call_tool_request(&self.peer, name, args)).await?)
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        self.send_notification(JsonRpcNotification::new("shutdown", None))
            .await
    }
}
