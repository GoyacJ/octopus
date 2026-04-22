use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use octopus_sdk::{
    builtin::register_builtins, AgentRuntimeBuilder, AskAnswer, AskError, AskPrompt, AskResolver,
    AssistantEvent, ContentBlock, EventRange, Message, ModelError, ModelId, ModelProvider,
    ModelRequest, ModelStream, NoopBackend, PermissionGate, PermissionMode, PermissionOutcome,
    PluginRegistry, ProviderDescriptor, ProviderId, Role, SessionEvent, SqliteJsonlSessionStore,
    StartSessionInput, StopReason, SubmitTurnInput, ToolCallRequest, ToolRegistry,
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
impl octopus_sdk::SecretVault for StaticVault {
    async fn get(
        &self,
        _ref_id: &str,
    ) -> Result<octopus_sdk::SecretValue, octopus_sdk::VaultError> {
        Ok(octopus_sdk::SecretValue::new(b"secret"))
    }

    async fn put(
        &self,
        _ref_id: &str,
        _value: octopus_sdk::SecretValue,
    ) -> Result<(), octopus_sdk::VaultError> {
        Ok(())
    }
}

struct ScriptedModelProvider {
    turns: Mutex<Vec<Vec<AssistantEvent>>>,
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

#[tokio::test]
async fn test_min_loop_events() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let store = Arc::new(
        SqliteJsonlSessionStore::open(
            &root.path().join("data/main.db"),
            &root.path().join("runtime/events"),
        )
        .expect("session store should open"),
    );
    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools).expect("builtins should register");

    let runtime = AgentRuntimeBuilder::new()
        .with_session_store(store)
        .with_model_provider(Arc::new(ScriptedModelProvider {
            turns: Mutex::new(vec![
                vec![
                    AssistantEvent::ToolUse {
                        id: octopus_sdk::ToolCallId("call-1".into()),
                        name: "bash".into(),
                        input: serde_json::json!({ "command": "printf 'sdk facade ok'" }),
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
        }))
        .with_secret_vault(Arc::new(StaticVault))
        .with_tool_registry(tools)
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
            permission_mode: PermissionMode::Default,
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
                    text: "run facade".into(),
                }],
            },
        })
        .await
        .expect("turn should complete");

    let mut stream = runtime
        .events(&handle.session_id, EventRange::default())
        .await
        .expect("event stream should open");
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.expect("event should decode"));
    }

    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::SessionStarted { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::ToolExecuted { name, .. } if name == "bash"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::AssistantMessage(message)
            if message.role == Role::Assistant
                && message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "final answer"))
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Render { .. })));
}
