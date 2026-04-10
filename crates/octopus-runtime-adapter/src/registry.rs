use std::collections::{BTreeMap, HashMap, HashSet};

use octopus_core::{
    AppError, CapabilityDescriptor, ConfiguredModelRecord, ConfiguredModelTokenQuota,
    ConfiguredModelTokenUsage, CredentialBinding, DefaultSelection, ModelCatalogSnapshot,
    ModelRegistryDiagnostics, ModelRegistryRecord, ModelSurfaceBinding,
    ProjectWorkspaceAssignments, ProviderConfig, ProviderRegistryRecord, ResolvedExecutionTarget,
    SurfaceDescriptor,
};
use serde_json::{json, Value};

#[path = "registry_baseline.rs"]
pub(super) mod baseline;
#[path = "registry_overrides.rs"]
mod overrides;
#[path = "registry_parse.rs"]
pub(super) mod parse;
#[path = "registry_resolution.rs"]
mod resolution;

use baseline::*;
use overrides::*;
use parse::*;
use resolution::*;

const WORKSPACE_MODELS_PAGE_METADATA_KEY: &str = "managedBy";
const WORKSPACE_MODELS_PAGE_METADATA_VALUE: &str = "workspace-models-page";
const CUSTOM_PROVIDER_TYPE: &str = "custom";
const CUSTOM_BASE_URL_PLACEHOLDER: &str = "https://api.example.com/v1";

#[derive(Clone)]
pub struct EffectiveModelRegistry {
    snapshot: ModelCatalogSnapshot,
    providers_by_id: HashMap<String, ProviderRegistryRecord>,
    models_by_id: HashMap<String, ModelRegistryRecord>,
    configured_models_by_id: HashMap<String, ConfiguredModelRecord>,
    credential_bindings_by_provider: HashMap<String, CredentialBinding>,
    allowed_configured_model_ids: Option<HashSet<String>>,
    plugin_max_output_tokens: Option<u32>,
}

impl EffectiveModelRegistry {
    pub fn from_effective_config(effective_config: &Value) -> Result<Self, AppError> {
        let mut providers = baseline_providers();
        let mut models = baseline_models();
        let mut default_selections = baseline_default_selections();
        let mut diagnostics = ModelRegistryDiagnostics {
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        if let Some(provider_overrides) = effective_config.get("providerOverrides") {
            apply_provider_overrides(&mut providers, provider_overrides)?;
        }
        if let Some(top_level_defaults) = effective_config.get("defaultSelections") {
            apply_default_selections(&mut default_selections, top_level_defaults);
        }
        if let Some(model_registry) = effective_config.get("modelRegistry") {
            if let Some(provider_overrides) = model_registry.get("providers") {
                apply_provider_overrides(&mut providers, provider_overrides)?;
            }
            if let Some(model_overrides) = model_registry.get("models") {
                apply_model_overrides(&mut models, model_overrides)?;
            }
            if let Some(defaults) = model_registry.get("defaultSelections") {
                apply_default_selections(&mut default_selections, defaults);
            }
        }

        let credential_bindings =
            build_credential_bindings(&providers, effective_config.get("credentialRefs"))?;
        let mut configured_models = build_configured_models(
            &providers,
            &models,
            &credential_bindings,
            effective_config.get("configuredModels"),
            &mut diagnostics,
        )?;
        let legacy_configured_models =
            build_legacy_configured_models(&models, &credential_bindings);
        if configured_models.is_empty() {
            configured_models = legacy_configured_models;
        } else {
            for (configured_model_id, configured_model) in legacy_configured_models {
                configured_models
                    .entry(configured_model_id)
                    .or_insert(configured_model);
            }
        }

        normalize_default_selection_configured_model_ids(
            &mut default_selections,
            &configured_models,
        );
        let allowed_configured_model_ids = apply_project_settings(
            &mut default_selections,
            &configured_models,
            effective_config.get("projectSettings"),
            effective_config.get("mcpServers"),
            &mut diagnostics,
        );
        let plugin_max_output_tokens = effective_config
            .get("plugins")
            .and_then(|plugins| plugins.get("maxOutputTokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32);

        validate_configured_models(&providers, &models, &configured_models, &mut diagnostics);
        for (purpose, selection) in &default_selections {
            let Some(configured_model_id) = selection.configured_model_id.as_deref() else {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` is missing configured model binding"
                ));
                continue;
            };
            let Some(configured_model) = configured_models.get(configured_model_id) else {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references unknown configured model `{configured_model_id}`"
                ));
                continue;
            };
            if !configured_model.enabled {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references disabled configured model `{configured_model_id}`"
                ));
            }

            let Some(provider) = providers.get(&configured_model.provider_id) else {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references unknown provider `{}`",
                    configured_model.provider_id
                ));
                continue;
            };
            if !provider.enabled {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references disabled provider `{}`",
                    configured_model.provider_id
                ));
            }

            let Some(model) = models.get(&configured_model.model_id) else {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references unknown model `{}`",
                    configured_model.model_id
                ));
                continue;
            };
            if !model.enabled {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` references disabled model `{}`",
                    configured_model.model_id
                ));
            }
            if model.provider_id != configured_model.provider_id {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` model `{}` does not belong to provider `{}`",
                    configured_model.model_id, configured_model.provider_id
                ));
            }
            if selection.model_id != configured_model.model_id {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` model `{}` does not match configured model `{}`",
                    selection.model_id, configured_model_id
                ));
            }
            if selection.provider_id != configured_model.provider_id {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` provider `{}` does not match configured model `{}`",
                    selection.provider_id, configured_model_id
                ));
            }
            if !model
                .surface_bindings
                .iter()
                .any(|binding| binding.enabled && binding.surface == selection.surface)
            {
                diagnostics.errors.push(format!(
                    "default selection `{purpose}` model `{}` does not support surface `{}`",
                    configured_model.model_id, selection.surface
                ));
            }
        }

        let providers_list = sorted_values(&providers, |record| record.provider_id.clone());
        let models_list = sorted_values(&models, |record| record.model_id.clone());
        let configured_models_list = sorted_values(&configured_models, |record| {
            record.configured_model_id.clone()
        });
        let credential_bindings_list =
            sorted_values(&credential_bindings, |record| record.provider_id.clone());

        let snapshot = ModelCatalogSnapshot {
            providers: providers_list.clone(),
            models: models_list.clone(),
            configured_models: configured_models_list.clone(),
            credential_bindings: credential_bindings_list.clone(),
            default_selections: default_selections.clone(),
            diagnostics: diagnostics.clone(),
        };

        Ok(Self {
            snapshot,
            providers_by_id: providers.into_iter().collect(),
            models_by_id: models.into_iter().collect(),
            configured_models_by_id: configured_models.into_iter().collect(),
            credential_bindings_by_provider: credential_bindings.into_iter().collect(),
            allowed_configured_model_ids,
            plugin_max_output_tokens,
        })
    }

    pub fn snapshot(&self) -> ModelCatalogSnapshot {
        self.snapshot.clone()
    }

    pub fn snapshot_with_usage(
        &self,
        used_tokens_by_configured_model_id: &HashMap<String, u64>,
    ) -> ModelCatalogSnapshot {
        let mut snapshot = self.snapshot();
        for configured_model in &mut snapshot.configured_models {
            let used_tokens = used_tokens_by_configured_model_id
                .get(&configured_model.configured_model_id)
                .copied()
                .unwrap_or(0);
            configured_model.token_usage =
                token_usage_summary(configured_model.token_quota.as_ref(), used_tokens);
        }
        snapshot
    }

    pub fn diagnostics(&self) -> &ModelRegistryDiagnostics {
        &self.snapshot.diagnostics
    }

    pub fn configured_model(&self, configured_model_id: &str) -> Option<&ConfiguredModelRecord> {
        self.configured_models_by_id.get(configured_model_id)
    }

    pub fn default_configured_model_id(&self, purpose: &str) -> Option<&str> {
        self.snapshot
            .default_selections
            .get(purpose)
            .and_then(|selection| selection.configured_model_id.as_deref())
    }

    pub fn default_provider_config(&self) -> ProviderConfig {
        let fallback = ProviderConfig {
            provider_id: "anthropic".into(),
            credential_ref: None,
            base_url: None,
            default_model: Some("claude-sonnet-4-5".into()),
            default_surface: Some("conversation".into()),
            protocol_family: Some("anthropic_messages".into()),
        };
        let Some(selection) = self.snapshot.default_selections.get("conversation") else {
            return fallback;
        };
        let Some(configured_model_id) = selection
            .configured_model_id
            .as_deref()
            .or(Some(selection.model_id.as_str()))
        else {
            return fallback;
        };
        let Ok(target) = self.resolve_target(configured_model_id, Some(&selection.surface)) else {
            return fallback;
        };
        ProviderConfig {
            provider_id: target.provider_id,
            credential_ref: target.credential_ref,
            base_url: target.base_url,
            default_model: Some(target.model_id),
            default_surface: Some(target.surface),
            protocol_family: Some(target.protocol_family),
        }
    }

    pub fn resolve_target(
        &self,
        configured_model_id: &str,
        preferred_surface: Option<&str>,
    ) -> Result<ResolvedExecutionTarget, AppError> {
        if self
            .allowed_configured_model_ids
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(configured_model_id))
        {
            return Err(AppError::invalid_input(format!(
                "configured model `{configured_model_id}` is not allowed for this project"
            )));
        }

        let configured_model = self
            .configured_models_by_id
            .get(configured_model_id)
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "configured model `{configured_model_id}` is not registered"
                ))
            })?;
        if !configured_model.enabled {
            return Err(AppError::invalid_input(format!(
                "configured model `{configured_model_id}` is disabled"
            )));
        }

        let model = self
            .models_by_id
            .get(&configured_model.model_id)
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "model `{}` is not registered",
                    configured_model.model_id
                ))
            })?;
        if !model.enabled {
            return Err(AppError::invalid_input(format!(
                "model `{}` is disabled",
                model.model_id
            )));
        }

        let provider = self
            .providers_by_id
            .get(&configured_model.provider_id)
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "provider `{}` for configured model `{configured_model_id}` is not registered",
                    configured_model.provider_id
                ))
            })?;
        if !provider.enabled {
            return Err(AppError::invalid_input(format!(
                "provider `{}` is disabled",
                provider.provider_id
            )));
        }

        let model_surface = preferred_surface
            .and_then(|surface| {
                model
                    .surface_bindings
                    .iter()
                    .find(|binding| binding.enabled && binding.surface == surface)
            })
            .or_else(|| {
                model
                    .surface_bindings
                    .iter()
                    .find(|binding| binding.enabled)
            })
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "model `{}` does not expose an enabled surface",
                    model.model_id
                ))
            })?;

        let provider_surface = provider
            .surfaces
            .iter()
            .find(|surface| {
                surface.enabled
                    && surface.surface == model_surface.surface
                    && surface.protocol_family == model_surface.protocol_family
            })
            .or_else(|| {
                provider
                    .surfaces
                    .iter()
                    .find(|surface| surface.enabled && surface.surface == model_surface.surface)
            })
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "provider `{}` does not support surface `{}` for configured model `{configured_model_id}`",
                    provider.provider_id, model_surface.surface
                ))
            })?;

        let credential_binding = self
            .credential_bindings_by_provider
            .get(&provider.provider_id)
            .cloned();
        let credential_ref = configured_model.credential_ref.clone().or_else(|| {
            credential_binding
                .as_ref()
                .map(|binding| binding.credential_ref.clone())
        });
        let base_url = configured_model
            .base_url
            .clone()
            .or_else(|| {
                credential_binding
                    .as_ref()
                    .and_then(|binding| binding.base_url.clone())
            })
            .or_else(|| Some(provider_surface.base_url.clone()))
            .map(|value| {
                normalize_execution_base_url(
                    &provider.provider_id,
                    &provider_surface.protocol_family,
                    value,
                )
            });

        Ok(ResolvedExecutionTarget {
            configured_model_id: configured_model.configured_model_id.clone(),
            configured_model_name: configured_model.name.clone(),
            provider_id: provider.provider_id.clone(),
            registry_model_id: model.model_id.clone(),
            model_id: model.model_id.clone(),
            surface: model_surface.surface.clone(),
            protocol_family: provider_surface.protocol_family.clone(),
            credential_ref,
            base_url,
            max_output_tokens: self.plugin_max_output_tokens.or(model.max_output_tokens),
            capabilities: model.capabilities.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_baseline_includes_ollama_and_vllm_providers() {
        let registry = EffectiveModelRegistry::from_effective_config(&json!({}))
            .expect("registry should load");
        let provider_ids = registry
            .snapshot()
            .providers
            .into_iter()
            .map(|provider| provider.provider_id)
            .collect::<Vec<_>>();

        assert!(provider_ids.contains(&"ollama".to_string()));
        assert!(provider_ids.contains(&"vllm".to_string()));
    }

    #[test]
    fn custom_placeholder_base_url_produces_validation_warning() {
        let registry = EffectiveModelRegistry::from_effective_config(&json!({
            "modelRegistry": {
                "providers": {
                    "custom-demo": {
                        "label": "Custom Demo",
                        "surfaces": [{
                            "surface": "conversation",
                            "protocolFamily": "openai_chat",
                            "transport": ["request_response", "sse"],
                            "authStrategy": "bearer",
                            "baseUrl": CUSTOM_BASE_URL_PLACEHOLDER,
                            "baseUrlPolicy": "allow_override",
                            "enabled": true,
                            "capabilities": []
                        }],
                        "metadata": {
                            "managedBy": WORKSPACE_MODELS_PAGE_METADATA_VALUE,
                            "providerType": CUSTOM_PROVIDER_TYPE
                        }
                    }
                },
                "models": {
                    "custom-demo/test-model": {
                        "providerId": "custom-demo",
                        "label": "test-model",
                        "description": "",
                        "family": "custom",
                        "track": "custom",
                        "enabled": true,
                        "recommendedFor": "",
                        "availability": "configured",
                        "defaultPermission": "auto",
                        "surfaceBindings": [{
                            "surface": "conversation",
                            "protocolFamily": "openai_chat",
                            "enabled": true
                        }],
                        "capabilities": [],
                        "metadata": {
                            "managedBy": WORKSPACE_MODELS_PAGE_METADATA_VALUE,
                            "providerType": CUSTOM_PROVIDER_TYPE
                        }
                    }
                }
            },
            "configuredModels": {
                "custom-configured": {
                    "configuredModelId": "custom-configured",
                    "name": "Custom Configured",
                    "providerId": "custom-demo",
                    "modelId": "custom-demo/test-model",
                    "baseUrl": CUSTOM_BASE_URL_PLACEHOLDER,
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("registry should load");

        assert!(registry
            .diagnostics()
            .warnings
            .iter()
            .any(|warning| warning.contains(CUSTOM_BASE_URL_PLACEHOLDER)));
    }

    #[test]
    fn minimax_defaults_to_anthropic_compat_execution_target() {
        let registry = EffectiveModelRegistry::from_effective_config(&json!({
            "configuredModels": {
                "minimax-primary": {
                    "configuredModelId": "minimax-primary",
                    "name": "MiniMax Primary",
                    "providerId": "minimax",
                    "modelId": "MiniMax-M2.7",
                    "baseUrl": "https://api.minimaxi.com",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("registry should load");

        let resolved = registry
            .resolve_target("minimax-primary", Some("conversation"))
            .expect("minimax target should resolve");

        assert_eq!(resolved.protocol_family, "anthropic_messages");
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("https://api.minimaxi.com/anthropic")
        );
    }

    #[test]
    fn plugin_max_output_tokens_overrides_registry_model_default_for_target() {
        let registry = EffectiveModelRegistry::from_effective_config(&json!({
            "plugins": {
                "maxOutputTokens": 4096
            },
            "configuredModels": {
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-opus-4-6",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("registry should load");

        let resolved = registry
            .resolve_target("quota-model", Some("conversation"))
            .expect("target should resolve");

        assert_eq!(resolved.max_output_tokens, Some(4096));
    }
}
