use std::{path::Path, process::Command, sync::Arc};

use async_trait::async_trait;
use futures::stream;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
    PermissionOutcome, SecretValue, SecretVault, SessionEvent, SessionId, ToolCallRequest,
    VaultError,
};
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use octopus_sdk_tools::{builtin::BashTool, Tool, ToolContext};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

struct AllowAll;
struct AskStub;
struct EventStub;
struct SecretStub;
struct SessionStub;

#[async_trait]
impl PermissionGate for AllowAll {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
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
        permissions: Arc::new(AllowAll),
        sandbox: octopus_sdk_tools::SandboxHandle::new(
            root.to_path_buf(),
            vec!["PATH".into()],
            "noop",
        ),
        session_store: Arc::new(SessionStub),
        secret_vault: Arc::new(SecretStub),
        ask_resolver: Arc::new(AskStub),
        event_sink: Arc::new(EventStub),
        working_dir: root.to_path_buf(),
        cancellation: CancellationToken::new(),
    }
}

fn text_output(result: octopus_sdk_tools::ToolResult) -> String {
    match result.content.as_slice() {
        [octopus_sdk_contracts::ContentBlock::Text { text }] => text.clone(),
        other => panic!("expected a single text block, got {other:?}"),
    }
}

#[tokio::test]
async fn bash_runs_simple_command() {
    let dir = tempdir().expect("tempdir should exist");
    let result = BashTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "command": "printf hello" }),
        )
        .await
        .expect("bash should succeed");

    assert_eq!(text_output(result), "hello");
}

#[tokio::test]
async fn bash_marks_non_zero_exit_as_error() {
    let dir = tempdir().expect("tempdir should exist");
    let result = BashTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "command": "printf boom 1>&2; exit 7" }),
        )
        .await
        .expect("bash should return a tool result");

    assert!(result.is_error);
    assert!(text_output(result).contains("boom"));
}

#[tokio::test]
async fn bash_truncates_large_output_at_default_limit() {
    let dir = tempdir().expect("tempdir should exist");

    let result = BashTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "command": "yes x | head -c 35000" }),
        )
        .await
        .expect("bash should succeed");

    assert!(text_output(result).contains("[output truncated"));
}

#[test]
fn bash_respects_env_override_for_max_output() {
    let output = Command::new(std::env::current_exe().expect("test binary should exist"))
        .env("RUN_BASH_ENV_OVERRIDE_CASE", "1")
        .env("BASH_MAX_OUTPUT_LENGTH", "100")
        .arg("--exact")
        .arg("bash_env_override_helper")
        .arg("--nocapture")
        .output()
        .expect("child test should run");

    assert!(
        output.status.success(),
        "child test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn bash_env_override_helper() {
    if std::env::var("RUN_BASH_ENV_OVERRIDE_CASE").ok().as_deref() != Some("1") {
        return;
    }

    let dir = tempdir().expect("tempdir should exist");

    let result = BashTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "command": "yes x | head -c 400" }),
        )
        .await
        .expect("bash should succeed");

    let text = text_output(result);
    assert!(text.contains("[output truncated"));
}
