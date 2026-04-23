use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("anthropic".to_string()),
        display_name: "Anthropic".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("anthropic.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("anthropic.conversation".to_string()),
        provider_id: ProviderId("anthropic".to_string()),
        protocol: ProtocolFamily::AnthropicMessages,
        base_url: "https://api.anthropic.com".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![
        Model {
            id: ModelId("claude-sonnet-4-5".to_string()),
            surface: SurfaceId("anthropic.conversation".to_string()),
            family: "claude-sonnet".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: true,
            },
            aliases: vec!["sonnet".to_string()],
        },
        Model {
            id: ModelId("claude-opus-4-6".to_string()),
            surface: SurfaceId("anthropic.conversation".to_string()),
            family: "claude-opus".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 32_000,
                supports_1m: true,
            },
            aliases: vec!["opus".to_string()],
        },
        Model {
            id: ModelId("claude-haiku-4-5-20251213".to_string()),
            surface: SurfaceId("anthropic.conversation".to_string()),
            family: "claude-haiku".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["haiku".to_string()],
        },
    ]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("sonnet", "claude-sonnet-4-5"),
        ("opus", "claude-opus-4-6"),
        ("haiku", "claude-haiku-4-5-20251213"),
    ]
}
