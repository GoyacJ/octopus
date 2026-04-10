use super::*;

pub(crate) fn requires_approval(permission_mode: &str) -> Result<bool, AppError> {
    let normalized_permission_mode = normalize_runtime_permission_mode_label(permission_mode)
        .ok_or_else(|| {
            AppError::invalid_input(format!("unsupported permission mode: {permission_mode}"))
        })?;
    Ok(normalized_permission_mode == RUNTIME_PERMISSION_WORKSPACE_WRITE)
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

    pub(super) fn resolve_execution_target_from_input(
        &self,
        config_snapshot_id: &str,
        input: &SubmitRuntimeTurnInput,
    ) -> Result<(EffectiveModelRegistry, String, ResolvedExecutionTarget), AppError> {
        let effective_config = self.config_snapshot_value(config_snapshot_id)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let configured_model_id = input
            .configured_model_id
            .as_deref()
            .or(input.model_id.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| {
                registry
                    .default_configured_model_id("conversation")
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| {
                AppError::invalid_input(
                    "configuredModelId or modelId is required when no conversation default is configured",
                )
            })?;
        let target = registry.resolve_target(&configured_model_id, None)?;
        Ok((registry, configured_model_id, target))
    }

    pub(super) fn resolve_submit_execution(
        &self,
        config_snapshot_id: &str,
        input: &SubmitRuntimeTurnInput,
    ) -> Result<(ResolvedExecutionTarget, ConfiguredModelRecord), AppError> {
        let (registry, configured_model_id, resolved_target) =
            self.resolve_execution_target_from_input(config_snapshot_id, input)?;
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

    pub(super) async fn execute_resolved_turn(
        &self,
        target: &ResolvedExecutionTarget,
        content: &str,
        actor_kind: Option<&str>,
        actor_id: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        let system_prompt =
            actor_context::resolve_actor_system_prompt(&self.state.paths, actor_kind, actor_id);
        self.state
            .executor
            .execute_turn(target, content, system_prompt.as_deref())
            .await
    }
}
