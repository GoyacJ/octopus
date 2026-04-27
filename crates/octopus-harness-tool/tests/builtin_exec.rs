#![cfg(feature = "builtin-toolset")]

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{future::BoxFuture, stream, StreamExt};
use harness_contracts::{
    AgentId, CapabilityRegistry, ClarifyAnswer, ClarifyPrompt, Decision, DecisionScope,
    OutboundUserMessage, PermissionError, PermissionSubject, SandboxError, SandboxExitStatus,
    TenantId, ToolCapability, ToolError, ToolResult, ToolUseId, UserMessageDelivery,
    WorkspaceAccess,
};
use harness_permission::{PermissionBroker, PermissionCheck, PermissionContext, PermissionRequest};
use harness_sandbox::{
    ActivityHandle, ExecContext, ExecOutcome, ExecSpec, KillScope, ProcessHandle, SandboxBackend,
    SandboxCapabilities, SessionSnapshotFile, SnapshotSpec,
};
use harness_tool::{
    builtin::{
        BashTool, ClarifyTool, SendMessageTool, WebSearchBackend, WebSearchRequest,
        WebSearchResult, WebSearchTool,
    },
    BuiltinToolset, InterruptToken, Tool, ToolContext, ToolRegistry,
};
use parking_lot::Mutex;
use serde_json::{json, Value};

#[tokio::test]
async fn bash_requires_sandbox_and_maps_command_permission() {
    let tool = BashTool::default();
    let input = json!({ "command": "printf hi", "cwd": "/tmp" });
    let check = tool
        .check_permission(&input, &tool_ctx(CapabilityRegistry::default(), None))
        .await;

    assert!(matches!(
        check,
        PermissionCheck::AskUser {
            subject: PermissionSubject::CommandExec { ref command, .. },
            scope: DecisionScope::ExactCommand { command: ref scoped_command, .. },
        } if command == "printf hi" && scoped_command == "printf hi"
    ));

    let error = execute_error(&tool, input, tool_ctx(CapabilityRegistry::default(), None)).await;
    assert!(matches!(
        error,
        ToolError::CapabilityMissing(ToolCapability::Custom(ref cap)) if cap == "sandbox_backend"
    ));
}

#[tokio::test]
async fn bash_executes_through_sandbox_and_collects_output() {
    let sandbox = Arc::new(FakeSandbox::new(
        Bytes::from_static(b"hello\n"),
        Bytes::from_static(b"warn\n"),
        SandboxExitStatus::Code(0),
    ));
    let tool = BashTool::default();

    let result = execute_final(
        &tool,
        json!({ "command": "echo hello", "cwd": "/work" }),
        tool_ctx(CapabilityRegistry::default(), Some(sandbox.clone())),
    )
    .await;

    let ToolResult::Structured(value) = result else {
        panic!("expected structured bash result");
    };
    assert_eq!(value["stdout"], "hello\n");
    assert_eq!(value["stderr"], "warn\n");
    assert_eq!(value["exit_status"], json!({ "code": 0 }));

    let specs = sandbox.recorded_execs();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].command, "echo hello");
    assert_eq!(
        specs[0].workspace_access,
        WorkspaceAccess::ReadWrite {
            allowed_writable_subpaths: vec![]
        }
    );
}

#[tokio::test]
async fn web_search_uses_network_permission_and_backend() {
    let tool = WebSearchTool::new(vec![Arc::new(FakeWebSearchBackend)]);
    let input = json!({ "query": "octopus harness", "max_results": 1 });
    let check = tool
        .check_permission(&input, &tool_ctx(CapabilityRegistry::default(), None))
        .await;

    assert!(matches!(
        check,
        PermissionCheck::AskUser {
            subject: PermissionSubject::NetworkAccess { ref host, .. },
            scope: DecisionScope::ToolName(ref tool),
        } if host == "web-search" && tool == "WebSearch"
    ));

    let result = execute_final(&tool, input, tool_ctx(CapabilityRegistry::default(), None)).await;
    assert_eq!(
        result,
        ToolResult::Structured(json!([{
            "title": "Octopus",
            "url": "https://example.test/octopus",
            "snippet": "Harness result",
            "score": 0.9
        }]))
    );

    let error = execute_error(
        &WebSearchTool::default(),
        json!({ "query": "octopus" }),
        tool_ctx(CapabilityRegistry::default(), None),
    )
    .await;
    assert!(matches!(
        error,
        ToolError::CapabilityMissing(ToolCapability::Custom(ref cap)) if cap == "web_search_backend"
    ));
}

#[tokio::test]
async fn clarify_and_send_message_use_capability_registry() {
    let mut caps = CapabilityRegistry::default();
    let clarify: Arc<dyn harness_contracts::ClarifyChannelCap> = Arc::new(FakeClarify);
    let messenger: Arc<dyn harness_contracts::UserMessengerCap> = Arc::new(FakeMessenger);
    caps.install(ToolCapability::ClarifyChannel, clarify);
    caps.install(ToolCapability::UserMessenger, messenger);

    let clarify_result = execute_final(
        &ClarifyTool::default(),
        json!({
            "prompt": "Pick one",
            "choices": [{ "id": "a", "label": "A" }],
            "multiple": false
        }),
        tool_ctx(caps.clone(), None),
    )
    .await;
    assert_eq!(
        clarify_result,
        ToolResult::Structured(json!({
            "answer": "A",
            "chosen_ids": ["a"]
        }))
    );

    let send_result = execute_final(
        &SendMessageTool::default(),
        json!({ "channel": "desktop", "body": "done" }),
        tool_ctx(caps, None),
    )
    .await;
    assert_eq!(
        send_result,
        ToolResult::Structured(json!({
            "message_id": "msg-1",
            "delivered": true
        }))
    );
}

#[test]
fn default_builtin_toolset_registers_m3_t04b_tools_without_forbidden_deps() {
    let registry = ToolRegistry::builder()
        .with_builtin_toolset(BuiltinToolset::Default)
        .build()
        .unwrap();

    for name in ["Bash", "WebSearch", "Clarify", "SendMessage"] {
        assert!(registry.get(name).is_some(), "{name} should be registered");
    }

    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    assert!(!manifest.contains("octopus-harness-model"));
    assert!(!manifest.contains("octopus-harness-journal"));
    assert!(!manifest.contains("octopus-harness-hook"));
}

async fn execute_final(tool: &dyn Tool, input: Value, ctx: ToolContext) -> ToolResult {
    tool.validate(&input, &ctx).await.unwrap();
    let mut stream = tool.execute(input, ctx).await.unwrap();
    match stream.next().await {
        Some(harness_tool::ToolEvent::Final(result)) => result,
        other => panic!("expected final result, got {other:?}"),
    }
}

async fn execute_error(tool: &dyn Tool, input: Value, ctx: ToolContext) -> ToolError {
    tool.validate(&input, &ctx).await.unwrap();
    match tool.execute(input, ctx).await {
        Ok(_) => panic!("expected tool error"),
        Err(error) => error,
    }
}

fn tool_ctx(
    cap_registry: CapabilityRegistry,
    sandbox: Option<Arc<dyn SandboxBackend>>,
) -> ToolContext {
    ToolContext {
        tool_use_id: ToolUseId::new(),
        run_id: harness_contracts::RunId::new(),
        session_id: harness_contracts::SessionId::new(),
        tenant_id: TenantId::SINGLE,
        agent_id: AgentId::from_u128(1),
        sandbox,
        permission_broker: Arc::new(AllowBroker),
        cap_registry: Arc::new(cap_registry),
        interrupt: InterruptToken::default(),
        parent_run: None,
    }
}

#[derive(Debug)]
struct AllowBroker;

#[async_trait]
impl PermissionBroker for AllowBroker {
    async fn decide(&self, _request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        Decision::AllowOnce
    }

    async fn persist(
        &self,
        _decision_id: harness_contracts::DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        Ok(())
    }
}

#[derive(Debug)]
struct FakeSandbox {
    recorded_execs: Mutex<Vec<ExecSpec>>,
    stdout: Bytes,
    stderr: Bytes,
    exit_status: SandboxExitStatus,
}

impl FakeSandbox {
    fn new(stdout: Bytes, stderr: Bytes, exit_status: SandboxExitStatus) -> Self {
        Self {
            recorded_execs: Mutex::new(Vec::new()),
            stdout,
            stderr,
            exit_status,
        }
    }

    fn recorded_execs(&self) -> Vec<ExecSpec> {
        self.recorded_execs.lock().clone()
    }
}

#[async_trait]
impl SandboxBackend for FakeSandbox {
    fn backend_id(&self) -> &'static str {
        "fake"
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            supports_streaming: true,
            ..SandboxCapabilities::default()
        }
    }

    async fn execute(
        &self,
        spec: ExecSpec,
        _ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError> {
        self.recorded_execs.lock().push(spec);
        Ok(ProcessHandle {
            pid: Some(42),
            stdout: Some(Box::pin(stream::once({
                let stdout = self.stdout.clone();
                async move { stdout }
            }))),
            stderr: Some(Box::pin(stream::once({
                let stderr = self.stderr.clone();
                async move { stderr }
            }))),
            stdin: None,
            cwd_marker: None,
            activity: Arc::new(FakeActivity {
                exit_status: self.exit_status.clone(),
            }),
        })
    }

    async fn snapshot_session(
        &self,
        _spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError> {
        Err(SandboxError::Message("not implemented".to_owned()))
    }

    async fn restore_session(&self, _snapshot: &SessionSnapshotFile) -> Result<(), SandboxError> {
        Err(SandboxError::Message("not implemented".to_owned()))
    }

    async fn shutdown(&self) -> Result<(), SandboxError> {
        Ok(())
    }
}

#[derive(Debug)]
struct FakeActivity {
    exit_status: SandboxExitStatus,
}

#[async_trait]
impl ActivityHandle for FakeActivity {
    async fn wait(&self) -> Result<ExecOutcome, SandboxError> {
        Ok(ExecOutcome {
            exit_status: self.exit_status.clone(),
            ..ExecOutcome::default()
        })
    }

    async fn kill(&self, _signal: i32, _scope: KillScope) -> Result<(), SandboxError> {
        Ok(())
    }

    fn touch(&self) {}

    fn last_activity(&self) -> Instant {
        Instant::now()
    }
}

struct FakeWebSearchBackend;

#[async_trait]
impl WebSearchBackend for FakeWebSearchBackend {
    async fn search(&self, request: WebSearchRequest) -> Result<Vec<WebSearchResult>, ToolError> {
        assert_eq!(request.query, "octopus harness");
        assert_eq!(request.max_results, Some(1));
        Ok(vec![WebSearchResult {
            title: "Octopus".to_owned(),
            url: "https://example.test/octopus".to_owned(),
            snippet: "Harness result".to_owned(),
            score: 0.9,
        }])
    }
}

struct FakeClarify;

impl harness_contracts::ClarifyChannelCap for FakeClarify {
    fn ask(&self, prompt: ClarifyPrompt) -> BoxFuture<'static, Result<ClarifyAnswer, ToolError>> {
        assert_eq!(prompt.prompt, "Pick one");
        Box::pin(async {
            Ok(ClarifyAnswer {
                answer: "A".to_owned(),
                chosen_ids: vec!["a".to_owned()],
            })
        })
    }
}

struct FakeMessenger;

impl harness_contracts::UserMessengerCap for FakeMessenger {
    fn send(
        &self,
        message: OutboundUserMessage,
    ) -> BoxFuture<'static, Result<UserMessageDelivery, ToolError>> {
        assert_eq!(message.channel, "desktop");
        assert_eq!(message.body, "done");
        Box::pin(async {
            Ok(UserMessageDelivery {
                message_id: "msg-1".to_owned(),
                delivered: true,
            })
        })
    }
}
