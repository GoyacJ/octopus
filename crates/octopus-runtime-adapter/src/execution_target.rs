use super::*;
use octopus_core::normalize_runtime_permission_mode_label;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn requires_approval(permission_mode: &str) -> Result<bool, AppError> {
    normalize_runtime_permission_mode_label(permission_mode).ok_or_else(|| {
        AppError::invalid_input(format!("unsupported permission mode: {permission_mode}"))
    })?;
    Ok(false)
}

fn configured_model_from_registry(
    registry: &EffectiveModelRegistry,
    configured_model_id: &str,
) -> Result<ConfiguredModelRecord, AppError> {
    registry
        .configured_model(configured_model_id)
        .cloned()
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "configured model `{configured_model_id}` is not registered"
            ))
        })
}

impl RuntimeAdapter {
    pub(super) fn resolve_execution_target(
        &self,
        config_snapshot_id: &str,
        configured_model_id: &str,
    ) -> Result<(EffectiveModelRegistry, ResolvedExecutionTarget), AppError> {
        let effective_config = self.config_snapshot_value(config_snapshot_id)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let target = registry.resolve_target(configured_model_id, None)?;
        Ok((registry, target))
    }

    pub(super) fn resolve_execution_target_from_session_policy(
        &self,
        session_policy: &session_policy::CompiledSessionPolicy,
    ) -> Result<(EffectiveModelRegistry, String, ResolvedExecutionTarget), AppError> {
        let effective_config = self.config_snapshot_value(&session_policy.config_snapshot_id)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let configured_model_id = session_policy
            .selected_configured_model_id
            .clone()
            .or_else(|| {
                registry
                    .default_configured_model_id("conversation")
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| {
                AppError::invalid_input(
                    "session-selected configured model is required when no conversation default is configured",
                )
            })?;
        let target = registry.resolve_target(&configured_model_id, None)?;
        Ok((registry, configured_model_id, target))
    }

    pub(super) fn resolve_submit_execution(
        &self,
        session_policy: &session_policy::CompiledSessionPolicy,
        _input: &SubmitRuntimeTurnInput,
    ) -> Result<(ResolvedExecutionTarget, ConfiguredModelRecord), AppError> {
        let (registry, configured_model_id, resolved_target) =
            self.resolve_execution_target_from_session_policy(session_policy)?;
        let configured_model = configured_model_from_registry(&registry, &configured_model_id)?;
        self.ensure_configured_model_quota_available(&configured_model)?;
        Ok((resolved_target, configured_model))
    }

    pub(super) fn resolve_approved_execution(
        &self,
        config_snapshot_id: &str,
        configured_model_id: &str,
    ) -> Result<(ResolvedExecutionTarget, ConfiguredModelRecord), AppError> {
        let (registry, resolved_target) =
            self.resolve_execution_target(config_snapshot_id, configured_model_id)?;
        let configured_model =
            configured_model_from_registry(&registry, &resolved_target.configured_model_id)?;
        self.ensure_configured_model_quota_available(&configured_model)?;
        Ok((resolved_target, configured_model))
    }

    fn resolve_execution_request_policy(
        &self,
        target: &ResolvedExecutionTarget,
    ) -> Result<octopus_core::ResolvedRequestPolicy, AppError> {
        let auth = self.resolve_model_auth(target)?;
        resolve_request_policy(target, &auth)
    }

    pub(super) async fn execute_resolved_prompt(
        &self,
        target: &ResolvedExecutionTarget,
        content: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        let request_policy = self.resolve_execution_request_policy(target)?;
        self.state
            .executor
            .execute_prompt(target, &request_policy, content, system_prompt)
            .await
    }

    pub(super) async fn execute_resolved_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        let request_policy = self.resolve_execution_request_policy(target)?;
        self.state
            .executor
            .execute_conversation_execution(target, &request_policy, request)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::requires_approval;

    #[test]
    fn permission_helpers_accept_runtime_modes() {
        assert!(!requires_approval("workspace-write").expect("workspace-write"));
        assert!(!requires_approval("danger-full-access").expect("danger-full-access"));
        assert!(!requires_approval("read-only").expect("read-only"));
    }
}
