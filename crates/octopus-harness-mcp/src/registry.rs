use std::{collections::BTreeMap, sync::Arc};

use harness_contracts::{canonical_mcp_tool_name, DeferPolicy, McpServerId};
use harness_tool::ToolRegistry;
use tokio::sync::RwLock;

use crate::{
    trust_level_for_source, FilterDecision, ManagedMcpConnection, McpConnection, McpError,
    McpEventSink, McpServerScope, McpServerSpec, McpToolDescriptor, McpToolWrapper, McpTransport,
};

#[derive(Clone, Default)]
pub struct McpRegistry {
    inner: Arc<RwLock<BTreeMap<McpServerId, ManagedMcpServer>>>,
}

#[derive(Clone)]
pub struct ManagedMcpServer {
    pub spec: McpServerSpec,
    pub scope: McpServerScope,
    pub connection: Arc<dyn McpConnection>,
    pub injected_tools: Vec<String>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add_ready_server(
        &self,
        spec: McpServerSpec,
        scope: McpServerScope,
        connection: Arc<dyn McpConnection>,
    ) -> Result<(), McpError> {
        let derived = trust_level_for_source(&spec.source);
        if spec.trust != derived {
            return Err(McpError::Protocol(format!(
                "trust mismatch for {}: expected {:?}, got {:?}",
                spec.server_id.0, derived, spec.trust
            )));
        }

        self.inner.write().await.insert(
            spec.server_id.clone(),
            ManagedMcpServer {
                spec,
                scope,
                connection,
                injected_tools: Vec::new(),
            },
        );
        Ok(())
    }

    pub async fn add_managed_server(
        &self,
        spec: McpServerSpec,
        scope: McpServerScope,
        transport: Arc<dyn McpTransport>,
        event_sink: Arc<dyn McpEventSink>,
    ) -> Result<(), McpError> {
        let derived = trust_level_for_source(&spec.source);
        if spec.trust != derived {
            return Err(McpError::Protocol(format!(
                "trust mismatch for {}: expected {:?}, got {:?}",
                spec.server_id.0, derived, spec.trust
            )));
        }

        let connection = Arc::new(
            ManagedMcpConnection::connect(transport, spec.clone(), scope.clone(), event_sink)
                .await?,
        );
        self.inner.write().await.insert(
            spec.server_id.clone(),
            ManagedMcpServer {
                spec,
                scope,
                connection,
                injected_tools: Vec::new(),
            },
        );
        Ok(())
    }

    pub async fn inject_tools_into(
        &self,
        tool_registry: &ToolRegistry,
        server_id: &McpServerId,
    ) -> Result<Vec<String>, McpError> {
        let managed = self
            .inner
            .read()
            .await
            .get(server_id)
            .cloned()
            .ok_or_else(|| McpError::ServerNotFound(server_id.0.clone()))?;

        let mcp_tools = managed.connection.list_tools().await?;
        let mut registered = Vec::new();

        for mcp_tool in mcp_tools {
            let canonical = canonical_tool_name(server_id, &mcp_tool.name)?;
            match managed.spec.tool_filter.evaluate(&canonical) {
                FilterDecision::Inject => {}
                FilterDecision::Skip { .. } => continue,
                FilterDecision::Reject { reason } => return Err(McpError::FilterConflict(reason)),
            }

            let defer_policy = resolve_defer_policy(&mcp_tool);
            let tool = McpToolWrapper::new(
                server_id.clone(),
                managed.spec.source.clone(),
                managed.spec.trust,
                mcp_tool,
                Arc::clone(&managed.connection),
                defer_policy,
                canonical.clone(),
            );
            tool_registry.register(Box::new(tool))?;
            registered.push(canonical);
        }

        if !registered.is_empty() {
            if let Some(managed) = self.inner.write().await.get_mut(server_id) {
                managed.injected_tools = registered.clone();
            }
        }

        Ok(registered)
    }
}

pub fn collapse_reserved_separator(
    server_id: &McpServerId,
    upstream: &str,
) -> Result<String, McpError> {
    let collapsed = upstream.replace("__", "_");
    canonical_mcp_tool_name(&server_id.0, &collapsed)
        .map_err(|error| McpError::ToolNamingViolation(error.to_string()))
}

fn canonical_tool_name(server_id: &McpServerId, upstream: &str) -> Result<String, McpError> {
    match canonical_mcp_tool_name(&server_id.0, upstream) {
        Ok(name) => Ok(name),
        Err(harness_contracts::ToolNameError::ReservedSeparator(_)) => {
            collapse_reserved_separator(server_id, upstream)
        }
        Err(error) => Err(McpError::ToolNamingViolation(error.to_string())),
    }
}

fn resolve_defer_policy(mcp_tool: &McpToolDescriptor) -> DeferPolicy {
    match mcp_tool.meta.get("anthropic/alwaysLoad") {
        Some(serde_json::Value::Bool(true)) => DeferPolicy::AlwaysLoad,
        _ => DeferPolicy::AutoDefer,
    }
}
