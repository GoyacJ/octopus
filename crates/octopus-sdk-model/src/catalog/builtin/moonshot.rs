use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("moonshot".to_string()),
        display_name: "Moonshot".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("moonshot.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("moonshot.conversation".to_string()),
        provider_id: ProviderId("moonshot".to_string()),
        protocol: ProtocolFamily::OpenAiChat,
        base_url: "https://api.moonshot.cn/v1".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![
        Model {
            id: ModelId("kimi-k2.5".to_string()),
            surface: SurfaceId("moonshot.conversation".to_string()),
            family: "kimi-k2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["kimi".to_string()],
        },
        Model {
            id: ModelId("kimi-k2-thinking".to_string()),
            surface: SurfaceId("moonshot.conversation".to_string()),
            family: "kimi-k2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["kimi-thinking".to_string()],
        },
        Model {
            id: ModelId("kimi-k2-thinking-turbo".to_string()),
            surface: SurfaceId("moonshot.conversation".to_string()),
            family: "kimi-k2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["kimi-fast".to_string()],
        },
        Model {
            id: ModelId("kimi-k2-0905-preview".to_string()),
            surface: SurfaceId("moonshot.conversation".to_string()),
            family: "kimi-k2".to_string(),
            track: ModelTrack::Preview,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec![],
        },
    ]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("kimi", "kimi-k2.5"),
        ("kimi-thinking", "kimi-k2-thinking"),
        ("kimi-fast", "kimi-k2-thinking-turbo"),
    ]
}
