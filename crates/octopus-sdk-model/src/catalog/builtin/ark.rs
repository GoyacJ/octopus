use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("ark".to_string()),
        display_name: "Ark".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("ark.responses".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("ark.responses".to_string()),
        provider_id: ProviderId("ark".to_string()),
        protocol: ProtocolFamily::OpenAiResponses,
        base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
        auth: AuthKind::ApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    vec![Model {
        id: ModelId("doubao-seed-1.6".to_string()),
        surface: SurfaceId("ark.responses".to_string()),
        family: "doubao-seed-1.6".to_string(),
        track: ModelTrack::Stable,
        context_window: ContextWindow {
            max_input_tokens: 128_000,
            max_output_tokens: 16_384,
            supports_1m: false,
        },
        aliases: vec!["doubao".to_string()],
    }]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![("doubao", "doubao-seed-1.6")]
}
