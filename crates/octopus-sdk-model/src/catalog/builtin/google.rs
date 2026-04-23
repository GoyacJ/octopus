use crate::{
    AuthKind, Model, ProtocolFamily, Provider, ProviderId, ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("google".to_string()),
        display_name: "Google".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::XApiKey,
        surfaces: vec![SurfaceId("google.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("google.conversation".to_string()),
        provider_id: ProviderId("google".to_string()),
        protocol: ProtocolFamily::GeminiNative,
        base_url: "https://generativelanguage.googleapis.com".to_string(),
        auth: AuthKind::XApiKey,
    }]
}

#[must_use]
pub(crate) fn models() -> Vec<Model> {
    Vec::new()
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    Vec::new()
}
