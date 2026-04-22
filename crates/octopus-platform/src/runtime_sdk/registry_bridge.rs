use std::collections::{BTreeMap, BTreeSet, HashMap};

use async_trait::async_trait;
use octopus_core::{
    AppError, CapabilityDescriptor, ConfiguredModelBudgetPolicy, ConfiguredModelRecord,
    ConfiguredModelTokenUsage, CredentialBinding, DefaultSelection, ModelCatalogSnapshot,
    ModelRegistryDiagnostics, ModelRegistryRecord, ModelSurfaceBinding, ProviderRegistryRecord,
    RuntimeExecutionClass, RuntimeExecutionProfile, SurfaceDescriptor,
};
use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::runtime::{ModelRegistryService, RuntimeConfigService};

use super::RuntimeSdkBridge;

const CANONICAL_DEFAULTS: &[(&str, &str, &str, &str)] = &[
    (
        "conversation",
        "anthropic",
        "claude-sonnet-4-5",
        "conversation",
    ),
    ("responses", "openai", "gpt-5.4", "responses"),
    ("fast", "openai", "gpt-5.4-mini", "responses"),
];

const CANONICAL_MODEL_ALIASES: &[(&str, &str)] = &[
    ("opus", "claude-opus-4-6"),
    ("sonnet", "claude-sonnet-4-5"),
    ("haiku", "claude-haiku-4-5-20251213"),
    ("grok", "grok-3"),
    ("grok-3", "grok-3"),
    ("grok-mini", "grok-3-mini"),
    ("grok-3-mini", "grok-3-mini"),
    ("grok-2", "grok-2"),
];

fn builtin_surface(
    surface: &str,
    protocol_family: &str,
    execution_class: RuntimeExecutionClass,
) -> ModelSurfaceBinding {
    ModelSurfaceBinding {
        surface: surface.into(),
        protocol_family: protocol_family.into(),
        enabled: execution_class != RuntimeExecutionClass::Unsupported,
        execution_profile: RuntimeExecutionProfile {
            execution_class,
            tool_loop: execution_class == RuntimeExecutionClass::AgentConversation,
            upstream_streaming: execution_class == RuntimeExecutionClass::AgentConversation,
        },
    }
}

fn provider_surface(surface: &str, protocol_family: &str, base_url: &str) -> SurfaceDescriptor {
    SurfaceDescriptor {
        surface: surface.into(),
        protocol_family: protocol_family.into(),
        transport: vec!["https".into()],
        auth_strategy: "x_api_key".into(),
        base_url: base_url.into(),
        base_url_policy: "allow_override".into(),
        enabled: true,
        capabilities: Vec::new(),
        execution_profile: RuntimeExecutionProfile {
            execution_class: match surface {
                "responses" => RuntimeExecutionClass::SingleShotGeneration,
                _ => RuntimeExecutionClass::AgentConversation,
            },
            tool_loop: surface != "responses",
            upstream_streaming: true,
        },
    }
}

fn builtin_provider(provider_id: &str) -> ProviderRegistryRecord {
    match provider_id {
        "anthropic" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "Anthropic".into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "anthropic_messages",
                "https://api.anthropic.com",
            )],
            metadata: Value::Object(Map::new()),
        },
        "openai" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "OpenAI".into(),
            enabled: true,
            surfaces: vec![
                provider_surface("conversation", "openai_chat", "https://api.openai.com/v1"),
                provider_surface("responses", "openai_responses", "https://api.openai.com/v1"),
            ],
            metadata: Value::Object(Map::new()),
        },
        "google" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "Google".into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "gemini_native",
                "https://generativelanguage.googleapis.com",
            )],
            metadata: Value::Object(Map::new()),
        },
        "minimax" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "MiniMax".into(),
            enabled: true,
            surfaces: vec![
                provider_surface(
                    "conversation",
                    "anthropic_messages",
                    "https://api.minimax.chat",
                ),
                provider_surface("conversation", "openai_chat", "https://api.minimax.chat"),
                provider_surface("conversation", "vendor_native", "https://api.minimax.chat"),
            ],
            metadata: Value::Object(Map::new()),
        },
        "xai" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "xAI".into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "openai_chat",
                "https://api.x.ai/v1",
            )],
            metadata: Value::Object(Map::new()),
        },
        "custom" => ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: "Custom".into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "openai_chat",
                "https://api.example.com/v1",
            )],
            metadata: Value::Object(Map::new()),
        },
        other => ProviderRegistryRecord {
            provider_id: other.into(),
            label: other.into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "openai_chat",
                "https://api.example.com/v1",
            )],
            metadata: Value::Object(Map::new()),
        },
    }
}

fn builtin_model(model_id: &str, provider_id: &str) -> ModelRegistryRecord {
    match model_id {
        "claude-sonnet-4-5" => model_record(
            model_id,
            provider_id,
            "Claude Sonnet 4.5",
            "Balanced Anthropic conversation model.",
            vec![builtin_surface(
                "conversation",
                "anthropic_messages",
                RuntimeExecutionClass::AgentConversation,
            )],
            Some(200_000),
            Some(8_192),
        ),
        "claude-opus-4-6" => model_record(
            model_id,
            provider_id,
            "Claude Opus 4.6",
            "High-capability Anthropic conversation model.",
            vec![builtin_surface(
                "conversation",
                "anthropic_messages",
                RuntimeExecutionClass::AgentConversation,
            )],
            Some(200_000),
            Some(8_192),
        ),
        "claude-haiku-4-5-20251213" => model_record(
            model_id,
            provider_id,
            "Claude Haiku 4.5",
            "Fast Anthropic conversation model.",
            vec![builtin_surface(
                "conversation",
                "anthropic_messages",
                RuntimeExecutionClass::AgentConversation,
            )],
            Some(200_000),
            Some(8_192),
        ),
        "gpt-5.4" => model_record(
            model_id,
            provider_id,
            "GPT-5.4",
            "Primary OpenAI Responses model.",
            vec![builtin_surface(
                "responses",
                "openai_responses",
                RuntimeExecutionClass::SingleShotGeneration,
            )],
            Some(200_000),
            Some(8_192),
        ),
        "gpt-5.4-mini" => model_record(
            model_id,
            provider_id,
            "GPT-5.4 Mini",
            "Fast OpenAI Responses model.",
            vec![builtin_surface(
                "responses",
                "openai_responses",
                RuntimeExecutionClass::SingleShotGeneration,
            )],
            Some(128_000),
            Some(8_192),
        ),
        "gpt-4o" => model_record(
            model_id,
            provider_id,
            "GPT-4o",
            "OpenAI conversation model.",
            vec![builtin_surface(
                "conversation",
                "openai_chat",
                RuntimeExecutionClass::AgentConversation,
            )],
            Some(128_000),
            Some(8_192),
        ),
        "gemini-2.5-flash" => model_record(
            model_id,
            provider_id,
            "Gemini 2.5 Flash",
            "Primary Gemini single-shot generation model.",
            vec![builtin_surface(
                "conversation",
                "gemini_native",
                RuntimeExecutionClass::SingleShotGeneration,
            )],
            Some(128_000),
            Some(8_192),
        ),
        "MiniMax-M2.7" => model_record(
            model_id,
            provider_id,
            "MiniMax M2.7",
            "Primary MiniMax conversation model.",
            vec![
                builtin_surface(
                    "conversation",
                    "anthropic_messages",
                    RuntimeExecutionClass::AgentConversation,
                ),
                builtin_surface(
                    "conversation",
                    "vendor_native",
                    RuntimeExecutionClass::Unsupported,
                ),
                builtin_surface(
                    "conversation",
                    "openai_chat",
                    RuntimeExecutionClass::AgentConversation,
                ),
            ],
            Some(128_000),
            Some(8_192),
        ),
        "grok-3" => model_record(
            model_id,
            provider_id,
            "Grok 3",
            "xAI conversation model.",
            vec![builtin_surface(
                "conversation",
                "openai_chat",
                RuntimeExecutionClass::AgentConversation,
            )],
            Some(128_000),
            Some(8_192),
        ),
        other => model_record(
            other,
            provider_id,
            other,
            "Workspace-configured model.",
            vec![builtin_surface(
                "conversation",
                default_protocol_family(provider_id, other, "conversation"),
                RuntimeExecutionClass::AgentConversation,
            )],
            None,
            None,
        ),
    }
}

fn model_record(
    model_id: &str,
    provider_id: &str,
    label: &str,
    description: &str,
    surface_bindings: Vec<ModelSurfaceBinding>,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
) -> ModelRegistryRecord {
    ModelRegistryRecord {
        model_id: model_id.into(),
        provider_id: provider_id.into(),
        label: label.into(),
        description: description.into(),
        family: provider_id.into(),
        track: "stable".into(),
        enabled: true,
        recommended_for: "general".into(),
        availability: "ga".into(),
        default_permission: "default".into(),
        surface_bindings,
        capabilities: vec![
            CapabilityDescriptor {
                capability_id: "streaming".into(),
                label: "Streaming".into(),
            },
            CapabilityDescriptor {
                capability_id: "tool_calling".into(),
                label: "Tool Calling".into(),
            },
        ],
        context_window,
        max_output_tokens,
        metadata: Value::Object(Map::new()),
    }
}

fn default_protocol_family(provider_id: &str, model_id: &str, surface: &str) -> &'static str {
    match (provider_id, model_id, surface) {
        ("anthropic", _, _) | ("minimax", "MiniMax-M2.7", _) => "anthropic_messages",
        ("google", _, _) => "gemini_native",
        ("openai", _, "responses") => "openai_responses",
        _ => "openai_chat",
    }
}

fn infer_surface_bindings(
    provider_id: &str,
    model_id: &str,
    raw: Option<&Value>,
) -> Vec<ModelSurfaceBinding> {
    let Some(raw) = raw else {
        return builtin_model(model_id, provider_id).surface_bindings;
    };
    let Some(items) = raw.as_array() else {
        return builtin_model(model_id, provider_id).surface_bindings;
    };

    let mut bindings = Vec::new();
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        let surface = object
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("conversation");
        let protocol_family = object
            .get("protocolFamily")
            .and_then(Value::as_str)
            .unwrap_or(default_protocol_family(provider_id, model_id, surface));
        let execution_profile = object
            .get("executionProfile")
            .and_then(Value::as_object)
            .map(|profile| RuntimeExecutionProfile {
                execution_class: match profile
                    .get("executionClass")
                    .and_then(Value::as_str)
                    .unwrap_or("unsupported")
                {
                    "agent_conversation" => RuntimeExecutionClass::AgentConversation,
                    "single_shot_generation" => RuntimeExecutionClass::SingleShotGeneration,
                    _ => RuntimeExecutionClass::Unsupported,
                },
                tool_loop: profile
                    .get("toolLoop")
                    .and_then(Value::as_bool)
                    .unwrap_or(surface != "responses"),
                upstream_streaming: profile
                    .get("upstreamStreaming")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            })
            .unwrap_or(RuntimeExecutionProfile {
                execution_class: if surface == "responses" {
                    RuntimeExecutionClass::SingleShotGeneration
                } else {
                    RuntimeExecutionClass::AgentConversation
                },
                tool_loop: surface != "responses",
                upstream_streaming: true,
            });
        bindings.push(ModelSurfaceBinding {
            surface: surface.into(),
            protocol_family: protocol_family.into(),
            enabled: object
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            execution_profile,
        });
    }

    if bindings.is_empty() {
        builtin_model(model_id, provider_id).surface_bindings
    } else {
        bindings
    }
}

fn configured_model_status(
    credential_ref: Option<&str>,
    enabled: bool,
    bridge: &RuntimeSdkBridge,
) -> String {
    if !enabled {
        return "disabled".into();
    }
    let Some(reference) = credential_ref.filter(|value| !value.trim().is_empty()) else {
        return "missing_credentials".into();
    };
    if reference.starts_with("env:") {
        return if std::env::var_os(reference.trim_start_matches("env:")).is_some() {
            "configured".into()
        } else {
            "missing_credentials".into()
        };
    }
    if reference.starts_with("secret-ref:") {
        return match bridge.state.secret_vault.get_optional_bytes(reference) {
            Ok(Some(_)) => "configured".into(),
            _ => "missing_credentials".into(),
        };
    }
    "configured".into()
}

fn token_usage_summary(
    budget_policy: Option<&ConfiguredModelBudgetPolicy>,
    used_tokens: u64,
) -> ConfiguredModelTokenUsage {
    let remaining_tokens = budget_policy
        .and_then(|policy| policy.total_budget_tokens)
        .map(|limit| limit.saturating_sub(used_tokens));
    let exhausted = remaining_tokens == Some(0) && budget_policy.is_some();

    ConfiguredModelTokenUsage {
        used_tokens,
        remaining_tokens,
        exhausted,
    }
}

fn canonical_model_id(model_id: &str) -> String {
    let normalized = model_id.trim().to_ascii_lowercase();
    CANONICAL_MODEL_ALIASES
        .iter()
        .find(|(alias, _)| alias == &normalized)
        .map(|(_, canonical)| (*canonical).to_string())
        .unwrap_or_else(|| model_id.trim().to_string())
}

fn parse_provider_overrides(
    effective_config: &Value,
    providers: &mut BTreeMap<String, ProviderRegistryRecord>,
) {
    for path in [
        effective_config.get("providerOverrides"),
        effective_config
            .get("modelRegistry")
            .and_then(|registry| registry.get("providers")),
    ] {
        let Some(entries) = path.and_then(Value::as_object) else {
            continue;
        };
        for (provider_id, entry) in entries {
            let Some(object) = entry.as_object() else {
                continue;
            };
            let label = object
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(provider_id);
            let surfaces = object
                .get("surfaces")
                .and_then(Value::as_array)
                .map(|surfaces| {
                    surfaces
                        .iter()
                        .filter_map(|surface| {
                            let object = surface.as_object()?;
                            Some(provider_surface(
                                object
                                    .get("surface")
                                    .and_then(Value::as_str)
                                    .unwrap_or("conversation"),
                                object
                                    .get("protocolFamily")
                                    .and_then(Value::as_str)
                                    .unwrap_or("openai_chat"),
                                object
                                    .get("baseUrl")
                                    .and_then(Value::as_str)
                                    .unwrap_or("https://api.example.com/v1"),
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|surfaces| !surfaces.is_empty())
                .unwrap_or_else(|| builtin_provider(provider_id).surfaces);

            providers.insert(
                provider_id.clone(),
                ProviderRegistryRecord {
                    provider_id: provider_id.clone(),
                    label: label.into(),
                    enabled: object
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    surfaces,
                    metadata: Value::Object(Map::new()),
                },
            );
        }
    }
}

fn parse_model_overrides(
    effective_config: &Value,
    models: &mut BTreeMap<String, ModelRegistryRecord>,
) {
    let Some(entries) = effective_config
        .get("modelRegistry")
        .and_then(|registry| registry.get("models"))
        .and_then(Value::as_object)
    else {
        return;
    };

    for (model_id, entry) in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let provider_id = object
            .get("providerId")
            .and_then(Value::as_str)
            .unwrap_or("custom");
        let label = object
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or(model_id);
        let description = object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Workspace-defined model.");
        models.insert(
            model_id.clone(),
            ModelRegistryRecord {
                label: label.into(),
                description: description.into(),
                surface_bindings: infer_surface_bindings(
                    provider_id,
                    model_id,
                    object.get("surfaceBindings"),
                ),
                provider_id: provider_id.into(),
                model_id: model_id.clone(),
                family: object
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or(provider_id)
                    .into(),
                track: object
                    .get("track")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .into(),
                enabled: object
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                recommended_for: object
                    .get("recommendedFor")
                    .and_then(Value::as_str)
                    .unwrap_or("general")
                    .into(),
                availability: object
                    .get("availability")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .into(),
                default_permission: object
                    .get("defaultPermission")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .into(),
                capabilities: Vec::new(),
                context_window: object
                    .get("contextWindow")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                max_output_tokens: object
                    .get("maxOutputTokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                metadata: Value::Object(Map::new()),
            },
        );
    }
}

fn parse_budget_policy(value: Option<&Value>) -> Option<ConfiguredModelBudgetPolicy> {
    serde_json::from_value(value?.clone()).ok()
}

fn build_default_selections(
    effective_config: &Value,
    configured_model_ids: &BTreeSet<String>,
) -> BTreeMap<String, DefaultSelection> {
    let mut selections = BTreeMap::new();
    for (purpose, provider_id, model_id, surface) in CANONICAL_DEFAULTS {
        let configured_model_id = configured_model_ids
            .get(&canonical_model_id(model_id))
            .cloned()
            .or_else(|| configured_model_ids.get(*model_id).cloned())
            .or_else(|| Some((*model_id).to_string()));
        selections.insert(
            (*purpose).to_string(),
            DefaultSelection {
                configured_model_id,
                provider_id: (*provider_id).to_string(),
                model_id: (*model_id).to_string(),
                surface: (*surface).to_string(),
            },
        );
    }

    if let Some(overrides) = effective_config
        .get("defaultSelections")
        .and_then(Value::as_object)
    {
        for (purpose, entry) in overrides {
            let Some(object) = entry.as_object() else {
                continue;
            };
            selections.insert(
                purpose.clone(),
                DefaultSelection {
                    configured_model_id: object
                        .get("configuredModelId")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    provider_id: object
                        .get("providerId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    model_id: object
                        .get("modelId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    surface: object
                        .get("surface")
                        .and_then(Value::as_str)
                        .unwrap_or("conversation")
                        .to_string(),
                },
            );
        }
    }

    selections
}

pub(crate) fn load_configured_model_usage_map(
    bridge: &RuntimeSdkBridge,
) -> Result<HashMap<String, u64>, AppError> {
    let connection = match Connection::open(&bridge.state.paths.db_path) {
        Ok(connection) => connection,
        Err(error) => return Err(AppError::database(error.to_string())),
    };
    let mut statement = match connection.prepare(
        "SELECT configured_model_id, settled_tokens
         FROM configured_model_budget_projections",
    ) {
        Ok(statement) => statement,
        Err(error) if error.to_string().contains("no such table") => return Ok(HashMap::new()),
        Err(error) => return Err(AppError::database(error.to_string())),
    };
    let rows = statement
        .query_map([], |row| {
            let configured_model_id: String = row.get(0)?;
            let settled_tokens: i64 = row.get(1)?;
            Ok((configured_model_id, settled_tokens))
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    let mut usage = HashMap::new();
    for row in rows {
        let (configured_model_id, settled_tokens) =
            row.map_err(|error| AppError::database(error.to_string()))?;
        usage.insert(configured_model_id, settled_tokens.max(0) as u64);
    }
    Ok(usage)
}

pub(crate) fn build_catalog_snapshot(
    bridge: &RuntimeSdkBridge,
    effective_config: &Value,
) -> Result<ModelCatalogSnapshot, AppError> {
    let mut providers = BTreeMap::new();
    let mut models = BTreeMap::new();
    for provider_id in ["anthropic", "openai", "google", "minimax", "xai", "custom"] {
        providers.insert(provider_id.to_string(), builtin_provider(provider_id));
    }
    for (model_id, provider_id) in [
        ("claude-sonnet-4-5", "anthropic"),
        ("claude-opus-4-6", "anthropic"),
        ("claude-haiku-4-5-20251213", "anthropic"),
        ("gpt-5.4", "openai"),
        ("gpt-5.4-mini", "openai"),
        ("gpt-4o", "openai"),
        ("gemini-2.5-flash", "google"),
        ("MiniMax-M2.7", "minimax"),
        ("grok-3", "xai"),
    ] {
        models.insert(model_id.to_string(), builtin_model(model_id, provider_id));
    }

    parse_provider_overrides(effective_config, &mut providers);
    parse_model_overrides(effective_config, &mut models);

    let usage = load_configured_model_usage_map(bridge)?;
    let mut diagnostics = ModelRegistryDiagnostics {
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let configured_models_value = effective_config
        .get("configuredModels")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut configured_models = Vec::new();
    let mut credential_bindings = Vec::new();
    let mut configured_model_ids = BTreeSet::new();

    if configured_models_value.is_empty() {
        for (_purpose, provider_id, model_id, _) in CANONICAL_DEFAULTS {
            let record = ConfiguredModelRecord {
                configured_model_id: (*model_id).to_string(),
                name: builtin_model(model_id, provider_id).label,
                provider_id: (*provider_id).to_string(),
                model_id: (*model_id).to_string(),
                credential_ref: None,
                base_url: None,
                budget_policy: None,
                token_usage: token_usage_summary(None, 0),
                enabled: true,
                source: "builtin".into(),
                status: "unconfigured".into(),
                configured: false,
            };
            if configured_model_ids.insert(record.configured_model_id.clone()) {
                configured_models.push(record);
            }
        }
    } else {
        for (configured_model_id, entry) in configured_models_value {
            let Some(object) = entry.as_object() else {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` must be a JSON object"
                ));
                continue;
            };
            let provider_id = object
                .get("providerId")
                .and_then(Value::as_str)
                .unwrap_or("custom");
            let model_id = object
                .get("modelId")
                .and_then(Value::as_str)
                .unwrap_or(configured_model_id.as_str());
            let canonical_id = canonical_model_id(model_id);
            let final_model_id = if models.contains_key(&canonical_id) {
                canonical_id
            } else {
                model_id.to_string()
            };

            providers
                .entry(provider_id.to_string())
                .or_insert_with(|| builtin_provider(provider_id));
            models
                .entry(final_model_id.clone())
                .or_insert_with(|| builtin_model(&final_model_id, provider_id));

            let credential_ref = object
                .get("credentialRef")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if let Some(reference) = credential_ref.clone() {
                let provider_label = providers
                    .get(provider_id)
                    .map(|provider| provider.label.clone())
                    .unwrap_or_else(|| provider_id.to_string());
                credential_bindings.push(CredentialBinding {
                    credential_ref: reference.clone(),
                    provider_id: provider_id.to_string(),
                    label: format!("{provider_label} credential"),
                    base_url: object
                        .get("baseUrl")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    status: configured_model_status(Some(reference.as_str()), true, bridge),
                    configured: true,
                    source: object
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or("workspace")
                        .to_string(),
                });
            }

            configured_model_ids.insert(configured_model_id.clone());
            configured_models.push(ConfiguredModelRecord {
                configured_model_id: configured_model_id.clone(),
                name: object
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(configured_model_id.as_str())
                    .to_string(),
                provider_id: provider_id.to_string(),
                model_id: final_model_id.clone(),
                credential_ref: credential_ref.clone(),
                base_url: object
                    .get("baseUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                budget_policy: parse_budget_policy(object.get("budgetPolicy")),
                token_usage: token_usage_summary(
                    parse_budget_policy(object.get("budgetPolicy")).as_ref(),
                    usage.get(&configured_model_id).copied().unwrap_or(0),
                ),
                enabled: object
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                source: object
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .to_string(),
                status: configured_model_status(
                    credential_ref.as_deref(),
                    object
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    bridge,
                ),
                configured: credential_ref.is_some(),
            });
        }
    }

    let default_selections = build_default_selections(effective_config, &configured_model_ids);

    Ok(ModelCatalogSnapshot {
        providers: providers.into_values().collect(),
        models: models.into_values().collect(),
        configured_models,
        credential_bindings,
        default_selections,
        diagnostics,
    })
}

#[async_trait]
impl ModelRegistryService for RuntimeSdkBridge {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError> {
        let effective = self.get_config().await?;
        build_catalog_snapshot(self, &effective.effective_config)
    }
}
