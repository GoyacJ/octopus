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

#[derive(Clone)]
pub(super) struct RuntimeAggregate {
    pub(super) detail: RuntimeSessionDetail,
    pub(super) events: Vec<RuntimeEventEnvelope>,
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
