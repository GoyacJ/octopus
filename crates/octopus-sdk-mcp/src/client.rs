use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    JsonRpcRequest, McpError, McpPrompt, McpResource, McpTool, McpToolResult, McpTransport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeResult {
    pub protocol_version: String,
}

pub struct McpClient {
    transport: Arc<dyn McpTransport>,
    server_id: String,
    initialized: AtomicBool,
    request_id: AtomicU64,
}

impl McpClient {
    #[must_use]
    pub fn new(server_id: impl Into<String>, transport: Arc<dyn McpTransport>) -> Self {
        Self {
            transport,
            server_id: server_id.into(),
            initialized: AtomicBool::new(false),
            request_id: AtomicU64::new(1),
        }
    }

    #[must_use]
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    pub async fn initialize(&self) -> Result<InitializeResult, McpError> {
        let response = self
            .transport
            .call(JsonRpcRequest::new(
                json!(self.next_request_id()),
                "initialize",
                Some(json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                })),
            ))
            .await?;

        let result: InitializeResultWire = self.decode_success(response)?;
        self.initialized.store(true, Ordering::SeqCst);
        Ok(InitializeResult {
            protocol_version: result.protocol_version,
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized().await?;
        let response = self
            .transport
            .call(JsonRpcRequest::new(
                json!(self.next_request_id()),
                "tools/list",
                Some(json!({})),
            ))
            .await?;
        let result: ListToolsResult = self.decode_success(response)?;
        Ok(result.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        self.ensure_initialized().await?;
        let response = self
            .transport
            .call(JsonRpcRequest::new(
                json!(self.next_request_id()),
                "tools/call",
                Some(json!({
                    "name": name,
                    "arguments": input,
                })),
            ))
            .await?;
        self.decode_success(response)
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        self.ensure_initialized().await?;
        let response = self
            .transport
            .call(JsonRpcRequest::new(
                json!(self.next_request_id()),
                "prompts/list",
                Some(json!({})),
            ))
            .await?;
        match self.decode_success::<ListPromptsResult>(response) {
            Ok(result) => Ok(result.prompts),
            Err(McpError::Protocol { message }) if is_not_implemented(&message) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        self.ensure_initialized().await?;
        let response = self
            .transport
            .call(JsonRpcRequest::new(
                json!(self.next_request_id()),
                "resources/list",
                Some(json!({})),
            ))
            .await?;
        match self.decode_success::<ListResourcesResult>(response) {
            Ok(result) => Ok(result.resources),
            Err(McpError::Protocol { message }) if is_not_implemented(&message) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    async fn ensure_initialized(&self) -> Result<(), McpError> {
        if self.is_initialized() {
            return Ok(());
        }

        let _ = self.initialize().await?;
        Ok(())
    }

    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    fn decode_success<T>(&self, response: crate::JsonRpcResponse) -> Result<T, McpError>
    where
        T: DeserializeOwned,
    {
        if let Some(error) = response.error {
            return Err(McpError::Protocol {
                message: format!("{} ({})", error.message, error.code),
            });
        }

        let result = response.result.ok_or_else(|| McpError::InvalidResponse {
            body_preview: "missing result field".into(),
        })?;

        serde_json::from_value(result).map_err(|error| McpError::InvalidResponse {
            body_preview: error.to_string(),
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeResultWire {
    protocol_version: String,
}

#[derive(Debug, serde::Deserialize)]
struct ListToolsResult {
    tools: Vec<McpTool>,
}

#[derive(Debug, serde::Deserialize)]
struct ListPromptsResult {
    prompts: Vec<McpPrompt>,
}

#[derive(Debug, serde::Deserialize)]
struct ListResourcesResult {
    resources: Vec<McpResource>,
}

fn is_not_implemented(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("not implemented") || normalized.contains("method not found")
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use serde_json::json;

    use super::McpClient;
    use crate::{
        JsonRpcRequest, JsonRpcResponse, McpError, McpPrompt, McpResource, McpToolResult,
        McpTransport,
    };

    struct MockTransport {
        requests: Arc<Mutex<Vec<String>>>,
        responses: Arc<Mutex<VecDeque<JsonRpcResponse>>>,
    }

    impl MockTransport {
        fn new(responses: Vec<JsonRpcResponse>) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            }
        }
    }

    #[async_trait]
    impl McpTransport for MockTransport {
        async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
            self.requests
                .lock()
                .expect("requests lock should work")
                .push(req.method.clone());

            self.responses
                .lock()
                .expect("responses lock should work")
                .pop_front()
                .ok_or_else(|| McpError::InvalidResponse {
                    body_preview: "missing mock response".into(),
                })
        }
    }

    #[tokio::test]
    async fn client_auto_initializes_then_lists_and_calls() {
        let transport = Arc::new(MockTransport::new(vec![
            JsonRpcResponse::success(json!(1), json!({ "protocolVersion": "2025-03-26" })),
            JsonRpcResponse::success(
                json!(2),
                json!({
                    "tools": [{
                        "name": "grep",
                        "description": "Search files",
                        "inputSchema": { "type": "object" }
                    }]
                }),
            ),
            JsonRpcResponse::success(
                json!(3),
                json!({
                    "content": [{ "type": "text", "text": "ok" }],
                    "isError": false
                }),
            ),
        ]));
        let client = McpClient::new("sdk", transport.clone());

        let tools = client
            .list_tools()
            .await
            .expect("list tools should succeed");
        let result = client
            .call_tool("grep", json!({ "pattern": "foo" }))
            .await
            .expect("call tool should succeed");

        assert!(client.is_initialized());
        assert_eq!(tools[0].name, "grep");
        assert_eq!(
            result,
            McpToolResult {
                content: vec![octopus_sdk_contracts::ContentBlock::Text { text: "ok".into() }],
                is_error: false,
            }
        );
        assert_eq!(
            transport
                .requests
                .lock()
                .expect("requests lock should work")
                .as_slice(),
            ["initialize", "tools/list", "tools/call"]
        );
    }

    #[tokio::test]
    async fn client_lists_prompts_and_resources_as_empty_when_not_implemented() {
        let transport = Arc::new(MockTransport::new(vec![
            JsonRpcResponse::success(json!(1), json!({ "protocolVersion": "2025-03-26" })),
            JsonRpcResponse::failure(
                json!(2),
                crate::JsonRpcError {
                    code: -32_601,
                    message: "method not found".into(),
                    data: None,
                },
            ),
            JsonRpcResponse::failure(
                json!(3),
                crate::JsonRpcError {
                    code: -32_601,
                    message: "not implemented".into(),
                    data: None,
                },
            ),
        ]));
        let client = McpClient::new("sdk", transport);

        let prompts = client
            .list_prompts()
            .await
            .expect("prompts should degrade to empty");
        let resources = client
            .list_resources()
            .await
            .expect("resources should degrade to empty");

        assert_eq!(prompts, Vec::<McpPrompt>::new());
        assert_eq!(resources, Vec::<McpResource>::new());
    }
}
