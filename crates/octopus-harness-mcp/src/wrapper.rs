use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    BudgetMetric, DecisionScope, DeferPolicy, McpOrigin, McpServerId, McpServerSource,
    OverflowAction, PermissionSubject, ProviderRestriction, ResultBudget, SemverString,
    ToolDescriptor, ToolError, ToolGroup, ToolOrigin, ToolProperties, ToolResult, ToolResultPart,
    TrustLevel,
};
use harness_tool::{PermissionCheck, Tool, ToolContext, ToolEvent, ToolStream, ValidationError};
use serde_json::Value;

use crate::{McpConnection, McpContent, McpError, McpToolDescriptor, McpToolResult};

#[derive(Clone)]
pub struct McpToolWrapper {
    descriptor: ToolDescriptor,
    upstream_name: String,
    connection: Arc<dyn McpConnection>,
    server_id: McpServerId,
}

impl McpToolWrapper {
    pub fn new(
        server_id: McpServerId,
        server_source: McpServerSource,
        server_trust: TrustLevel,
        mcp_tool: McpToolDescriptor,
        connection: Arc<dyn McpConnection>,
        defer_policy: DeferPolicy,
        canonical_name: String,
    ) -> Self {
        let upstream_name = mcp_tool.name.clone();
        let description = mcp_tool
            .description
            .clone()
            .unwrap_or_else(|| format!("MCP tool {upstream_name}"));
        let descriptor = ToolDescriptor {
            name: canonical_name,
            display_name: upstream_name.clone(),
            description: description.clone(),
            category: "mcp".to_owned(),
            group: ToolGroup::Network,
            version: SemverString::from("0.1.0"),
            input_schema: mcp_tool.input_schema,
            output_schema: mcp_tool.output_schema,
            dynamic_schema: false,
            properties: ToolProperties {
                is_concurrency_safe: false,
                is_read_only: false,
                is_destructive: true,
                long_running: None,
                defer_policy,
            },
            trust_level: server_trust,
            required_capabilities: Vec::new(),
            budget: ResultBudget {
                metric: BudgetMetric::Chars,
                limit: 64_000,
                on_overflow: OverflowAction::Truncate,
                preview_head_chars: 4_000,
                preview_tail_chars: 1_000,
            },
            provider_restriction: ProviderRestriction::All,
            origin: ToolOrigin::Mcp(McpOrigin {
                server_id: server_id.clone(),
                upstream_name: upstream_name.clone(),
                server_meta: mcp_tool.meta,
                server_source,
                server_trust,
            }),
            search_hint: Some(description),
        };

        Self {
            descriptor,
            upstream_name,
            connection,
            server_id,
        }
    }

    pub fn upstream_name(&self) -> &str {
        &self.upstream_name
    }
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        if input.is_object() {
            Ok(())
        } else {
            Err(ValidationError::from(
                "mcp tool input must be a JSON object",
            ))
        }
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: PermissionSubject::McpToolCall {
                server: self.server_id.0.clone(),
                tool: self.upstream_name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let result = self
            .connection
            .call_tool(&self.upstream_name, input)
            .await
            .map_err(to_tool_error)?;

        if result.is_error {
            return Ok(Box::pin(stream::iter([ToolEvent::Error(
                ToolError::Message(result_error_message(&result)),
            )])));
        }

        Ok(Box::pin(stream::iter([ToolEvent::Final(
            into_tool_result(result),
        )])))
    }
}

fn to_tool_error(error: McpError) -> ToolError {
    ToolError::Message(error.to_string())
}

fn into_tool_result(result: McpToolResult) -> ToolResult {
    let mut content = result.content;
    if content.len() == 1 {
        return match content.remove(0) {
            McpContent::Text { text } => ToolResult::Text(text),
            McpContent::Json { value } => ToolResult::Structured(value),
        };
    }

    ToolResult::Mixed(
        content
            .into_iter()
            .map(|part| match part {
                McpContent::Text { text } => ToolResultPart::Text { text },
                McpContent::Json { value } => ToolResultPart::Structured {
                    value,
                    schema_ref: None,
                },
            })
            .collect(),
    )
}

fn result_error_message(result: &McpToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|content| match content {
            McpContent::Text { text } => Some(text.clone()),
            McpContent::Json { .. } => None,
        })
        .unwrap_or_else(|| "mcp tool returned an error".to_owned())
}
