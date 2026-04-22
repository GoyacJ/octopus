mod builder;
mod config_bridge;
mod execution_bridge;
mod registry_bridge;
mod secret_vault;
mod session_bridge;

use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use octopus_core::{
    timestamp_now, AppError, RuntimeAuthStateSummary, RuntimeCapabilityPlanSummary,
    RuntimeEventEnvelope, RuntimeMemorySelectionSummary, RuntimeMemorySummary, RuntimeMessage,
    RuntimePolicyDecisionSummary, RuntimeRunCheckpoint, RuntimeRunSnapshot, RuntimeSessionDetail,
    RuntimeSessionPolicySnapshot, RuntimeSessionSummary, RuntimeTraceContext, RuntimeTraceItem,
    RuntimeUsageSummary,
};
use octopus_sdk::{EventId, PermissionMode, SessionId};
use tokio::sync::{broadcast, Mutex};

pub use builder::{RuntimeSdkDeps, RuntimeSdkFactory};
pub(crate) use registry_bridge::build_catalog_snapshot;
pub(crate) use secret_vault::RuntimeSecretVault;

#[derive(Clone)]
pub(crate) struct RuntimeSdkPaths {
    pub(crate) root: PathBuf,
    pub(crate) runtime_config_dir: PathBuf,
    pub(crate) runtime_project_config_dir: PathBuf,
    pub(crate) runtime_user_config_dir: PathBuf,
    pub(crate) db_path: PathBuf,
    pub(crate) runtime_secret_master_key_path: PathBuf,
}

impl RuntimeSdkPaths {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let config_dir = root.join("config");
        let data_dir = root.join("data");
        let runtime_config_dir = config_dir.join("runtime");
        let runtime_project_config_dir = runtime_config_dir.join("projects");
        let runtime_user_config_dir = runtime_config_dir.join("users");
        let runtime_secrets_dir = data_dir.join("secrets");

        Self {
            root,
            runtime_config_dir,
            runtime_project_config_dir,
            runtime_user_config_dir,
            db_path: data_dir.join("main.db"),
            runtime_secret_master_key_path: runtime_secrets_dir.join("runtime-master.key"),
        }
    }

    pub(crate) fn ensure_layout(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.runtime_config_dir)?;
        fs::create_dir_all(&self.runtime_project_config_dir)?;
        fs::create_dir_all(&self.runtime_user_config_dir)?;
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = self.runtime_secret_master_key_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct RuntimeSdkBridge {
    pub(crate) state: Arc<RuntimeSdkState>,
}

pub(crate) struct RuntimeSdkState {
    pub(crate) workspace_id: String,
    pub(crate) workspace_root: PathBuf,
    pub(crate) paths: RuntimeSdkPaths,
    pub(crate) default_model: String,
    pub(crate) default_permission_mode: PermissionMode,
    pub(crate) default_token_budget: u32,
    pub(crate) runtime: Arc<octopus_sdk::AgentRuntime>,
    pub(crate) secret_vault: Arc<RuntimeSecretVault>,
    pub(crate) sessions: Mutex<HashMap<String, RuntimeSessionProjection>>,
    pub(crate) order: Mutex<Vec<String>>,
    pub(crate) broadcasters: Mutex<HashMap<String, broadcast::Sender<RuntimeEventEnvelope>>>,
}

pub(crate) struct RuntimeSessionProjection {
    pub(crate) metadata: RuntimeSessionMetadata,
    pub(crate) detail: RuntimeSessionDetail,
    pub(crate) events: Vec<RuntimeEventEnvelope>,
    pub(crate) head_event_id: Option<EventId>,
}

#[derive(Clone)]
pub(crate) struct RuntimeSessionMetadata {
    pub(crate) session_id: SessionId,
    pub(crate) conversation_id: String,
    pub(crate) project_id: String,
    pub(crate) title: String,
    pub(crate) session_kind: String,
    pub(crate) selected_actor_ref: String,
    pub(crate) configured_model_id: Option<String>,
    pub(crate) configured_model_name: Option<String>,
    pub(crate) runtime_model_id: Option<String>,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) config_snapshot_id: String,
    pub(crate) effective_config_hash: String,
    pub(crate) started_from_scope_set: Vec<String>,
}

impl RuntimeSdkBridge {
    pub(crate) fn new(state: RuntimeSdkState) -> Self {
        Self {
            state: Arc::new(state),
        }
    }

    pub(crate) fn synthetic_run_id(session_id: &str) -> String {
        format!("run-{session_id}")
    }

    pub(crate) fn build_run_snapshot(
        metadata: &RuntimeSessionMetadata,
        run_id: String,
        status: &str,
        current_step: &str,
        now: u64,
        next_action: Option<String>,
        consumed_tokens: Option<u32>,
    ) -> RuntimeRunSnapshot {
        RuntimeRunSnapshot {
            id: run_id,
            session_id: metadata.session_id.0.clone(),
            conversation_id: metadata.conversation_id.clone(),
            status: status.into(),
            current_step: current_step.into(),
            started_at: now,
            updated_at: now,
            selected_memory: Vec::new(),
            freshness_summary: None,
            pending_memory_proposal: None,
            memory_state_ref: format!("memory-state-{}", metadata.session_id.0),
            configured_model_id: metadata.configured_model_id.clone(),
            configured_model_name: metadata.configured_model_name.clone(),
            model_id: metadata.runtime_model_id.clone(),
            consumed_tokens,
            next_action,
            config_snapshot_id: metadata.config_snapshot_id.clone(),
            effective_config_hash: metadata.effective_config_hash.clone(),
            started_from_scope_set: metadata.started_from_scope_set.clone(),
            run_kind: "primary".into(),
            parent_run_id: None,
            actor_ref: metadata.selected_actor_ref.clone(),
            delegated_by_tool_call_id: None,
            workflow_run: None,
            workflow_run_detail: None,
            mailbox_ref: None,
            handoff_ref: None,
            background_state: None,
            worker_dispatch: None,
            approval_state: "not-required".into(),
            approval_target: None,
            auth_target: None,
            usage_summary: RuntimeUsageSummary::default(),
            artifact_refs: Vec::new(),
            deliverable_refs: Vec::new(),
            trace_context: RuntimeTraceContext::default(),
            checkpoint: RuntimeRunCheckpoint::default(),
            capability_plan_summary: RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            pending_mediation: None,
            last_execution_outcome: None,
            last_mediation_outcome: None,
            resolved_target: None,
            requested_actor_kind: None,
            requested_actor_id: Some(metadata.selected_actor_ref.clone()),
            resolved_actor_kind: None,
            resolved_actor_id: Some(metadata.selected_actor_ref.clone()),
            resolved_actor_label: Some(metadata.selected_actor_ref.clone()),
        }
    }

    pub(crate) fn build_session_detail(
        metadata: RuntimeSessionMetadata,
        status: &str,
        run: RuntimeRunSnapshot,
        updated_at: u64,
    ) -> RuntimeSessionDetail {
        let summary = RuntimeSessionSummary {
            id: metadata.session_id.0.clone(),
            conversation_id: metadata.conversation_id.clone(),
            project_id: metadata.project_id.clone(),
            title: metadata.title.clone(),
            session_kind: metadata.session_kind.clone(),
            status: status.into(),
            updated_at,
            last_message_preview: None,
            config_snapshot_id: metadata.config_snapshot_id.clone(),
            effective_config_hash: metadata.effective_config_hash.clone(),
            started_from_scope_set: metadata.started_from_scope_set.clone(),
            selected_actor_ref: metadata.selected_actor_ref.clone(),
            manifest_revision: "sdk-bridge".into(),
            session_policy: RuntimeSessionPolicySnapshot::default(),
            active_run_id: run.id.clone(),
            subrun_count: 0,
            workflow: None,
            pending_mailbox: None,
            background_run: None,
            memory_summary: RuntimeMemorySummary::default(),
            memory_selection_summary: RuntimeMemorySelectionSummary::default(),
            pending_memory_proposal_count: 0,
            memory_state_ref: run.memory_state_ref.clone(),
            capability_summary: RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            auth_state_summary: RuntimeAuthStateSummary::default(),
            pending_mediation: None,
            policy_decision_summary: RuntimePolicyDecisionSummary::default(),
            last_execution_outcome: None,
        };

        RuntimeSessionDetail {
            summary,
            selected_actor_ref: metadata.selected_actor_ref.clone(),
            manifest_revision: "sdk-bridge".into(),
            session_policy: RuntimeSessionPolicySnapshot::default(),
            active_run_id: run.id.clone(),
            subrun_count: 0,
            workflow: None,
            pending_mailbox: None,
            background_run: None,
            memory_summary: RuntimeMemorySummary::default(),
            memory_selection_summary: RuntimeMemorySelectionSummary::default(),
            pending_memory_proposal_count: 0,
            memory_state_ref: run.memory_state_ref.clone(),
            capability_summary: RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            auth_state_summary: RuntimeAuthStateSummary::default(),
            pending_mediation: None,
            policy_decision_summary: RuntimePolicyDecisionSummary::default(),
            last_execution_outcome: None,
            run,
            subruns: Vec::new(),
            handoffs: Vec::new(),
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        }
    }

    pub(crate) async fn projection(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionProjection, AppError> {
        self.state
            .sessions
            .lock()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("runtime session `{session_id}`")))
    }

    pub(crate) async fn upsert_projection(&self, projection: Box<RuntimeSessionProjection>) {
        let session_id = projection.metadata.session_id.0.clone();
        let mut sessions = self.state.sessions.lock().await;
        let mut order = self.state.order.lock().await;
        if !sessions.contains_key(&session_id) {
            order.insert(0, session_id.clone());
        }
        sessions.insert(session_id, *projection);
    }

    pub(crate) async fn session_sender(
        &self,
        session_id: &str,
    ) -> broadcast::Sender<RuntimeEventEnvelope> {
        let mut broadcasters = self.state.broadcasters.lock().await;
        broadcasters
            .entry(session_id.into())
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(64);
                sender
            })
            .clone()
    }

    pub(crate) fn runtime_error(error: impl ToString) -> AppError {
        AppError::runtime(error.to_string())
    }

    pub(crate) fn invalid_input(message: impl Into<String>) -> AppError {
        AppError::invalid_input(message.into())
    }

    pub(crate) fn now() -> u64 {
        timestamp_now()
    }
}

impl Clone for RuntimeSessionProjection {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            detail: self.detail.clone(),
            events: self.events.clone(),
            head_event_id: self.head_event_id.clone(),
        }
    }
}

pub(crate) fn runtime_message(
    id: String,
    session_id: &str,
    conversation_id: &str,
    sender_type: &str,
    sender_label: &str,
    content: String,
    timestamp: u64,
    configured_model_id: Option<String>,
    model_id: Option<String>,
) -> RuntimeMessage {
    RuntimeMessage {
        id,
        session_id: session_id.into(),
        conversation_id: conversation_id.into(),
        sender_type: sender_type.into(),
        sender_label: sender_label.into(),
        content,
        timestamp,
        configured_model_id: configured_model_id.clone(),
        configured_model_name: configured_model_id,
        model_id,
        status: "completed".into(),
        requested_actor_kind: None,
        requested_actor_id: None,
        resolved_actor_kind: None,
        resolved_actor_id: None,
        resolved_actor_label: None,
        used_default_actor: None,
        resource_ids: None,
        attachments: None,
        artifacts: None,
        deliverable_refs: None,
        usage: None,
        tool_calls: None,
        process_entries: None,
    }
}

pub(crate) fn runtime_trace(
    id: String,
    session_id: &str,
    run_id: &str,
    conversation_id: &str,
    kind: &str,
    title: &str,
    detail: String,
    timestamp: u64,
) -> RuntimeTraceItem {
    RuntimeTraceItem {
        id,
        session_id: session_id.into(),
        run_id: run_id.into(),
        conversation_id: conversation_id.into(),
        kind: kind.into(),
        title: title.into(),
        detail,
        tone: "info".into(),
        timestamp,
        actor: "sdk-bridge".into(),
        actor_kind: None,
        actor_id: None,
        related_message_id: None,
        related_tool_name: None,
    }
}
