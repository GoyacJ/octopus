use crate::{
    AuthKind, ContextWindow, Model, ModelId, ModelTrack, ProtocolFamily, Provider, ProviderId,
    ProviderStatus, Surface, SurfaceId,
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
    vec![
        Model {
            id: ModelId("gemini-2.5-pro".to_string()),
            surface: SurfaceId("google.conversation".to_string()),
            family: "gemini-2.5".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 1_000_000,
                max_output_tokens: 32_000,
                supports_1m: true,
            },
            aliases: vec!["gemini-pro".to_string()],
        },
        Model {
            id: ModelId("gemini-2.5-flash".to_string()),
            surface: SurfaceId("google.conversation".to_string()),
            family: "gemini-2.5".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 1_000_000,
                max_output_tokens: 16_384,
                supports_1m: false,
            },
            aliases: vec!["gemini-flash".to_string()],
        },
    ]
}

#[must_use]
pub(crate) fn aliases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gemini-pro", "gemini-2.5-pro"),
        ("gemini-flash", "gemini-2.5-flash"),
    ]
}
