use super::*;
use crate::model_runtime::{
    parse_model_credential_reference, CredentialReference, CREDENTIAL_SOURCE_PROBE_OVERRIDE,
};

#[derive(Debug, Clone)]
struct ManagedConfiguredModelCredentialWrite {
    credential_ref: String,
    api_key: String,
    previous_value: Option<String>,
}

pub(crate) fn apply_validation(
    mut effective: RuntimeEffectiveConfig,
    validation: RuntimeConfigValidationResult,
) -> RuntimeEffectiveConfig {
    effective.validation = validation;
    effective
}

impl RuntimeAdapter {
    fn ensure_workspace_managed_credentials_supported(
        target_scope: RuntimeConfigScopeKind,
        configured_model_credentials: &[RuntimeConfiguredModelCredentialInput],
    ) -> Result<(), AppError> {
        if target_scope == RuntimeConfigScopeKind::Workspace
            || configured_model_credentials.is_empty()
        {
            return Ok(());
        }

        Err(AppError::invalid_input(
            "configured model credentials are only supported for workspace runtime config",
        ))
    }

    fn workspace_target_document_mut(
        documents: &mut [RuntimeConfigDocumentRecord],
    ) -> Result<&mut RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter_mut()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    fn workspace_target_document(
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<&RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    fn apply_workspace_managed_credentials(
        &self,
        documents: &mut [RuntimeConfigDocumentRecord],
        configured_model_credentials: &[RuntimeConfiguredModelCredentialInput],
    ) -> Result<Vec<ManagedConfiguredModelCredentialWrite>, AppError> {
        if configured_model_credentials.is_empty() {
            return Ok(Vec::new());
        }

        let target = Self::workspace_target_document_mut(documents)?;
        let document = target.document.get_or_insert_with(BTreeMap::new);
        let Some(JsonValue::Object(configured_models)) = document.get_mut("configuredModels")
        else {
            return Err(AppError::invalid_input(
                "configured model credentials require configuredModels entries in the workspace patch",
            ));
        };

        let mut seen = HashSet::new();
        let mut writes = Vec::with_capacity(configured_model_credentials.len());

        for input in configured_model_credentials {
            let configured_model_id = input.configured_model_id.trim();
            if configured_model_id.is_empty() {
                return Err(AppError::invalid_input(
                    "configured model credential input requires configuredModelId",
                ));
            }
            if !seen.insert(configured_model_id.to_string()) {
                return Err(AppError::invalid_input(format!(
                    "duplicate configured model credential input `{configured_model_id}`"
                )));
            }

            let api_key = input.api_key.trim();
            if api_key.is_empty() {
                return Err(AppError::invalid_input(
                    "configured model credential input requires apiKey",
                ));
            }

            let Some(JsonValue::Object(configured_model)) =
                configured_models.get_mut(configured_model_id)
            else {
                return Err(AppError::invalid_input(format!(
                    "configured model credential target `{configured_model_id}` is missing from the workspace patch"
                )));
            };

            let credential_ref = self.configured_model_secret_reference(configured_model_id);
            configured_model.insert(
                "credentialRef".to_string(),
                JsonValue::String(credential_ref.clone()),
            );

            writes.push(ManagedConfiguredModelCredentialWrite {
                previous_value: self.state.secret_store.get_secret(&credential_ref)?,
                credential_ref,
                api_key: api_key.to_string(),
            });
        }

        Ok(writes)
    }

    fn rollback_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes.iter().rev() {
            if let Some(previous_value) = write.previous_value.as_deref() {
                self.state
                    .secret_store
                    .put_secret(&write.credential_ref, previous_value)?;
            } else {
                self.state
                    .secret_store
                    .delete_secret(&write.credential_ref)?;
            }
        }
        Ok(())
    }

    fn persist_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes {
            self.state
                .secret_store
                .put_secret(&write.credential_ref, &write.api_key)?;
            let stored_secret = self.state.secret_store.get_secret(&write.credential_ref)?;
            match stored_secret.as_deref() {
                Some(value) if value == write.api_key => {}
                Some(_) => {
                    return Err(AppError::runtime(format!(
                        "managed credential `{}` could not be verified after saving to local encrypted secret store",
                        write.credential_ref
                    )));
                }
                None => {
                    return Err(AppError::runtime(format!(
                        "managed credential `{}` is missing from local encrypted secret store after saving",
                        write.credential_ref
                    )));
                }
            }
        }
        Ok(())
    }

    fn workspace_owned_managed_credential_refs(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<HashSet<String>, AppError> {
        let mut refs = HashSet::new();
        let target = match Self::workspace_target_document(documents) {
            Ok(target) => target,
            Err(_) => return Ok(refs),
        };
        let Some(document) = target.document.as_ref() else {
            return Ok(refs);
        };
        let Some(configured_models) = document
            .get("configuredModels")
            .and_then(JsonValue::as_object)
        else {
            return Ok(refs);
        };

        for (entry_key, entry) in configured_models {
            let Some(entry_object) = entry.as_object() else {
                continue;
            };
            let configured_model_id = entry_object
                .get("configuredModelId")
                .and_then(JsonValue::as_str)
                .unwrap_or(entry_key);
            let Some(reference) = entry_object
                .get("credentialRef")
                .and_then(JsonValue::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            let Ok(Some(CredentialReference::ManagedSecret(reference))) =
                parse_model_credential_reference(Some(reference))
            else {
                continue;
            };

            if reference == self.configured_model_secret_reference(configured_model_id) {
                refs.insert(reference.to_string());
            }
        }

        Ok(refs)
    }

    fn cleanup_orphaned_workspace_managed_credentials(
        &self,
        previous_refs: &HashSet<String>,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<(), AppError> {
        let retained_refs = self.workspace_owned_managed_credential_refs(documents)?;
        for orphaned_ref in previous_refs.difference(&retained_refs) {
            self.state.secret_store.delete_secret(orphaned_ref)?;
        }
        Ok(())
    }

    fn restore_document_state(
        &self,
        previous: &RuntimeConfigDocumentRecord,
    ) -> Result<(), AppError> {
        if let Some(document) = previous.document.as_ref() {
            return self.write_runtime_document(&previous.storage_path, document);
        }

        match fs::remove_file(&previous.storage_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) async fn probe_configured_model_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
        configured_model_id: &str,
        api_key: Option<&str>,
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

        let mut probe_target = resolved_target.clone();
        if let Some(api_key) = api_key.map(str::trim).filter(|value| !value.is_empty()) {
            probe_target.credential_ref = Some(api_key.to_string());
            probe_target.credential_source = CREDENTIAL_SOURCE_PROBE_OVERRIDE.to_string();
        }

        let response = match self
            .execute_resolved_prompt(&probe_target, "Reply with exactly OK.", None)
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
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let mut documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        if target_scope == RuntimeConfigScopeKind::Workspace {
            self.apply_workspace_managed_credentials(
                &mut documents,
                &patch.configured_model_credentials,
            )?;
        }
        self.validate_registry_documents(&documents)
    }

    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        Self::ensure_workspace_managed_credentials_supported(
            Self::parse_scope(&patch.scope)?,
            &patch.configured_model_credentials,
        )?;
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
        Self::ensure_workspace_managed_credentials_supported(
            Self::parse_scope(&patch.scope)?,
            &patch.configured_model_credentials,
        )?;
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
        self.probe_configured_model_documents(
            &documents,
            &input.configured_model_id,
            input.api_key.as_deref(),
        )
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
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let existing_documents = self.resolve_documents(None, None)?;
        let previous_target = existing_documents
            .iter()
            .find(|document| document.scope == target_scope)
            .cloned()
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let previous_managed_refs =
            self.workspace_owned_managed_credential_refs(&existing_documents)?;

        let mut documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        let writes = if target_scope == RuntimeConfigScopeKind::Workspace {
            self.apply_workspace_managed_credentials(
                &mut documents,
                &patch.configured_model_credentials,
            )?
        } else {
            Vec::new()
        };
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        if let Err(error) = self.persist_managed_credential_writes(&writes) {
            self.rollback_managed_credential_writes(&writes)?;
            return Err(error);
        }
        if let Err(error) = self.write_document(target) {
            self.rollback_managed_credential_writes(&writes)?;
            return Err(error);
        }

        let reloaded = self.resolve_documents(None, None)?;
        if let Err(error) =
            self.cleanup_orphaned_workspace_managed_credentials(&previous_managed_refs, &reloaded)
        {
            self.restore_document_state(&previous_target)?;
            self.rollback_managed_credential_writes(&writes)?;
            return Err(error);
        }
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use octopus_infra::build_infra_bundle;
    use runtime::JsonValue;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::secret_store::RuntimeSecretStore;

    #[derive(Debug, Default)]
    struct WriteOnlyRuntimeSecretStore;

    impl RuntimeSecretStore for WriteOnlyRuntimeSecretStore {
        fn put_secret(&self, _reference: &str, _value: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn get_secret(&self, _reference: &str) -> Result<Option<String>, AppError> {
            Ok(None)
        }

        fn delete_secret(&self, _reference: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn test_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("octopus-runtime-config-service-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root");
        root
    }

    fn write_json(path: &Path, value: serde_json::Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("config dir");
        }
        fs::write(path, serde_json::to_vec_pretty(&value).expect("json")).expect("write config");
    }

    #[test]
    fn applies_validation_to_effective_config() {
        let effective = RuntimeEffectiveConfig {
            effective_config: json!({}),
            effective_config_hash: "hash".into(),
            sources: Vec::new(),
            validation: RuntimeConfigValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            secret_references: Vec::new(),
        };
        let validation = RuntimeConfigValidationResult {
            valid: false,
            errors: vec!["boom".into()],
            warnings: vec!["warn".into()],
        };

        let updated = apply_validation(effective, validation.clone());
        assert_eq!(updated.validation, validation);
    }

    #[tokio::test]
    async fn save_config_persists_workspace_managed_model_credentials_in_one_boundary() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
        );
        let configured_model_id = "anthropic-inline";
        let managed_reference = adapter.configured_model_secret_reference(configured_model_id);

        let saved = adapter
            .save_config(
                "workspace",
                RuntimeConfigPatch {
                    scope: "workspace".into(),
                    patch: json!({
                        "configuredModels": {
                            configured_model_id: {
                                "configuredModelId": configured_model_id,
                                "name": "Claude Inline",
                                "providerId": "anthropic",
                                "modelId": "claude-sonnet-4-5",
                                "enabled": true,
                                "source": "workspace"
                            }
                        }
                    }),
                    configured_model_credentials: vec![RuntimeConfiguredModelCredentialInput {
                        configured_model_id: configured_model_id.into(),
                        api_key: "sk-ant-saved-secret".into(),
                    }],
                },
            )
            .await
            .expect("save runtime config with managed credential");

        let workspace_source = saved
            .sources
            .iter()
            .find(|source| source.scope == "workspace")
            .expect("workspace source");
        let workspace_document = workspace_source
            .document
            .as_ref()
            .expect("workspace document");
        assert_eq!(
            workspace_document
                .pointer("/configuredModels/anthropic-inline/credentialRef")
                .and_then(serde_json::Value::as_str),
            Some(managed_reference.as_str())
        );
        assert_eq!(
            adapter
                .resolve_secret_reference(&managed_reference)
                .expect("resolve stored secret"),
            Some("sk-ant-saved-secret".into())
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn save_config_restores_managed_model_credentials_when_workspace_write_fails() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
        let configured_model_id = "anthropic-inline";
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
        );
        let managed_reference = adapter.configured_model_secret_reference(configured_model_id);
        write_json(
            &workspace_config_path,
            json!({
                "configuredModels": {
                    configured_model_id: {
                        "configuredModelId": configured_model_id,
                        "name": "Claude Inline",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": managed_reference.clone(),
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }),
        );

        adapter
            .state
            .secret_store
            .put_secret(&managed_reference, "sk-ant-original-secret")
            .expect("seed managed secret");

        let mut permissions = fs::metadata(&workspace_config_path)
            .expect("workspace config metadata")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&workspace_config_path, permissions)
            .expect("set workspace config readonly");

        let save_result = adapter
            .save_config(
                "workspace",
                RuntimeConfigPatch {
                    scope: "workspace".into(),
                    patch: json!({
                        "configuredModels": {
                            configured_model_id: {
                                "name": "Claude Updated"
                            }
                        }
                    }),
                    configured_model_credentials: vec![RuntimeConfiguredModelCredentialInput {
                        configured_model_id: configured_model_id.into(),
                        api_key: "sk-ant-updated-secret".into(),
                    }],
                },
            )
            .await;

        let mut reset_permissions = fs::metadata(&workspace_config_path)
            .expect("workspace config metadata after save")
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            reset_permissions.set_mode(0o600);
        }
        #[cfg(not(unix))]
        {
            reset_permissions.set_readonly(false);
        }
        fs::set_permissions(&workspace_config_path, reset_permissions)
            .expect("reset workspace config permissions");

        assert!(
            save_result.is_err(),
            "workspace save should fail when file is readonly"
        );
        assert_eq!(
            adapter
                .resolve_secret_reference(&managed_reference)
                .expect("resolve compensated secret"),
            Some("sk-ant-original-secret".into())
        );

        let stored_document =
            RuntimeAdapter::read_optional_runtime_document(&workspace_config_path)
                .expect("read stored workspace document")
                .expect("stored workspace document");
        let stored_value =
            RuntimeAdapter::runtime_json_to_serde(&JsonValue::Object(stored_document));
        assert_eq!(
            stored_value
                .pointer("/configuredModels/anthropic-inline/name")
                .and_then(serde_json::Value::as_str),
            Some("Claude Inline")
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn save_config_deletes_orphaned_managed_model_credentials_after_model_removal() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
        let configured_model_id = "anthropic-inline";
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
        );
        let managed_reference = adapter.configured_model_secret_reference(configured_model_id);
        write_json(
            &workspace_config_path,
            json!({
                "configuredModels": {
                    configured_model_id: {
                        "configuredModelId": configured_model_id,
                        "name": "Claude Inline",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": managed_reference.clone(),
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }),
        );

        adapter
            .state
            .secret_store
            .put_secret(&managed_reference, "sk-ant-delete-me")
            .expect("seed managed secret");

        adapter
            .save_config(
                "workspace",
                RuntimeConfigPatch {
                    scope: "workspace".into(),
                    patch: json!({
                        "configuredModels": {
                            configured_model_id: serde_json::Value::Null
                        }
                    }),
                    configured_model_credentials: Vec::new(),
                },
            )
            .await
            .expect("save runtime config after model removal");

        assert_eq!(
            adapter
                .resolve_secret_reference(&managed_reference)
                .expect("resolve deleted secret"),
            None
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn save_config_rejects_managed_model_credentials_when_secret_store_readback_is_missing() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let workspace_config_path = infra.paths.runtime_config_dir.join("workspace.json");
        let configured_model_id = "anthropic-inline";
        let adapter = RuntimeAdapter::new_with_executor_and_secret_store(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
            Arc::new(WriteOnlyRuntimeSecretStore),
        );

        let save_result = adapter
            .save_config(
                "workspace",
                RuntimeConfigPatch {
                    scope: "workspace".into(),
                    patch: json!({
                        "configuredModels": {
                            configured_model_id: {
                                "configuredModelId": configured_model_id,
                                "name": "Claude Inline",
                                "providerId": "anthropic",
                                "modelId": "claude-sonnet-4-5",
                                "enabled": true,
                                "source": "workspace"
                            }
                        }
                    }),
                    configured_model_credentials: vec![RuntimeConfiguredModelCredentialInput {
                        configured_model_id: configured_model_id.into(),
                        api_key: "sk-ant-saved-secret".into(),
                    }],
                },
            )
            .await;

        assert!(
            save_result.is_err(),
            "workspace save should fail when a managed credential cannot be read back from secure storage"
        );

        let stored_document =
            RuntimeAdapter::read_optional_runtime_document(&workspace_config_path)
                .expect("read workspace document");
        assert!(
            stored_document.is_none(),
            "failed managed credential write must not persist a broken secret-ref into runtime config"
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }
}
