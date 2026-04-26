use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    DecisionScope, PermissionSubject, ToolCapability, ToolDescriptor, ToolError, ToolGroup,
    ToolResult,
};
use harness_permission::PermissionCheck;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchRequest {
    pub query: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub score: f64,
}

#[async_trait]
pub trait WebSearchBackend: Send + Sync + 'static {
    async fn search(&self, request: WebSearchRequest) -> Result<Vec<WebSearchResult>, ToolError>;
}

#[derive(Clone)]
pub struct WebSearchTool {
    descriptor: ToolDescriptor,
    backends: Vec<Arc<dyn WebSearchBackend>>,
}

impl WebSearchTool {
    pub fn new(backends: Vec<Arc<dyn WebSearchBackend>>) -> Self {
        Self {
            descriptor: Self::default_descriptor(),
            backends,
        }
    }

    fn default_descriptor() -> ToolDescriptor {
        super::descriptor(
            "WebSearch",
            "Web search",
            "Search the web using a configured backend.",
            ToolGroup::Network,
            true,
            true,
            false,
            32_000,
            Vec::new(),
            super::object_schema(
                &["query"],
                json!({
                    "query": { "type": "string" },
                    "max_results": { "type": "integer", "minimum": 1 }
                }),
            ),
        )
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self {
            descriptor: Self::default_descriptor(),
            backends: Vec::new(),
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        request(input)?;
        Ok(())
    }

    async fn check_permission(&self, _input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: PermissionSubject::NetworkAccess {
                host: "web-search".to_owned(),
                port: None,
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let backend = self.backends.first().ok_or_else(|| {
            ToolError::CapabilityMissing(ToolCapability::Custom("web_search_backend".to_owned()))
        })?;
        let results = backend
            .search(request(&input).map_err(validation_error)?)
            .await?;
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(
                serde_json::to_value(results)
                    .map_err(|error| ToolError::Message(error.to_string()))?,
            ),
        )])))
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn request(input: &Value) -> Result<WebSearchRequest, ValidationError> {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ValidationError::from("query is required"))?
        .to_owned();
    let max_results = input
        .get("max_results")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    Ok(WebSearchRequest { query, max_results })
}
