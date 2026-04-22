use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_cli::run_once::run_once;
use octopus_sdk::{
    builtin::register_builtins, AgentRuntime, AssistantEvent, AskAnswer, AskError, AskPrompt,
    AskResolver, ContentBlock, Message, ModelError, ModelId, ModelProvider, ModelRequest,
    ModelStream, NoopBackend, PermissionGate, PermissionMode, PermissionOutcome, ProviderDescriptor,
    ProviderId, Role, SecretValue, SessionEvent, SqliteJsonlSessionStore, StartSessionInput,
    StopReason, ToolCallRequest, ToolRegistry, VaultError,
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
    async fn resolve(
        &self,
        prompt_id: &str,
        _prompt: &AskPrompt,
    ) -> Result<AskAnswer, AskError> {
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
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new(b"secret"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
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
async fn test_run_once_uses_sdk_runtime() {
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
    let runtime = Arc::new(
        AgentRuntime::builder()
            .with_session_store(store.clone())
            .with_model_provider(Arc::new(ScriptedModelProvider {
                turns: Mutex::new(vec![vec![
                    AssistantEvent::TextDelta("cli reply".into()),
                    AssistantEvent::MessageStop {
                        stop_reason: StopReason::EndTurn,
                    },
                ]]),
            }))
            .with_secret_vault(Arc::new(StaticVault))
            .with_tool_registry(tools)
            .with_permission_gate(Arc::new(AllowAllGate))
            .with_ask_resolver(Arc::new(StaticAskResolver))
            .with_sandbox_backend(Arc::new(NoopBackend))
            .build()
            .expect("runtime should build"),
    );

    let handle = run_once(
        runtime.clone(),
        StartSessionInput {
            session_id: None,
            working_dir: root.path().to_path_buf(),
            permission_mode: PermissionMode::Default,
            model: ModelId("cli-model".into()),
            config_snapshot_id: "cfg-cli".into(),
            effective_config_hash: "hash-cli".into(),
            token_budget: 8_192,
        },
        "hello cli".into(),
    )
    .await
    .expect("cli run_once should succeed");

    let snapshot = runtime
        .snapshot(&handle.session_id)
        .await
        .expect("snapshot should exist");
    assert_eq!(snapshot.config_snapshot_id, "cfg-cli");

    let events = {
        let mut stream = runtime
            .events(&handle.session_id, octopus_sdk::EventRange::default())
            .await
            .expect("stream should open");
        let mut events = Vec::new();
        use futures::StreamExt;
        while let Some(event) = stream.next().await {
            events.push(event.expect("event should decode"));
        }
        events
    };

    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::AssistantMessage(Message {
            role: Role::Assistant,
            content,
        }) if content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "cli reply"))
    )));
}
