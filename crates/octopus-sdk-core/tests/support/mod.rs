use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ContentBlock, Message,
    PermissionGate, PermissionMode, PermissionOutcome, SecretValue, SecretVault, SessionEvent,
    SessionId, ToolCallRequest, Usage, VaultError,
};
use octopus_sdk_core::{AgentRuntime, AgentRuntimeBuilder, StartSessionInput};
use octopus_sdk_model::{
    ModelError, ModelId, ModelProvider, ModelRequest, ModelStream, ProtocolFamily,
    ProviderDescriptor, ProviderId,
};
use octopus_sdk_sandbox::NoopBackend;
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_tools::{builtin::register_builtins, ToolRegistry};
use tempfile::TempDir;

pub struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

pub struct StaticAskResolver;

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

pub struct FixedPermissionGate {
    pub outcome: PermissionOutcome,
}

#[async_trait]
impl PermissionGate for FixedPermissionGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        self.outcome.clone()
    }
}

pub struct FixedAskResolver {
    pub option_id: &'static str,
    pub text: &'static str,
}

#[async_trait]
impl AskResolver for FixedAskResolver {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: prompt_id.into(),
            option_id: self.option_id.into(),
            text: self.text.into(),
        })
    }
}

pub struct StaticSecretVault;

#[async_trait]
impl SecretVault for StaticSecretVault {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new(b"secret"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

pub struct ScriptedModelProvider {
    turns: Mutex<Vec<ScriptedTurn>>,
    requests: Mutex<Vec<ModelRequest>>,
}

pub enum ScriptedTurn {
    Events(Vec<AssistantEvent>),
    Error(ModelError),
}

impl ScriptedModelProvider {
    #[must_use]
    pub fn new(turns: Vec<Vec<AssistantEvent>>) -> Self {
        Self::with_turns(turns.into_iter().map(ScriptedTurn::Events).collect())
    }

    #[must_use]
    pub fn with_turns(turns: Vec<ScriptedTurn>) -> Self {
        Self {
            turns: Mutex::new(turns),
            requests: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .expect("requests lock should stay available")
            .clone()
    }
}

#[async_trait]
impl ModelProvider for ScriptedModelProvider {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
        self.requests
            .lock()
            .expect("requests lock should stay available")
            .push(req);
        let turn = self
            .turns
            .lock()
            .expect("turns lock should stay available")
            .remove(0);
        match turn {
            ScriptedTurn::Events(events) => {
                Ok(Box::pin(futures::stream::iter(events.into_iter().map(Ok))))
            }
            ScriptedTurn::Error(error) => Err(error),
        }
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

pub fn temp_store() -> (TempDir, Arc<SqliteJsonlSessionStore>) {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let store = SqliteJsonlSessionStore::open(
        &root.path().join("data/main.db"),
        &root.path().join("runtime/events"),
    )
    .expect("session store should open");
    (root, Arc::new(store))
}

pub fn runtime_builder(
    model: Arc<dyn ModelProvider>,
    store: Arc<dyn SessionStore>,
) -> AgentRuntimeBuilder {
    runtime_builder_with_controls(
        model,
        store,
        Arc::new(AllowAllGate),
        Arc::new(StaticAskResolver),
    )
}

pub fn runtime_builder_with_controls(
    model: Arc<dyn ModelProvider>,
    store: Arc<dyn SessionStore>,
    permission_gate: Arc<dyn PermissionGate>,
    ask_resolver: Arc<dyn AskResolver>,
) -> AgentRuntimeBuilder {
    let mut tool_registry = ToolRegistry::new();
    register_builtins(&mut tool_registry).expect("builtins should register");

    AgentRuntime::builder()
        .with_session_store(store)
        .with_model_provider(model)
        .with_secret_vault(Arc::new(StaticSecretVault))
        .with_tool_registry(tool_registry)
        .with_permission_gate(permission_gate)
        .with_ask_resolver(ask_resolver)
        .with_sandbox_backend(Arc::new(NoopBackend))
}

pub fn start_input(root: &TempDir) -> StartSessionInput {
    StartSessionInput {
        session_id: None,
        working_dir: root.path().to_path_buf(),
        permission_mode: PermissionMode::Default,
        model: ModelId("test-model".into()),
        config_snapshot_id: "cfg-1".into(),
        effective_config_hash: "hash-1".into(),
        token_budget: 8_192,
    }
}

pub async fn collect_events(runtime: &AgentRuntime, session_id: &SessionId) -> Vec<SessionEvent> {
    let mut stream = runtime
        .events(session_id, octopus_sdk_session::EventRange::default())
        .await
        .expect("event stream should open");
    let mut events = Vec::new();

    while let Some(event) = stream.next().await {
        events.push(event.expect("event should decode"));
    }

    events
}

pub fn text_message(text: &str) -> Message {
    Message {
        role: octopus_sdk_contracts::Role::User,
        content: vec![ContentBlock::Text { text: text.into() }],
    }
}

pub fn usage(input_tokens: u32, output_tokens: u32) -> Usage {
    Usage {
        input_tokens,
        output_tokens,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    }
}
