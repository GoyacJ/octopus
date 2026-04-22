use std::{collections::HashSet, fs};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use octopus_core::{AppError, RuntimeConfiguredModelCredentialInput, RuntimeSecretReferenceStatus};
use serde_json::{Map, Value};

use crate::runtime_sdk::RuntimeSdkBridge;

use super::{
    ManagedConfiguredModelCredentialWrite, RuntimeConfigDocumentRecord, RuntimeConfigScopeKind,
};

impl RuntimeSdkBridge {
    pub(crate) fn configured_model_secret_reference(&self, configured_model_id: &str) -> String {
        format!(
            "secret-ref:workspace:{}:configured-model:{}",
            self.state.workspace_id,
            BASE64_STANDARD.encode(configured_model_id)
        )
    }

    pub(crate) fn configured_model_secret_status(&self, reference: &str) -> &'static str {
        if let Some(env_key) = reference.strip_prefix("env:") {
            return if std::env::var_os(env_key).is_some() {
                "reference-present"
            } else {
                "reference-missing"
            };
        }
        if reference.starts_with("secret-ref:") {
            return match self.state.secret_vault.get_optional_bytes(reference) {
                Ok(Some(_)) => "reference-present",
                Ok(None) => "reference-missing",
                Err(_) => "reference-error",
            };
        }
        if Self::is_reference_value(reference) {
            return "reference-unsupported";
        }
        "reference-inline"
    }

    pub(crate) fn is_sensitive_key(key: &str) -> bool {
        let normalized = key
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        [
            "apikey",
            "token",
            "secret",
            "password",
            "authorization",
            "authtoken",
            "clientsecret",
            "accesskey",
            "credentialref",
        ]
        .iter()
        .any(|needle| normalized.contains(needle))
    }

    pub(crate) fn is_reference_value(value: &str) -> bool {
        value.starts_with("env:")
            || value.starts_with("keychain:")
            || value.starts_with("op://")
            || value.starts_with("vault:")
            || value.starts_with("secret-ref:")
    }

    pub(crate) fn record_secret_reference(
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
        scope: &str,
        path: &str,
        reference: Option<String>,
        status: &str,
    ) {
        if secret_references.iter().any(|existing| {
            existing.scope == scope
                && existing.path == path
                && existing.reference == reference
                && existing.status == status
        }) {
            return;
        }

        secret_references.push(RuntimeSecretReferenceStatus {
            scope: scope.to_string(),
            path: path.to_string(),
            reference,
            status: status.to_string(),
        });
    }

    pub(crate) fn workspace_target_document_mut(
        documents: &mut [RuntimeConfigDocumentRecord],
    ) -> Result<&mut RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter_mut()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    pub(crate) fn workspace_target_document(
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<&RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    pub(crate) fn ensure_workspace_managed_credentials_supported(
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

    pub(crate) fn apply_workspace_managed_credentials(
        &self,
        documents: &mut [RuntimeConfigDocumentRecord],
        configured_model_credentials: &[RuntimeConfiguredModelCredentialInput],
    ) -> Result<Vec<ManagedConfiguredModelCredentialWrite>, AppError> {
        if configured_model_credentials.is_empty() {
            return Ok(Vec::new());
        }

        let target = Self::workspace_target_document_mut(documents)?;
        let document = target.document.get_or_insert_with(Map::new);
        let Some(Value::Object(configured_models)) = document.get_mut("configuredModels") else {
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

            let Some(Value::Object(configured_model)) =
                configured_models.get_mut(configured_model_id)
            else {
                return Err(AppError::invalid_input(format!(
                    "configured model credential target `{configured_model_id}` is missing from the workspace patch"
                )));
            };

            let credential_ref = self.configured_model_secret_reference(configured_model_id);
            configured_model.insert(
                "credentialRef".to_string(),
                Value::String(credential_ref.clone()),
            );

            writes.push(ManagedConfiguredModelCredentialWrite {
                previous_value: self.state.secret_vault.get_optional_utf8(&credential_ref)?,
                credential_ref,
                api_key: api_key.to_string(),
            });
        }

        Ok(writes)
    }

    pub(crate) fn rollback_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes.iter().rev() {
            if let Some(previous_value) = write.previous_value.as_deref() {
                self.state
                    .secret_vault
                    .put_utf8(&write.credential_ref, previous_value)?;
            } else {
                self.state.secret_vault.delete(&write.credential_ref)?;
            }
        }
        Ok(())
    }

    pub(crate) fn persist_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes {
            self.state
                .secret_vault
                .put_utf8(&write.credential_ref, &write.api_key)?;
            let stored_secret = self
                .state
                .secret_vault
                .get_optional_utf8(&write.credential_ref)?;
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

    pub(crate) fn workspace_owned_managed_credential_refs(
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
        let Some(configured_models) = document.get("configuredModels").and_then(Value::as_object)
        else {
            return Ok(refs);
        };

        for (entry_key, entry) in configured_models {
            let Some(entry_object) = entry.as_object() else {
                continue;
            };
            let configured_model_id = entry_object
                .get("configuredModelId")
                .and_then(Value::as_str)
                .unwrap_or(entry_key);
            let Some(reference) = entry_object
                .get("credentialRef")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            if reference == self.configured_model_secret_reference(configured_model_id) {
                refs.insert(reference.to_string());
            }
        }

        Ok(refs)
    }

    pub(crate) fn cleanup_orphaned_workspace_managed_credentials(
        &self,
        previous_refs: &HashSet<String>,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<(), AppError> {
        let retained_refs = self.workspace_owned_managed_credential_refs(documents)?;
        for orphaned_ref in previous_refs.difference(&retained_refs) {
            self.state.secret_vault.delete(orphaned_ref)?;
        }
        Ok(())
    }

    pub(crate) fn restore_document_state(
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
}
