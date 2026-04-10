use super::*;

pub(crate) fn apply_validation(
    mut effective: RuntimeEffectiveConfig,
    validation: RuntimeConfigValidationResult,
) -> RuntimeEffectiveConfig {
    effective.validation = validation;
    effective
}

impl RuntimeAdapter {
    pub(super) async fn probe_configured_model_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
        configured_model_id: &str,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let validation = self.validate_registry_documents(documents)?;
        if !validation.valid {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: validation.errors,
                warnings: validation.warnings,
            });
        }

        let effective_config = self.load_effective_config_json(documents)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let resolved_target = match registry.resolve_target(configured_model_id, None) {
            Ok(target) => target,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: false,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: None,
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };
        let configured_model = match registry.configured_model(configured_model_id).cloned() {
            Some(configured_model) => configured_model,
            None => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: false,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: None,
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![format!(
                        "configured model `{configured_model_id}` is not registered"
                    )],
                    warnings: validation.warnings,
                });
            }
        };

        if let Err(error) = self.ensure_configured_model_quota_available(&configured_model) {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: true,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: Some(configured_model.name.clone()),
                request_id: None,
                consumed_tokens: None,
                errors: vec![error.to_string()],
                warnings: validation.warnings,
            });
        }

        let response = match self
            .execute_resolved_turn(&resolved_target, "Reply with exactly OK.", None, None)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: true,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: Some(configured_model.name.clone()),
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };

        let consumed_tokens = match self.resolve_consumed_tokens(&configured_model, &response) {
            Ok(consumed_tokens) => consumed_tokens,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: true,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: Some(configured_model.name.clone()),
                    request_id: response.request_id.clone(),
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };

        let now = timestamp_now();
        self.state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: None,
                run_id: None,
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                metric: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("turns")
                    .into(),
                amount: response.total_tokens.map(i64::from).unwrap_or(1),
                unit: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("count")
                    .into(),
                created_at: now,
            })
            .await?;
        if let Some(consumed_tokens) = consumed_tokens {
            self.increment_configured_model_usage(
                &resolved_target.configured_model_id,
                consumed_tokens,
                now,
            )?;
        }

        Ok(RuntimeConfiguredModelProbeResult {
            valid: true,
            reachable: true,
            configured_model_id: configured_model_id.to_string(),
            configured_model_name: Some(configured_model.name),
            request_id: response.request_id,
            consumed_tokens,
            errors: Vec::new(),
            warnings: validation.warnings,
        })
    }
}

#[async_trait]
impl ModelRegistryService for RuntimeAdapter {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let registry = self.effective_registry(&documents)?;
        let usage = self.load_configured_model_usage_map()?;
        Ok(registry.snapshot_with_usage(&usage))
    }
}

#[async_trait]
impl RuntimeConfigService for RuntimeAdapter {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn get_project_config(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(Some(project_id), Some(user_id))?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn get_user_config(&self, user_id: &str) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, Some(user_id))?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents =
            self.patched_documents(Self::parse_scope(&patch.scope)?, None, None, &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents = self.patched_documents(
            Self::parse_scope(&patch.scope)?,
            Some(project_id),
            Some(user_id),
            &patch.patch,
        )?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents = self.patched_documents(
            Self::parse_scope(&patch.scope)?,
            None,
            Some(user_id),
            &patch.patch,
        )?;
        self.validate_registry_documents(&documents)
    }

    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let scope = Self::parse_scope(&input.scope)?;
        if scope != RuntimeConfigScopeKind::Workspace {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: input.configured_model_id,
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: vec!["configured model probe only supports workspace scope".into()],
                warnings: Vec::new(),
            });
        }

        let documents = self.patched_documents(scope, None, None, &input.patch)?;
        self.probe_configured_model_documents(&documents, &input.configured_model_id)
            .await
    }

    async fn save_config(
        &self,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != scope {
            return Err(AppError::invalid_input(
                "runtime config patch scope must match route scope",
            ));
        }

        let target_scope = Self::parse_scope(scope)?;
        let documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(None, None)?;
        let effective = self.build_effective_config(&reloaded)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != "project" {
            return Err(AppError::invalid_input(
                "project runtime config patch scope must be project",
            ));
        }

        let documents = self.patched_documents(
            RuntimeConfigScopeKind::Project,
            Some(project_id),
            Some(user_id),
            &patch.patch,
        )?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Project)
            .ok_or_else(|| AppError::not_found("project runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(Some(project_id), Some(user_id))?;
        let effective = self.build_effective_config(&reloaded)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != "user" {
            return Err(AppError::invalid_input(
                "user runtime config patch scope must be user",
            ));
        }

        let documents = self.patched_documents(
            RuntimeConfigScopeKind::User,
            None,
            Some(user_id),
            &patch.patch,
        )?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::User)
            .ok_or_else(|| AppError::not_found("user runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(None, Some(user_id))?;
        let effective = self.build_effective_config(&reloaded)?;
        Ok(apply_validation(effective, validation))
    }
}
