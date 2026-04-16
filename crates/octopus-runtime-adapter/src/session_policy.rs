use super::*;
use octopus_core::RuntimeTargetPolicyDecision;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompiledSessionPolicy {
    #[serde(default)]
    pub(crate) user_id: String,
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
    #[serde(default)]
    pub(crate) capability_decisions: RuntimeCapabilityPolicyDecisions,
    #[serde(default)]
    pub(crate) target_decisions: BTreeMap<String, RuntimeTargetPolicyDecision>,
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
            .runtime_state_dir
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
        let raw = fs::read(&path)?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub(crate) async fn compile_session_policy(
        &self,
        session_id: &str,
        manifest: &actor_manifest::CompiledActorManifest,
        snapshot: &RuntimeConfigSnapshotSummary,
        selected_configured_model_id: Option<&str>,
        execution_permission_mode: &str,
        user_id: &str,
        project_id: Option<&str>,
        owner_permission_ceiling: Option<&str>,
    ) -> Result<CompiledSessionPolicy, AppError> {
        policy_compiler::compile_session_policy(
            self,
            session_id,
            manifest,
            snapshot,
            selected_configured_model_id,
            execution_permission_mode,
            user_id,
            project_id,
            owner_permission_ceiling,
        )
        .await
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
        Ok(octopus_core::clamp_runtime_permission_mode(
            normalized_requested,
            &session_policy.execution_permission_mode,
        ))
    }
}
