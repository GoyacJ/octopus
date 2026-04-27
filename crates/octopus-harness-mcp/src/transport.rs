use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use futures::{stream, Stream};
use serde_json::Value;

use crate::{
    McpError, McpPrompt, McpPromptMessages, McpResource, McpResourceContents, McpServerSpec,
    McpToolDescriptor, McpToolResult,
};

pub type ListChangedEvent = Pin<Box<dyn Stream<Item = McpChange> + Send + 'static>>;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum McpChange {
    ToolsListChanged,
    ResourcesUpdated { uri: Option<String> },
    PromptsListChanged,
}

#[async_trait]
pub trait McpTransport: Send + Sync + 'static {
    fn transport_id(&self) -> &str;

    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError>;
}

#[async_trait]
pub trait McpConnection: Send + Sync + 'static {
    fn connection_id(&self) -> &str;

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError>;

    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError>;

    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        Ok(Vec::new())
    }

    async fn read_resource(&self, uri: &str) -> Result<McpResourceContents, McpError> {
        Err(McpError::Protocol(format!(
            "resources/read not implemented for {uri}"
        )))
    }

    async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        Ok(Vec::new())
    }

    async fn get_prompt(&self, name: &str, _args: Value) -> Result<McpPromptMessages, McpError> {
        Err(McpError::Protocol(format!(
            "prompts/get not implemented for {name}"
        )))
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, McpError> {
        Ok(Box::pin(stream::empty()))
    }

    async fn shutdown(&self) -> Result<(), McpError>;
}
