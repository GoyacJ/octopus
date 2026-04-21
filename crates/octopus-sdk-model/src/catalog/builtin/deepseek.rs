use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("deepseek".to_string()),
        display_name: "DeepSeek".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("deepseek.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("deepseek.conversation".to_string()),
        provider_id: ProviderId("deepseek".to_string()),
        protocol: ProtocolFamily::OpenAiChat,
        base_url: "https://api.deepseek.com".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![Model {
        id: ModelId("deepseek-chat".to_string()),
        surface: SurfaceId("deepseek.conversation".to_string()),
        family: "deepseek-chat".to_string(),
        track: ModelTrack::LatestAlias,
        context_window: ContextWindow {
            max_input_tokens: 128_000,
            max_output_tokens: 8_192,
            supports_1m: false,
        },
        aliases: vec!["deepseek".to_string()],
    }]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![("deepseek", "deepseek-chat")]
}
