use super::*;

fn permission_rank(value: &str) -> Option<u8> {
    match value {
        RUNTIME_PERMISSION_READ_ONLY => Some(0),
        RUNTIME_PERMISSION_WORKSPACE_WRITE => Some(1),
        RUNTIME_PERMISSION_DANGER_FULL_ACCESS => Some(2),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompiledSessionPolicy {
    pub(crate) selected_actor_ref: String,
    pub(crate) selected_configured_model_id: Option<String>,
    pub(crate) execution_permission_mode: String,
    pub(crate) config_snapshot_id: String,
    pub(crate) effective_config_hash: String,
    pub(crate) started_from_scope_set: Vec<String>,
    pub(crate) manifest_revision: String,
    pub(crate) capability_policy: serde_json::Value,
    pub(crate) memory_policy: serde_json::Value,
    pub(crate) delegation_policy: serde_json::Value,
    pub(crate) approval_preference: serde_json::Value,
    pub(crate) manifest_snapshot_ref: String,
    pub(crate) session_policy_snapshot_ref: String,
}

impl CompiledSessionPolicy {
    pub(crate) fn contract_snapshot(&self) -> RuntimeSessionPolicySnapshot {
        RuntimeSessionPolicySnapshot {
            selected_actor_ref: self.selected_actor_ref.clone(),
            selected_configured_model_id: self
                .selected_configured_model_id
                .clone()
                .unwrap_or_default(),
            execution_permission_mode: self.execution_permission_mode.clone(),
            config_snapshot_id: self.config_snapshot_id.clone(),
            manifest_revision: self.manifest_revision.clone(),
            capability_policy: self.capability_policy.clone(),
            memory_policy: self.memory_policy.clone(),
            delegation_policy: self.delegation_policy.clone(),
            approval_preference: self.approval_preference.clone(),
        }
    }
}

impl RuntimeAdapter {
    pub(crate) fn session_policy_snapshot_path(&self, policy_snapshot_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{policy_snapshot_ref}.json"))
    }

    pub(crate) fn persist_session_policy_snapshot(
        &self,
        policy_snapshot_ref: &str,
        policy: &CompiledSessionPolicy,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_vec_pretty(policy)?;
        let path = self.session_policy_snapshot_path(policy_snapshot_ref);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, payload)?;
        Ok(())
    }

    pub(crate) fn load_session_policy_snapshot(
        &self,
        policy_snapshot_ref: &str,
    ) -> Result<CompiledSessionPolicy, AppError> {
        let path = self.session_policy_snapshot_path(policy_snapshot_ref);
        let raw = fs::read(path)?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub(crate) fn compile_session_policy(
        &self,
        session_id: &str,
        manifest: &actor_manifest::CompiledActorManifest,
        snapshot: &RuntimeConfigSnapshotSummary,
        selected_configured_model_id: Option<&str>,
        execution_permission_mode: &str,
    ) -> Result<CompiledSessionPolicy, AppError> {
        let normalized_execution_permission_mode =
            octopus_core::normalize_runtime_permission_mode_label(execution_permission_mode)
                .ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "unsupported permission mode: {execution_permission_mode}"
                    ))
                })?
                .to_string();
        let manifest_permission_ceiling =
            octopus_core::normalize_runtime_permission_mode_label(manifest.permission_ceiling())
                .unwrap_or(RUNTIME_PERMISSION_WORKSPACE_WRITE);
        if permission_rank(&normalized_execution_permission_mode)
            > permission_rank(manifest_permission_ceiling)
        {
            return Err(AppError::invalid_input(format!(
                "session permission mode `{normalized_execution_permission_mode}` exceeds actor permission ceiling `{manifest_permission_ceiling}`"
            )));
        }

        Ok(CompiledSessionPolicy {
            selected_actor_ref: manifest.actor_ref().to_string(),
            selected_configured_model_id: selected_configured_model_id
                .map(ToOwned::to_owned)
                .or_else(|| manifest.default_model_ref().map(ToOwned::to_owned)),
            execution_permission_mode: normalized_execution_permission_mode,
            config_snapshot_id: snapshot.id.clone(),
            effective_config_hash: snapshot.effective_config_hash.clone(),
            started_from_scope_set: snapshot.started_from_scope_set.clone(),
            manifest_revision: manifest.manifest_revision().to_string(),
            capability_policy: manifest.capability_policy_value(),
            memory_policy: manifest.memory_policy_value(),
            delegation_policy: manifest.delegation_policy_value(),
            approval_preference: manifest.approval_preference_value(),
            manifest_snapshot_ref: format!("{session_id}-manifest"),
            session_policy_snapshot_ref: format!("{session_id}-policy"),
        })
    }

    pub(crate) fn narrow_permission_mode(
        &self,
        session_policy: &CompiledSessionPolicy,
        requested_permission_mode: Option<&str>,
    ) -> Result<String, AppError> {
        let Some(requested_permission_mode) = requested_permission_mode else {
            return Ok(session_policy.execution_permission_mode.clone());
        };
        let normalized_requested =
            octopus_core::normalize_runtime_permission_mode_label(requested_permission_mode)
                .ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "unsupported permission mode: {requested_permission_mode}"
                    ))
                })?;
        if permission_rank(normalized_requested)
            > permission_rank(&session_policy.execution_permission_mode)
        {
            return Err(AppError::invalid_input(format!(
                "turn permission mode `{normalized_requested}` exceeds session ceiling `{}`",
                session_policy.execution_permission_mode
            )));
        }
        Ok(normalized_requested.to_string())
    }
}
