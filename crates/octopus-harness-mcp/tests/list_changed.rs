use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use harness_contracts::{
    Event, McpServerId, McpServerSource, SessionId, ToolPoolChangeSource,
    ToolsListChangedDisposition,
};
use harness_mcp::{
    ListChangedDisposition, McpConnection, McpError, McpEventSink, McpRegistry, McpServerScope,
    McpServerSpec, McpToolDescriptor, McpToolResult, TransportChoice,
};
use harness_tool::{BuiltinToolset, ToolRegistry};
use serde_json::{json, Value};

#[tokio::test]
async fn auto_defer_list_changed_updates_registry_and_events() {
    let tool_registry = ToolRegistry::builder()
        .with_builtin_toolset(BuiltinToolset::Empty)
        .build()
        .expect("registry");
    let connection = Arc::new(MutableTools::new(vec![tool("old", false)]));
    let registry = registry_with(
        connection.clone(),
        McpServerScope::Session(SessionId::from_u128(1)),
    )
    .await;
    registry
        .inject_tools_into(&tool_registry, &server_id())
        .await
        .expect("initial inject");
    connection.set_tools(vec![tool("new", false)]);
    let sink = Arc::new(CollectingSink::default());

    let outcome = registry
        .handle_list_changed(&tool_registry, &server_id(), sink.clone())
        .await
        .expect("list changed");

    assert_eq!(outcome.disposition, ListChangedDisposition::DeferredApplied);
    assert!(tool_registry.get("mcp__fixture__old").is_none());
    assert!(tool_registry.get("mcp__fixture__new").is_some());
    let events = sink.events();
    assert!(matches!(
        events.first(),
        Some(Event::McpToolsListChanged(event))
            if event.added_count == 1
                && event.removed_count == 1
                && event.disposition == ToolsListChangedDisposition::DeferredApplied
    ));
    assert!(matches!(
        events.get(1),
        Some(Event::ToolDeferredPoolChanged(event))
            if matches!(
                event.source,
                ToolPoolChangeSource::McpListChanged { server_id: ref changed_server_id }
                    if changed_server_id == &server_id()
            )
    ));
}

#[tokio::test]
async fn always_load_list_changed_is_pending_for_reload() {
    let tool_registry = ToolRegistry::builder()
        .with_builtin_toolset(BuiltinToolset::Empty)
        .build()
        .expect("registry");
    let connection = Arc::new(MutableTools::new(vec![tool("old", false)]));
    let registry = registry_with(connection.clone(), McpServerScope::Global).await;
    registry
        .inject_tools_into(&tool_registry, &server_id())
        .await
        .expect("initial inject");
    connection.set_tools(vec![tool("old", false), tool("always", true)]);

    let outcome = registry
        .handle_list_changed(
            &tool_registry,
            &server_id(),
            Arc::new(CollectingSink::default()),
        )
        .await
        .expect("list changed");

    assert_eq!(
        outcome.disposition,
        ListChangedDisposition::PendingForReload
    );
    assert!(tool_registry.get("mcp__fixture__always").is_none());
    assert_eq!(
        registry.pending_list_changed_servers().await,
        vec![server_id()]
    );
}

#[tokio::test]
async fn unchanged_or_rejected_list_changed_does_not_mutate_registry() {
    let tool_registry = ToolRegistry::builder()
        .with_builtin_toolset(BuiltinToolset::Empty)
        .build()
        .expect("registry");
    let connection = Arc::new(MutableTools::new(vec![tool("old", false)]));
    let registry = registry_with(connection.clone(), McpServerScope::Global).await;
    registry
        .inject_tools_into(&tool_registry, &server_id())
        .await
        .expect("initial inject");

    let no_change = registry
        .handle_list_changed(
            &tool_registry,
            &server_id(),
            Arc::new(CollectingSink::default()),
        )
        .await
        .expect("no change");
    assert_eq!(no_change.disposition, ListChangedDisposition::NoChange);

    connection.set_tools(vec![tool("bad name", false)]);
    let rejected = registry
        .handle_list_changed(
            &tool_registry,
            &server_id(),
            Arc::new(CollectingSink::default()),
        )
        .await;
    assert!(matches!(rejected, Err(McpError::ToolNamingViolation(_))));
    assert!(tool_registry.get("mcp__fixture__old").is_some());
}

async fn registry_with(connection: Arc<MutableTools>, scope: McpServerScope) -> McpRegistry {
    let registry = McpRegistry::new();
    registry
        .add_ready_server(spec(), scope, connection)
        .await
        .expect("server");
    registry
}

fn spec() -> McpServerSpec {
    McpServerSpec::new(
        server_id(),
        "fixture",
        TransportChoice::InProcess,
        McpServerSource::Workspace,
    )
}

fn server_id() -> McpServerId {
    McpServerId("fixture".to_owned())
}

fn tool(name: &str, always_load: bool) -> McpToolDescriptor {
    let mut meta = BTreeMap::new();
    if always_load {
        meta.insert("anthropic/alwaysLoad".to_owned(), json!(true));
    }
    McpToolDescriptor {
        name: name.to_owned(),
        description: Some(format!("{name} tool")),
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        meta,
    }
}

struct MutableTools {
    tools: Mutex<Vec<McpToolDescriptor>>,
}

impl MutableTools {
    fn new(tools: Vec<McpToolDescriptor>) -> Self {
        Self {
            tools: Mutex::new(tools),
        }
    }

    fn set_tools(&self, tools: Vec<McpToolDescriptor>) {
        *self.tools.lock().expect("tools") = tools;
    }
}

#[async_trait]
impl McpConnection for MutableTools {
    fn connection_id(&self) -> &'static str {
        "mutable-tools"
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        Ok(self.tools.lock().expect("tools").clone())
    }

    async fn call_tool(&self, _name: &str, _args: Value) -> Result<McpToolResult, McpError> {
        Ok(McpToolResult::text("ok"))
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }
}

#[derive(Default)]
struct CollectingSink {
    events: Mutex<Vec<Event>>,
}

impl CollectingSink {
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("events").clone()
    }
}

impl McpEventSink for CollectingSink {
    fn emit(&self, event: Event) {
        self.events.lock().expect("events").push(event);
    }
}
