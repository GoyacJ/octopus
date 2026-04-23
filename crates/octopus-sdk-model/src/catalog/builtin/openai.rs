use crate::{
    AuthKind, Model, ProtocolFamily, Provider, ProviderId, ProviderStatus, Surface, SurfaceId,
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
    Vec::new()
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    Vec::new()
}
