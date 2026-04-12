use serde_json::Value as JsonValue;

use crate::mcp::mcp_tool_name;
use crate::mcp_lifecycle_hardened::{McpDegradedReport, McpErrorSurface, McpFailedServer};

use super::{
    unsupported_server_failed_server, ManagedMcpPrompt, ManagedMcpTool, McpGetPromptParams,
    McpGetPromptResult, McpListPromptsParams, McpListPromptsResult, McpListResourcesParams,
    McpListResourcesResult, McpReadResourceParams, McpReadResourceResult, McpServerManager,
    McpServerManagerError, McpToolCallParams, McpToolCallResult, McpToolDiscoveryReport, ToolRoute,
    MCP_LIST_TOOLS_TIMEOUT_MS,
};

impl McpServerManager {
    pub async fn discover_tools(&mut self) -> Result<Vec<ManagedMcpTool>, McpServerManagerError> {
        let server_names = self.servers.keys().cloned().collect::<Vec<_>>();
        let mut discovered_tools = Vec::new();

        for server_name in server_names {
            let server_tools = self.discover_tools_for_server(&server_name).await?;
            self.clear_routes_for_server(&server_name);
            self.register_discovered_tools(server_tools, &mut discovered_tools);
        }

        Ok(discovered_tools)
    }

    pub async fn discover_tools_best_effort(&mut self) -> McpToolDiscoveryReport {
        let server_names = self.server_names();
        let mut discovered_tools = Vec::new();
        let mut working_servers = Vec::new();
        let mut failed_servers = Vec::new();

        for server_name in server_names {
            match self.discover_tools_for_server(&server_name).await {
                Ok(server_tools) => {
                    working_servers.push(server_name.clone());
                    self.clear_routes_for_server(&server_name);
                    self.register_discovered_tools(server_tools, &mut discovered_tools);
                }
                Err(error) => {
                    self.clear_routes_for_server(&server_name);
                    failed_servers.push(error.discovery_failure(&server_name));
                }
            }
        }

        let degraded_failed_servers = failed_servers
            .iter()
            .map(|failure| McpFailedServer {
                server_name: failure.server_name.clone(),
                phase: failure.phase,
                error: McpErrorSurface::new(
                    failure.phase,
                    Some(failure.server_name.clone()),
                    failure.error.clone(),
                    failure.context.clone(),
                    failure.recoverable,
                ),
            })
            .chain(
                self.unsupported_servers
                    .iter()
                    .map(unsupported_server_failed_server),
            )
            .collect::<Vec<_>>();
        let degraded_startup = (!working_servers.is_empty() && !degraded_failed_servers.is_empty())
            .then(|| {
                McpDegradedReport::new(
                    working_servers,
                    degraded_failed_servers,
                    discovered_tools
                        .iter()
                        .map(|tool| tool.qualified_name.clone())
                        .collect(),
                    Vec::new(),
                )
            });

        McpToolDiscoveryReport {
            tools: discovered_tools,
            failed_servers,
            unsupported_servers: self.unsupported_servers.clone(),
            degraded_startup,
        }
    }

    pub async fn call_tool(
        &mut self,
        qualified_tool_name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<super::JsonRpcResponse<McpToolCallResult>, McpServerManagerError> {
        let route = self
            .tool_index
            .get(qualified_tool_name)
            .cloned()
            .ok_or_else(|| McpServerManagerError::UnknownTool {
                qualified_name: qualified_tool_name.to_string(),
            })?;

        let timeout_ms = self.tool_call_timeout_ms(&route.server_name)?;

        self.ensure_server_ready(&route.server_name).await?;
        let request_id = self.take_request_id();
        let response =
            {
                let server = self.server_mut(&route.server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: route.server_name.clone(),
                        method: "tools/call",
                        details: "server process missing after initialization".to_string(),
                    }
                })?;
                Self::run_process_request(
                    &route.server_name,
                    "tools/call",
                    timeout_ms,
                    process.call_tool(
                        request_id,
                        McpToolCallParams {
                            name: route.raw_name,
                            arguments,
                            meta: None,
                        },
                    ),
                )
                .await
            };

        if let Err(error) = &response {
            if Self::should_reset_server(error) {
                self.reset_server(&route.server_name).await?;
            }
        }

        response
    }

    pub async fn list_prompts(
        &mut self,
        server_name: &str,
    ) -> Result<McpListPromptsResult, McpServerManagerError> {
        let mut attempts = 0;

        loop {
            match self.list_prompts_once(server_name).await {
                Ok(prompts) => return Ok(prompts),
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            }
        }
    }

    pub async fn get_prompt(
        &mut self,
        server_name: &str,
        name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<McpGetPromptResult, McpServerManagerError> {
        let mut attempts = 0;

        loop {
            match self.get_prompt_once(server_name, name, arguments.clone()).await {
                Ok(prompt) => return Ok(prompt),
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            }
        }
    }

    pub async fn list_resources(
        &mut self,
        server_name: &str,
    ) -> Result<McpListResourcesResult, McpServerManagerError> {
        let mut attempts = 0;

        loop {
            match self.list_resources_once(server_name).await {
                Ok(resources) => return Ok(resources),
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            }
        }
    }

    pub async fn read_resource(
        &mut self,
        server_name: &str,
        uri: &str,
    ) -> Result<McpReadResourceResult, McpServerManagerError> {
        let mut attempts = 0;

        loop {
            match self.read_resource_once(server_name, uri).await {
                Ok(resource) => return Ok(resource),
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            }
        }
    }

    fn register_discovered_tools(
        &mut self,
        server_tools: Vec<ManagedMcpTool>,
        discovered_tools: &mut Vec<ManagedMcpTool>,
    ) {
        for tool in server_tools {
            self.tool_index.insert(
                tool.qualified_name.clone(),
                ToolRoute {
                    server_name: tool.server_name.clone(),
                    raw_name: tool.raw_name.clone(),
                },
            );
            discovered_tools.push(tool);
        }
    }

    async fn discover_tools_for_server(
        &mut self,
        server_name: &str,
    ) -> Result<Vec<ManagedMcpTool>, McpServerManagerError> {
        let mut attempts = 0;

        loop {
            match self.discover_tools_for_server_once(server_name).await {
                Ok(tools) => return Ok(tools),
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            }
        }
    }

    async fn discover_tools_for_server_once(
        &mut self,
        server_name: &str,
    ) -> Result<Vec<ManagedMcpTool>, McpServerManagerError> {
        self.ensure_server_ready(server_name).await?;

        let mut discovered_tools = Vec::new();
        let mut cursor = None;
        loop {
            let request_id = self.take_request_id();
            let response = {
                let server = self.server_mut(server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: server_name.to_string(),
                        method: "tools/list",
                        details: "server process missing after initialization".to_string(),
                    }
                })?;
                Self::run_process_request(
                    server_name,
                    "tools/list",
                    MCP_LIST_TOOLS_TIMEOUT_MS,
                    process.list_tools(
                        request_id,
                        Some(super::McpListToolsParams {
                            cursor: cursor.clone(),
                        }),
                    ),
                )
                .await?
            };

            if let Some(error) = response.error {
                return Err(McpServerManagerError::JsonRpc {
                    server_name: server_name.to_string(),
                    method: "tools/list",
                    error,
                });
            }

            let result = response
                .result
                .ok_or_else(|| McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method: "tools/list",
                    details: "missing result payload".to_string(),
                })?;

            for tool in result.tools {
                let qualified_name = mcp_tool_name(server_name, &tool.name);
                discovered_tools.push(ManagedMcpTool {
                    server_name: server_name.to_string(),
                    qualified_name,
                    raw_name: tool.name.clone(),
                    tool,
                });
            }

            match result.next_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => break,
            }
        }

        Ok(discovered_tools)
    }

    async fn list_resources_once(
        &mut self,
        server_name: &str,
    ) -> Result<McpListResourcesResult, McpServerManagerError> {
        self.ensure_server_ready(server_name).await?;

        let mut resources = Vec::new();
        let mut cursor = None;
        loop {
            let request_id = self.take_request_id();
            let response = {
                let server = self.server_mut(server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: server_name.to_string(),
                        method: "resources/list",
                        details: "server process missing after initialization".to_string(),
                    }
                })?;
                Self::run_process_request(
                    server_name,
                    "resources/list",
                    MCP_LIST_TOOLS_TIMEOUT_MS,
                    process.list_resources(
                        request_id,
                        Some(McpListResourcesParams {
                            cursor: cursor.clone(),
                        }),
                    ),
                )
                .await?
            };

            if let Some(error) = response.error {
                return Err(McpServerManagerError::JsonRpc {
                    server_name: server_name.to_string(),
                    method: "resources/list",
                    error,
                });
            }

            let result = response
                .result
                .ok_or_else(|| McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method: "resources/list",
                    details: "missing result payload".to_string(),
                })?;

            resources.extend(result.resources);

            match result.next_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => break,
            }
        }

        Ok(McpListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn list_prompts_once(
        &mut self,
        server_name: &str,
    ) -> Result<McpListPromptsResult, McpServerManagerError> {
        self.ensure_server_ready(server_name).await?;

        let mut prompts = Vec::new();
        let mut cursor = None;
        loop {
            let request_id = self.take_request_id();
            let response = {
                let server = self.server_mut(server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: server_name.to_string(),
                        method: "prompts/list",
                        details: "server process missing after initialization".to_string(),
                    }
                })?;
                Self::run_process_request(
                    server_name,
                    "prompts/list",
                    MCP_LIST_TOOLS_TIMEOUT_MS,
                    process.list_prompts(
                        request_id,
                        Some(McpListPromptsParams {
                            cursor: cursor.clone(),
                        }),
                    ),
                )
                .await?
            };

            if let Some(error) = response.error {
                return Err(McpServerManagerError::JsonRpc {
                    server_name: server_name.to_string(),
                    method: "prompts/list",
                    error,
                });
            }

            let result = response
                .result
                .ok_or_else(|| McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method: "prompts/list",
                    details: "missing result payload".to_string(),
                })?;

            prompts.extend(result.prompts);

            match result.next_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => break,
            }
        }

        Ok(McpListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    async fn read_resource_once(
        &mut self,
        server_name: &str,
        uri: &str,
    ) -> Result<McpReadResourceResult, McpServerManagerError> {
        self.ensure_server_ready(server_name).await?;

        let request_id = self.take_request_id();
        let response =
            {
                let server = self.server_mut(server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: server_name.to_string(),
                        method: "resources/read",
                        details: "server process missing after initialization".to_string(),
                    }
                })?;
                Self::run_process_request(
                    server_name,
                    "resources/read",
                    MCP_LIST_TOOLS_TIMEOUT_MS,
                    process.read_resource(
                        request_id,
                        McpReadResourceParams {
                            uri: uri.to_string(),
                        },
                    ),
                )
                .await?
            };

        if let Some(error) = response.error {
            return Err(McpServerManagerError::JsonRpc {
                server_name: server_name.to_string(),
                method: "resources/read",
                error,
            });
        }

        response
            .result
            .ok_or_else(|| McpServerManagerError::InvalidResponse {
                server_name: server_name.to_string(),
                method: "resources/read",
                details: "missing result payload".to_string(),
            })
    }

    pub async fn discover_prompts_for_server(
        &mut self,
        server_name: &str,
    ) -> Result<Vec<ManagedMcpPrompt>, McpServerManagerError> {
        let prompts = self.list_prompts(server_name).await?;
        Ok(prompts
            .prompts
            .into_iter()
            .map(|prompt| ManagedMcpPrompt {
                server_name: server_name.to_string(),
                qualified_name: format!("mcp_prompt__{}__{}", server_name, prompt.name),
                raw_name: prompt.name.clone(),
                prompt,
            })
            .collect())
    }

    async fn get_prompt_once(
        &mut self,
        server_name: &str,
        name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<McpGetPromptResult, McpServerManagerError> {
        self.ensure_server_ready(server_name).await?;

        let request_id = self.take_request_id();
        let response = {
            let server = self.server_mut(server_name)?;
            let process = server.process.as_mut().ok_or_else(|| {
                McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method: "prompts/get",
                    details: "server process missing after initialization".to_string(),
                }
            })?;
            Self::run_process_request(
                server_name,
                "prompts/get",
                MCP_LIST_TOOLS_TIMEOUT_MS,
                process.get_prompt(
                    request_id,
                    McpGetPromptParams {
                        name: name.to_string(),
                        arguments,
                    },
                ),
            )
            .await?
        };

        if let Some(error) = response.error {
            return Err(McpServerManagerError::JsonRpc {
                server_name: server_name.to_string(),
                method: "prompts/get",
                error,
            });
        }

        response
            .result
            .ok_or_else(|| McpServerManagerError::InvalidResponse {
                server_name: server_name.to_string(),
                method: "prompts/get",
                details: "missing result payload".to_string(),
            })
    }
}
