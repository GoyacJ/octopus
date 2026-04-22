use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use async_trait::async_trait;
use futures::Stream;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AssistantEvent, EventId, PermissionGate, PermissionMode, PermissionOutcome, SessionEvent,
    SessionId, StopReason, SubagentOutput, SubagentSpec, SubagentSummary, TaskBudget,
    ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct SlowEchoProvider {
    delay_ms: u64,
}

#[async_trait]
impl ModelProvider for SlowEchoProvider {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
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
            Ok(AssistantEvent::TextDelta(text)),
            Ok(AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                input_tokens: 5,
                output_tokens: 5,
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

#[tokio::test]
async fn test_fan_out_concurrency() {
    let runtime = test_runtime(100);
    let workers = OrchestratorWorkers::new(runtime.parent, 5);
    let specs = (0..10)
        .map(|index| sample_spec(&format!("worker-{index}")))
        .collect::<Vec<_>>();
    let inputs = (0..10)
        .map(|index| format!("worker-{index}"))
        .collect::<Vec<_>>();
    let started = Instant::now();
    let results = workers.run(specs, inputs).await;
    let elapsed = started.elapsed();

    assert_eq!(results.len(), 10);
    assert!(
        elapsed <= std::time::Duration::from_millis(300),
        "expected 5-wide concurrency, elapsed = {elapsed:?}"
    );
    assert!(results.into_iter().all(|result| matches!(
        result,
        Ok(SubagentOutput::Summary { .. }) | Ok(SubagentOutput::FileRef { .. })
    )));
}

#[test]
fn test_fan_in_merge() {
    let merged = OrchestratorWorkers::fan_in(vec![
        summary_output("alpha", "session-a", 1, 10, 20),
        summary_output("beta", "session-b", 2, 20, 30),
        summary_output("gamma", "session-c", 3, 30, 40),
    ]);

    match merged {
        SubagentOutput::Summary { text, meta } => {
            assert!(text.contains("- alpha"));
            assert!(text.contains("- beta"));
            assert!(text.contains("- gamma"));
            assert_eq!(meta.session_id.0, "session-a");
            assert_eq!(meta.turns, 6);
            assert_eq!(meta.tokens_used, 60);
            assert_eq!(meta.duration_ms, 90);
            assert_eq!(meta.trace_id, "fan-in:trace-session-a");
        }
        other => panic!("expected merged summary, got {other:?}"),
    }
}

struct TestRuntime {
    parent: ParentSessionContext,
}

fn test_runtime(delay_ms: u64) -> TestRuntime {
    let root = tempfile::tempdir().expect("tempdir should create").keep();
    let store = Arc::new(InMemorySessionStore::default());
    let parent_session = SessionId("parent-fan-out".into());

    let parent = ParentSessionContext {
        session_id: parent_session,
        session_store: store,
        model: Arc::new(SlowEchoProvider { delay_ms }),
        tools: Arc::new(tool_registry(vec!["ToolA"])),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root),
    };

    TestRuntime { parent }
}

fn sample_spec(id: &str) -> SubagentSpec {
    SubagentSpec {
        id: id.into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: vec!["ToolA".into()],
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

fn summary_output(
    text: &str,
    session_id: &str,
    turns: u16,
    tokens_used: u32,
    duration_ms: u64,
) -> SubagentOutput {
    SubagentOutput::Summary {
        text: text.into(),
        meta: SubagentSummary {
            session_id: SessionId(session_id.into()),
            turns,
            tokens_used,
            duration_ms,
            trace_id: format!("trace-{session_id}"),
        },
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
