use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("bigmodel".to_string()),
        display_name: "BigModel".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("bigmodel.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("bigmodel.conversation".to_string()),
        provider_id: ProviderId("bigmodel".to_string()),
        protocol: ProtocolFamily::OpenAiChat,
        base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![
        Model {
            id: ModelId("glm-5".to_string()),
            surface: SurfaceId("bigmodel.conversation".to_string()),
            family: "glm-5".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 128_000,
                max_output_tokens: 8_192,
                supports_1m: false,
            },
            aliases: vec!["glm".to_string()],
        },
        Model {
            id: ModelId("glm-5-turbo".to_string()),
            surface: SurfaceId("bigmodel.conversation".to_string()),
            family: "glm-5".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 128_000,
                max_output_tokens: 8_192,
                supports_1m: false,
            },
            aliases: vec!["glm-turbo".to_string()],
        },
        Model {
            id: ModelId("glm-4.7".to_string()),
            surface: SurfaceId("bigmodel.conversation".to_string()),
            family: "glm-4.x".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 128_000,
                max_output_tokens: 8_192,
                supports_1m: false,
            },
            aliases: vec![],
        },
    ]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![("glm", "glm-5"), ("glm-turbo", "glm-5-turbo")]
}
