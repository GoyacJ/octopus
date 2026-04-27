use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError, McpToolDescriptor,
    McpToolResult,
};

#[cfg(feature = "http")]
mod http;
#[cfg(feature = "stdio")]
mod stdio;
#[cfg(feature = "websocket")]
mod websocket;

#[cfg(feature = "http")]
pub use http::HttpTransport;
#[cfg(feature = "stdio")]
pub use stdio::StdioTransport;
#[cfg(feature = "websocket")]
pub use websocket::WebsocketTransport;

#[derive(Debug, Deserialize)]
struct ListToolsResult {
    tools: Vec<McpToolDescriptor>,
}

pub(crate) struct JsonRpcPeer {
    next_id: AtomicU64,
}

impl JsonRpcPeer {
    pub(crate) fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
        }
    }

    pub(crate) fn request(&self, method: &str, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest::new(
            json!(self.next_id.fetch_add(1, Ordering::SeqCst)),
            method,
            params,
        )
    }
}

pub(crate) fn initialized_notification() -> JsonRpcNotification {
    JsonRpcNotification::new("notifications/initialized", None)
}

pub(crate) fn initialize_request(peer: &JsonRpcPeer) -> JsonRpcRequest {
    peer.request(
        "initialize",
        Some(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
            }
        })),
    )
}

pub(crate) fn list_tools_request(peer: &JsonRpcPeer) -> JsonRpcRequest {
    peer.request("tools/list", Some(json!({})))
}

pub(crate) fn call_tool_request(peer: &JsonRpcPeer, name: &str, args: Value) -> JsonRpcRequest {
    peer.request(
        "tools/call",
        Some(json!({
            "name": name,
            "arguments": args,
        })),
    )
}

pub(crate) fn decode_list_tools(
    response: JsonRpcResponse,
) -> Result<Vec<McpToolDescriptor>, McpError> {
    Ok(decode_success::<ListToolsResult>(response)?.tools)
}

pub(crate) fn decode_tool_result(response: JsonRpcResponse) -> Result<McpToolResult, McpError> {
    decode_success(response)
}

pub(crate) fn decode_success<T>(response: JsonRpcResponse) -> Result<T, McpError>
where
    T: DeserializeOwned,
{
    if let Some(error) = response.error {
        return Err(McpError::Protocol(format!(
            "{} ({})",
            error.message, error.code
        )));
    }

    let result = response
        .result
        .ok_or_else(|| McpError::InvalidResponse("missing result field".into()))?;
    serde_json::from_value(result).map_err(|error| McpError::InvalidResponse(error.to_string()))
}

#[cfg(any(feature = "stdio", feature = "websocket"))]
pub(crate) fn response_key(id: &Value) -> String {
    serde_json::to_string(id).expect("json-rpc ids should serialize")
}

#[cfg(feature = "websocket")]
pub(crate) fn notification_change(method: &str) -> Option<crate::McpChange> {
    match method {
        "tools/list_changed" | "notifications/tools/list_changed" => {
            Some(crate::McpChange::ToolsListChanged)
        }
        "resources/updated" | "notifications/resources/updated" => {
            Some(crate::McpChange::ResourcesUpdated { uri: None })
        }
        "prompts/list_changed" | "notifications/prompts/list_changed" => {
            Some(crate::McpChange::PromptsListChanged)
        }
        _ => None,
    }
}
