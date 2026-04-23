use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AssistantEvent, PermissionGate, PermissionMode, PermissionOutcome, PluginSourceTag,
    PluginSummary, PluginsSnapshot, RenderKind, Role, SessionEvent, SessionId, StopReason,
    SubagentOutput, SubagentSpec, TaskBudget, ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_observability::{session_span_id, session_trace_id, NoopTracer};
use octopus_sdk_session::{EventRange, SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext, ParentTraceContext};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct EchoProvider {
    turns: Mutex<Vec<Vec<AssistantEvent>>>,
}

#[async_trait]
impl ModelProvider for EchoProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::iter(
            self.turns
                .lock()
                .expect("turns mutex should stay available")
                .remove(0)
                .into_iter()
                .map(Ok),
        )))
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
async fn test_parent_child_isolation() {
    let runtime = test_runtime();
    let workers = OrchestratorWorkers::new(runtime.parent.clone(), 5);
    let results = workers
        .run(
            vec![sample_spec(vec!["ToolA"])],
            vec!["child answer".to_string()],
        )
        .await;

    let output = results
        .into_iter()
        .next()
        .expect("one result expected")
        .expect("worker should succeed");
    let child_session_id = match output {
        SubagentOutput::Summary { meta, .. } | SubagentOutput::FileRef { meta, .. } => {
            meta.session_id
        }
        SubagentOutput::Json { meta, .. } => meta.session_id,
    };

    let child_events = collect_events(&runtime.store, &child_session_id).await;
    assert!(child_events.iter().all(|event| match event {
        SessionEvent::ToolExecuted { name, .. } => name != "ToolB" && name != "ToolC",
        _ => true,
    }));
    assert!(child_events.iter().any(|event| matches!(
        event,
        SessionEvent::ToolExecuted { name, .. } if name == "ToolA"
    )));

    let parent_events = collect_events(&runtime.store, &runtime.parent.session_id).await;
    let non_started = parent_events
        .into_iter()
        .filter(|event| !matches!(event, SessionEvent::SessionStarted { .. }))
        .collect::<Vec<_>>();

    assert_eq!(non_started.len(), 1);
    match &non_started[0] {
        SessionEvent::Render { blocks, .. } => {
            let block = &blocks[0];
            assert_eq!(block.kind, RenderKind::Markdown);
            assert_eq!(block.payload["title"], "subagent.summary");
            assert!(block.payload["text"]
                .as_str()
                .is_some_and(|text| text.contains("- child answer")));
        }
        other => panic!("expected condensed render event, got {other:?}"),
    }

    assert!(non_started.iter().all(|event| match event {
        SessionEvent::Render { .. } => true,
        SessionEvent::UserMessage(message) | SessionEvent::AssistantMessage(message) => {
            message.role != Role::Tool
        }
        _ => false,
    }));
}

#[derive(Clone)]
struct TestRuntime {
    parent: ParentSessionContext,
    store: Arc<SqliteJsonlSessionStore>,
}

fn test_runtime() -> TestRuntime {
    let root = tempfile::tempdir().expect("tempdir should create").keep();
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    let store =
        Arc::new(SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open"));
    let parent_session = SessionId("parent-condensed".into());

    futures::executor::block_on(store.append_session_started(
        &parent_session,
        std::path::PathBuf::from("."),
        octopus_sdk_contracts::PermissionMode::Default,
        "main".into(),
        "cfg-parent".into(),
        "hash-parent".into(),
        8_192,
        Some(sample_plugins_snapshot()),
    ))
    .expect("parent session should start");

    let parent = ParentSessionContext {
        session_id: parent_session,
        session_store: store.clone(),
        model: Arc::new(EchoProvider {
            turns: Mutex::new(vec![
                vec![
                    AssistantEvent::ToolUse {
                        id: octopus_sdk_contracts::ToolCallId("tool-a-1".into()),
                        name: "ToolA".into(),
                        input: serde_json::json!({}),
                    },
                    AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                        input_tokens: 2,
                        output_tokens: 2,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop {
                        stop_reason: StopReason::ToolUse,
                    },
                ],
                vec![
                    AssistantEvent::TextDelta("child answer".into()),
                    AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                        input_tokens: 4,
                        output_tokens: 6,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop {
                        stop_reason: StopReason::EndTurn,
                    },
                ],
            ]),
        }),
        tools: Arc::new(tool_registry(vec!["ToolA", "ToolB", "ToolC"])),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root),
        trace: ParentTraceContext {
            trace_id: session_trace_id("parent-condensed"),
            span_id: session_span_id("parent-condensed"),
            agent_role: "main".into(),
            model_id: "main".into(),
            model_version: "test".into(),
            config_snapshot_id: "cfg-parent".into(),
            tracer: Arc::new(NoopTracer),
        },
    };

    TestRuntime { parent, store }
}

async fn collect_events(
    store: &SqliteJsonlSessionStore,
    session_id: &SessionId,
) -> Vec<SessionEvent> {
    store
        .stream(session_id, EventRange::default())
        .await
        .expect("stream should open")
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|event| event.expect("event should decode"))
        .collect::<Vec<_>>()
}

fn sample_spec(allowed_tools: Vec<&str>) -> SubagentSpec {
    SubagentSpec {
        id: "researcher".into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: allowed_tools.into_iter().map(str::to_string).collect(),
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

fn tool_registry(tool_names: Vec<&str>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for name in tool_names {
        registry
            .register(Arc::new(DummyTool::new(name)))
            .expect("tool should register");
    }

    registry
}

fn sample_plugins_snapshot() -> PluginsSnapshot {
    PluginsSnapshot {
        api_version: "1.0.0".into(),
        plugins: vec![PluginSummary {
            id: "example-noop-tool".into(),
            version: "0.1.0".into(),
            git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
            source: PluginSourceTag::Bundled,
            enabled: true,
            components_count: 1,
        }],
    }
}
