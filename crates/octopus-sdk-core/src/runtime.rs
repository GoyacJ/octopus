use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use octopus_sdk_context::SystemPromptBuilder;
use octopus_sdk_contracts::{
    ContentBlock, EventId, Message, PermissionMode, PluginsSnapshot, RenderBlock, RenderKind,
    RenderLifecycle, RenderMeta, Role, RunId, SessionEvent, SessionId,
};
use octopus_sdk_model::{ModelId, ModelProvider};
use octopus_sdk_observability::{
    session_span_id, session_trace_id, TraceSpan, TraceValue, Tracer, UsageLedger,
};
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_sandbox::SandboxBackend;
use octopus_sdk_session::{EventRange, EventStream, SessionSnapshot, SessionStore};
use tokio_util::sync::CancellationToken;

use crate::{
    brain_loop, RunHandle, RuntimeError, SessionHandle, StartSessionInput, SubmitTurnInput,
};

#[derive(Clone)]
pub(crate) struct SessionRuntimeState {
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub model: ModelId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub token_budget: u32,
    pub pending_restore: Option<RestoredContext>,
}

#[derive(Debug, Clone)]
pub(crate) struct RestoredContext {
    pub project_notes: Option<RestoredArtifact>,
    pub session_notes: Option<RestoredArtifact>,
    pub session_todos: Option<RestoredArtifact>,
}

#[derive(Debug, Clone)]
pub(crate) struct RestoredArtifact {
    pub relative_path: String,
    pub content: String,
}

impl RestoredContext {
    fn is_empty(&self) -> bool {
        self.project_notes.is_none() && self.session_notes.is_none() && self.session_todos.is_none()
    }

    pub(crate) fn as_system_message(&self) -> Message {
        let mut sections = vec![
            "<context_restored>".to_string(),
            "Resume from persisted workspace artifacts before continuing.".to_string(),
        ];
        if let Some(notes) = &self.project_notes {
            sections.push(restored_section("project_notes", notes));
        }
        if let Some(notes) = &self.session_notes {
            sections.push(restored_section("session_notes", notes));
        }
        if let Some(todos) = &self.session_todos {
            sections.push(restored_section("session_todos", todos));
        }
        sections.push("</context_restored>".to_string());

        Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: sections.join("\n"),
            }],
        }
    }

    fn as_render_event(&self) -> SessionEvent {
        SessionEvent::Render {
            blocks: vec![RenderBlock {
                kind: RenderKind::Record,
                payload: serde_json::json!({
                    "title": "context_restored",
                    "rows": restored_rows(self),
                }),
                meta: RenderMeta {
                    id: EventId::new_v4(),
                    parent: None,
                    ts_ms: now_millis(),
                },
            }],
            lifecycle: RenderLifecycle::assistant_message(),
        }
    }
}

pub(crate) struct RuntimeInner {
    pub session_store: Arc<dyn SessionStore>,
    pub model_provider: Arc<dyn ModelProvider>,
    pub secret_vault: Arc<dyn octopus_sdk_contracts::SecretVault>,
    pub tool_registry: octopus_sdk_tools::ToolRegistry,
    pub permission_gate: Arc<dyn octopus_sdk_contracts::PermissionGate>,
    pub ask_resolver: Arc<dyn octopus_sdk_contracts::AskResolver>,
    pub sandbox_backend: Arc<dyn SandboxBackend>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugins_snapshot: PluginsSnapshot,
    pub tracer: Arc<dyn Tracer>,
    pub usage_ledger: Arc<UsageLedger>,
    pub prompt_builder: SystemPromptBuilder,
    pub sessions: tokio::sync::Mutex<HashMap<String, SessionRuntimeState>>,
    pub active_runs: tokio::sync::Mutex<HashMap<String, CancellationToken>>,
}

#[derive(Clone)]
pub struct AgentRuntime {
    inner: Arc<RuntimeInner>,
}

impl AgentRuntime {
    pub(crate) fn new(inner: Arc<RuntimeInner>) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn builder() -> crate::AgentRuntimeBuilder {
        crate::AgentRuntimeBuilder::new()
    }

    pub async fn start_session(
        &self,
        input: StartSessionInput,
    ) -> Result<SessionHandle, RuntimeError> {
        let session_id = input.session_id.unwrap_or_else(SessionId::new_v4);
        let state = SessionRuntimeState {
            working_dir: input.working_dir.clone(),
            permission_mode: input.permission_mode,
            model: input.model.clone(),
            config_snapshot_id: input.config_snapshot_id.clone(),
            effective_config_hash: input.effective_config_hash.clone(),
            token_budget: input.token_budget,
            pending_restore: None,
        };
        self.inner
            .session_store
            .append_session_started(
                &session_id,
                input.working_dir.clone(),
                input.permission_mode,
                input.model.0.clone(),
                input.config_snapshot_id.clone(),
                input.effective_config_hash.clone(),
                input.token_budget,
                Some(self.inner.plugins_snapshot.clone()),
            )
            .await?;
        self.inner
            .sessions
            .lock()
            .await
            .insert(session_id.0.clone(), state.clone());
        self.inner.tracer.record(
            TraceSpan::new("session_started")
                .with_trace_id(session_trace_id(&session_id.0))
                .with_span_id(session_span_id(&session_id.0))
                .with_agent_role("main")
                .with_field("session_id", TraceValue::String(session_id.0.clone()))
                .with_field("model_id", TraceValue::String(state.model.0.clone()))
                .with_field(
                    "model_version",
                    TraceValue::String(self.inner.model_provider.describe().catalog_version),
                )
                .with_field(
                    "config_snapshot_id",
                    TraceValue::String(state.config_snapshot_id.clone()),
                ),
        );

        Ok(session_handle(session_id, &state))
    }

    pub async fn submit_turn(&self, input: SubmitTurnInput) -> Result<RunHandle, RuntimeError> {
        let session = {
            let mut sessions = self.inner.sessions.lock().await;
            let state = sessions.get_mut(&input.session_id.0).ok_or_else(|| {
                RuntimeError::SessionStateMissing {
                    session_id: input.session_id.0.clone(),
                }
            })?;
            let session = state.clone();
            state.pending_restore = None;
            session
        };
        let run_id = RunId::new_v4();
        let cancellation = CancellationToken::new();
        self.inner
            .active_runs
            .lock()
            .await
            .insert(run_id.0.clone(), cancellation.clone());
        let run_handle = RunHandle {
            run_id: run_id.clone(),
            session_id: input.session_id.clone(),
        };

        let submit_result =
            brain_loop::submit_turn(Arc::clone(&self.inner), session, input, cancellation).await;
        self.inner.active_runs.lock().await.remove(&run_id.0);
        submit_result?;

        Ok(run_handle)
    }

    pub async fn resume(&self, session_id: &SessionId) -> Result<SessionHandle, RuntimeError> {
        let snapshot = self.inner.session_store.wake(session_id).await?;
        let pending_restore = load_restored_context(&snapshot.working_dir, session_id).await?;
        if let Some(restored) = pending_restore.as_ref() {
            self.inner
                .session_store
                .append(session_id, restored.as_render_event())
                .await?;
            self.inner.tracer.record(
                TraceSpan::new("context_restored")
                    .with_trace_id(session_trace_id(&snapshot.id.0))
                    .with_span_id(format!("context-restored:{}", snapshot.id.0))
                    .with_parent_span_id(session_span_id(&snapshot.id.0))
                    .with_agent_role("main")
                    .with_field("session_id", TraceValue::String(snapshot.id.0.clone()))
                    .with_field(
                        "project_notes_present",
                        TraceValue::Bool(restored.project_notes.is_some()),
                    )
                    .with_field(
                        "session_notes_present",
                        TraceValue::Bool(restored.session_notes.is_some()),
                    )
                    .with_field(
                        "session_todos_present",
                        TraceValue::Bool(restored.session_todos.is_some()),
                    ),
            );
        }
        let state = SessionRuntimeState {
            working_dir: snapshot.working_dir.clone(),
            permission_mode: snapshot.permission_mode,
            model: ModelId(snapshot.model.clone()),
            config_snapshot_id: snapshot.config_snapshot_id.clone(),
            effective_config_hash: snapshot.effective_config_hash.clone(),
            token_budget: snapshot.token_budget,
            pending_restore,
        };
        self.inner
            .sessions
            .lock()
            .await
            .insert(session_id.0.clone(), state.clone());

        Ok(session_handle(snapshot.id, &state))
    }

    pub async fn events(
        &self,
        session_id: &SessionId,
        range: EventRange,
    ) -> Result<EventStream, RuntimeError> {
        self.inner
            .session_store
            .stream(session_id, range)
            .await
            .map_err(RuntimeError::from)
    }

    pub async fn cancel(&self, run_id: &RunId) -> bool {
        if let Some(token) = self.inner.active_runs.lock().await.remove(&run_id.0) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub async fn snapshot(&self, session_id: &SessionId) -> Result<SessionSnapshot, RuntimeError> {
        self.inner
            .session_store
            .snapshot(session_id)
            .await
            .map_err(RuntimeError::from)
    }
}

fn session_handle(session_id: SessionId, state: &SessionRuntimeState) -> SessionHandle {
    SessionHandle {
        session_id,
        working_dir: state.working_dir.clone(),
        permission_mode: state.permission_mode,
        model: state.model.clone(),
        config_snapshot_id: state.config_snapshot_id.clone(),
        effective_config_hash: state.effective_config_hash.clone(),
        token_budget: state.token_budget,
    }
}

async fn load_restored_context(
    working_dir: &Path,
    session_id: &SessionId,
) -> Result<Option<RestoredContext>, RuntimeError> {
    let project_notes = read_optional_artifact(working_dir.join("NOTES.md"), working_dir).await?;
    let session_notes = read_optional_artifact(
        working_dir
            .join("runtime")
            .join("notes")
            .join(format!("{}.md", session_id.0)),
        working_dir,
    )
    .await?;
    let session_todos = read_optional_artifact(
        working_dir
            .join("runtime")
            .join("todos")
            .join(format!("{}.json", session_id.0)),
        working_dir,
    )
    .await?;
    let restored = RestoredContext {
        project_notes,
        session_notes,
        session_todos,
    };

    Ok((!restored.is_empty()).then_some(restored))
}

async fn read_optional_artifact(
    path: PathBuf,
    working_dir: &Path,
) -> Result<Option<RestoredArtifact>, RuntimeError> {
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(Some(RestoredArtifact {
            relative_path: relative_path(working_dir, &path),
            content: truncate_restore_content(&content),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(RuntimeError::Hook(error.to_string())),
    }
}

fn restored_rows(restored: &RestoredContext) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    if let Some(notes) = &restored.project_notes {
        rows.push(serde_json::json!({
            "label": "project_notes",
            "value": notes.relative_path,
            "preview": notes.content,
        }));
    }
    if let Some(notes) = &restored.session_notes {
        rows.push(serde_json::json!({
            "label": "session_notes",
            "value": notes.relative_path,
            "preview": notes.content,
        }));
    }
    if let Some(todos) = &restored.session_todos {
        rows.push(serde_json::json!({
            "label": "session_todos",
            "value": todos.relative_path,
            "preview": todos.content,
        }));
    }
    rows
}

fn restored_section(kind: &str, artifact: &RestoredArtifact) -> String {
    format!(
        "<{kind} path=\"{}\">\n{}\n</{kind}>",
        artifact.relative_path, artifact.content
    )
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn truncate_restore_content(content: &str) -> String {
    const MAX_CHARS: usize = 1_200;
    if content.chars().count() <= MAX_CHARS {
        return content.to_string();
    }

    let mut truncated = content.chars().take(MAX_CHARS).collect::<String>();
    truncated.push_str("\n...[truncated]");
    truncated
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64
}
