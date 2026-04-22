use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    AssistantEvent, PermissionGate, PermissionMode, PermissionOutcome, PluginSourceTag,
    PluginSummary, PluginsSnapshot, SessionId, StopReason, SubagentOutput, SubagentSpec,
    TaskBudget, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext, SubagentContext};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct StaticModelProvider {
    turns: Mutex<Vec<Vec<AssistantEvent>>>,
    requests: AtomicUsize,
}

#[async_trait]
impl ModelProvider for StaticModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        self.requests.fetch_add(1, Ordering::SeqCst);
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
async fn test_allowed_tools_is_subset() {
    let runtime = test_runtime(
        vec![AssistantEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        }],
        vec!["ToolA", "ToolB", "ToolC"],
    );
    let context =
        SubagentContext::from_parent(runtime.parent, sample_spec(1, vec!["ToolA", "ToolZ"]));

    assert_eq!(context.allowed_tools(), vec!["ToolA".to_string()]);

    let allowed = context
        .permissions
        .check(&ToolCallRequest {
            id: ToolCallId("call-allow".into()),
            name: "ToolA".into(),
            input: serde_json::json!({}),
        })
        .await;
    let denied = context
        .permissions
        .check(&ToolCallRequest {
            id: ToolCallId("call-deny".into()),
            name: "ToolB".into(),
            input: serde_json::json!({}),
        })
        .await;

    assert_eq!(allowed, PermissionOutcome::Allow);
    assert_eq!(
        denied,
        PermissionOutcome::Deny {
            reason: "tool 'ToolB' is not allowed for subagent".into(),
        }
    );
}

#[tokio::test]
async fn test_subagent_file_ref_switch() {
    let small_text = "a".repeat(1_024);
    let large_text = "b".repeat(5_120);

    let small_runtime = test_runtime(
        stream_events(&small_text, 40, StopReason::EndTurn),
        vec!["ToolA"],
    );
    let small_workers = OrchestratorWorkers::new(small_runtime.parent.clone(), 1);
    let small_output = small_workers
        .run_worker(sample_spec(1, vec!["ToolA"]), "summarize the repo")
        .await
        .expect("small summary should succeed");

    match small_output {
        SubagentOutput::Summary { text, meta } => {
            assert_eq!(text.len(), 1_024);
            let snapshot = small_runtime
                .store
                .snapshot(&meta.session_id)
                .await
                .expect("child snapshot should exist");
            assert_eq!(snapshot.usage.input_tokens, 20);
            assert_eq!(snapshot.usage.output_tokens, 20);
        }
        other => panic!("expected inline summary, got {other:?}"),
    }

    let large_runtime = test_runtime(
        stream_events(&large_text, 60, StopReason::EndTurn),
        vec!["ToolA"],
    );
    let large_workers = OrchestratorWorkers::new(large_runtime.parent.clone(), 1);
    let large_output = large_workers
        .run_worker(sample_spec(1, vec!["ToolA"]), "write a long note")
        .await
        .expect("large summary should succeed");

    match large_output {
        SubagentOutput::FileRef { path, bytes, meta } => {
            assert_eq!(
                path,
                PathBuf::from("runtime/notes").join(format!("{}.md", meta.session_id.0))
            );
            assert_eq!(bytes, 5_120);
            let stored = fs::read_to_string(large_runtime.root.join(&path))
                .expect("scratchpad file should exist");
            assert_eq!(stored.len(), 5_120);
            let snapshot = large_runtime
                .store
                .snapshot(&meta.session_id)
                .await
                .expect("child snapshot should exist");
            assert_eq!(snapshot.usage.input_tokens, 30);
            assert_eq!(snapshot.usage.output_tokens, 30);
        }
        other => panic!("expected file ref, got {other:?}"),
    }
}

#[tokio::test]
async fn test_depth_limit() {
    let runtime = test_runtime(
        vec![AssistantEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        }],
        vec!["ToolA"],
    );
    let workers = OrchestratorWorkers::new(runtime.parent, 1);
    let error = workers
        .run_worker(sample_spec(3, vec!["ToolA"]), "too deep")
        .await
        .expect_err("depth limit should reject");

    assert_eq!(
        error,
        octopus_sdk_contracts::SubagentError::DepthExceeded { depth: 3 }
    );
}

#[tokio::test]
async fn test_subagent_stops_after_max_turns() {
    let runtime = test_runtime_with_turns(
        vec![
            vec![
                AssistantEvent::ToolUse {
                    id: ToolCallId("call-1".into()),
                    name: "ToolA".into(),
                    input: serde_json::json!({}),
                },
                AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                    input_tokens: 5,
                    output_tokens: 5,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                AssistantEvent::ToolUse {
                    id: ToolCallId("call-2".into()),
                    name: "ToolA".into(),
                    input: serde_json::json!({}),
                },
                AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                    input_tokens: 5,
                    output_tokens: 5,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            stream_events("should not run", 10, StopReason::EndTurn),
        ],
        vec!["ToolA"],
    );
    let workers = OrchestratorWorkers::new(runtime.parent.clone(), 1);

    let output = workers
        .run_worker(sample_spec(1, vec!["ToolA"]), "loop until capped")
        .await
        .expect("worker should stop on max_turns");

    let SubagentOutput::Summary { text, meta } = output else {
        panic!("expected summary output");
    };
    assert!(text.is_empty());
    assert_eq!(meta.turns, 2);
    assert_eq!(runtime.model.request_count(), 2);
}

#[derive(Clone)]
struct TestRuntime {
    root: PathBuf,
    parent: ParentSessionContext,
    store: Arc<SqliteJsonlSessionStore>,
    model: Arc<StaticModelProvider>,
}

fn test_runtime(events: Vec<AssistantEvent>, tool_names: Vec<&str>) -> TestRuntime {
    test_runtime_with_turns(vec![events], tool_names)
}

fn test_runtime_with_turns(turns: Vec<Vec<AssistantEvent>>, tool_names: Vec<&str>) -> TestRuntime {
    let root = tempfile::tempdir().expect("tempdir should create").keep();
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    let store =
        Arc::new(SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open"));
    let parent_session = SessionId("parent-session".into());

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

    let model = Arc::new(StaticModelProvider {
        turns: Mutex::new(turns),
        requests: AtomicUsize::new(0),
    });
    let parent = ParentSessionContext {
        session_id: parent_session,
        session_store: store.clone(),
        model: model.clone(),
        tools: Arc::new(tool_registry(tool_names)),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root.clone()),
    };

    TestRuntime {
        root,
        parent,
        store,
        model,
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

fn sample_spec(depth: u8, allowed_tools: Vec<&str>) -> SubagentSpec {
    SubagentSpec {
        id: "researcher".into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: allowed_tools.into_iter().map(str::to_string).collect(),
        model_role: "subagent-default".into(),
        permission_mode: PermissionMode::Default,
        task_budget: TaskBudget {
            total: 100,
            completion_threshold: 0.9,
        },
        max_turns: 2,
        depth,
    }
}

fn stream_events(text: &str, total_tokens: u32, stop_reason: StopReason) -> Vec<AssistantEvent> {
    vec![
        AssistantEvent::TextDelta(text.to_string()),
        AssistantEvent::Usage(octopus_sdk_contracts::Usage {
            input_tokens: total_tokens / 2,
            output_tokens: total_tokens / 2,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }),
        AssistantEvent::MessageStop { stop_reason },
    ]
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

impl StaticModelProvider {
    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}
