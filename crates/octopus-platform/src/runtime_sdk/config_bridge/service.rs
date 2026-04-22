use async_trait::async_trait;
use octopus_core::{
    AppError, RuntimeConfigPatch, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig,
};

use crate::runtime::RuntimeConfigService;
use crate::runtime_sdk::RuntimeSdkBridge;

use super::{apply_validation, RuntimeConfigScopeKind};

#[async_trait]
impl RuntimeConfigService for RuntimeSdkBridge {
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
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents =
            self.patched_documents(target_scope, Some(project_id), Some(user_id), &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents = self.patched_documents(target_scope, None, Some(user_id), &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let target_scope = Self::parse_scope(&input.scope)?;
        let documents = self.patched_documents(target_scope, None, None, &input.patch)?;
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
            self.restore_document_state(&previous_target)?;
            return Err(error);
        }
        if let Err(error) =
            self.cleanup_orphaned_workspace_managed_credentials(&previous_managed_refs, &documents)
        {
            self.restore_document_state(&previous_target)?;
            return Err(error);
        }

        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents =
            self.patched_documents(target_scope, Some(project_id), Some(user_id), &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }
        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents = self.patched_documents(target_scope, None, Some(user_id), &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }
        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }
}
