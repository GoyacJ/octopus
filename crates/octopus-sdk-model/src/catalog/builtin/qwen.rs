use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("qwen".to_string()),
        display_name: "Qwen".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("qwen.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("qwen.conversation".to_string()),
        provider_id: ProviderId("qwen".to_string()),
        protocol: ProtocolFamily::OpenAiChat,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![Model {
        id: ModelId("qwen3-coder-plus".to_string()),
        surface: SurfaceId("qwen.conversation".to_string()),
        family: "qwen3-coder".to_string(),
        track: ModelTrack::Stable,
        context_window: ContextWindow {
            max_input_tokens: 128_000,
            max_output_tokens: 8_192,
            supports_1m: false,
        },
        aliases: vec!["qwen-coder".to_string()],
    }]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![("qwen-coder", "qwen3-coder-plus")]
}
