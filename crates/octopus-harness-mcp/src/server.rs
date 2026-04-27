use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use futures::StreamExt;
use harness_contracts::{MessagePart, ToolDescriptor, ToolResult, ToolResultPart, ToolUseId};
use harness_tool::{PermissionCheck, ToolContext, ToolEvent, ToolRegistry};
use serde_json::{json, Value};

use crate::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpContent, McpToolDescriptor, McpToolResult,
};

const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
const JSONRPC_INVALID_PARAMS: i32 = -32602;
const JSONRPC_INTERNAL_ERROR: i32 = -32603;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpServerError {
    #[error("missing tool context factory")]
    MissingToolContextFactory,
    #[error("invalid params: {0}")]
    InvalidParams(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerPolicy {
    pub server_name: String,
    pub server_version: String,
}

impl Default for McpServerPolicy {
    fn default() -> Self {
        Self {
            server_name: "octopus-harness-mcp".to_owned(),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[async_trait]
pub trait ToolContextFactory: Send + Sync + 'static {
    async fn create_tool_context(
        &self,
        tool_name: &str,
        arguments: &Value,
    ) -> Result<ToolContext, McpServerError>;
}

#[derive(Clone)]
pub struct StaticToolContextFactory {
    context: ToolContext,
}

impl StaticToolContextFactory {
    pub fn new(context: ToolContext) -> Self {
        Self { context }
    }
}

#[async_trait]
impl ToolContextFactory for StaticToolContextFactory {
    async fn create_tool_context(
        &self,
        _tool_name: &str,
        _arguments: &Value,
    ) -> Result<ToolContext, McpServerError> {
        let mut context = self.context.clone();
        context.tool_use_id = ToolUseId::new();
        Ok(context)
    }
}

#[derive(Clone)]
pub struct McpServerAdapter {
    registry: ToolRegistry,
    policy: McpServerPolicy,
    tool_context_factory: Arc<dyn ToolContextFactory>,
}

impl McpServerAdapter {
    pub fn builder(registry: ToolRegistry) -> McpServerAdapterBuilder {
        McpServerAdapterBuilder {
            registry,
            policy: McpServerPolicy::default(),
            tool_context_factory: None,
        }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.initialize(),
            "ping" | "shutdown" => Ok(json!({})),
            "tools/list" => Ok(self.list_tools()),
            "tools/call" => self.call_tool(request.params.as_ref()).await,
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            method => Err(jsonrpc_error(
                JSONRPC_METHOD_NOT_FOUND,
                format!("method not found: {method}"),
            )),
        };

        match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(error) => JsonRpcResponse::failure(request.id, error),
        }
    }

    fn initialize(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {},
            },
            "serverInfo": {
                "name": self.policy.server_name,
                "version": self.policy.server_version,
            }
        }))
    }

    fn list_tools(&self) -> Value {
        let tools = self
            .registry
            .snapshot()
            .as_descriptors()
            .into_iter()
            .map(tool_descriptor_to_mcp)
            .collect::<Vec<_>>();
        json!({ "tools": tools })
    }

    async fn call_tool(&self, params: Option<&Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| jsonrpc_error(JSONRPC_INVALID_PARAMS, "tools/call missing params"))?;
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| jsonrpc_error(JSONRPC_INVALID_PARAMS, "tools/call missing name"))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !arguments.is_object() {
            return Err(jsonrpc_error(
                JSONRPC_INVALID_PARAMS,
                "tools/call arguments must be an object",
            ));
        }

        let tool = self.registry.get(name).ok_or_else(|| {
            jsonrpc_error(JSONRPC_INVALID_PARAMS, format!("unknown tool: {name}"))
        })?;
        let context = self
            .tool_context_factory
            .create_tool_context(name, &arguments)
            .await
            .map_err(server_error_to_jsonrpc)?;

        if let Err(error) = tool.validate(&arguments, &context).await {
            return Ok(tool_error_result(format!("validation: {error}")));
        }

        match tool.check_permission(&arguments, &context).await {
            PermissionCheck::Allowed => {}
            PermissionCheck::Denied { reason } => {
                return Ok(tool_error_result(format!("permission denied: {reason}")));
            }
            PermissionCheck::AskUser { .. } => {
                return Ok(tool_error_result("permission required"));
            }
            PermissionCheck::DangerousCommand { pattern, severity } => {
                return Ok(tool_error_result(format!(
                    "permission required for dangerous command {pattern} ({severity:?})"
                )));
            }
        }

        let stream = match tool.execute(arguments, context).await {
            Ok(stream) => stream,
            Err(error) => return Ok(tool_error_result(error.to_string())),
        };
        let result = collect_tool_stream(stream).await;
        serde_json::to_value(result).map_err(|error| {
            jsonrpc_error(
                JSONRPC_INTERNAL_ERROR,
                format!("failed to encode tool result: {error}"),
            )
        })
    }
}

pub struct McpServerAdapterBuilder {
    registry: ToolRegistry,
    policy: McpServerPolicy,
    tool_context_factory: Option<Arc<dyn ToolContextFactory>>,
}

impl McpServerAdapterBuilder {
    #[must_use]
    pub fn with_policy(mut self, policy: McpServerPolicy) -> Self {
        self.policy = policy;
        self
    }

    #[must_use]
    pub fn with_tool_context_factory<T>(mut self, factory: T) -> Self
    where
        T: ToolContextFactory,
    {
        self.tool_context_factory = Some(Arc::new(factory));
        self
    }

    pub fn build(self) -> Result<McpServerAdapter, McpServerError> {
        Ok(McpServerAdapter {
            registry: self.registry,
            policy: self.policy,
            tool_context_factory: self
                .tool_context_factory
                .ok_or(McpServerError::MissingToolContextFactory)?,
        })
    }
}

fn tool_descriptor_to_mcp(descriptor: &ToolDescriptor) -> McpToolDescriptor {
    McpToolDescriptor {
        name: descriptor.name.clone(),
        description: Some(descriptor.description.clone()),
        input_schema: descriptor.input_schema.clone(),
        output_schema: descriptor.output_schema.clone(),
        meta: BTreeMap::new(),
    }
}

async fn collect_tool_stream(mut stream: harness_tool::ToolStream) -> McpToolResult {
    let mut content = Vec::new();
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress(_) => {}
            ToolEvent::Partial(part) => content.extend(message_part_to_mcp(part)),
            ToolEvent::Final(result) => {
                content.extend(tool_result_to_mcp_content(result));
                return McpToolResult {
                    content,
                    is_error: false,
                };
            }
            ToolEvent::Error(error) => return mcp_error_result(error.to_string()),
        }
    }

    mcp_error_result("tool stream ended without final result")
}

fn tool_result_to_mcp_content(result: ToolResult) -> Vec<McpContent> {
    match result {
        ToolResult::Text(text) => vec![McpContent::Text { text }],
        ToolResult::Structured(value) => vec![McpContent::Json { value }],
        ToolResult::Blob {
            content_type,
            blob_ref,
        } => vec![McpContent::Json {
            value: json!({
                "contentType": content_type,
                "blobRef": blob_ref,
            }),
        }],
        ToolResult::Mixed(parts) => parts.into_iter().map(tool_result_part_to_mcp).collect(),
        other => vec![McpContent::Json {
            value: serde_json::to_value(other).unwrap_or_else(|_| json!({})),
        }],
    }
}

fn tool_result_part_to_mcp(part: ToolResultPart) -> McpContent {
    match part {
        ToolResultPart::Text { text } | ToolResultPart::Code { text, .. } => {
            McpContent::Text { text }
        }
        ToolResultPart::Structured { value, .. } => McpContent::Json { value },
        other => McpContent::Json {
            value: serde_json::to_value(other).unwrap_or_else(|_| json!({})),
        },
    }
}

fn message_part_to_mcp(part: MessagePart) -> Vec<McpContent> {
    match part {
        MessagePart::Text(text) => vec![McpContent::Text { text }],
        MessagePart::ToolResult { content, .. } => tool_result_to_mcp_content(content),
        other => vec![McpContent::Json {
            value: serde_json::to_value(other).unwrap_or_else(|_| json!({})),
        }],
    }
}

fn tool_error_result(message: impl Into<String>) -> Value {
    serde_json::to_value(mcp_error_result(message)).expect("mcp error result serializes")
}

fn mcp_error_result(message: impl Into<String>) -> McpToolResult {
    McpToolResult {
        content: vec![McpContent::Text {
            text: message.into(),
        }],
        is_error: true,
    }
}

fn server_error_to_jsonrpc(error: McpServerError) -> JsonRpcError {
    jsonrpc_error(JSONRPC_INTERNAL_ERROR, error.to_string())
}

fn jsonrpc_error(code: i32, message: impl Into<String>) -> JsonRpcError {
    JsonRpcError {
        code,
        message: message.into(),
        data: None,
    }
}
