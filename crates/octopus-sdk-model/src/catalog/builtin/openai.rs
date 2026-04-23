use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("openai".to_string()),
        display_name: "OpenAI".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![
            SurfaceId("openai.conversation".to_string()),
            SurfaceId("openai.responses".to_string()),
        ],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![
        Surface {
            id: SurfaceId("openai.conversation".to_string()),
            provider_id: ProviderId("openai".to_string()),
            protocol: ProtocolFamily::OpenAiChat,
            base_url: "https://api.openai.com/v1".to_string(),
            auth: AuthKind::ApiKey,
        },
        Surface {
            id: SurfaceId("openai.responses".to_string()),
            provider_id: ProviderId("openai".to_string()),
            protocol: ProtocolFamily::OpenAiResponses,
            base_url: "https://api.openai.com/v1".to_string(),
            auth: AuthKind::ApiKey,
        },
    ]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![
        Model {
            id: ModelId("gpt-5.4".to_string()),
            surface: SurfaceId("openai.responses".to_string()),
            family: "gpt-5.4".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 32_000,
                supports_1m: false,
            },
            aliases: vec!["gpt-5".to_string()],
        },
        Model {
            id: ModelId("gpt-5.4-mini".to_string()),
            surface: SurfaceId("openai.responses".to_string()),
            family: "gpt-5.4".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 128_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["gpt-5-mini".to_string()],
        },
        Model {
            id: ModelId("gpt-5.4-nano".to_string()),
            surface: SurfaceId("openai.responses".to_string()),
            family: "gpt-5.4".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 128_000,
                max_output_tokens: 8_192,
                supports_1m: false,
            },
            aliases: vec!["gpt-5-nano".to_string()],
        },
    ]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gpt-5", "gpt-5.4"),
        ("gpt-5-mini", "gpt-5.4-mini"),
        ("gpt-5-nano", "gpt-5.4-nano"),
    ]
}
