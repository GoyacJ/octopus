use super::*;

pub(super) fn sorted_values<T, F>(records: &BTreeMap<String, T>, key_fn: F) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> String,
{
    let mut values = records.values().cloned().collect::<Vec<_>>();
    values.sort_by_key(|record| key_fn(record));
    values
}

pub(super) fn capability(capability_id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        capability_id: capability_id.into(),
        label: capability_id.replace('_', " "),
    }
}

pub(super) fn token_usage_summary(
    policy: Option<&ConfiguredModelBudgetPolicy>,
    used_tokens: u64,
) -> ConfiguredModelTokenUsage {
    let total_tokens = policy.and_then(|entry| entry.total_budget_tokens);
    ConfiguredModelTokenUsage {
        used_tokens,
        remaining_tokens: total_tokens.map(|total| total.saturating_sub(used_tokens)),
        exhausted: total_tokens.is_some_and(|total| used_tokens >= total),
    }
}

pub(super) fn binding(surface: &str, protocol_family: &str) -> ModelSurfaceBinding {
    ModelSurfaceBinding {
        surface: surface.into(),
        protocol_family: protocol_family.into(),
        enabled: true,
        execution_profile: RuntimeExecutionProfile::default(),
    }
}

pub(super) fn surface(
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
        execution_profile: RuntimeExecutionProfile::default(),
    }
}

pub(super) fn provider(
    provider_id: &str,
    label: &str,
    surfaces: Vec<SurfaceDescriptor>,
) -> ProviderRegistryRecord {
    ProviderRegistryRecord {
        provider_id: provider_id.into(),
        label: label.into(),
        enabled: true,
        surfaces,
        metadata: json!({}),
    }
}

pub(super) fn model(
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

pub(crate) fn baseline_providers() -> BTreeMap<String, ProviderRegistryRecord> {
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
                    "x_api_key",
                    "https://api.anthropic.com",
                    "allow_override",
                    &[
                        "streaming",
                        "tool_calling",
                        "structured_output",
                        "reasoning",
                    ],
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
                    &[
                        "streaming",
                        "tool_calling",
                        "structured_output",
                        "reasoning",
                    ],
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
                        &[
                            "streaming",
                            "tool_calling",
                            "structured_output",
                            "reasoning",
                        ],
                    ),
                    surface(
                        "conversation",
                        "anthropic_messages",
                        &["request_response", "sse"],
                        "x_api_key",
                        "https://api.deepseek.com/anthropic",
                        "allow_override",
                        &[
                            "streaming",
                            "tool_calling",
                            "structured_output",
                            "reasoning",
                        ],
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
                        "x_api_key",
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
                    &[
                        "streaming",
                        "tool_calling",
                        "structured_output",
                        "reasoning",
                    ],
                )],
            ),
        ),
        (
            "bigmodel".into(),
            provider(
                "bigmodel",
                "BigModel",
                vec![surface(
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
                )],
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
                        "x_api_key",
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
                vec![surface(
                    "responses",
                    "openai_responses",
                    &["request_response", "sse"],
                    "bearer",
                    "https://ark.cn-beijing.volces.com/api/v3",
                    "allow_override",
                    &[
                        "streaming",
                        "tool_calling",
                        "structured_output",
                        "files",
                        "context_cache",
                    ],
                )],
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
                        "api_key",
                        "https://generativelanguage.googleapis.com",
                        "allow_override",
                        &[
                            "streaming",
                            "tool_calling",
                            "structured_output",
                            "vision_input",
                            "web_search",
                        ],
                    ),
                    surface(
                        "realtime",
                        "gemini_native",
                        &["websocket"],
                        "api_key",
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

pub(super) fn baseline_models() -> BTreeMap<String, ModelRegistryRecord> {
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                    "files",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "vision_input",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "files",
                    "context_cache",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                    "files",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "reasoning",
                    "vision_input",
                    "web_search",
                ],
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
                &[
                    "streaming",
                    "tool_calling",
                    "structured_output",
                    "vision_input",
                ],
                None,
                None,
            ),
        ),
    ])
}

pub(super) fn baseline_default_selections() -> BTreeMap<String, DefaultSelection> {
    crate::model_runtime::CanonicalModelPolicy
        .default_selections()
        .iter()
        .map(|selection| {
            (
                selection.purpose.to_string(),
                DefaultSelection {
                    configured_model_id: Some(selection.model_id.to_string()),
                    provider_id: selection.provider_id.to_string(),
                    model_id: selection.model_id.to_string(),
                    surface: selection.surface.to_string(),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::baseline_providers;

    #[test]
    fn baseline_registry_contains_openai_provider() {
        let providers = baseline_providers();
        assert!(providers.contains_key("openai"));
    }
}
