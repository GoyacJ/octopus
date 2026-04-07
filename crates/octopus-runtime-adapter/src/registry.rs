use std::collections::{BTreeMap, HashMap, HashSet};

use octopus_core::{
    AppError, CapabilityDescriptor, ConfiguredModelRecord, ConfiguredModelTokenQuota,
    ConfiguredModelTokenUsage, CredentialBinding, DefaultSelection, ModelCatalogSnapshot,
    ModelRegistryDiagnostics, ModelRegistryRecord, ModelSurfaceBinding, ProjectWorkspaceAssignments,
    ProviderConfig, ProviderRegistryRecord, ResolvedExecutionTarget, SurfaceDescriptor,
};
use serde_json::{json, Value};

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
        let legacy_configured_models = build_legacy_configured_models(&models, &credential_bindings);
        if configured_models.is_empty() {
            configured_models = legacy_configured_models;
        } else {
            for (configured_model_id, configured_model) in legacy_configured_models {
                configured_models
                    .entry(configured_model_id)
                    .or_insert(configured_model);
            }
        }

        normalize_default_selection_configured_model_ids(&mut default_selections, &configured_models);
        let allowed_configured_model_ids = apply_project_settings(
            &mut default_selections,
            &configured_models,
            effective_config.get("projectSettings"),
            effective_config.get("mcpServers"),
            &mut diagnostics,
        );

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
        let configured_models_list =
            sorted_values(&configured_models, |record| record.configured_model_id.clone());
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

        let provider = self.providers_by_id.get(&configured_model.provider_id).ok_or_else(|| {
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
            .or_else(|| model.surface_bindings.iter().find(|binding| binding.enabled))
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
        let credential_ref = configured_model
            .credential_ref
            .clone()
            .or_else(|| credential_binding.as_ref().map(|binding| binding.credential_ref.clone()));
        let base_url = configured_model
            .base_url
            .clone()
            .or_else(|| credential_binding.as_ref().and_then(|binding| binding.base_url.clone()))
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
            capabilities: model.capabilities.clone(),
        })
    }
}

fn sorted_values<T, F>(records: &BTreeMap<String, T>, key_fn: F) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> String,
{
    let mut values = records.values().cloned().collect::<Vec<_>>();
    values.sort_by_key(|record| key_fn(record));
    values
}

fn capability(capability_id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        capability_id: capability_id.into(),
        label: capability_id.replace('_', " "),
    }
}

fn token_usage_summary(
    quota: Option<&ConfiguredModelTokenQuota>,
    used_tokens: u64,
) -> ConfiguredModelTokenUsage {
    let total_tokens = quota.and_then(|entry| entry.total_tokens);
    ConfiguredModelTokenUsage {
        used_tokens,
        remaining_tokens: total_tokens.map(|total| total.saturating_sub(used_tokens)),
        exhausted: total_tokens.is_some_and(|total| used_tokens >= total),
    }
}

fn binding(surface: &str, protocol_family: &str) -> ModelSurfaceBinding {
    ModelSurfaceBinding {
        surface: surface.into(),
        protocol_family: protocol_family.into(),
        enabled: true,
    }
}

fn surface(
    surface_id: &str,
    protocol_family: &str,
    transport: &[&str],
    auth_strategy: &str,
    base_url: &str,
    base_url_policy: &str,
    capabilities: &[&str],
) -> SurfaceDescriptor {
    SurfaceDescriptor {
        surface: surface_id.into(),
        protocol_family: protocol_family.into(),
        transport: transport.iter().map(|value| (*value).to_string()).collect(),
        auth_strategy: auth_strategy.into(),
        base_url: base_url.into(),
        base_url_policy: base_url_policy.into(),
        enabled: true,
        capabilities: capabilities.iter().map(|value| capability(value)).collect(),
    }
}

fn provider(provider_id: &str, label: &str, surfaces: Vec<SurfaceDescriptor>) -> ProviderRegistryRecord {
    ProviderRegistryRecord {
        provider_id: provider_id.into(),
        label: label.into(),
        enabled: true,
        surfaces,
        metadata: json!({}),
    }
}

fn model(
    model_id: &str,
    provider_id: &str,
    label: &str,
    description: &str,
    family: &str,
    track: &str,
    recommended_for: &str,
    surface_bindings: Vec<ModelSurfaceBinding>,
    capabilities: &[&str],
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
) -> ModelRegistryRecord {
    ModelRegistryRecord {
        model_id: model_id.into(),
        provider_id: provider_id.into(),
        label: label.into(),
        description: description.into(),
        family: family.into(),
        track: track.into(),
        enabled: true,
        recommended_for: recommended_for.into(),
        availability: "healthy".into(),
        default_permission: "auto".into(),
        surface_bindings,
        capabilities: capabilities.iter().map(|value| capability(value)).collect(),
        context_window,
        max_output_tokens,
        metadata: json!({}),
    }
}

fn baseline_providers() -> BTreeMap<String, ProviderRegistryRecord> {
    BTreeMap::from([
        (
            "anthropic".into(),
            provider(
                "anthropic",
                "Anthropic",
                vec![surface(
                    "conversation",
                    "anthropic_messages",
                    &["request_response", "sse"],
                    "bearer",
                    "https://api.anthropic.com",
                    "allow_override",
                    &["streaming", "tool_calling", "structured_output", "reasoning"],
                )],
            ),
        ),
        (
            "openai".into(),
            provider(
                "openai",
                "OpenAI",
                vec![
                    surface(
                        "conversation",
                        "openai_chat",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.openai.com/v1",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                    surface(
                        "responses",
                        "openai_responses",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.openai.com/v1",
                        "allow_override",
                        &[
                            "streaming",
                            "tool_calling",
                            "structured_output",
                            "files",
                            "web_search",
                            "computer_use",
                            "mcp",
                        ],
                    ),
                ],
            ),
        ),
        (
            "xai".into(),
            provider(
                "xai",
                "xAI",
                vec![surface(
                    "conversation",
                    "openai_chat",
                    &["request_response", "sse"],
                    "bearer",
                    "https://api.x.ai/v1",
                    "allow_override",
                    &["streaming", "tool_calling", "structured_output", "reasoning"],
                )],
            ),
        ),
        (
            "deepseek".into(),
            provider(
                "deepseek",
                "DeepSeek",
                vec![
                    surface(
                        "conversation",
                        "openai_chat",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.deepseek.com",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output", "reasoning"],
                    ),
                    surface(
                        "conversation",
                        "anthropic_messages",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.deepseek.com/anthropic",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output", "reasoning"],
                    ),
                ],
            ),
        ),
        (
            "minimax".into(),
            provider(
                "minimax",
                "MiniMax",
                vec![
                    surface(
                        "conversation",
                        "anthropic_messages",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.minimaxi.com/anthropic",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                    surface(
                        "conversation",
                        "vendor_native",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.minimaxi.com",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                    surface(
                        "conversation",
                        "openai_chat",
                        &["request_response", "sse"],
                        "bearer",
                        "https://api.minimaxi.com",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                ],
            ),
        ),
        (
            "moonshot".into(),
            provider(
                "moonshot",
                "Moonshot",
                vec![surface(
                    "conversation",
                    "openai_chat",
                    &["request_response", "sse"],
                    "bearer",
                    "https://api.moonshot.cn/v1",
                    "allow_override",
                    &["streaming", "tool_calling", "structured_output", "reasoning"],
                )],
            ),
        ),
        (
            "bigmodel".into(),
            provider(
                "bigmodel",
                "BigModel",
                vec![
                    surface(
                        "conversation",
                        "openai_chat",
                        &["request_response", "sse"],
                        "bearer",
                        "https://open.bigmodel.cn/api/paas/v4",
                        "allow_override",
                        &[
                            "streaming",
                            "tool_calling",
                            "structured_output",
                            "context_cache",
                            "mcp",
                            "web_search",
                            "batch",
                        ],
                    ),
                ],
            ),
        ),
        (
            "qwen".into(),
            provider(
                "qwen",
                "Qwen",
                vec![
                    surface(
                        "conversation",
                        "openai_chat",
                        &["request_response", "sse"],
                        "bearer",
                        "https://dashscope.aliyuncs.com/compatible-mode/v1",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                    surface(
                        "conversation",
                        "anthropic_messages",
                        &["request_response", "sse"],
                        "bearer",
                        "https://dashscope.aliyuncs.com/api/v2/apps/claude-code-proxy",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output"],
                    ),
                    surface(
                        "realtime",
                        "vendor_native",
                        &["websocket", "request_response"],
                        "bearer",
                        "https://dashscope.aliyuncs.com",
                        "allow_override",
                        &["audio_io", "realtime"],
                    ),
                ],
            ),
        ),
        (
            "ark".into(),
            provider(
                "ark",
                "Ark",
                vec![
                    surface(
                        "responses",
                        "openai_responses",
                        &["request_response", "sse"],
                        "bearer",
                        "https://ark.cn-beijing.volces.com/api/v3",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output", "files", "context_cache"],
                    ),
                ],
            ),
        ),
        (
            "google".into(),
            provider(
                "google",
                "Google",
                vec![
                    surface(
                        "conversation",
                        "gemini_native",
                        &["request_response", "sse"],
                        "x_api_key",
                        "https://generativelanguage.googleapis.com",
                        "allow_override",
                        &["streaming", "tool_calling", "structured_output", "vision_input", "web_search"],
                    ),
                    surface(
                        "realtime",
                        "gemini_native",
                        &["websocket"],
                        "x_api_key",
                        "https://generativelanguage.googleapis.com",
                        "allow_override",
                        &["audio_io", "realtime"],
                    ),
                ],
            ),
        ),
        (
            "ollama".into(),
            provider(
                "ollama",
                "Ollama",
                vec![surface(
                    "conversation",
                    "openai_chat",
                    &["request_response", "sse"],
                    "bearer",
                    "http://127.0.0.1:11434/v1",
                    "allow_override",
                    &["streaming", "tool_calling", "structured_output"],
                )],
            ),
        ),
        (
            "vllm".into(),
            provider(
                "vllm",
                "vLLM",
                vec![surface(
                    "conversation",
                    "openai_chat",
                    &["request_response", "sse"],
                    "bearer",
                    "http://127.0.0.1:8000/v1",
                    "allow_override",
                    &["streaming", "tool_calling", "structured_output"],
                )],
            ),
        ),
    ])
}

fn baseline_models() -> BTreeMap<String, ModelRegistryRecord> {
    BTreeMap::from([
        (
            "claude-sonnet-4-5".into(),
            model(
                "claude-sonnet-4-5",
                "anthropic",
                "Claude Sonnet 4.5",
                "Balanced reasoning model for daily runtime turns.",
                "claude-sonnet",
                "stable",
                "Planning, coding, and reviews",
                vec![binding("conversation", "anthropic_messages")],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                Some(200_000),
                Some(64_000),
            ),
        ),
        (
            "claude-opus-4-6".into(),
            model(
                "claude-opus-4-6",
                "anthropic",
                "Claude Opus 4.6",
                "Highest capability Anthropic model.",
                "claude-opus",
                "stable",
                "Heavy reasoning and synthesis",
                vec![binding("conversation", "anthropic_messages")],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                Some(200_000),
                Some(32_000),
            ),
        ),
        (
            "gpt-5.4".into(),
            model(
                "gpt-5.4",
                "openai",
                "GPT-5.4",
                "OpenAI flagship reasoning model.",
                "gpt-5.4",
                "stable",
                "High capability multimodal work",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("responses", "openai_responses"),
                ],
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                    "vision_input",
                    "files",
                    "web_search",
                    "computer_use",
                    "mcp",
                ],
                Some(400_000),
                Some(128_000),
            ),
        ),
        (
            "gpt-5.4-mini".into(),
            model(
                "gpt-5.4-mini",
                "openai",
                "GPT-5.4 Mini",
                "Balanced fast OpenAI model.",
                "gpt-5.4",
                "stable",
                "Fast orchestration and coding",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("responses", "openai_responses"),
                ],
                &["streaming", "tool_calling", "structured_output", "reasoning", "files"],
                Some(400_000),
                Some(128_000),
            ),
        ),
        (
            "gpt-5.4-nano".into(),
            model(
                "gpt-5.4-nano",
                "openai",
                "GPT-5.4 Nano",
                "Small fast OpenAI model.",
                "gpt-5.4",
                "stable",
                "Low-latency tasks",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("responses", "openai_responses"),
                ],
                &["streaming", "tool_calling", "structured_output"],
                Some(400_000),
                Some(64_000),
            ),
        ),
        (
            "grok-3".into(),
            model(
                "grok-3",
                "xai",
                "Grok 3",
                "xAI reasoning model.",
                "grok",
                "stable",
                "General chat and reasoning",
                vec![binding("conversation", "openai_chat")],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                Some(131_072),
                Some(64_000),
            ),
        ),
        (
            "grok-3-mini".into(),
            model(
                "grok-3-mini",
                "xai",
                "Grok 3 Mini",
                "Smaller xAI model.",
                "grok",
                "stable",
                "Fast general chat",
                vec![binding("conversation", "openai_chat")],
                &["streaming", "tool_calling", "structured_output"],
                Some(131_072),
                Some(64_000),
            ),
        ),
        (
            "deepseek-chat".into(),
            model(
                "deepseek-chat",
                "deepseek",
                "DeepSeek Chat",
                "General DeepSeek conversation model.",
                "deepseek-chat",
                "latest_alias",
                "General chat and coding",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("conversation", "anthropic_messages"),
                ],
                &["streaming", "tool_calling", "structured_output"],
                Some(128_000),
                Some(64_000),
            ),
        ),
        (
            "deepseek-reasoner".into(),
            model(
                "deepseek-reasoner",
                "deepseek",
                "DeepSeek Reasoner",
                "Reasoning-focused DeepSeek model.",
                "deepseek-reasoner",
                "latest_alias",
                "Reasoning-heavy work",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("conversation", "anthropic_messages"),
                ],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                Some(128_000),
                Some(64_000),
            ),
        ),
        (
            "MiniMax-M2.7".into(),
            model(
                "MiniMax-M2.7",
                "minimax",
                "MiniMax M2.7",
                "Primary MiniMax conversation model.",
                "MiniMax-M2",
                "stable",
                "Conversation and coding",
                vec![
                    binding("conversation", "anthropic_messages"),
                    binding("conversation", "vendor_native"),
                    binding("conversation", "openai_chat"),
                ],
                &["streaming", "tool_calling", "structured_output"],
                None,
                None,
            ),
        ),
        (
            "MiniMax-M2.5".into(),
            model(
                "MiniMax-M2.5",
                "minimax",
                "MiniMax M2.5",
                "Secondary MiniMax conversation model.",
                "MiniMax-M2",
                "stable",
                "Conversation and structured output",
                vec![
                    binding("conversation", "anthropic_messages"),
                    binding("conversation", "vendor_native"),
                    binding("conversation", "openai_chat"),
                ],
                &["streaming", "tool_calling", "structured_output"],
                None,
                None,
            ),
        ),
        (
            "kimi-k2.5".into(),
            model(
                "kimi-k2.5",
                "moonshot",
                "Kimi K2.5",
                "Primary Moonshot agent model.",
                "kimi-k2",
                "stable",
                "Agentic coding and reasoning",
                vec![binding("conversation", "openai_chat")],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                None,
                None,
            ),
        ),
        (
            "kimi-k2-thinking".into(),
            model(
                "kimi-k2-thinking",
                "moonshot",
                "Kimi K2 Thinking",
                "Moonshot reasoning variant.",
                "kimi-k2",
                "stable",
                "Reasoning-heavy work",
                vec![binding("conversation", "openai_chat")],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                None,
                None,
            ),
        ),
        (
            "glm-5".into(),
            model(
                "glm-5",
                "bigmodel",
                "GLM-5",
                "Primary BigModel model.",
                "glm-5",
                "stable",
                "Structured output and MCP",
                vec![binding("conversation", "openai_chat")],
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "context_cache",
                    "mcp",
                    "web_search",
                    "batch",
                ],
                None,
                None,
            ),
        ),
        (
            "glm-5-turbo".into(),
            model(
                "glm-5-turbo",
                "bigmodel",
                "GLM-5 Turbo",
                "Fast BigModel line.",
                "glm-5",
                "stable",
                "Fast general tasks",
                vec![binding("conversation", "openai_chat")],
                &["streaming", "tool_calling", "structured_output"],
                None,
                None,
            ),
        ),
        (
            "qwen3-max".into(),
            model(
                "qwen3-max",
                "qwen",
                "Qwen3 Max",
                "High capability Qwen model.",
                "qwen3",
                "stable",
                "High-capability conversation",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("conversation", "anthropic_messages"),
                ],
                &["streaming", "tool_calling", "structured_output"],
                None,
                None,
            ),
        ),
        (
            "qwen3-coder-plus".into(),
            model(
                "qwen3-coder-plus",
                "qwen",
                "Qwen3 Coder Plus",
                "Coding-focused Qwen model.",
                "qwen3-coder",
                "stable",
                "Coding and agent work",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("conversation", "anthropic_messages"),
                ],
                &["streaming", "tool_calling", "structured_output", "reasoning"],
                None,
                None,
            ),
        ),
        (
            "qwen3-vl-plus".into(),
            model(
                "qwen3-vl-plus",
                "qwen",
                "Qwen3 VL Plus",
                "Vision-language Qwen model.",
                "qwen3-vl",
                "stable",
                "Vision input and multimodal work",
                vec![
                    binding("conversation", "openai_chat"),
                    binding("conversation", "anthropic_messages"),
                ],
                &["streaming", "tool_calling", "structured_output", "vision_input"],
                None,
                None,
            ),
        ),
        (
            "doubao-seed-1.6".into(),
            model(
                "doubao-seed-1.6",
                "ark",
                "Doubao Seed 1.6",
                "Primary Ark responses model.",
                "doubao-seed-1.6",
                "stable",
                "General responses and multimodal work",
                vec![binding("responses", "openai_responses")],
                &["streaming", "tool_calling", "structured_output", "files", "context_cache"],
                None,
                None,
            ),
        ),
        (
            "doubao-seed-1.6-thinking".into(),
            model(
                "doubao-seed-1.6-thinking",
                "ark",
                "Doubao Seed 1.6 Thinking",
                "Reasoning-oriented Ark model.",
                "doubao-seed-1.6",
                "stable",
                "Reasoning-heavy responses",
                vec![binding("responses", "openai_responses")],
                &["streaming", "tool_calling", "structured_output", "reasoning", "files"],
                None,
                None,
            ),
        ),
        (
            "gemini-2.5-pro".into(),
            model(
                "gemini-2.5-pro",
                "google",
                "Gemini 2.5 Pro",
                "Primary Gemini conversation model.",
                "gemini-2.5",
                "stable",
                "Multimodal reasoning and tools",
                vec![binding("conversation", "gemini_native")],
                &["streaming", "tool_calling", "structured_output", "reasoning", "vision_input", "web_search"],
                None,
                None,
            ),
        ),
        (
            "gemini-2.5-flash".into(),
            model(
                "gemini-2.5-flash",
                "google",
                "Gemini 2.5 Flash",
                "Fast Gemini line.",
                "gemini-2.5",
                "stable",
                "Fast multimodal work",
                vec![binding("conversation", "gemini_native")],
                &["streaming", "tool_calling", "structured_output", "vision_input"],
                None,
                None,
            ),
        ),
    ])
}

fn baseline_default_selections() -> BTreeMap<String, DefaultSelection> {
    BTreeMap::from([
        (
            "conversation".into(),
            DefaultSelection {
                configured_model_id: Some("claude-sonnet-4-5".into()),
                provider_id: "anthropic".into(),
                model_id: "claude-sonnet-4-5".into(),
                surface: "conversation".into(),
            },
        ),
        (
            "responses".into(),
            DefaultSelection {
                configured_model_id: Some("gpt-5.4".into()),
                provider_id: "openai".into(),
                model_id: "gpt-5.4".into(),
                surface: "responses".into(),
            },
        ),
        (
            "fast".into(),
            DefaultSelection {
                configured_model_id: Some("gpt-5.4-mini".into()),
                provider_id: "openai".into(),
                model_id: "gpt-5.4-mini".into(),
                surface: "responses".into(),
            },
        ),
    ])
}

fn apply_provider_overrides(
    providers: &mut BTreeMap<String, ProviderRegistryRecord>,
    overrides: &Value,
) -> Result<(), AppError> {
    let Some(object) = overrides.as_object() else {
        return Ok(());
    };

    for (provider_id, value) in object {
        let mut record = providers
            .get(provider_id)
            .cloned()
            .unwrap_or_else(|| ProviderRegistryRecord {
                provider_id: provider_id.clone(),
                label: titleize(provider_id),
                enabled: true,
                surfaces: Vec::new(),
                metadata: json!({}),
            });

        if let Some(label) = value.get("label").and_then(Value::as_str) {
            record.label = label.to_string();
        }
        if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
            record.enabled = enabled;
        }
        if let Some(metadata) = value.get("metadata") {
            record.metadata = metadata.clone();
        }
        if let Some(surfaces) = value.get("surfaces").and_then(Value::as_array) {
            record.surfaces = surfaces
                .iter()
                .map(parse_surface_descriptor)
                .collect::<Result<Vec<_>, AppError>>()?;
        }

        providers.insert(provider_id.clone(), record);
    }

    Ok(())
}

fn apply_model_overrides(
    models: &mut BTreeMap<String, ModelRegistryRecord>,
    overrides: &Value,
) -> Result<(), AppError> {
    let Some(object) = overrides.as_object() else {
        return Ok(());
    };

    for (model_id, value) in object {
        let mut record = models
            .get(model_id)
            .cloned()
            .unwrap_or_else(|| ModelRegistryRecord {
                model_id: model_id.clone(),
                provider_id: value
                    .get("providerId")
                    .and_then(Value::as_str)
                    .unwrap_or("custom")
                    .to_string(),
                label: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string(),
                description: value
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                family: value
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string(),
                track: value
                    .get("track")
                    .and_then(Value::as_str)
                    .unwrap_or("custom")
                    .to_string(),
                enabled: true,
                recommended_for: value
                    .get("recommendedFor")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                availability: value
                    .get("availability")
                    .and_then(Value::as_str)
                    .unwrap_or("configured")
                    .to_string(),
                default_permission: value
                    .get("defaultPermission")
                    .and_then(Value::as_str)
                    .unwrap_or("auto")
                    .to_string(),
                surface_bindings: Vec::new(),
                capabilities: Vec::new(),
                context_window: value
                    .get("contextWindow")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                max_output_tokens: value
                    .get("maxOutputTokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                metadata: value.get("metadata").cloned().unwrap_or_else(|| json!({})),
            });

        if let Some(provider_id) = value.get("providerId").and_then(Value::as_str) {
            record.provider_id = provider_id.to_string();
        }
        if let Some(label) = value.get("label").and_then(Value::as_str) {
            record.label = label.to_string();
        }
        if let Some(description) = value.get("description").and_then(Value::as_str) {
            record.description = description.to_string();
        }
        if let Some(family) = value.get("family").and_then(Value::as_str) {
            record.family = family.to_string();
        }
        if let Some(track) = value.get("track").and_then(Value::as_str) {
            record.track = track.to_string();
        }
        if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
            record.enabled = enabled;
        }
        if let Some(recommended_for) = value.get("recommendedFor").and_then(Value::as_str) {
            record.recommended_for = recommended_for.to_string();
        }
        if let Some(availability) = value.get("availability").and_then(Value::as_str) {
            record.availability = availability.to_string();
        }
        if let Some(default_permission) = value.get("defaultPermission").and_then(Value::as_str) {
            record.default_permission = default_permission.to_string();
        }
        if let Some(context_window) = value.get("contextWindow").and_then(Value::as_u64) {
            record.context_window = Some(context_window as u32);
        }
        if let Some(max_output_tokens) = value.get("maxOutputTokens").and_then(Value::as_u64) {
            record.max_output_tokens = Some(max_output_tokens as u32);
        }
        if let Some(metadata) = value.get("metadata") {
            record.metadata = metadata.clone();
        }
        if let Some(surface_bindings) = value.get("surfaceBindings").and_then(Value::as_array) {
            record.surface_bindings = surface_bindings
                .iter()
                .map(parse_surface_binding)
                .collect::<Result<Vec<_>, AppError>>()?;
        }
        if let Some(capabilities) = value.get("capabilities").and_then(Value::as_array) {
            record.capabilities = parse_capabilities(capabilities)?;
        }

        models.insert(model_id.clone(), record);
    }

    Ok(())
}

fn apply_default_selections(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    overrides: &Value,
) {
    let Some(object) = overrides.as_object() else {
        return;
    };

    for (purpose, value) in object {
        let provider_id = value
            .get("providerId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let model_id = value
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let surface = value
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("conversation");
        if provider_id.is_empty() || model_id.is_empty() {
            continue;
        }
        default_selections.insert(
            purpose.clone(),
            DefaultSelection {
                configured_model_id: value
                    .get("configuredModelId")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                surface: surface.to_string(),
            },
        );
    }
}

fn normalize_default_selection_configured_model_ids(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
) {
    for selection in default_selections.values_mut() {
        if selection.configured_model_id.is_some() {
            continue;
        }
        if configured_models.contains_key(&selection.model_id) {
            selection.configured_model_id = Some(selection.model_id.clone());
            continue;
        }
        if let Some(configured_model) = configured_models.values().find(|configured_model| {
            configured_model.provider_id == selection.provider_id
                && configured_model.model_id == selection.model_id
        }) {
            selection.configured_model_id = Some(configured_model.configured_model_id.clone());
        }
    }
}

fn apply_project_settings(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    project_settings_value: Option<&Value>,
    mcp_servers_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<HashSet<String>> {
    let Some(project_settings) = project_settings_value.and_then(Value::as_object) else {
        return None;
    };
    let workspace_assignments = parse_workspace_assignments(
        project_settings.get("workspaceAssignments"),
        diagnostics,
    );

    let allowed_configured_model_ids = project_settings
        .get("models")
        .and_then(|value| {
            apply_project_model_settings(
                default_selections,
                configured_models,
                workspace_assignments.as_ref(),
                value,
                diagnostics,
            )
        });

    if let Some(tool_settings) = project_settings.get("tools") {
        validate_project_tool_settings(
            tool_settings,
            workspace_assignments.as_ref(),
            mcp_servers_value,
            diagnostics,
        );
    }

    if let Some(agent_settings) = project_settings.get("agents") {
        validate_project_agent_settings(agent_settings, workspace_assignments.as_ref(), diagnostics);
    }

    allowed_configured_model_ids
}

fn parse_workspace_assignments(
    assignments_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<ProjectWorkspaceAssignments> {
    let Some(assignments_value) = assignments_value else {
        return None;
    };
    match serde_json::from_value::<ProjectWorkspaceAssignments>(assignments_value.clone()) {
        Ok(assignments) => Some(assignments),
        Err(error) => {
            diagnostics.errors.push(format!(
                "projectSettings.workspaceAssignments is invalid: {error}"
            ));
            None
        }
    }
}

fn apply_project_model_settings(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    models_value: &Value,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<HashSet<String>> {
    let Some(models_object) = models_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.models must be an object".into());
        return None;
    };

    let allowed_configured_model_ids = models_object
        .get("allowedConfiguredModelIds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if allowed_configured_model_ids.is_empty() {
        diagnostics.errors.push(
            "projectSettings.models.allowedConfiguredModelIds must include at least one configured model"
                .into(),
        );
        return None;
    }

    let default_configured_model_id = models_object
        .get("defaultConfiguredModelId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default_configured_model_id.is_empty() {
        diagnostics
            .errors
            .push("projectSettings.models.defaultConfiguredModelId is required".into());
        return None;
    }
    if !allowed_configured_model_ids
        .iter()
        .any(|configured_model_id| configured_model_id == default_configured_model_id)
    {
        diagnostics.errors.push(format!(
            "projectSettings.models.defaultConfiguredModelId `{default_configured_model_id}` must be included in allowedConfiguredModelIds"
        ));
        return None;
    }

    let assigned_configured_model_ids = workspace_assignments
        .and_then(|assignments| assignments.models.as_ref())
        .map(|models| {
            models
                .configured_model_ids
                .iter()
                .cloned()
                .collect::<HashSet<_>>()
        });

    for configured_model_id in &allowed_configured_model_ids {
        if !configured_models.contains_key(configured_model_id) {
            diagnostics.errors.push(format!(
                "projectSettings.models.allowedConfiguredModelIds references unknown configured model `{configured_model_id}`"
            ));
        }
        if assigned_configured_model_ids
            .as_ref()
            .is_some_and(|assigned| !assigned.contains(configured_model_id))
        {
            diagnostics.errors.push(format!(
                "projectSettings.models.allowedConfiguredModelIds contains unassigned configured model `{configured_model_id}`"
            ));
        }
    }

    let Some(default_configured_model) = configured_models.get(default_configured_model_id) else {
        diagnostics.errors.push(format!(
            "projectSettings.models.defaultConfiguredModelId references unknown configured model `{default_configured_model_id}`"
        ));
        return None;
    };

    let conversation_surface = default_selections
        .get("conversation")
        .map(|selection| selection.surface.clone())
        .unwrap_or_else(|| "conversation".into());
    default_selections.insert(
        "conversation".into(),
        DefaultSelection {
            configured_model_id: Some(default_configured_model.configured_model_id.clone()),
            provider_id: default_configured_model.provider_id.clone(),
            model_id: default_configured_model.model_id.clone(),
            surface: conversation_surface,
        },
    );

    Some(
        allowed_configured_model_ids
            .into_iter()
            .collect::<HashSet<_>>(),
    )
}

fn validate_project_tool_settings(
    tools_value: &Value,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    mcp_servers_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let Some(tools_object) = tools_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.tools must be an object".into());
        return;
    };
    let assigned_source_keys = workspace_assignments
        .and_then(|assignments| assignments.tools.as_ref())
        .map(|tools| tools.source_keys.iter().cloned().collect::<HashSet<_>>());

    if let Some(enabled_source_keys) = tools_object.get("enabledSourceKeys") {
        let Some(enabled_source_keys) = enabled_source_keys.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.tools.enabledSourceKeys must be an array".into());
            return;
        };
        if enabled_source_keys.is_empty() {
            diagnostics
                .errors
                .push("projectSettings.tools.enabledSourceKeys must include at least one sourceKey".into());
        }
        for source_key in enabled_source_keys.iter().filter_map(Value::as_str) {
            if assigned_source_keys
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(source_key))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.enabledSourceKeys contains unassigned sourceKey `{source_key}`"
                ));
            }
        }
    }

    let Some(overrides) = tools_object.get("overrides") else {
        return;
    };
    let Some(overrides_object) = overrides.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.tools.overrides must be an object".into());
        return;
    };

    let known_mcp_server_names = mcp_servers_value
        .and_then(Value::as_object)
        .map(|servers| servers.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();

    for (source_key, override_value) in overrides_object {
        if assigned_source_keys
            .as_ref()
            .is_some_and(|assigned| !assigned.contains(source_key))
        {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides contains unassigned sourceKey `{source_key}`"
            ));
        }
        let Some(override_object) = override_value.as_object() else {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides.{source_key} must be an object"
            ));
            continue;
        };

        let permission_mode = override_object
            .get("permissionMode")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(permission_mode, "allow" | "ask" | "readonly" | "deny") {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides.{source_key}.permissionMode `{permission_mode}` is unsupported"
            ));
        }

        if let Some(builtin_key) = source_key.strip_prefix("builtin:") {
            if builtin_key.trim().is_empty() {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides contains an invalid builtin sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        if let Some(server_name) = source_key.strip_prefix("mcp:") {
            if server_name.trim().is_empty() || !known_mcp_server_names.contains(server_name) {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides references unknown mcp sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        if let Some(skill_path) = source_key.strip_prefix("skill:") {
            if skill_path.trim().is_empty() {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides contains an invalid skill sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        diagnostics.errors.push(format!(
            "projectSettings.tools.overrides contains unsupported sourceKey `{source_key}`"
        ));
    }
}

fn validate_project_agent_settings(
    agents_value: &Value,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let Some(agents_object) = agents_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.agents must be an object".into());
        return;
    };

    let assigned_agent_ids = workspace_assignments
        .and_then(|assignments| assignments.agents.as_ref())
        .map(|agents| agents.agent_ids.iter().cloned().collect::<HashSet<_>>());
    let assigned_team_ids = workspace_assignments
        .and_then(|assignments| assignments.agents.as_ref())
        .map(|agents| agents.team_ids.iter().cloned().collect::<HashSet<_>>());

    if let Some(enabled_agent_ids) = agents_object.get("enabledAgentIds") {
        let Some(enabled_agent_ids) = enabled_agent_ids.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.agents.enabledAgentIds must be an array".into());
            return;
        };
        for agent_id in enabled_agent_ids.iter().filter_map(Value::as_str) {
            if assigned_agent_ids
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(agent_id))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.agents.enabledAgentIds contains unassigned agent `{agent_id}`"
                ));
            }
        }
    }

    if let Some(enabled_team_ids) = agents_object.get("enabledTeamIds") {
        let Some(enabled_team_ids) = enabled_team_ids.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.agents.enabledTeamIds must be an array".into());
            return;
        };
        for team_id in enabled_team_ids.iter().filter_map(Value::as_str) {
            if assigned_team_ids
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(team_id))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.agents.enabledTeamIds contains unassigned team `{team_id}`"
                ));
            }
        }
    }
}

fn validate_configured_models(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    models: &BTreeMap<String, ModelRegistryRecord>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let mut names = HashMap::<String, String>::new();

    for configured_model in configured_models.values() {
        let trimmed_name = configured_model.name.trim();
        if trimmed_name.is_empty() {
            diagnostics.errors.push(format!(
                "configured model `{}` is missing a display name",
                configured_model.configured_model_id
            ));
        } else if let Some(existing) =
            names.insert(trimmed_name.to_lowercase(), configured_model.configured_model_id.clone())
        {
            diagnostics.errors.push(format!(
                "configured model name `{trimmed_name}` is duplicated by `{existing}` and `{}`",
                configured_model.configured_model_id
            ));
        }

        let Some(provider) = providers.get(&configured_model.provider_id) else {
            diagnostics.errors.push(format!(
                "configured model `{}` references unknown provider `{}`",
                configured_model.configured_model_id, configured_model.provider_id
            ));
            continue;
        };
        let Some(model) = models.get(&configured_model.model_id) else {
            diagnostics.errors.push(format!(
                "configured model `{}` references unknown model `{}`",
                configured_model.configured_model_id, configured_model.model_id
            ));
            continue;
        };
        if model.provider_id != provider.provider_id {
            diagnostics.errors.push(format!(
                "configured model `{}` model `{}` does not belong to provider `{}`",
                configured_model.configured_model_id, configured_model.model_id, configured_model.provider_id
            ));
        }

        let provider_type = provider
            .metadata
            .get("providerType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let provider_managed_by_page = provider
            .metadata
            .get(WORKSPACE_MODELS_PAGE_METADATA_KEY)
            .and_then(Value::as_str)
            == Some(WORKSPACE_MODELS_PAGE_METADATA_VALUE);
        if provider_type == CUSTOM_PROVIDER_TYPE && provider_managed_by_page {
            let provider_surface_base_url = provider
                .surfaces
                .iter()
                .find(|surface| surface.enabled && surface.surface == "conversation")
                .map(|surface| surface.base_url.as_str())
                .or_else(|| provider.surfaces.iter().find(|surface| surface.enabled).map(|surface| surface.base_url.as_str()));
            let configured_base_url = configured_model.base_url.as_deref();
            if configured_base_url == Some(CUSTOM_BASE_URL_PLACEHOLDER)
                || provider_surface_base_url == Some(CUSTOM_BASE_URL_PLACEHOLDER)
            {
                diagnostics.warnings.push(format!(
                    "configured model `{}` still uses the custom provider placeholder base URL `{}`",
                    configured_model.configured_model_id, CUSTOM_BASE_URL_PLACEHOLDER
                ));
            }
        }
    }
}

fn build_configured_models(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    models: &BTreeMap<String, ModelRegistryRecord>,
    credential_bindings: &BTreeMap<String, CredentialBinding>,
    configured_models_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Result<BTreeMap<String, ConfiguredModelRecord>, AppError> {
    let Some(object) = configured_models_value.and_then(Value::as_object) else {
        return Ok(BTreeMap::new());
    };

    let mut configured_models = BTreeMap::new();
    for (key, value) in object {
        let configured_model_id = value
            .get("configuredModelId")
            .and_then(Value::as_str)
            .unwrap_or(key)
            .to_string();
        let model_id = value
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let provider_id = value
            .get("providerId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| models.get(&model_id).map(|model| model.provider_id.clone()))
            .unwrap_or_default();
        let credential_ref = value
            .get("credentialRef")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let configured = credential_ref
            .as_deref()
            .map(reference_present)
            .transpose()?
            .unwrap_or(false);
        let status = if credential_ref.is_some() {
            if configured {
                "configured"
            } else {
                "error"
            }
        } else if credential_bindings.contains_key(&provider_id) {
            "configured"
        } else {
            "unconfigured"
        };
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| models.get(&model_id).map(|model| model.label.clone()))
            .unwrap_or_else(|| configured_model_id.clone());
        let token_quota = parse_token_quota(
            value.get("tokenQuota"),
            &configured_model_id,
            diagnostics,
        );

        let _ = providers;
        configured_models.insert(
            configured_model_id.clone(),
            ConfiguredModelRecord {
                configured_model_id,
                name,
                provider_id,
                model_id,
                credential_ref,
                base_url: value
                    .get("baseUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                token_quota: token_quota.clone(),
                token_usage: token_usage_summary(token_quota.as_ref(), 0),
                enabled: value.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                source: value
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .to_string(),
                status: status.to_string(),
                configured,
            },
        );
    }

    Ok(configured_models)
}

fn build_legacy_configured_models(
    models: &BTreeMap<String, ModelRegistryRecord>,
    credential_bindings: &BTreeMap<String, CredentialBinding>,
) -> BTreeMap<String, ConfiguredModelRecord> {
    let mut configured_models = BTreeMap::new();

    for model in models.values() {
        let binding = credential_bindings.get(&model.provider_id);
        configured_models.insert(
            model.model_id.clone(),
            ConfiguredModelRecord {
                configured_model_id: model.model_id.clone(),
                name: model.label.clone(),
                provider_id: model.provider_id.clone(),
                model_id: model.model_id.clone(),
                credential_ref: binding.map(|entry| entry.credential_ref.clone()),
                base_url: binding.and_then(|entry| entry.base_url.clone()),
                token_quota: None,
                token_usage: token_usage_summary(None, 0),
                enabled: model.enabled,
                source: "legacy".into(),
                status: binding
                    .map(|entry| entry.status.clone())
                    .unwrap_or_else(|| "missing".into()),
                configured: binding.map(|entry| entry.configured).unwrap_or(false),
            },
        );
    }

    configured_models
}

fn parse_token_quota(
    token_quota_value: Option<&Value>,
    configured_model_id: &str,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<ConfiguredModelTokenQuota> {
    let Some(token_quota_value) = token_quota_value else {
        return None;
    };
    let Some(token_quota_object) = token_quota_value.as_object() else {
        diagnostics.errors.push(format!(
            "configured model `{configured_model_id}` tokenQuota must be an object"
        ));
        return None;
    };

    let total_tokens = match token_quota_object.get("totalTokens") {
        None | Some(Value::Null) => None,
        Some(Value::Number(number)) => {
            let Some(value) = number.as_u64() else {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` tokenQuota.totalTokens must be a positive integer"
                ));
                return None;
            };
            if value == 0 {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` tokenQuota.totalTokens must be greater than zero"
                ));
                return None;
            }
            Some(value)
        }
        Some(_) => {
            diagnostics.errors.push(format!(
                "configured model `{configured_model_id}` tokenQuota.totalTokens must be a positive integer"
            ));
            return None;
        }
    };

    Some(ConfiguredModelTokenQuota { total_tokens })
}

fn build_credential_bindings(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    configured_refs: Option<&Value>,
) -> Result<BTreeMap<String, CredentialBinding>, AppError> {
    let configured_object = configured_refs.and_then(Value::as_object);
    let mut bindings = BTreeMap::new();

    for provider in providers.values() {
        let env_name = default_credential_env(&provider.provider_id);
        let configured_value = configured_object.and_then(|entries| entries.get(&provider.provider_id));

        let mut credential_ref = env_name.map(|value| format!("env:{value}"));
        let mut label = format!("{} Primary", provider.label);
        let mut base_url = None;
        let mut source = "baseline".to_string();
        let mut status = "unconfigured".to_string();

        if let Some(value) = configured_value {
            source = "runtime_config".into();
            if let Some(reference) = value.as_str() {
                credential_ref = Some(reference.to_string());
            } else if let Some(object) = value.as_object() {
                if let Some(reference) = object
                    .get("credentialRef")
                    .or_else(|| object.get("reference"))
                    .and_then(Value::as_str)
                {
                    credential_ref = Some(reference.to_string());
                }
                if let Some(configured_label) = object.get("label").and_then(Value::as_str) {
                    label = configured_label.to_string();
                }
                if let Some(configured_base_url) = object.get("baseUrl").and_then(Value::as_str) {
                    base_url = Some(configured_base_url.to_string());
                }
                if let Some(configured_status) = object.get("status").and_then(Value::as_str) {
                    status = configured_status.to_string();
                }
            }
        }

        let configured = credential_ref
            .as_deref()
            .map(reference_present)
            .transpose()?
            .unwrap_or(false);

        if status == "unconfigured" {
            status = if configured {
                "configured".into()
            } else {
                "unconfigured".into()
            };
        }

        bindings.insert(
            provider.provider_id.clone(),
            CredentialBinding {
                credential_ref: credential_ref
                    .unwrap_or_else(|| format!("env:{}", env_name.unwrap_or("MISSING_API_KEY"))),
                provider_id: provider.provider_id.clone(),
                label,
                base_url,
                status,
                configured,
                source,
            },
        );
    }

    Ok(bindings)
}

fn parse_surface_descriptor(value: &Value) -> Result<SurfaceDescriptor, AppError> {
    Ok(SurfaceDescriptor {
        surface: required_string(value, "surface")?,
        protocol_family: required_string(value, "protocolFamily")?,
        transport: value
            .get("transport")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        auth_strategy: value
            .get("authStrategy")
            .and_then(Value::as_str)
            .unwrap_or("bearer")
            .to_string(),
        base_url: value
            .get("baseUrl")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        base_url_policy: value
            .get("baseUrlPolicy")
            .and_then(Value::as_str)
            .unwrap_or("allow_override")
            .to_string(),
        enabled: value.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        capabilities: value
            .get("capabilities")
            .and_then(Value::as_array)
            .map(|entries| parse_capabilities(entries))
            .transpose()?
            .unwrap_or_default(),
    })
}

fn parse_surface_binding(value: &Value) -> Result<ModelSurfaceBinding, AppError> {
    Ok(ModelSurfaceBinding {
        surface: required_string(value, "surface")?,
        protocol_family: value
            .get("protocolFamily")
            .and_then(Value::as_str)
            .unwrap_or("openai_chat")
            .to_string(),
        enabled: value.get("enabled").and_then(Value::as_bool).unwrap_or(true),
    })
}

fn parse_capabilities(entries: &[Value]) -> Result<Vec<CapabilityDescriptor>, AppError> {
    entries
        .iter()
        .map(|entry| {
            if let Some(capability_id) = entry.as_str() {
                return Ok(capability(capability_id));
            }
            Ok(CapabilityDescriptor {
                capability_id: required_string(entry, "capabilityId")?,
                label: entry
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| {
                        entry.get("capabilityId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                    })
                    .to_string(),
            })
        })
        .collect()
}

fn required_string(value: &Value, key: &str) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::invalid_input(format!("registry field `{key}` must be a string")))
}

fn titleize(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_credential_env(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "openai" => Some("OPENAI_API_KEY"),
        "xai" => Some("XAI_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        "minimax" => Some("MINIMAX_API_KEY"),
        "moonshot" => Some("MOONSHOT_API_KEY"),
        "bigmodel" => Some("BIGMODEL_API_KEY"),
        "qwen" => Some("DASHSCOPE_API_KEY"),
        "ark" => Some("ARK_API_KEY"),
        "google" => Some("GOOGLE_API_KEY"),
        _ => None,
    }
}

fn reference_present(reference: &str) -> Result<bool, AppError> {
    if let Some(env_key) = reference.strip_prefix("env:") {
        return Ok(std::env::var_os(env_key).is_some());
    }
    Ok(!reference.trim().is_empty())
}

fn normalize_execution_base_url(
    provider_id: &str,
    protocol_family: &str,
    base_url: String,
) -> String {
    let normalized = base_url.trim_end_matches('/').to_string();
    if provider_id == "minimax"
        && protocol_family == "anthropic_messages"
        && normalized == "https://api.minimaxi.com"
    {
        return "https://api.minimaxi.com/anthropic".to_string();
    }
    normalized
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
}
