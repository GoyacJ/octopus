use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("minimax".to_string()),
        display_name: "MiniMax".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![
            SurfaceId("minimax.conversation".to_string()),
            SurfaceId("minimax.openai".to_string()),
        ],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![
        Surface {
            id: SurfaceId("minimax.conversation".to_string()),
            provider_id: ProviderId("minimax".to_string()),
            protocol: ProtocolFamily::AnthropicMessages,
            base_url: "https://api.minimaxi.com/anthropic".to_string(),
            auth: AuthKind::ApiKey,
        },
        Surface {
            id: SurfaceId("minimax.openai".to_string()),
            provider_id: ProviderId("minimax".to_string()),
            protocol: ProtocolFamily::OpenAiChat,
            base_url: "https://api.minimaxi.com".to_string(),
            auth: AuthKind::ApiKey,
        },
    ]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![
        Model {
            id: ModelId("MiniMax-M2.7".to_string()),
            surface: SurfaceId("minimax.conversation".to_string()),
            family: "MiniMax-M2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["minimax".to_string()],
        },
        Model {
            id: ModelId("MiniMax-M2.5".to_string()),
            surface: SurfaceId("minimax.conversation".to_string()),
            family: "MiniMax-M2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec![],
        },
        Model {
            id: ModelId("MiniMax-M2.1".to_string()),
            surface: SurfaceId("minimax.conversation".to_string()),
            family: "MiniMax-M2".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec![],
        },
        Model {
            id: ModelId("MiniMax-M2".to_string()),
            surface: SurfaceId("minimax.conversation".to_string()),
            family: "MiniMax-M2".to_string(),
            track: ModelTrack::Stable,
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
    vec![("minimax", "MiniMax-M2.7")]
}
