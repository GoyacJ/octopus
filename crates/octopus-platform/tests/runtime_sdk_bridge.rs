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
    ProviderDescriptor, ProviderId, Role, SecretValue, SecretVault, SessionEvent,
    SqliteJsonlSessionStore, StartSessionInput, StopReason, SubmitTurnInput, ToolCallRequest,
    ToolRegistry, VaultError,
};

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

fn build_bridge(root: &std::path::Path) -> Arc<octopus_platform::RuntimeSdkBridge> {
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
            turns: std::sync::Mutex::new(vec![
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
            ]),
        }),
        tool_registry: tools,
        permission_gate: Arc::new(AllowAllGate),
        ask_resolver: Arc::new(StaticAskResolver),
        sandbox_backend: Arc::new(NoopBackend),
        plugin_registry,
        plugins_snapshot,
        tracer: Arc::new(octopus_sdk::NoopTracer),
        task_fn: None,
    })
    .build()
    .expect("bridge should build")
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
