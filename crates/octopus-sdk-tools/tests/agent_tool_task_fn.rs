use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use futures::Stream;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AssistantEvent, EventId, HookDecision, HookEvent, PermissionGate, PermissionMode,
    PermissionOutcome, SessionEvent, SessionId, StopReason, SubagentSpec, TaskBudget,
    ToolCallRequest, ToolCategory,
};
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_observability::{session_span_id, session_trace_id, NoopTracer};
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext, ParentTraceContext};
use octopus_sdk_tools::{
    builtin::AgentTool, Tool, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec,
};

mod support;

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct EchoProvider;

#[async_trait]
impl ModelProvider for EchoProvider {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
        let text = req
            .messages
            .first()
            .and_then(|message| message.content.first())
            .and_then(|block| match block {
                octopus_sdk_contracts::ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "empty".into());

        Ok(Box::pin(futures::stream::iter(vec![
            Ok(AssistantEvent::TextDelta(format!("summary: {text}"))),
            Ok(AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                input_tokens: 5,
                output_tokens: 7,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            })),
            Ok(AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            }),
        ])))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

struct DummyTool {
    spec: ToolSpec,
}

impl DummyTool {
    fn new(name: &str) -> Self {
        Self {
            spec: ToolSpec {
                name: name.into(),
                description: format!("{name} tool"),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                category: ToolCategory::Read,
            },
        }
    }
}

#[async_trait]
impl Tool for DummyTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::default())
    }
}

struct RecordingHook {
    seen: Arc<Mutex<Vec<HookEvent>>>,
}

#[async_trait]
impl Hook for RecordingHook {
    fn name(&self) -> &str {
        "recording-hook"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        self.seen
            .lock()
            .expect("seen lock should stay available")
            .push(event.clone());
        HookDecision::Continue
    }
}

#[tokio::test]
async fn test_agent_tool_with_task_fn() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let parent = ParentSessionContext {
        session_id: SessionId("parent-task-tool".into()),
        session_store: Arc::new(InMemorySessionStore::default()),
        model: Arc::new(EchoProvider),
        tools: Arc::new(tool_registry(vec!["ToolA"])),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root.path().to_path_buf()),
        trace: parent_trace("parent-task-tool"),
    };
    let tool = AgentTool::new().with_task_fn(OrchestratorWorkers::new(parent, 2).into_task_fn());
    let result = tool
        .execute(
            support::tool_context(
                root.path(),
                Arc::new(support::StubAskResolver {
                    answer: Err(octopus_sdk_contracts::AskError::NotResolvable),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({
                "spec": sample_spec("worker-1"),
                "input": "build summary"
            }),
        )
        .await
        .expect("agent tool should succeed with injected task fn");

    assert!(!result.is_error);
    assert_eq!(support::text_output(result), "summary: build summary");
}

#[tokio::test]
async fn test_agent_tool_emits_subagent_hooks() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let parent = ParentSessionContext {
        session_id: SessionId("parent-task-tool".into()),
        session_store: Arc::new(InMemorySessionStore::default()),
        model: Arc::new(EchoProvider),
        tools: Arc::new(tool_registry(vec!["ToolA"])),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root.path().to_path_buf()),
        trace: parent_trace("parent-task-tool"),
    };
    let tool = AgentTool::new().with_task_fn(OrchestratorWorkers::new(parent, 2).into_task_fn());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let ctx = support::tool_context(
        root.path(),
        Arc::new(support::StubAskResolver {
            answer: Err(octopus_sdk_contracts::AskError::NotResolvable),
        }),
        Arc::new(support::RecordingEventSink::new()),
    );
    ctx.hooks.register(
        "record-subagent",
        Arc::new(RecordingHook {
            seen: Arc::clone(&seen),
        }),
        HookSource::Workspace,
        10,
    );

    let result = tool
        .execute(
            ctx,
            serde_json::json!({
                "spec": sample_spec("worker-1"),
                "input": "build summary"
            }),
        )
        .await
        .expect("agent tool should succeed with injected task fn");

    assert!(!result.is_error);
    assert_eq!(support::text_output(result), "summary: build summary");

    let events = seen
        .lock()
        .expect("seen lock should stay available")
        .clone();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        HookEvent::SubagentSpawn { parent_session, spec }
            if parent_session.0 == "session-1" && spec.id == "worker-1"
    ));
    assert!(matches!(
        &events[1],
        HookEvent::SubagentReturn { parent_session, summary }
            if parent_session.0 == "session-1" && summary.session_id.0 == "child-0"
    ));
}

fn sample_spec(id: &str) -> SubagentSpec {
    SubagentSpec {
        id: id.into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: vec!["ToolA".into()],
        agent_role: "worker".into(),
        model_role: "subagent-default".into(),
        permission_mode: PermissionMode::Default,
        task_budget: TaskBudget {
            total: 100,
            completion_threshold: 0.9,
        },
        max_turns: 2,
        depth: 1,
    }
}

fn parent_trace(session_id: &str) -> ParentTraceContext {
    ParentTraceContext {
        trace_id: session_trace_id(session_id),
        span_id: session_span_id(session_id),
        agent_role: "main".into(),
        model_id: "main".into(),
        model_version: "test".into(),
        config_snapshot_id: "cfg-parent".into(),
        tracer: Arc::new(NoopTracer),
    }
}

fn tool_registry(tool_names: Vec<&str>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for name in tool_names {
        registry
            .register(Arc::new(DummyTool::new(name)))
            .expect("tool should register");
    }

    registry
}

#[derive(Default)]
struct InMemorySessionStore {
    next_event: AtomicU64,
    next_child: AtomicU64,
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn append(&self, _id: &SessionId, _event: SessionEvent) -> Result<EventId, SessionError> {
        Ok(EventId(format!(
            "event-{}",
            self.next_event.fetch_add(1, Ordering::Relaxed)
        )))
    }

    async fn append_session_started(
        &self,
        _id: &SessionId,
        _working_dir: std::path::PathBuf,
        _permission_mode: octopus_sdk_contracts::PermissionMode,
        _model: String,
        _config_snapshot_id: String,
        _effective_config_hash: String,
        _token_budget: u32,
        _plugins_snapshot: Option<octopus_sdk_contracts::PluginsSnapshot>,
    ) -> Result<EventId, SessionError> {
        Ok(EventId(format!(
            "event-{}",
            self.next_event.fetch_add(1, Ordering::Relaxed)
        )))
    }

    async fn new_child_session(
        &self,
        _parent_id: &SessionId,
        _spec: &SubagentSpec,
    ) -> Result<SessionId, SessionError> {
        Ok(SessionId(format!(
            "child-{}",
            self.next_child.fetch_add(1, Ordering::Relaxed)
        )))
    }

    async fn stream(
        &self,
        _id: &SessionId,
        _range: EventRange,
    ) -> Result<EventStream, SessionError> {
        let empty: Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>> =
            Box::pin(futures::stream::empty());
        Ok(empty)
    }

    async fn snapshot(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn fork(&self, _id: &SessionId, _from: EventId) -> Result<SessionId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn wake(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        Err(SessionError::NotFound)
    }
}
