use std::collections::BTreeMap;
use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::config::McpTransport;
use crate::mcp_client::McpClientBootstrap;
use crate::mcp_lifecycle_hardened::{
    McpDegradedReport, McpErrorSurface, McpFailedServer, McpLifecyclePhase,
};

mod mcp_lifecycle;
mod mcp_routing;
#[cfg(test)]
mod mcp_tests;
mod mcp_tools;
mod mcp_transport;

pub use mcp_transport::spawn_mcp_stdio_process;

#[cfg(test)]
const MCP_INITIALIZE_TIMEOUT_MS: u64 = 200;
#[cfg(not(test))]
const MCP_INITIALIZE_TIMEOUT_MS: u64 = 10_000;

#[cfg(test)]
const MCP_LIST_TOOLS_TIMEOUT_MS: u64 = 300;
#[cfg(not(test))]
const MCP_LIST_TOOLS_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest<T = JsonValue> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    #[must_use]
    pub fn new(id: JsonRpcId, method: impl Into<String>, params: Option<T>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse<T = JsonValue> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeParams {
    pub protocol_version: String,
    pub capabilities: JsonValue,
    pub client_info: McpInitializeClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeResult {
    pub protocol_version: String,
    pub capabilities: JsonValue,
    pub server_info: McpInitializeServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpInitializeServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpListToolsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonValue>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpListToolsResult {
    pub tools: Vec<McpTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonValue>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpToolCallContent {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub data: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResult {
    #[serde(default)]
    pub content: Vec<McpToolCallContent>,
    #[serde(default)]
    pub structured_content: Option<JsonValue>,
    #[serde(default)]
    pub is_error: Option<bool>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpListResourcesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResource {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonValue>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpListResourcesResult {
    pub resources: Vec<McpResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpReadResourceParams {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResourceContents {
    pub uri: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpReadResourceResult {
    pub contents: Vec<McpResourceContents>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManagedMcpTool {
    pub server_name: String,
    pub qualified_name: String,
    pub raw_name: String,
    pub tool: McpTool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedMcpServer {
    pub server_name: String,
    pub transport: McpTransport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDiscoveryFailure {
    pub server_name: String,
    pub phase: McpLifecyclePhase,
    pub error: String,
    pub recoverable: bool,
    pub context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McpToolDiscoveryReport {
    pub tools: Vec<ManagedMcpTool>,
    pub failed_servers: Vec<McpDiscoveryFailure>,
    pub unsupported_servers: Vec<UnsupportedMcpServer>,
    pub degraded_startup: Option<McpDegradedReport>,
}

#[derive(Debug)]
pub enum McpServerManagerError {
    Io(io::Error),
    Transport {
        server_name: String,
        method: &'static str,
        source: io::Error,
    },
    JsonRpc {
        server_name: String,
        method: &'static str,
        error: JsonRpcError,
    },
    InvalidResponse {
        server_name: String,
        method: &'static str,
        details: String,
    },
    Timeout {
        server_name: String,
        method: &'static str,
        timeout_ms: u64,
    },
    UnknownTool {
        qualified_name: String,
    },
    UnknownServer {
        server_name: String,
    },
}

impl std::fmt::Display for McpServerManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Transport {
                server_name,
                method,
                source,
            } => write!(
                f,
                "MCP server `{server_name}` transport failed during {method}: {source}"
            ),
            Self::JsonRpc {
                server_name,
                method,
                error,
            } => write!(
                f,
                "MCP server `{server_name}` returned JSON-RPC error for {method}: {} ({})",
                error.message, error.code
            ),
            Self::InvalidResponse {
                server_name,
                method,
                details,
            } => write!(
                f,
                "MCP server `{server_name}` returned invalid response for {method}: {details}"
            ),
            Self::Timeout {
                server_name,
                method,
                timeout_ms,
            } => write!(
                f,
                "MCP server `{server_name}` timed out after {timeout_ms} ms while handling {method}"
            ),
            Self::UnknownTool { qualified_name } => {
                write!(f, "unknown MCP tool `{qualified_name}`")
            }
            Self::UnknownServer { server_name } => write!(f, "unknown MCP server `{server_name}`"),
        }
    }
}

impl std::error::Error for McpServerManagerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Transport { source, .. } => Some(source),
            Self::JsonRpc { .. }
            | Self::InvalidResponse { .. }
            | Self::Timeout { .. }
            | Self::UnknownTool { .. }
            | Self::UnknownServer { .. } => None,
        }
    }
}

impl From<io::Error> for McpServerManagerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl McpServerManagerError {
    fn lifecycle_phase(&self) -> McpLifecyclePhase {
        match self {
            Self::Io(_) => McpLifecyclePhase::SpawnConnect,
            Self::Transport { method, .. }
            | Self::JsonRpc { method, .. }
            | Self::InvalidResponse { method, .. }
            | Self::Timeout { method, .. } => lifecycle_phase_for_method(method),
            Self::UnknownTool { .. } => McpLifecyclePhase::ToolDiscovery,
            Self::UnknownServer { .. } => McpLifecyclePhase::ServerRegistration,
        }
    }

    fn recoverable(&self) -> bool {
        !matches!(
            self.lifecycle_phase(),
            McpLifecyclePhase::InitializeHandshake
        ) && matches!(self, Self::Transport { .. } | Self::Timeout { .. })
    }

    fn discovery_failure(&self, server_name: &str) -> McpDiscoveryFailure {
        let phase = self.lifecycle_phase();
        let recoverable = self.recoverable();
        let context = self.error_context();

        McpDiscoveryFailure {
            server_name: server_name.to_string(),
            phase,
            error: self.to_string(),
            recoverable,
            context,
        }
    }

    fn error_context(&self) -> BTreeMap<String, String> {
        match self {
            Self::Io(error) => BTreeMap::from([("kind".to_string(), error.kind().to_string())]),
            Self::Transport {
                server_name,
                method,
                source,
            } => BTreeMap::from([
                ("server".to_string(), server_name.clone()),
                ("method".to_string(), (*method).to_string()),
                ("io_kind".to_string(), source.kind().to_string()),
            ]),
            Self::JsonRpc {
                server_name,
                method,
                error,
            } => BTreeMap::from([
                ("server".to_string(), server_name.clone()),
                ("method".to_string(), (*method).to_string()),
                ("jsonrpc_code".to_string(), error.code.to_string()),
            ]),
            Self::InvalidResponse {
                server_name,
                method,
                details,
            } => BTreeMap::from([
                ("server".to_string(), server_name.clone()),
                ("method".to_string(), (*method).to_string()),
                ("details".to_string(), details.clone()),
            ]),
            Self::Timeout {
                server_name,
                method,
                timeout_ms,
            } => BTreeMap::from([
                ("server".to_string(), server_name.clone()),
                ("method".to_string(), (*method).to_string()),
                ("timeout_ms".to_string(), timeout_ms.to_string()),
            ]),
            Self::UnknownTool { qualified_name } => {
                BTreeMap::from([("qualified_tool".to_string(), qualified_name.clone())])
            }
            Self::UnknownServer { server_name } => {
                BTreeMap::from([("server".to_string(), server_name.clone())])
            }
        }
    }
}

fn lifecycle_phase_for_method(method: &str) -> McpLifecyclePhase {
    match method {
        "initialize" => McpLifecyclePhase::InitializeHandshake,
        "tools/list" => McpLifecyclePhase::ToolDiscovery,
        "resources/list" => McpLifecyclePhase::ResourceDiscovery,
        "resources/read" | "tools/call" => McpLifecyclePhase::Invocation,
        _ => McpLifecyclePhase::ErrorSurfacing,
    }
}

fn unsupported_server_failed_server(server: &UnsupportedMcpServer) -> McpFailedServer {
    McpFailedServer {
        server_name: server.server_name.clone(),
        phase: McpLifecyclePhase::ServerRegistration,
        error: McpErrorSurface::new(
            McpLifecyclePhase::ServerRegistration,
            Some(server.server_name.clone()),
            server.reason.clone(),
            BTreeMap::from([("transport".to_string(), format!("{:?}", server.transport))]),
            false,
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolRoute {
    server_name: String,
    raw_name: String,
}

#[derive(Debug)]
struct ManagedMcpServer {
    bootstrap: McpClientBootstrap,
    process: Option<McpStdioProcess>,
    initialized: bool,
}

#[derive(Debug)]
pub struct McpServerManager {
    servers: BTreeMap<String, ManagedMcpServer>,
    unsupported_servers: Vec<UnsupportedMcpServer>,
    tool_index: BTreeMap<String, ToolRoute>,
    next_request_id: u64,
}

#[derive(Debug)]
pub struct McpStdioProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}
