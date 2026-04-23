use octopus_core::{
    CapabilityDescriptor, ConfiguredModelBudgetPolicy, ConfiguredModelTokenUsage,
    ModelRegistryRecord, ModelSurfaceBinding, ProviderRegistryRecord, RuntimeExecutionClass,
    RuntimeExecutionProfile, SurfaceDescriptor,
};
use octopus_sdk::{
    builtin_canonical_model_id, builtin_compat_model, builtin_compat_models, AuthKind, Model,
    ModelCatalog, ModelTrack, ProtocolFamily, Provider, Surface,
};
use serde_json::{Map, Value};

use crate::runtime_sdk::RuntimeSdkBridge;

fn live_catalog() -> ModelCatalog {
    ModelCatalog::new_builtin()
}

fn surface_name(surface_id: &str) -> String {
    surface_id
        .rsplit('.')
        .next()
        .unwrap_or(surface_id)
        .to_string()
}

fn runtime_execution_profile(
    protocol_family: &ProtocolFamily,
    enabled: bool,
) -> RuntimeExecutionProfile {
    if !enabled {
        return RuntimeExecutionProfile {
            execution_class: RuntimeExecutionClass::Unsupported,
            tool_loop: false,
            upstream_streaming: false,
        };
    }

    RuntimeExecutionProfile {
        execution_class: match protocol_family {
            ProtocolFamily::OpenAiResponses => RuntimeExecutionClass::SingleShotGeneration,
            _ => RuntimeExecutionClass::AgentConversation,
        },
        tool_loop: !matches!(protocol_family, ProtocolFamily::OpenAiResponses),
        upstream_streaming: true,
    }
}

fn provider_surface_descriptor(surface: &Surface, enabled: bool) -> SurfaceDescriptor {
    SurfaceDescriptor {
        surface: surface_name(&surface.id.0),
        protocol_family: surface.protocol.to_string(),
        transport: vec!["https".into()],
        auth_strategy: surface.auth.to_string(),
        base_url: surface.base_url.clone(),
        base_url_policy: "allow_override".into(),
        enabled,
        capabilities: Vec::new(),
        execution_profile: runtime_execution_profile(&surface.protocol, enabled),
    }
}

pub(crate) fn provider_surface(
    surface: &str,
    protocol_family: &str,
    base_url: &str,
) -> SurfaceDescriptor {
    SurfaceDescriptor {
        surface: surface.into(),
        protocol_family: protocol_family.into(),
        transport: vec!["https".into()],
        auth_strategy: AuthKind::ApiKey.to_string(),
        base_url: base_url.into(),
        base_url_policy: "allow_override".into(),
        enabled: true,
        capabilities: Vec::new(),
        execution_profile: RuntimeExecutionProfile {
            execution_class: if surface == "responses" {
                RuntimeExecutionClass::SingleShotGeneration
            } else {
                RuntimeExecutionClass::AgentConversation
            },
            tool_loop: surface != "responses",
            upstream_streaming: true,
        },
    }
}

fn provider_record(
    provider: &Provider,
    surfaces: Vec<SurfaceDescriptor>,
    enabled: bool,
) -> ProviderRegistryRecord {
    ProviderRegistryRecord {
        provider_id: provider.id.0.clone(),
        label: provider.display_name.clone(),
        enabled,
        surfaces,
        metadata: Value::Object(Map::new()),
    }
}

fn capabilities() -> Vec<CapabilityDescriptor> {
    vec![
        CapabilityDescriptor {
            capability_id: "streaming".into(),
            label: "Streaming".into(),
        },
        CapabilityDescriptor {
            capability_id: "tool_calling".into(),
            label: "Tool Calling".into(),
        },
    ]
}

fn model_recommended_for(model: &Model) -> &'static str {
    let model_id = model.id.0.as_str();
    if model_id.contains("haiku")
        || model_id.contains("flash")
        || model_id.contains("turbo")
        || model_id.contains("lite")
    {
        "fast"
    } else if model_id.contains("coder") {
        "coding"
    } else if model_id.contains("vl") || model.family.contains("vl") {
        "vision"
    } else {
        "general"
    }
}

fn project_model(
    provider: &Provider,
    surface: &Surface,
    model: &Model,
    enabled: bool,
    availability: &str,
    description: &str,
) -> ModelRegistryRecord {
    ModelRegistryRecord {
        model_id: model.id.0.clone(),
        provider_id: provider.id.0.clone(),
        label: model.id.0.clone(),
        description: description.into(),
        family: model.family.clone(),
        track: match model.track {
            ModelTrack::Preview => "preview",
            ModelTrack::Stable => "stable",
            ModelTrack::LatestAlias => "latest_alias",
            ModelTrack::Deprecated => "deprecated",
            ModelTrack::Sunset => "sunset",
        }
        .into(),
        enabled,
        recommended_for: model_recommended_for(model).into(),
        availability: availability.into(),
        default_permission: "default".into(),
        surface_bindings: vec![ModelSurfaceBinding {
            surface: surface_name(&surface.id.0),
            protocol_family: surface.protocol.to_string(),
            enabled,
            execution_profile: runtime_execution_profile(&surface.protocol, enabled),
        }],
        capabilities: capabilities(),
        context_window: Some(model.context_window.max_input_tokens),
        max_output_tokens: Some(model.context_window.max_output_tokens),
        metadata: Value::Object(Map::new()),
    }
}

fn live_provider_record(provider_id: &str) -> Option<ProviderRegistryRecord> {
    let catalog = live_catalog();
    let provider = catalog
        .list_providers()
        .iter()
        .find(|provider| provider.id.0 == provider_id)?;
    let surfaces = catalog
        .list_surfaces()
        .iter()
        .filter(|surface| surface.provider_id == provider.id)
        .map(|surface| provider_surface_descriptor(surface, true))
        .collect::<Vec<_>>();
    Some(provider_record(provider, surfaces, true))
}

fn live_model_record(model_id: &str) -> Option<ModelRegistryRecord> {
    let catalog = live_catalog();
    let model = catalog
        .list_models()
        .iter()
        .find(|model| model.id.0 == model_id)?;
    let surface = catalog
        .list_surfaces()
        .iter()
        .find(|surface| surface.id == model.surface)?;
    let provider = catalog
        .list_providers()
        .iter()
        .find(|provider| provider.id == surface.provider_id)?;
    Some(project_model(
        provider,
        surface,
        model,
        true,
        "ga",
        "Builtin live model derived from vendor matrix.",
    ))
}

fn compat_provider_record(provider_id: &str) -> Option<ProviderRegistryRecord> {
    let compat_models = builtin_compat_models()
        .into_iter()
        .filter(|entry| entry.provider.id.0 == provider_id)
        .collect::<Vec<_>>();
    let provider = compat_models.first()?.provider.clone();
    let mut surface_ids = std::collections::BTreeSet::new();
    let surfaces = compat_models
        .into_iter()
        .filter_map(|entry| {
            if surface_ids.insert(entry.surface.id.0.clone()) {
                Some(provider_surface_descriptor(&entry.surface, false))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    Some(provider_record(&provider, surfaces, false))
}

pub(crate) fn builtin_provider(provider_id: &str) -> ProviderRegistryRecord {
    live_provider_record(provider_id)
        .or_else(|| compat_provider_record(provider_id))
        .unwrap_or_else(|| ProviderRegistryRecord {
            provider_id: provider_id.into(),
            label: provider_id.into(),
            enabled: true,
            surfaces: vec![provider_surface(
                "conversation",
                "openai_chat",
                "https://api.example.com/v1",
            )],
            metadata: Value::Object(Map::new()),
        })
}

pub(crate) fn builtin_model(model_id: &str, provider_id: &str) -> ModelRegistryRecord {
    let canonical_id = builtin_canonical_model_id(model_id);

    live_model_record(&canonical_id)
        .filter(|record| record.provider_id == provider_id)
        .unwrap_or_else(|| ModelRegistryRecord {
            model_id: canonical_id.clone(),
            provider_id: provider_id.into(),
            label: canonical_id.clone(),
            description: "Workspace-configured model.".into(),
            family: provider_id.into(),
            track: "workspace".into(),
            enabled: true,
            recommended_for: "general".into(),
            availability: "workspace".into(),
            default_permission: "default".into(),
            surface_bindings: vec![ModelSurfaceBinding {
                surface: "conversation".into(),
                protocol_family: default_protocol_family(
                    provider_id,
                    &canonical_id,
                    "conversation",
                )
                .into(),
                enabled: true,
                execution_profile: RuntimeExecutionProfile {
                    execution_class: RuntimeExecutionClass::AgentConversation,
                    tool_loop: true,
                    upstream_streaming: true,
                },
            }],
            capabilities: capabilities(),
            context_window: None,
            max_output_tokens: None,
            metadata: Value::Object(Map::new()),
        })
}

pub(crate) fn hidden_builtin_model(
    model_id: &str,
    provider_id: &str,
) -> Option<ModelRegistryRecord> {
    // Hidden builtin metadata is only for configuredModels compat projection.
    // It must not become part of the live builtin catalog.
    let entry = builtin_compat_model(model_id)?;
    if entry.provider.id.0 != provider_id {
        return None;
    }

    Some(project_model(
        &entry.provider,
        &entry.surface,
        &entry.model,
        false,
        "unsupported",
        "Configured builtin kept as runtime-unsupported compat metadata.",
    ))
}

fn default_protocol_family(provider_id: &str, model_id: &str, surface: &str) -> &'static str {
    match (provider_id, model_id, surface) {
        ("anthropic", _, _) => "anthropic_messages",
        ("google", _, _) => "gemini_native",
        ("openai", _, "responses") => "openai_responses",
        _ => "openai_chat",
    }
}

pub(crate) fn infer_surface_bindings(
    provider_id: &str,
    model_id: &str,
    raw: Option<&Value>,
) -> Vec<ModelSurfaceBinding> {
    let fallback = builtin_model(model_id, provider_id).surface_bindings;
    let Some(raw) = raw else {
        return fallback;
    };
    let Some(items) = raw.as_array() else {
        return fallback;
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
        fallback
    } else {
        bindings
    }
}

pub(crate) fn configured_model_status(
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

pub(crate) fn token_usage_summary(
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

pub(crate) fn canonical_model_id(model_id: &str) -> String {
    builtin_canonical_model_id(model_id)
}
