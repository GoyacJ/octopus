use crate::{
    AuthKind, Model, ProtocolFamily, Provider, ProviderId, ProviderStatus, Surface, SurfaceId,
};

#[must_use]
pub(crate) fn provider() -> Provider {
    Provider {
        id: ProviderId("minimax".to_string()),
        display_name: "MiniMax".to_string(),
        status: ProviderStatus::Active,
        auth: AuthKind::ApiKey,
        surfaces: vec![SurfaceId("minimax.conversation".to_string())],
    }
}

#[must_use]
pub(crate) fn surfaces() -> Vec<Surface> {
    vec![Surface {
        id: SurfaceId("minimax.conversation".to_string()),
        provider_id: ProviderId("minimax".to_string()),
        protocol: ProtocolFamily::VendorNative,
        base_url: "https://api.minimaxi.com".to_string(),
        auth: AuthKind::ApiKey,
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
