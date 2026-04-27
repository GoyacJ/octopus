use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::StreamExt;
use harness_contracts::{
    canonical_mcp_tool_name, CapabilityRegistry, Decision, DecisionId, DecisionScope, DeferPolicy,
    McpServerId, McpServerSource, PermissionError, PluginId, SessionId, TenantId, ToolResult,
    ToolUseId, TrustLevel,
};
use harness_mcp::{
    collapse_reserved_separator, trust_level_for_source, FilterConflict, FilterDecision,
    JsonRpcRequest, JsonRpcResponse, ListChangedEvent, McpChange, McpClient, McpConnection,
    McpRegistry, McpResource, McpResourceContents, McpServerScope, McpServerSpec, McpTimeouts,
    McpToolDescriptor, McpToolFilter, McpToolGlob, McpToolResult, ReconnectPolicy, SamplingPolicy,
    StdioEnv, TransportChoice,
};
use harness_permission::{PermissionBroker, PermissionContext, PermissionRequest};
use harness_tool::{InterruptToken, ToolContext, ToolEvent, ToolRegistry};
use serde_json::{json, Value};

#[test]
fn jsonrpc_request_response_round_trips() {
    let request = JsonRpcRequest::new(
        json!(7),
        "tools/call",
        Some(json!({ "name": "grep", "arguments": { "pattern": "mcp" } })),
    );

    let value = serde_json::to_value(&request).expect("request serializes");
    let decoded: JsonRpcRequest = serde_json::from_value(value).expect("request deserializes");

    assert_eq!(decoded.jsonrpc, "2.0");
    assert_eq!(decoded.method, "tools/call");
    assert_eq!(
        decoded.params,
        Some(json!({ "name": "grep", "arguments": { "pattern": "mcp" } }))
    );

    let response = JsonRpcResponse::success(json!(7), json!({ "ok": true }));
    let value = serde_json::to_value(&response).expect("response serializes");
    let decoded: JsonRpcResponse = serde_json::from_value(value).expect("response deserializes");

    assert_eq!(decoded.result, Some(json!({ "ok": true })));
    assert!(decoded.error.is_none());
}

#[tokio::test]
async fn transport_and_connection_traits_are_object_safe() {
    let transport: Arc<dyn harness_mcp::McpTransport> =
        Arc::new(MockTransport::new(MockConnection::default()));
    let spec = server_spec("slack", McpServerSource::Workspace);

    let connection = McpClient::new(transport)
        .connect(spec)
        .await
        .expect("mock transport connects");

    assert_eq!(connection.connection_id(), "mock");
}

#[test]
fn server_source_derives_trust_level() {
    assert_eq!(
        trust_level_for_source(&McpServerSource::Workspace),
        TrustLevel::AdminTrusted
    );
    assert_eq!(
        trust_level_for_source(&McpServerSource::Policy),
        TrustLevel::AdminTrusted
    );
    assert_eq!(
        trust_level_for_source(&McpServerSource::Managed {
            registry_url: "https://registry.example".into()
        }),
        TrustLevel::AdminTrusted
    );
    assert_eq!(
        trust_level_for_source(&McpServerSource::User),
        TrustLevel::UserControlled
    );
    assert_eq!(
        trust_level_for_source(&McpServerSource::Dynamic {
            registered_by: "user".into()
        }),
        TrustLevel::UserControlled
    );
    assert_eq!(
        trust_level_for_source(&McpServerSource::Plugin(PluginId("plugin".into()))),
        TrustLevel::UserControlled,
        "plugin source lacks trust in contracts, so MCP fails closed"
    );
}

#[test]
fn stdio_default_env_denies_common_credentials() {
    let deny = StdioEnv::default_deny_envs();

    for key in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "GITHUB_TOKEN",
        "KUBECONFIG",
        "NPM_TOKEN",
        "HARNESS_*",
    ] {
        assert!(deny.contains(key), "missing deny env {key}");
    }

    assert!(matches!(
        StdioEnv::default(),
        StdioEnv::InheritWithDeny { .. }
    ));
}

#[test]
fn canonical_mcp_names_reject_or_collapse_reserved_separator() {
    assert_eq!(
        canonical_mcp_tool_name("slack", "post_message").expect("canonical name"),
        "mcp__slack__post_message"
    );
    assert!(canonical_mcp_tool_name("bad__server", "post_message").is_err());

    assert_eq!(
        collapse_reserved_separator(&McpServerId("slack".into()), "bulk__import")
            .expect("collapsed canonical name"),
        "mcp__slack__bulk_import"
    );
}

#[test]
fn tool_filter_applies_allow_deny_and_conflict_policy() {
    let filter = McpToolFilter {
        allow: vec![McpToolGlob("mcp__slack__*".into())],
        deny: vec![McpToolGlob("mcp__slack__delete_*".into())],
        on_conflict: FilterConflict::DenyWins,
    };

    assert_eq!(
        filter.evaluate("mcp__slack__post_message"),
        FilterDecision::Inject
    );
    assert!(matches!(
        filter.evaluate("mcp__slack__delete_channel"),
        FilterDecision::Skip { .. }
    ));
    assert!(matches!(
        filter.evaluate("mcp__github__create_issue"),
        FilterDecision::Skip { .. }
    ));
}

#[tokio::test]
async fn registry_injects_mcp_tool_wrapper_and_executes_mock_connection() {
    let connection = MockConnection {
        tools: vec![McpToolDescriptor {
            name: "post_message".into(),
            description: Some("Post a message".into()),
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } }
            }),
            output_schema: None,
            meta: BTreeMap::new(),
        }],
        ..Default::default()
    };
    connection
        .results
        .lock()
        .expect("results lock")
        .push_back(McpToolResult::text("sent"));

    let mcp_registry = McpRegistry::new();
    let server_id = McpServerId("slack".into());
    let spec = server_spec("slack", McpServerSource::Workspace);
    mcp_registry
        .add_ready_server(
            spec,
            McpServerScope::Session(SessionId::new()),
            Arc::new(connection),
        )
        .await
        .expect("server registers");

    let tool_registry = ToolRegistry::builder().build().expect("tool registry");
    let injected = mcp_registry
        .inject_tools_into(&tool_registry, &server_id)
        .await
        .expect("tools inject");

    assert_eq!(injected, vec!["mcp__slack__post_message"]);
    let descriptor = tool_registry
        .snapshot()
        .descriptor("mcp__slack__post_message")
        .expect("descriptor exists")
        .as_ref()
        .clone();
    assert_eq!(descriptor.properties.defer_policy, DeferPolicy::AutoDefer);
    assert_eq!(descriptor.trust_level, TrustLevel::AdminTrusted);

    let tool = tool_registry
        .get("mcp__slack__post_message")
        .expect("tool registered");
    let mut stream = tool
        .execute(json!({ "text": "hello" }), tool_context())
        .await
        .expect("tool executes");

    let event = stream.next().await.expect("final event");
    assert_eq!(event, ToolEvent::Final(ToolResult::Text("sent".into())));
}

#[test]
fn policy_defaults_are_fail_closed_or_bounded() {
    assert_eq!(
        McpTimeouts::default().handshake.as_secs(),
        5,
        "handshake timeout should stay bounded"
    );
    assert_eq!(
        ReconnectPolicy::default().max_attempts,
        0,
        "0 means unlimited retries"
    );
    assert!(ReconnectPolicy::default().keep_deferred_during_reconnect);
    assert!(SamplingPolicy::denied().is_denied());
}

fn server_spec(id: &str, source: McpServerSource) -> McpServerSpec {
    McpServerSpec::new(
        McpServerId(id.into()),
        format!("{id} server"),
        TransportChoice::InProcess,
        source,
    )
}

fn tool_context() -> ToolContext {
    ToolContext {
        tool_use_id: ToolUseId::new(),
        run_id: harness_contracts::RunId::new(),
        session_id: SessionId::new(),
        tenant_id: TenantId::SINGLE,
        agent_id: harness_contracts::AgentId::from_u128(1),
        workspace_root: std::path::PathBuf::from("."),
        sandbox: None,
        permission_broker: Arc::new(AllowBroker),
        cap_registry: Arc::new(CapabilityRegistry::default()),
        interrupt: InterruptToken::new(),
        parent_run: None,
    }
}

#[derive(Default)]
struct MockConnection {
    tools: Vec<McpToolDescriptor>,
    results: Mutex<VecDeque<McpToolResult>>,
}

#[async_trait]
impl McpConnection for MockConnection {
    fn connection_id(&self) -> &'static str {
        "mock"
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, harness_mcp::McpError> {
        Ok(self.tools.clone())
    }

    async fn call_tool(
        &self,
        _name: &str,
        _args: Value,
    ) -> Result<McpToolResult, harness_mcp::McpError> {
        self.results
            .lock()
            .expect("results lock")
            .pop_front()
            .ok_or_else(|| harness_mcp::McpError::Protocol("missing mock result".into()))
    }

    async fn list_resources(&self) -> Result<Vec<McpResource>, harness_mcp::McpError> {
        Ok(Vec::new())
    }

    async fn read_resource(
        &self,
        _uri: &str,
    ) -> Result<McpResourceContents, harness_mcp::McpError> {
        Err(harness_mcp::McpError::Protocol("not implemented".into()))
    }

    async fn subscribe_changes(&self) -> Result<ListChangedEvent, harness_mcp::McpError> {
        Ok(Box::pin(futures::stream::empty::<McpChange>()))
    }

    async fn shutdown(&self) -> Result<(), harness_mcp::McpError> {
        Ok(())
    }
}

struct MockTransport {
    connection: Arc<dyn McpConnection>,
}

impl MockTransport {
    fn new(connection: MockConnection) -> Self {
        Self {
            connection: Arc::new(connection),
        }
    }
}

#[async_trait]
impl harness_mcp::McpTransport for MockTransport {
    fn transport_id(&self) -> &'static str {
        "mock"
    }

    async fn connect(
        &self,
        _spec: McpServerSpec,
    ) -> Result<Arc<dyn McpConnection>, harness_mcp::McpError> {
        Ok(Arc::clone(&self.connection))
    }
}

struct AllowBroker;

#[async_trait]
impl PermissionBroker for AllowBroker {
    async fn decide(&self, _request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        Decision::AllowOnce
    }

    async fn persist(
        &self,
        _decision_id: DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        Ok(())
    }
}
