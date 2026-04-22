use octopus_core::{
    CapabilityDescriptor, ConfiguredModelBudgetPolicy, ConfiguredModelTokenUsage,
    ModelRegistryRecord, ModelSurfaceBinding, ProviderRegistryRecord, RuntimeExecutionClass,
    RuntimeExecutionProfile, SurfaceDescriptor,
};
use serde_json::{Map, Value};

use crate::runtime_sdk::RuntimeSdkBridge;

pub(crate) const CANONICAL_DEFAULTS: &[(&str, &str, &str, &str)] = &[
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

pub(crate) fn provider_surface(
    surface: &str,
    protocol_family: &str,
    base_url: &str,
) -> SurfaceDescriptor {
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

pub(crate) fn builtin_provider(provider_id: &str) -> ProviderRegistryRecord {
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

pub(crate) fn builtin_model(model_id: &str, provider_id: &str) -> ModelRegistryRecord {
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

pub(crate) fn infer_surface_bindings(
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
    let normalized = model_id.trim().to_ascii_lowercase();
    CANONICAL_MODEL_ALIASES
        .iter()
        .find(|(alias, _)| alias == &normalized)
        .map(|(_, canonical)| (*canonical).to_string())
        .unwrap_or_else(|| model_id.trim().to_string())
}
