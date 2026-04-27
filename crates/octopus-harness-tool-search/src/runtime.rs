use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use harness_contracts::{Event, ToolDescriptor, ToolError, ToolName};
use harness_model::ModelCapabilities;

use crate::ReloadHandle;

pub const TOOL_SEARCH_RUNTIME_CAPABILITY: &str = "tool_search_runtime";

#[async_trait]
pub trait ToolSearchRuntimeCap: Send + Sync + 'static {
    async fn snapshot(&self) -> Result<ToolSearchRuntimeSnapshot, ToolError>;

    async fn emit_event(&self, _event: Event) -> Result<(), ToolError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ToolSearchRuntimeSnapshot {
    pub deferred_tools: Vec<ToolDescriptor>,
    pub loaded_tool_names: BTreeSet<ToolName>,
    pub discovered_tool_names: BTreeSet<ToolName>,
    pub pending_mcp_servers: Vec<String>,
    pub model_caps: Arc<ModelCapabilities>,
    pub reload_handle: Option<Arc<dyn ReloadHandle>>,
}
