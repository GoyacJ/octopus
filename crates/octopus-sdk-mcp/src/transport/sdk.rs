use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError, McpToolResult,
    McpTransport, ToolDirectory,
};

pub struct SdkTransport {
    directory: Arc<dyn ToolDirectory>,
}

impl SdkTransport {
    #[must_use]
    pub fn from_directory(directory: Arc<dyn ToolDirectory>) -> Self {
        Self { directory }
    }
}

#[async_trait]
impl McpTransport for SdkTransport {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let response = match req.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                req.id,
                json!({
                    "protocolVersion": "2025-03-26"
                }),
            ),
            "tools/list" => JsonRpcResponse::success(
                req.id,
                json!({
                    "tools": self.directory.list_tools(),
                }),
            ),
            "tools/call" => {
                let params = req.params.ok_or_else(|| McpError::InvalidResponse {
                    body_preview: "missing tools/call params".into(),
                })?;
                let name = params
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| McpError::InvalidResponse {
                        body_preview: "missing tool name".into(),
                    })?;
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result: McpToolResult = self.directory.call_tool(name, arguments).await?;
                JsonRpcResponse::success(
                    req.id,
                    serde_json::to_value(result).map_err(|error| McpError::InvalidResponse {
                        body_preview: error.to_string(),
                    })?,
                )
            }
            other => JsonRpcResponse::failure(
                req.id,
                JsonRpcError {
                    code: -32_601,
                    message: format!("method not found: {other}"),
                    data: None,
                },
            ),
        };

        Ok(response)
    }

    async fn notify(&self, _msg: JsonRpcNotification) -> Result<(), McpError> {
        Ok(())
    }
}
