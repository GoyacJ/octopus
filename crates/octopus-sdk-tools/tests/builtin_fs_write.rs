use std::{fs, path::Path, sync::Arc};

use async_trait::async_trait;
use futures::stream;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
    PermissionOutcome, SecretValue, SecretVault, SessionEvent, SessionId, ToolCallRequest,
    VaultError,
};
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use octopus_sdk_tools::{
    builtin::{FileEditTool, FileWriteTool},
    Tool, ToolContext, ToolError,
};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

struct PathGuard;
struct AskStub;
struct EventStub;
struct SecretStub;
struct SessionStub;

#[async_trait]
impl PermissionGate for PathGuard {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome {
        if call.input.get("path").and_then(serde_json::Value::as_str) == Some("/etc/passwd") {
            return PermissionOutcome::Deny {
                reason: "writes outside workspace are denied".into(),
            };
        }
        PermissionOutcome::Allow
    }
}

#[async_trait]
impl AskResolver for AskStub {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: prompt_id.into(),
            option_id: "ok".into(),
            text: "ok".into(),
        })
    }
}

impl EventSink for EventStub {
    fn emit(&self, _event: SessionEvent) {}
}

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
        _config_snapshot_id: String,
        _effective_config_hash: String,
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

fn tool_context(root: &Path) -> ToolContext {
    ToolContext {
        session_id: SessionId("session-1".into()),
        permissions: Arc::new(PathGuard),
        sandbox: octopus_sdk_tools::SandboxHandle::new(root.to_path_buf(), Vec::new(), "noop"),
        session_store: Arc::new(SessionStub),
        secret_vault: Arc::new(SecretStub),
        ask_resolver: Arc::new(AskStub),
        event_sink: Arc::new(EventStub),
        working_dir: root.to_path_buf(),
        cancellation: CancellationToken::new(),
    }
}

#[tokio::test]
async fn file_write_creates_file_atomically() {
    let dir = tempdir().expect("tempdir should exist");

    FileWriteTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "nested/out.txt", "content": "hello" }),
        )
        .await
        .expect("write should succeed");

    assert_eq!(
        fs::read_to_string(dir.path().join("nested/out.txt")).expect("file should exist"),
        "hello"
    );
}

#[tokio::test]
async fn file_edit_replaces_all_matches_when_requested() {
    let dir = tempdir().expect("tempdir should exist");
    fs::write(dir.path().join("notes.txt"), "alpha\nbeta\nalpha\n").expect("file should write");

    FileEditTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "notes.txt", "old_string": "alpha", "new_string": "omega", "replace_all": true }),
        )
        .await
        .expect("edit should succeed");

    assert_eq!(
        fs::read_to_string(dir.path().join("notes.txt")).expect("file should read"),
        "omega\nbeta\nomega\n"
    );
}

#[tokio::test]
async fn file_write_respects_permission_denials() {
    let dir = tempdir().expect("tempdir should exist");
    let error = FileWriteTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "/etc/passwd", "content": "nope" }),
        )
        .await
        .expect_err("write should be denied");

    assert!(matches!(error, ToolError::Permission { .. }));
}
