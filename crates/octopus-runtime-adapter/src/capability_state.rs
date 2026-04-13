use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedCapabilityState {
    session_state: tools::SessionCapabilityState,
    snapshot: RuntimeCapabilityStateSnapshot,
}

pub(crate) fn capability_state_snapshot(
    state: &tools::SessionCapabilityState,
) -> RuntimeCapabilityStateSnapshot {
    RuntimeCapabilityStateSnapshot {
        activated_tools: state.activated_tools().iter().cloned().collect(),
        granted_tools: state.granted_tools().iter().cloned().collect(),
        pending_tools: state.pending_tools().iter().cloned().collect(),
        approved_tools: state.approved_tools().iter().cloned().collect(),
        auth_resolved_tools: state.auth_resolved_tools().iter().cloned().collect(),
        hidden_tools: Vec::new(),
        injected_skill_message_count: state.injected_skill_messages().len() as u64,
        granted_tool_count: state.granted_tools().len() as u64,
        model_override: state.model_override().map(ToOwned::to_owned),
        effort_override: state.effort_override().map(ToOwned::to_owned),
    }
}

impl RuntimeAdapter {
    pub(crate) fn capability_state_snapshot_path(&self, state_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{state_ref}.json"))
    }

    pub(crate) fn load_capability_store(
        &self,
        state_ref: Option<&str>,
    ) -> Result<tools::SessionCapabilityStore, AppError> {
        let Some(state_ref) = state_ref.filter(|value| !value.trim().is_empty()) else {
            return Ok(tools::SessionCapabilityStore::default());
        };
        let path = self.capability_state_snapshot_path(state_ref);
        if !path.exists() {
            return Ok(tools::SessionCapabilityStore::default());
        }
        let raw = fs::read(path)?;
        let persisted: PersistedCapabilityState = serde_json::from_slice(&raw)?;
        Ok(tools::SessionCapabilityStore::from_shared(Arc::new(Mutex::new(
            persisted.session_state,
        ))))
    }

    pub(crate) fn load_capability_state_snapshot(
        &self,
        state_ref: Option<&str>,
    ) -> Result<Option<RuntimeCapabilityStateSnapshot>, AppError> {
        let Some(state_ref) = state_ref.filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };
        let path = self.capability_state_snapshot_path(state_ref);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read(path)?;
        let persisted: PersistedCapabilityState = serde_json::from_slice(&raw)?;
        Ok(Some(persisted.snapshot))
    }

    pub(crate) fn persist_capability_store(
        &self,
        state_ref: &str,
        store: &tools::SessionCapabilityStore,
    ) -> Result<RuntimeCapabilityStateSnapshot, AppError> {
        let state = store.snapshot();
        let snapshot = capability_state_snapshot(&state);
        let path = self.capability_state_snapshot_path(state_ref);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(&PersistedCapabilityState {
                session_state: state,
                snapshot: snapshot.clone(),
            })?,
        )?;
        Ok(snapshot)
    }
}
