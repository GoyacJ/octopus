use std::{collections::BTreeMap, sync::Arc};

use harness_contracts::{
    canonical_mcp_tool_name, now, DeferPolicy, DeferredToolHint, Event, McpServerId,
    McpToolsListChangedEvent, ToolDeferredPoolChangedEvent, ToolPoolChangeSource,
    ToolsListChangedDisposition,
};
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
    pub injected_tools: BTreeMap<String, DeferPolicy>,
    pub pending_list_changed: bool,
}

pub type ListChangedDisposition = ToolsListChangedDisposition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListChangedOutcome {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub disposition: ListChangedDisposition,
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
                injected_tools: BTreeMap::new(),
                pending_list_changed: false,
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
                injected_tools: BTreeMap::new(),
                pending_list_changed: false,
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
        let mut injected_snapshot = BTreeMap::new();

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
            injected_snapshot.insert(canonical.clone(), defer_policy);
            registered.push(canonical);
        }

        if !registered.is_empty() {
            if let Some(managed) = self.inner.write().await.get_mut(server_id) {
                managed.injected_tools = injected_snapshot;
            }
        }

        Ok(registered)
    }

    pub async fn handle_list_changed(
        &self,
        tool_registry: &ToolRegistry,
        server_id: &McpServerId,
        event_sink: Arc<dyn McpEventSink>,
    ) -> Result<ListChangedOutcome, McpError> {
        let managed = self
            .inner
            .read()
            .await
            .get(server_id)
            .cloned()
            .ok_or_else(|| McpError::ServerNotFound(server_id.0.clone()))?;
        let latest = self.snapshot_for_latest_tools(
            &managed,
            server_id,
            managed.connection.list_tools().await?,
        )?;
        let latest_policies = latest
            .iter()
            .map(|(name, (_, policy))| (name.clone(), *policy))
            .collect::<BTreeMap<_, _>>();
        let added = latest
            .keys()
            .filter(|name| !managed.injected_tools.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();
        let removed = managed
            .injected_tools
            .keys()
            .filter(|name| !latest.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();

        if added.is_empty() && removed.is_empty() {
            let outcome = ListChangedOutcome {
                added,
                removed,
                disposition: ToolsListChangedDisposition::NoChange,
            };
            emit_tools_list_changed(&managed, server_id, &outcome, &event_sink);
            return Ok(outcome);
        }

        let has_always_load_delta = added.iter().chain(removed.iter()).any(|name| {
            latest_policies
                .get(name)
                .or_else(|| managed.injected_tools.get(name))
                == Some(&DeferPolicy::AlwaysLoad)
        });

        if has_always_load_delta {
            if let Some(managed) = self.inner.write().await.get_mut(server_id) {
                managed.pending_list_changed = true;
            }
            let outcome = ListChangedOutcome {
                added,
                removed,
                disposition: ToolsListChangedDisposition::PendingForReload,
            };
            emit_tools_list_changed(&managed, server_id, &outcome, &event_sink);
            return Ok(outcome);
        }

        for name in &removed {
            if tool_registry.get(name).is_some() {
                tool_registry.deregister(name)?;
            }
        }
        for name in &added {
            let (mcp_tool, defer_policy) = latest
                .get(name)
                .cloned()
                .ok_or_else(|| McpError::Protocol(format!("missing added tool: {name}")))?;
            let tool = McpToolWrapper::new(
                server_id.clone(),
                managed.spec.source.clone(),
                managed.spec.trust,
                mcp_tool,
                Arc::clone(&managed.connection),
                defer_policy,
                name.clone(),
            );
            tool_registry.register(Box::new(tool))?;
        }
        if let Some(managed) = self.inner.write().await.get_mut(server_id) {
            managed.injected_tools = latest_policies;
            managed.pending_list_changed = false;
        }

        let outcome = ListChangedOutcome {
            added,
            removed,
            disposition: ToolsListChangedDisposition::DeferredApplied,
        };
        emit_tools_list_changed(&managed, server_id, &outcome, &event_sink);
        emit_deferred_pool_changed(&managed, server_id, &outcome, tool_registry, &event_sink);
        Ok(outcome)
    }

    pub async fn pending_list_changed_servers(&self) -> Vec<McpServerId> {
        self.inner
            .read()
            .await
            .iter()
            .filter_map(|(server_id, managed)| {
                managed.pending_list_changed.then_some(server_id.clone())
            })
            .collect()
    }

    fn snapshot_for_latest_tools(
        &self,
        managed: &ManagedMcpServer,
        server_id: &McpServerId,
        latest_tools: Vec<McpToolDescriptor>,
    ) -> Result<BTreeMap<String, (McpToolDescriptor, DeferPolicy)>, McpError> {
        let mut latest = BTreeMap::new();
        for mcp_tool in latest_tools {
            let canonical = canonical_tool_name(server_id, &mcp_tool.name)?;
            match managed.spec.tool_filter.evaluate(&canonical) {
                FilterDecision::Inject => {
                    let defer_policy = resolve_defer_policy(&mcp_tool);
                    latest.insert(canonical, (mcp_tool, defer_policy));
                }
                FilterDecision::Skip { .. } => {}
                FilterDecision::Reject { reason } => return Err(McpError::FilterConflict(reason)),
            }
        }
        Ok(latest)
    }
}

fn emit_tools_list_changed(
    managed: &ManagedMcpServer,
    server_id: &McpServerId,
    outcome: &ListChangedOutcome,
    event_sink: &Arc<dyn McpEventSink>,
) {
    event_sink.emit(Event::McpToolsListChanged(McpToolsListChangedEvent {
        session_id: session_id_for_scope(&managed.scope),
        server_id: server_id.clone(),
        received_at: now(),
        pending_since: (outcome.disposition == ToolsListChangedDisposition::PendingForReload)
            .then(now),
        added_count: outcome.added.len().try_into().unwrap_or(u32::MAX),
        removed_count: outcome.removed.len().try_into().unwrap_or(u32::MAX),
        disposition: outcome.disposition.clone(),
    }));
}

fn emit_deferred_pool_changed(
    managed: &ManagedMcpServer,
    server_id: &McpServerId,
    outcome: &ListChangedOutcome,
    tool_registry: &ToolRegistry,
    event_sink: &Arc<dyn McpEventSink>,
) {
    let McpServerScope::Session(session_id) = managed.scope else {
        return;
    };
    event_sink.emit(Event::ToolDeferredPoolChanged(
        ToolDeferredPoolChangedEvent {
            session_id,
            added: outcome
                .added
                .iter()
                .cloned()
                .map(|name| DeferredToolHint { name, hint: None })
                .collect(),
            removed: outcome.removed.clone(),
            source: ToolPoolChangeSource::McpListChanged {
                server_id: server_id.clone(),
            },
            deferred_total: tool_registry
                .snapshot()
                .as_descriptors()
                .into_iter()
                .filter(|descriptor| descriptor.properties.defer_policy == DeferPolicy::AutoDefer)
                .count()
                .try_into()
                .unwrap_or(u32::MAX),
            at: now(),
        },
    ));
}

fn session_id_for_scope(scope: &McpServerScope) -> Option<harness_contracts::SessionId> {
    match scope {
        McpServerScope::Session(session_id) => Some(*session_id),
        McpServerScope::Global | McpServerScope::Agent(_) => None,
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
