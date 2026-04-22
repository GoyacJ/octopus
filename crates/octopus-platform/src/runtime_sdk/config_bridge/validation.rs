use octopus_core::{AppError, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeResult};
use serde_json::Value;

use crate::runtime_sdk::build_catalog_snapshot;
use crate::runtime_sdk::RuntimeSdkBridge;

use super::{
    RuntimeConfigDocumentRecord, RuntimeConfigScopeKind, DEPRECATED_RUNTIME_CONFIG_TOP_LEVEL_KEYS,
    KNOWN_RUNTIME_CONFIG_TOP_LEVEL_KEYS,
};

impl RuntimeSdkBridge {
    pub(crate) fn validate_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        for document in documents {
            let Some(record) = document.document.as_ref() else {
                continue;
            };

            for key in record.keys() {
                if let Some((_, replacement)) = DEPRECATED_RUNTIME_CONFIG_TOP_LEVEL_KEYS
                    .iter()
                    .find(|(deprecated, _)| key == deprecated)
                {
                    warnings.push(format!(
                        "{}: deprecated runtime config key `{key}`; use `{replacement}` instead",
                        document.display_path
                    ));
                    continue;
                }
                if !KNOWN_RUNTIME_CONFIG_TOP_LEVEL_KEYS.contains(&key.as_str()) {
                    warnings.push(format!(
                        "{}: unknown runtime config key `{key}`",
                        document.display_path
                    ));
                }
            }

            if let Some(configured_models) = record.get("configuredModels") {
                let Some(configured_models) = configured_models.as_object() else {
                    errors.push(format!(
                        "{}: configuredModels must be a JSON object",
                        document.display_path
                    ));
                    continue;
                };
                for (configured_model_id, entry) in configured_models {
                    let Some(entry_object) = entry.as_object() else {
                        errors.push(format!(
                            "{}: configuredModels.{configured_model_id} must be a JSON object",
                            document.display_path
                        ));
                        continue;
                    };
                    for field in ["providerId", "modelId", "name"] {
                        if entry_object.get(field).and_then(Value::as_str).is_none() {
                            errors.push(format!(
                                "{}: configuredModels.{configured_model_id}.{field} is required",
                                document.display_path
                            ));
                        }
                    }
                }
            }
        }

        Ok(RuntimeConfigValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    pub(crate) fn validate_registry_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let effective_config = self.load_effective_config_json(documents)?;
        let mut validation = self.validate_documents(documents)?;
        let snapshot = build_catalog_snapshot(self, &effective_config)?;
        validation
            .warnings
            .extend(snapshot.diagnostics.warnings.clone());
        validation
            .errors
            .extend(snapshot.diagnostics.errors.clone());
        validation.valid = validation.errors.is_empty();
        Ok(validation)
    }

    pub(crate) fn patched_documents(
        &self,
        scope: RuntimeConfigScopeKind,
        project_id: Option<&str>,
        user_id: Option<&str>,
        patch: &Value,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        let patch_object = patch
            .as_object()
            .ok_or_else(|| AppError::invalid_input("runtime config patch must be a JSON object"))?;

        let mut documents = self.resolve_documents(project_id, user_id)?;
        let target_document = documents
            .iter_mut()
            .find(|document| document.scope == scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let mut next = target_document.document.clone().unwrap_or_default();
        Self::merge_patch(&mut next, patch_object);
        target_document.exists = true;
        target_document.loaded = true;
        target_document.document = Some(next);

        Ok(documents)
    }

    pub(crate) async fn probe_configured_model_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
        configured_model_id: &str,
        _api_key: Option<&str>,
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
        let snapshot = build_catalog_snapshot(self, &effective_config)?;
        let configured_model = snapshot
            .configured_models
            .iter()
            .find(|record| record.configured_model_id == configured_model_id)
            .cloned();

        let Some(configured_model) = configured_model else {
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
        };

        let reachable = configured_model.enabled
            && configured_model.status == "configured"
            && snapshot
                .models
                .iter()
                .any(|model| model.model_id == configured_model.model_id);

        let errors = if reachable {
            Vec::new()
        } else {
            vec![format!(
                "configured model `{configured_model_id}` is not ready for probe"
            )]
        };

        Ok(RuntimeConfiguredModelProbeResult {
            valid: true,
            reachable,
            configured_model_id: configured_model_id.to_string(),
            configured_model_name: Some(configured_model.name),
            request_id: reachable.then(|| "runtime-sdk-probe".to_string()),
            consumed_tokens: reachable.then_some(0),
            errors,
            warnings: validation.warnings,
        })
    }
}
