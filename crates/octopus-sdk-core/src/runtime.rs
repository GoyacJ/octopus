use std::{collections::HashMap, path::PathBuf, sync::Arc};

use octopus_sdk_context::SystemPromptBuilder;
use octopus_sdk_contracts::{PermissionMode, PluginsSnapshot, RunId, SessionId};
use octopus_sdk_model::{ModelId, ModelProvider};
use octopus_sdk_observability::{TraceSpan, TraceValue, Tracer, UsageLedger};
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_sandbox::SandboxBackend;
use octopus_sdk_session::{EventRange, EventStream, SessionSnapshot, SessionStore};
use tokio_util::sync::CancellationToken;

use crate::{brain_loop, RuntimeError, RunHandle, SessionHandle, StartSessionInput, SubmitTurnInput};

#[derive(Clone)]
pub(crate) struct SessionRuntimeState {
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub model: ModelId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub token_budget: u32,
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
        self.inner
            .session_store
            .append_session_started(
                &session_id,
                input.config_snapshot_id.clone(),
                input.effective_config_hash.clone(),
                Some(self.inner.plugins_snapshot.clone()),
            )
            .await?;
        self.inner
            .sessions
            .lock()
            .await
            .insert(
                session_id.0.clone(),
                SessionRuntimeState {
                    working_dir: input.working_dir.clone(),
                    permission_mode: input.permission_mode,
                    model: input.model.clone(),
                    config_snapshot_id: input.config_snapshot_id.clone(),
                    effective_config_hash: input.effective_config_hash.clone(),
                    token_budget: input.token_budget,
                },
            );
        self.inner.tracer.record(
            TraceSpan::new("session_started")
                .with_field("session_id", TraceValue::String(session_id.0.clone())),
        );

        Ok(SessionHandle {
            session_id,
            working_dir: input.working_dir,
            permission_mode: input.permission_mode,
            model: input.model,
            config_snapshot_id: input.config_snapshot_id,
            effective_config_hash: input.effective_config_hash,
            token_budget: input.token_budget,
        })
    }

    pub async fn submit_turn(&self, input: SubmitTurnInput) -> Result<RunHandle, RuntimeError> {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(&input.session_id.0)
            .cloned()
            .ok_or_else(|| RuntimeError::SessionStateMissing {
                session_id: input.session_id.0.clone(),
            })?;
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
        let state = self
            .inner
            .sessions
            .lock()
            .await
            .get(&session_id.0)
            .cloned()
            .unwrap_or(SessionRuntimeState {
                working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                permission_mode: PermissionMode::Default,
                model: ModelId("main".into()),
                config_snapshot_id: snapshot.config_snapshot_id.clone(),
                effective_config_hash: snapshot.effective_config_hash.clone(),
                token_budget: 8_192,
            });

        Ok(SessionHandle {
            session_id: snapshot.id,
            working_dir: state.working_dir,
            permission_mode: state.permission_mode,
            model: state.model,
            config_snapshot_id: state.config_snapshot_id,
            effective_config_hash: state.effective_config_hash,
            token_budget: state.token_budget,
        })
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
