use super::*;

pub(super) struct RuntimeState {
    pub(super) workspace_id: String,
    pub(super) paths: WorkspacePaths,
    pub(super) observation: Arc<dyn ObservationService>,
    pub(super) config_loader: ConfigLoader,
    pub(super) executor: Arc<dyn RuntimeModelExecutor>,
    pub(super) sessions: Mutex<HashMap<String, RuntimeAggregate>>,
    pub(super) config_snapshots: Mutex<HashMap<String, Value>>,
    pub(super) order: Mutex<Vec<String>>,
    pub(super) broadcasters: Mutex<HashMap<String, broadcast::Sender<RuntimeEventEnvelope>>>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct RuntimeAggregateMetadata {
    pub(super) manifest_snapshot_ref: String,
    pub(super) session_policy_snapshot_ref: String,
}

#[derive(Clone)]
pub(super) struct RuntimeAggregate {
    pub(super) detail: RuntimeSessionDetail,
    pub(super) events: Vec<RuntimeEventEnvelope>,
    pub(super) metadata: RuntimeAggregateMetadata,
}

pub(super) fn sync_runtime_session_detail(detail: &mut RuntimeSessionDetail) {
    if detail.selected_actor_ref.is_empty() {
        detail.selected_actor_ref = detail.summary.selected_actor_ref.clone();
    }
    if detail.manifest_revision.is_empty() {
        detail.manifest_revision = detail.summary.manifest_revision.clone();
    }
    if detail.session_policy.execution_permission_mode.is_empty() {
        detail.session_policy = detail.summary.session_policy.clone();
    }
    if detail.active_run_id.is_empty() {
        detail.active_run_id = if detail.summary.active_run_id.is_empty() {
            detail.run.id.clone()
        } else {
            detail.summary.active_run_id.clone()
        };
    }
    if detail.subrun_count == 0 {
        detail.subrun_count = detail.summary.subrun_count;
    }
    if detail.workflow.is_none() {
        detail.workflow = detail.summary.workflow.clone();
    }
    if detail.pending_mailbox.is_none() {
        detail.pending_mailbox = detail.summary.pending_mailbox.clone();
    }
    if detail.background_run.is_none() {
        detail.background_run = detail.summary.background_run.clone();
    }
    if detail.memory_summary.summary.is_empty() && !detail.summary.memory_summary.summary.is_empty()
    {
        detail.memory_summary = detail.summary.memory_summary.clone();
    }
    if detail.memory_selection_summary.selected_count == 0
        && detail.summary.memory_selection_summary.selected_count > 0
    {
        detail.memory_selection_summary = detail.summary.memory_selection_summary.clone();
    }
    if detail.pending_memory_proposal_count == 0 && detail.summary.pending_memory_proposal_count > 0
    {
        detail.pending_memory_proposal_count = detail.summary.pending_memory_proposal_count;
    }
    if detail.memory_state_ref.is_empty() && !detail.summary.memory_state_ref.is_empty() {
        detail.memory_state_ref = detail.summary.memory_state_ref.clone();
    }
    if detail.capability_summary.visible_tools.is_empty()
        && !detail.summary.capability_summary.visible_tools.is_empty()
    {
        detail.capability_summary = detail.summary.capability_summary.clone();
    }

    detail.summary.selected_actor_ref = detail.selected_actor_ref.clone();
    detail.summary.manifest_revision = detail.manifest_revision.clone();
    detail.summary.session_policy = detail.session_policy.clone();
    detail.summary.active_run_id = detail.active_run_id.clone();
    detail.summary.subrun_count = detail.subrun_count;
    detail.summary.workflow = detail.workflow.clone();
    detail.summary.pending_mailbox = detail.pending_mailbox.clone();
    detail.summary.background_run = detail.background_run.clone();
    detail.summary.memory_summary = detail.memory_summary.clone();
    detail.summary.memory_selection_summary = detail.memory_selection_summary.clone();
    detail.summary.pending_memory_proposal_count = detail.pending_memory_proposal_count;
    detail.summary.memory_state_ref = detail.memory_state_ref.clone();
    detail.summary.capability_summary = detail.capability_summary.clone();
    detail.summary.provider_state_summary = detail.provider_state_summary.clone();
    detail.summary.pending_mediation = detail.pending_mediation.clone();
    detail.summary.capability_state_ref = detail.capability_state_ref.clone();
    detail.summary.last_execution_outcome = detail.last_execution_outcome.clone();

    if detail.run.actor_ref.is_empty() {
        detail.run.actor_ref = detail.selected_actor_ref.clone();
    }
    if detail.run.trace_context.session_id.is_empty() {
        detail.run.trace_context.session_id = detail.summary.id.clone();
    }
}

pub(super) fn optional_project_id(project_id: &str) -> Option<String> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

pub(super) fn merge_project_assignments(
    effective_config: &mut Value,
    assignments: Option<&ProjectWorkspaceAssignments>,
) {
    let Some(assignments) = assignments else {
        return;
    };
    let Some(root) = effective_config.as_object_mut() else {
        return;
    };

    let project_settings = root
        .entry("projectSettings".to_string())
        .or_insert_with(|| json!({}));
    let Some(project_settings_object) = project_settings.as_object_mut() else {
        return;
    };

    let project_assignments_value = serde_json::to_value(assignments).unwrap_or_else(|_| json!({}));
    project_settings_object.insert(
        "workspaceAssignments".to_string(),
        project_assignments_value,
    );
}

impl RuntimeAdapter {
    pub(super) fn session_sender(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Sender<RuntimeEventEnvelope>, AppError> {
        let mut broadcasters = self
            .state
            .broadcasters
            .lock()
            .map_err(|_| AppError::runtime("broadcast mutex poisoned"))?;
        Ok(broadcasters
            .entry(session_id.to_string())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone())
    }

    pub(super) fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.state.paths.db_path)
            .map_err(|error| AppError::database(error.to_string()))
    }

    pub(super) fn load_project_assignments(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectWorkspaceAssignments>, AppError> {
        let connection = self.open_db()?;
        let assignments_json = connection
            .query_row(
                "SELECT assignments_json FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .flatten();
        assignments_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
            .transpose()
            .map_err(|error| AppError::database(error.to_string()))
    }

    pub(super) fn load_project_assignments_for_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<Option<ProjectWorkspaceAssignments>, AppError> {
        let project_id = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Project)
            .and_then(|document| document.owner_id.as_deref());
        match project_id {
            Some(project_id) => self.load_project_assignments(project_id),
            None => Ok(None),
        }
    }
}
