use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures::stream;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, HookDecision, HookEvent,
    PermissionGate, PermissionOutcome, RewritePayload, SecretValue, SecretVault, SessionEvent,
    SessionId, ToolCallRequest, VaultError,
};
use octopus_sdk_hooks::{Hook, HookRunner, HookSource};
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

fn tool_context(root: &Path) -> ToolContext {
    tool_context_with_hooks(root, Arc::new(HookRunner::new()))
}

fn tool_context_with_hooks(root: &Path, hooks: Arc<HookRunner>) -> ToolContext {
    ToolContext {
        session_id: SessionId("session-1".into()),
        tool_call_id: None,
        permissions: Arc::new(PathGuard),
        sandbox: octopus_sdk_tools::SandboxHandle::new(root.to_path_buf(), Vec::new(), "noop"),
        session_store: Arc::new(SessionStub),
        secret_vault: Arc::new(SecretStub),
        ask_resolver: Arc::new(AskStub),
        event_sink: Arc::new(EventStub),
        working_dir: root.to_path_buf(),
        hooks,
        permission_context: octopus_sdk_contracts::ToolPermissionContext::for_mode(
            octopus_sdk_contracts::PermissionMode::Default,
        ),
        cancellation: CancellationToken::new(),
    }
}

struct FileWriteHook {
    seen: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Hook for FileWriteHook {
    fn name(&self) -> &str {
        "file-write-hook"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        match event {
            HookEvent::PreFileWrite { path, content, .. } => {
                self.seen
                    .lock()
                    .expect("seen lock should stay available")
                    .push(format!("pre:{path}:{content}"));
                HookDecision::Rewrite(RewritePayload::FileWrite {
                    path: path
                        .replace("out.txt", "rewritten.txt")
                        .replace("notes.txt", "rewritten.txt"),
                    content: format!("{content}::hooked"),
                })
            }
            HookEvent::PostFileWrite { path, .. } => {
                self.seen
                    .lock()
                    .expect("seen lock should stay available")
                    .push(format!("post:{path}"));
                HookDecision::Continue
            }
            _ => HookDecision::Continue,
        }
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

#[tokio::test]
async fn file_write_runs_pre_and_post_file_write_hooks_on_real_path() {
    let dir = tempdir().expect("tempdir should exist");
    let hooks = Arc::new(HookRunner::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    hooks.register(
        "file-write-hook",
        Arc::new(FileWriteHook {
            seen: Arc::clone(&seen),
        }),
        HookSource::Workspace,
        10,
    );

    FileWriteTool::new()
        .execute(
            tool_context_with_hooks(dir.path(), hooks),
            serde_json::json!({ "path": "nested/out.txt", "content": "hello" }),
        )
        .await
        .expect("write should succeed");

    assert!(!dir.path().join("nested/out.txt").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("nested/rewritten.txt"))
            .expect("rewritten file should exist"),
        "hello::hooked"
    );
    let seen = seen
        .lock()
        .expect("seen lock should stay available")
        .clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].ends_with("/nested/out.txt:hello"));
    assert!(seen[1].ends_with("/nested/rewritten.txt"));
}

#[tokio::test]
async fn file_edit_runs_file_write_hooks_on_the_edited_path() {
    let dir = tempdir().expect("tempdir should exist");
    fs::write(dir.path().join("notes.txt"), "alpha\nbeta\n").expect("file should write");
    let hooks = Arc::new(HookRunner::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    hooks.register(
        "file-write-hook",
        Arc::new(FileWriteHook {
            seen: Arc::clone(&seen),
        }),
        HookSource::Workspace,
        10,
    );

    FileEditTool::new()
        .execute(
            tool_context_with_hooks(dir.path(), hooks),
            serde_json::json!({ "path": "notes.txt", "old_string": "alpha", "new_string": "omega", "replace_all": false }),
        )
        .await
        .expect("edit should succeed");

    assert_eq!(
        fs::read_to_string(dir.path().join("notes.txt")).expect("original file should remain"),
        "alpha\nbeta\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("rewritten.txt")).expect("rewritten file should exist"),
        "omega\nbeta\n::hooked"
    );
    let seen = seen
        .lock()
        .expect("seen lock should stay available")
        .clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].ends_with("/notes.txt:omega\nbeta\n"));
    assert!(seen[1].ends_with("/rewritten.txt"));
}
