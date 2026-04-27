use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use harness_contracts::{
    Event, McpConnectionLostReason, McpServerId, McpServerSource, SessionId, TrustLevel,
};
use harness_mcp::{
    ListChangedEvent, ManagedMcpConnection, McpChange, McpConnection, McpConnectionState, McpError,
    McpEventSink, McpRegistry, McpServerScope, McpServerSpec, McpToolDescriptor, McpToolResult,
    McpTransport, ReconnectPolicy, TransportChoice,
};
use harness_tool::ToolRegistry;
use serde_json::{json, Value};
use tokio::sync::Notify;

#[test]
fn reconnect_policy_backoff_caps_and_unlimited_attempts() {
    let policy = ReconnectPolicy {
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_millis(250),
        backoff_jitter: 0.0,
        ..ReconnectPolicy::default()
    };

    policy.validate().expect("policy validates");
    assert_eq!(policy.backoff_for_attempt(1), Duration::from_millis(100));
    assert_eq!(policy.backoff_for_attempt(2), Duration::from_millis(200));
    assert_eq!(policy.backoff_for_attempt(3), Duration::from_millis(250));
    assert!(!policy.is_exhausted(10));

    let invalid = ReconnectPolicy {
        backoff_jitter: 1.5,
        ..ReconnectPolicy::default()
    };
    assert!(invalid.validate().is_err());
}

#[tokio::test]
async fn managed_connection_emits_first_recovered_on_initial_connect() {
    let sink = Arc::new(RecordingSink::default());
    let managed = managed_connection(
        policy(0),
        MockTransport::new(vec![Ok(MockConnection::default())]),
        sink.clone(),
    )
    .await;

    assert_eq!(managed.state().await, McpConnectionState::Ready);
    assert!(sink.events().iter().any(|event| {
        matches!(
            event,
            Event::McpConnectionRecovered(recovered)
                if recovered.was_first
                    && recovered.server_id == McpServerId("reconnect".into())
                    && recovered.attempts_used == 0
        )
    }));
}

#[tokio::test]
async fn managed_connection_enters_reconnecting_after_call_connection_error() {
    let notify = Arc::new(Notify::new());
    let sink = Arc::new(RecordingSink::default());
    let managed = managed_connection(
        policy(0),
        MockTransport::new(vec![
            Ok(MockConnection::with_results(vec![Err(
                McpError::Connection("lost".into()),
            )])),
            Ok(MockConnection::with_results(vec![Ok(McpToolResult::text(
                "after",
            ))])),
        ])
        .with_attempt_notify(notify.clone()),
        sink.clone(),
    )
    .await;

    assert!(matches!(
        managed.call_tool("search", json!({})).await,
        Err(McpError::Connection(_))
    ));
    wait_for_reconnecting(&managed).await;
    assert_eq!(
        managed.call_tool("search", json!({})).await,
        Err(McpError::Connection("mcp server reconnecting".into()))
    );
    assert!(sink.events().iter().any(|event| {
        matches!(
            event,
            Event::McpConnectionLost(lost)
                if lost.attempts_so_far == 0
                    && !lost.terminal
                    && matches!(lost.reason, McpConnectionLostReason::Other(_))
        )
    }));

    notify.notified().await;
}

#[tokio::test]
async fn managed_connection_reconnects_and_allows_calls_again() {
    let notify = Arc::new(Notify::new());
    let sink = Arc::new(RecordingSink::default());
    let managed = managed_connection(
        policy(0),
        MockTransport::new(vec![
            Ok(MockConnection::with_results(vec![Err(
                McpError::Connection("lost".into()),
            )])),
            Ok(MockConnection::with_results(vec![Ok(McpToolResult::text(
                "after",
            ))])),
        ])
        .with_attempt_notify(notify.clone()),
        sink.clone(),
    )
    .await;

    assert!(managed.call_tool("search", json!({})).await.is_err());
    notify.notified().await;
    wait_for_ready(&managed).await;

    assert_eq!(
        managed.call_tool("search", json!({})).await,
        Ok(McpToolResult::text("after"))
    );
    assert!(sink.events().iter().any(|event| {
        matches!(
            event,
            Event::McpConnectionRecovered(recovered)
                if !recovered.was_first
                    && recovered.attempts_used == 1
                    && !recovered.schema_changed
        )
    }));
}

#[tokio::test]
async fn managed_connection_terminal_failure_after_max_attempts() {
    let notify = Arc::new(Notify::new());
    let sink = Arc::new(RecordingSink::default());
    let managed = managed_connection(
        policy(1),
        MockTransport::new(vec![
            Ok(MockConnection::with_results(vec![Err(
                McpError::Connection("lost".into()),
            )])),
            Err(McpError::Connection("still down".into())),
        ])
        .with_attempt_notify(notify.clone()),
        sink.clone(),
    )
    .await;

    assert!(managed.call_tool("search", json!({})).await.is_err());
    notify.notified().await;
    wait_for_failed(&managed).await;

    assert!(matches!(
        managed.call_tool("search", json!({})).await,
        Err(McpError::Connection(_))
    ));
    assert!(sink.events().iter().any(|event| {
        matches!(
            event,
            Event::McpConnectionLost(lost)
                if lost.terminal && lost.attempts_so_far == 1
        )
    }));
}

#[tokio::test]
async fn managed_connection_resets_attempts_after_success_reset_window() {
    let notify = Arc::new(Notify::new());
    let sink = Arc::new(RecordingSink::default());
    let mut reconnect = policy(0);
    reconnect.success_reset_after = Duration::from_millis(10);
    let managed = managed_connection(
        reconnect,
        MockTransport::new(vec![
            Ok(MockConnection::with_results(vec![Err(
                McpError::Connection("lost".into()),
            )])),
            Ok(MockConnection::with_results(vec![Ok(McpToolResult::text(
                "after",
            ))])),
        ])
        .with_attempt_notify(notify.clone()),
        sink,
    )
    .await;

    assert!(managed.call_tool("search", json!({})).await.is_err());
    notify.notified().await;
    wait_for_ready(&managed).await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    assert_eq!(managed.attempts_so_far(), 0);
}

#[tokio::test]
async fn registry_add_managed_server_injects_tools_after_initial_connect() {
    let registry = McpRegistry::new();
    let spec = spec(policy(0));
    let server_id = spec.server_id.clone();
    registry
        .add_managed_server(
            spec,
            McpServerScope::Session(SessionId::new()),
            Arc::new(MockTransport::new(vec![Ok(MockConnection {
                tools: vec![tool("search")],
                ..Default::default()
            })])),
            Arc::new(RecordingSink::default()),
        )
        .await
        .expect("managed server registered");

    let tool_registry = ToolRegistry::builder().build().expect("tool registry");
    let injected = registry
        .inject_tools_into(&tool_registry, &server_id)
        .await
        .expect("tools inject");

    assert_eq!(injected, vec!["mcp__reconnect__search"]);
    assert_eq!(
        tool_registry
            .snapshot()
            .descriptor("mcp__reconnect__search")
            .expect("descriptor exists")
            .trust_level,
        TrustLevel::AdminTrusted
    );
}

fn policy(max_attempts: u32) -> ReconnectPolicy {
    ReconnectPolicy {
        max_attempts,
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(1),
        backoff_jitter: 0.0,
        success_reset_after: Duration::from_secs(60),
        keep_deferred_during_reconnect: true,
    }
}

fn spec(reconnect: ReconnectPolicy) -> McpServerSpec {
    let mut spec = McpServerSpec::new(
        McpServerId("reconnect".into()),
        "Reconnect",
        TransportChoice::InProcess,
        McpServerSource::Workspace,
    );
    spec.reconnect = reconnect;
    spec
}

async fn managed_connection(
    reconnect: ReconnectPolicy,
    transport: MockTransport,
    sink: Arc<RecordingSink>,
) -> ManagedMcpConnection {
    ManagedMcpConnection::connect(
        Arc::new(transport),
        spec(reconnect),
        McpServerScope::Session(SessionId::new()),
        sink,
    )
    .await
    .expect("managed connection")
}

async fn wait_for_ready(managed: &ManagedMcpConnection) {
    wait_for_state(managed, |state| matches!(state, McpConnectionState::Ready)).await;
}

async fn wait_for_reconnecting(managed: &ManagedMcpConnection) {
    wait_for_state(managed, |state| {
        matches!(state, McpConnectionState::Reconnecting { .. })
    })
    .await;
}

async fn wait_for_failed(managed: &ManagedMcpConnection) {
    wait_for_state(managed, |state| {
        matches!(state, McpConnectionState::Failed { .. })
    })
    .await;
}

async fn wait_for_state(
    managed: &ManagedMcpConnection,
    predicate: impl Fn(&McpConnectionState) -> bool,
) {
    for _ in 0..100 {
        let state = managed.state().await;
        if predicate(&state) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    panic!("state did not converge: {:?}", managed.state().await);
}

fn tool(name: &str) -> McpToolDescriptor {
    McpToolDescriptor {
        name: name.into(),
        description: Some(format!("{name} tool")),
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        meta: BTreeMap::new(),
    }
}

#[derive(Default)]
struct RecordingSink {
    events: Mutex<Vec<Event>>,
}

impl RecordingSink {
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("events lock").clone()
    }
}

impl McpEventSink for RecordingSink {
    fn emit(&self, event: Event) {
        self.events.lock().expect("events lock").push(event);
    }
}

#[derive(Clone)]
struct MockTransport {
    outcomes: Arc<Mutex<VecDeque<Result<MockConnection, McpError>>>>,
    attempt_notify: Option<Arc<Notify>>,
}

impl MockTransport {
    fn new(outcomes: Vec<Result<MockConnection, McpError>>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(VecDeque::from(outcomes))),
            attempt_notify: None,
        }
    }

    fn with_attempt_notify(mut self, notify: Arc<Notify>) -> Self {
        self.attempt_notify = Some(notify);
        self
    }
}

#[async_trait]
impl McpTransport for MockTransport {
    fn transport_id(&self) -> &'static str {
        "mock"
    }

    async fn connect(&self, _spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        if let Some(notify) = &self.attempt_notify {
            notify.notify_waiters();
        }
        self.outcomes
            .lock()
            .expect("outcomes lock")
            .pop_front()
            .unwrap_or_else(|| Err(McpError::Connection("no mock outcome".into())))
            .map(|connection| Arc::new(connection) as Arc<dyn McpConnection>)
    }
}

#[derive(Default)]
struct MockConnection {
    tools: Vec<McpToolDescriptor>,
    results: Mutex<VecDeque<Result<McpToolResult, McpError>>>,
}

impl MockConnection {
    fn with_results(results: Vec<Result<McpToolResult, McpError>>) -> Self {
        Self {
            tools: vec![tool("search")],
            results: Mutex::new(VecDeque::from(results)),
        }
    }
}

#[async_trait]
impl McpConnection for MockConnection {
    fn connection_id(&self) -> &'static str {
        "mock-connection"
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        Ok(self.tools.clone())
    }

    async fn call_tool(&self, _name: &str, _args: Value) -> Result<McpToolResult, McpError> {
        self.results
            .lock()
            .expect("results lock")
            .pop_front()
            .unwrap_or_else(|| Ok(McpToolResult::text("ok")))
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, McpError> {
        Ok(Box::pin(futures::stream::iter([
            McpChange::ToolsListChanged,
        ])))
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }
}
