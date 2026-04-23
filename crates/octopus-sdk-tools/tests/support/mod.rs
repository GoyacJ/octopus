#![allow(dead_code)]

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::stream;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
    PermissionOutcome, SecretValue, SecretVault, SessionEvent, SessionId, ToolCallRequest,
    VaultError,
};
use octopus_sdk_hooks::HookRunner;
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use octopus_sdk_tools::{SandboxHandle, ToolContext, ToolResult};
use tokio_util::sync::CancellationToken;

pub struct AllowAll;

#[async_trait]
impl PermissionGate for AllowAll {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

pub struct StubAskResolver {
    pub answer: Result<AskAnswer, AskError>,
}

#[async_trait]
impl AskResolver for StubAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        self.answer.clone()
    }
}

pub struct RecordingEventSink {
    events: Mutex<Vec<SessionEvent>>,
}

impl RecordingEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn events(&self) -> Vec<SessionEvent> {
        self.events
            .lock()
            .expect("events lock should not be poisoned")
            .clone()
    }
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events
            .lock()
            .expect("events lock should not be poisoned")
            .push(event);
    }
}

pub struct SecretStub;
pub struct SessionStub;

#[async_trait]
impl SecretVault for SecretStub {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new(b"secret"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

#[async_trait]
impl SessionStore for SessionStub {
    async fn append(&self, _id: &SessionId, _event: SessionEvent) -> Result<EventId, SessionError> {
        Err(SessionError::NotFound)
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
        Err(SessionError::NotFound)
    }

    async fn new_child_session(
        &self,
        _parent_id: &SessionId,
        _spec: &octopus_sdk_contracts::SubagentSpec,
    ) -> Result<SessionId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn stream(
        &self,
        _id: &SessionId,
        _range: EventRange,
    ) -> Result<EventStream, SessionError> {
        Ok(Box::pin(stream::empty()))
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

pub fn tool_context(
    root: &Path,
    ask_resolver: Arc<dyn AskResolver>,
    event_sink: Arc<dyn EventSink>,
) -> ToolContext {
    ToolContext {
        session_id: SessionId("session-1".into()),
        tool_call_id: None,
        permissions: Arc::new(AllowAll),
        sandbox: SandboxHandle::new(root.to_path_buf(), Vec::new(), "noop"),
        session_store: Arc::new(SessionStub),
        secret_vault: Arc::new(SecretStub),
        ask_resolver,
        event_sink,
        working_dir: root.to_path_buf(),
        hooks: Arc::new(HookRunner::new()),
        permission_context: octopus_sdk_contracts::ToolPermissionContext::for_mode(
            octopus_sdk_contracts::PermissionMode::Default,
        ),
        cancellation: CancellationToken::new(),
    }
}

pub fn text_output(result: ToolResult) -> String {
    match result.content.as_slice() {
        [octopus_sdk_contracts::ContentBlock::Text { text }] => text.clone(),
        other => panic!("expected a single text block, got {other:?}"),
    }
}
