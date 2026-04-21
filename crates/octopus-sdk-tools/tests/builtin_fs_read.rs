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
    builtin::{FileReadTool, GlobTool, GrepTool},
    Tool, ToolContext,
};
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
        sandbox: octopus_sdk_tools::SandboxHandle {
            cwd: root.to_path_buf(),
            env_allowlist: Vec::new(),
        },
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
async fn file_read_reads_full_file_with_inline_line_numbers() {
    let dir = tempdir().expect("tempdir should exist");
    fs::write(dir.path().join("notes.txt"), "alpha\nbeta\n").expect("file should write");

    let result = FileReadTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "notes.txt" }),
        )
        .await
        .expect("read should succeed");

    assert_eq!(text_output(result), "000001|alpha\n000002|beta");
}

#[tokio::test]
async fn file_read_respects_offset_and_limit() {
    let dir = tempdir().expect("tempdir should exist");
    fs::write(dir.path().join("notes.txt"), "alpha\nbeta\ngamma\n").expect("file should write");

    let result = FileReadTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "notes.txt", "offset": 1, "limit": 1 }),
        )
        .await
        .expect("read should succeed");

    assert_eq!(text_output(result), "000002|beta");
}

#[tokio::test]
async fn glob_matches_workspace_files() {
    let dir = tempdir().expect("tempdir should exist");
    fs::create_dir_all(dir.path().join("src")).expect("src dir should exist");
    fs::write(dir.path().join("src/lib.rs"), "fn main() {}").expect("rs file should write");
    fs::write(dir.path().join("src/readme.md"), "# hi").expect("md file should write");

    let result = GlobTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "src", "pattern": "**/*.rs" }),
        )
        .await
        .expect("glob should succeed");

    assert_eq!(text_output(result), "src/lib.rs");
}

#[tokio::test]
async fn grep_respects_regex_and_case_insensitive() {
    let dir = tempdir().expect("tempdir should exist");
    fs::create_dir_all(dir.path().join("src")).expect("src dir should exist");
    fs::write(dir.path().join("src/lib.rs"), "Hello\nworld\nHELLO\n")
        .expect("rs file should write");

    let result = GrepTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({
                "path": "src",
                "pattern": "hello",
                "glob": "**/*.rs",
                "output_mode": "content",
                "case_insensitive": true
            }),
        )
        .await
        .expect("grep should succeed");

    let text = text_output(result);
    assert!(text.contains("src/lib.rs:1:Hello"));
    assert!(text.contains("src/lib.rs:3:HELLO"));
}

#[tokio::test]
async fn file_read_appends_truncation_note_when_output_exceeds_limit() {
    let dir = tempdir().expect("tempdir should exist");
    let content = (0..2_100)
        .map(|index| format!("line-{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.path().join("huge.txt"), content).expect("large file should write");

    let result = FileReadTool::new()
        .execute(
            tool_context(dir.path()),
            serde_json::json!({ "path": "huge.txt" }),
        )
        .await
        .expect("read should succeed");

    let text = text_output(result);
    assert!(text.contains("... truncated after 2000 lines or 512000 bytes"));
}
