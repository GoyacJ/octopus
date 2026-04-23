use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use octopus_platform::{
    RuntimeExecutionService, RuntimeSdkDeps, RuntimeSdkFactory, RuntimeSessionService,
};
use octopus_sdk::{
    register_builtins, AgentRuntimeBuilder, AskAnswer, AskError, AskPrompt, AskResolver,
    AssistantEvent, ContentBlock, EventRange, Message, ModelError, ModelId, ModelProvider,
    ModelRequest, ModelStream, NoopBackend, PermissionGate, PermissionOutcome, PluginRegistry,
    PluginSourceTag, ProviderDescriptor, ProviderId, Role, SecretValue, SecretVault, SessionEvent,
    SessionStore, SqliteJsonlSessionStore, StartSessionInput, StopReason, SubagentOutput,
    SubagentSpec, SubmitTurnInput, TaskBudget, TaskFn, ToolCallRequest, ToolRegistry, VaultError,
};
use octopus_sdk_observability::{session_span_id, session_trace_id};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct StaticAskResolver;

#[async_trait]
impl AskResolver for StaticAskResolver {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: prompt_id.into(),
            option_id: "approve".into(),
            text: "approved".into(),
        })
    }
}

struct StaticVault;

#[async_trait]
impl SecretVault for StaticVault {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new("secret"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

struct ScriptedModelProvider {
    turns: std::sync::Mutex<Vec<Vec<AssistantEvent>>>,
}

#[async_trait]
impl ModelProvider for ScriptedModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::iter(
            self.turns
                .lock()
                .expect("turns lock should stay available")
                .remove(0)
                .into_iter()
                .map(Ok),
        )))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![octopus_sdk::ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

fn build_bridge_with_turns(
    root: &std::path::Path,
    turns: Vec<Vec<AssistantEvent>>,
    task_fn: Option<Arc<dyn TaskFn>>,
) -> Arc<octopus_platform::RuntimeSdkBridge> {
    let store = Arc::new(
        SqliteJsonlSessionStore::open(&root.join("data/main.db"), &root.join("runtime/events"))
            .expect("session store should open"),
    );
    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools).expect("builtins should register");
    let plugin_registry = PluginRegistry::new();
    let plugins_snapshot = plugin_registry.get_snapshot();

    RuntimeSdkFactory::new(RuntimeSdkDeps {
        workspace_id: "ws-local".into(),
        workspace_root: root.to_path_buf(),
        default_model: ModelId("test-model".into()),
        default_permission_mode: octopus_sdk::PermissionMode::Default,
        default_token_budget: 8_192,
        session_store: store,
        model_provider: Arc::new(ScriptedModelProvider {
            turns: std::sync::Mutex::new(turns),
        }),
        tool_registry: tools,
        permission_gate: Arc::new(AllowAllGate),
        ask_resolver: Arc::new(StaticAskResolver),
        sandbox_backend: Arc::new(NoopBackend),
        plugin_registry,
        plugins_snapshot,
        tracer: Arc::new(octopus_sdk::NoopTracer),
        task_fn,
    })
    .build()
    .expect("bridge should build")
}

fn build_bridge(root: &std::path::Path) -> Arc<octopus_platform::RuntimeSdkBridge> {
    build_bridge_with_turns(
        root,
        vec![
            vec![
                AssistantEvent::ToolUse {
                    id: octopus_sdk::ToolCallId("call-1".into()),
                    name: "bash".into(),
                    input: serde_json::json!({ "command": "printf 'bridge ok'" }),
                },
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                AssistantEvent::TextDelta("final answer".into()),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
        None,
    )
}

struct StaticTaskFn;

#[async_trait]
impl TaskFn for StaticTaskFn {
    async fn run(
        &self,
        _spec: &SubagentSpec,
        input: &str,
    ) -> Result<SubagentOutput, octopus_sdk::SubagentError> {
        Ok(SubagentOutput::Summary {
            text: format!("subagent: {input}"),
            meta: octopus_sdk::SubagentSummary {
                session_id: octopus_sdk::SessionId("subagent-1".into()),
                parent_session_id: octopus_sdk::SessionId("session-1".into()),
                resume_session_id: Some(octopus_sdk::SessionId("subagent-1".into())),
                spec_id: "worker-1".into(),
                agent_role: "worker".into(),
                parent_agent_role: "main".into(),
                turns: 1,
                tokens_used: 3,
                duration_ms: 1,
                trace_id: session_trace_id("session-1"),
                span_id: "subagent:subagent-1".into(),
                parent_span_id: session_span_id("session-1"),
                model_id: "test-model".into(),
                model_version: "test".into(),
                config_snapshot_id: "cfg-1".into(),
                permission_mode: octopus_sdk::PermissionMode::Default,
                allowed_tools: vec!["task".into()],
            },
        })
    }
}

#[test]
fn runtime_sdk_registry_input_hides_non_live_stub_builtins() {
    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools).expect("builtins should register");

    for name in ["web_search", "task", "skill", "task_list", "task_get"] {
        assert!(
            tools.get(name).is_none(),
            "{name} should not enter the runtime registry before live wiring exists"
        );
    }
}

#[tokio::test]
async fn runtime_sdk_live_builder_bootstraps_bundled_plugins_snapshot() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let bridge =
        RuntimeSdkFactory::build_live("ws-live", root.path().to_path_buf(), "claude-sonnet-4-5")
            .expect("live bridge should build");

    let detail = bridge
        .create_session(
            octopus_core::CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some("project-live".into()),
                title: "Live Plugin Session".into(),
                session_kind: Some("project".into()),
                selected_actor_ref: "agent:test".into(),
                selected_configured_model_id: None,
                execution_permission_mode: "default".into(),
            },
            "user-owner",
        )
        .await
        .expect("session should create");
    let store = SqliteJsonlSessionStore::open(
        &root.path().join("data/main.db"),
        &root.path().join("runtime/events"),
    )
    .expect("session store should reopen");
    let snapshot = store
        .snapshot(&octopus_sdk::SessionId(detail.summary.id.clone()))
        .await
        .expect("live builder should persist a plugins snapshot");

    assert_eq!(
        snapshot
            .plugins_snapshot
            .plugins
            .first()
            .map(|plugin| plugin.id.as_str()),
        Some("example-noop-tool")
    );
    assert_eq!(
        snapshot
            .plugins_snapshot
            .plugins
            .first()
            .map(|plugin| plugin.source),
        Some(PluginSourceTag::Bundled)
    );
}

#[tokio::test]
async fn runtime_sdk_bridge_projects_sessions_runs_and_events() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let bridge = build_bridge(root.path());

    let detail = bridge
        .create_session(
            octopus_core::CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some("project-1".into()),
                title: "Bridge Session".into(),
                session_kind: Some("project".into()),
                selected_actor_ref: "agent:test".into(),
                selected_configured_model_id: Some("test-model".into()),
                execution_permission_mode: "default".into(),
            },
            "user-owner",
        )
        .await
        .expect("session should create");

    assert_eq!(detail.summary.title, "Bridge Session");
    assert_eq!(detail.summary.project_id, "project-1");

    let sessions = bridge.list_sessions().await.expect("list sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, detail.summary.id);

    let run = bridge
        .submit_turn(
            &detail.summary.id,
            octopus_core::SubmitRuntimeTurnInput {
                content: "run bridge".into(),
                permission_mode: Some("default".into()),
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("turn should complete");
    assert_eq!(run.status, "completed");

    let detail = bridge
        .get_session(&detail.summary.id)
        .await
        .expect("session detail");
    assert_eq!(detail.run.id, run.id);
    assert_eq!(
        detail.summary.last_message_preview.as_deref(),
        Some("final answer")
    );

    let events = bridge
        .list_events(&detail.summary.id, None)
        .await
        .expect("events should list");
    assert!(events
        .iter()
        .any(|event| event.event_type == "runtime.session.started"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "runtime.tool.executed"));
    assert!(events.iter().any(|event| {
        event
            .message
            .as_ref()
            .map(|message| message.content == "final answer")
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn runtime_sdk_bridge_executes_task_tool_when_task_fn_present() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let bridge = build_bridge_with_turns(
        root.path(),
        vec![
            vec![
                AssistantEvent::ToolUse {
                    id: octopus_sdk::ToolCallId("task-1".into()),
                    name: "task".into(),
                    input: serde_json::json!({
                        "spec": SubagentSpec {
                            id: "worker-1".into(),
                            system_prompt: "Be concise.".into(),
                            allowed_tools: Vec::new(),
                            agent_role: "worker".into(),
                            model_role: "subagent-default".into(),
                            permission_mode: octopus_sdk::PermissionMode::Default,
                            task_budget: TaskBudget {
                                total: 100,
                                completion_threshold: 0.9,
                            },
                            max_turns: 1,
                            depth: 1,
                        },
                        "input": "delegate this"
                    }),
                },
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                AssistantEvent::TextDelta("task finished".into()),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
        Some(Arc::new(StaticTaskFn)),
    );

    let detail = bridge
        .create_session(
            octopus_core::CreateRuntimeSessionInput {
                conversation_id: String::new(),
                project_id: Some("project-task".into()),
                title: "Task Session".into(),
                session_kind: Some("project".into()),
                selected_actor_ref: "agent:test".into(),
                selected_configured_model_id: Some("test-model".into()),
                execution_permission_mode: "default".into(),
            },
            "user-owner",
        )
        .await
        .expect("session should create");

    let run = bridge
        .submit_turn(
            &detail.summary.id,
            octopus_core::SubmitRuntimeTurnInput {
                content: "run task".into(),
                permission_mode: Some("default".into()),
                recall_mode: None,
                ignored_memory_ids: Vec::new(),
                memory_intent: None,
            },
        )
        .await
        .expect("turn should complete");
    assert_eq!(run.status, "completed");

    let events = bridge
        .list_events(&detail.summary.id, None)
        .await
        .expect("events should list");
    assert!(events
        .iter()
        .any(|event| event.event_type == "runtime.tool.executed"));
    assert!(!events.iter().any(|event| {
        event
            .message
            .as_ref()
            .map(|message| {
                message.content.contains("TaskFn not injected")
                    || message.content.contains("[tool-error]")
            })
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn runtime_sdk_bridge_uses_sdk_store_contract() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let store = Arc::new(
        SqliteJsonlSessionStore::open(
            &root.path().join("data/main.db"),
            &root.path().join("runtime/events"),
        )
        .expect("session store should open"),
    );
    let runtime = AgentRuntimeBuilder::new()
        .with_session_store(store.clone())
        .with_model_provider(Arc::new(ScriptedModelProvider {
            turns: std::sync::Mutex::new(vec![vec![
                AssistantEvent::TextDelta("store check".into()),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                },
            ]]),
        }))
        .with_secret_vault(Arc::new(StaticVault))
        .with_tool_registry(ToolRegistry::new())
        .with_permission_gate(Arc::new(AllowAllGate))
        .with_ask_resolver(Arc::new(StaticAskResolver))
        .with_sandbox_backend(Arc::new(NoopBackend))
        .with_plugin_registry(PluginRegistry::new())
        .build()
        .expect("runtime should build");

    let handle = runtime
        .start_session(StartSessionInput {
            session_id: None,
            working_dir: root.path().to_path_buf(),
            permission_mode: octopus_sdk::PermissionMode::Default,
            model: ModelId("test-model".into()),
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            token_budget: 8_192,
        })
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "stream check".into(),
                }],
            },
        })
        .await
        .expect("turn should complete");

    let mut stream = runtime
        .events(&handle.session_id, EventRange::default())
        .await
        .expect("stream should open");
    let mut saw_message = false;
    while let Some(event) = stream.next().await {
        if matches!(
            event.expect("event should decode"),
            SessionEvent::AssistantMessage(message)
                if message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "store check"))
        ) {
            saw_message = true;
        }
    }

    assert!(saw_message);
}
