//! Provider, surface, and model data structs land here in Task 2.

use serde::{Deserialize, Serialize};

use crate::{AuthKind, ModelId, ModelTrack, ProtocolFamily, ProviderId, SurfaceId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Active,
    Deprecated,
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub display_name: String,
    pub status: ProviderStatus,
    pub auth: AuthKind,
    pub surfaces: Vec<SurfaceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    pub id: SurfaceId,
    pub provider_id: ProviderId,
    pub protocol: ProtocolFamily,
    pub base_url: String,
    pub auth: AuthKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub surface: SurfaceId,
    pub family: String,
    pub track: ModelTrack,
    pub context_window: ContextWindow,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextWindow {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub supports_1m: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub supported_families: Vec<ProtocolFamily>,
    pub catalog_version: String,
}

#[cfg(test)]
mod tests {
    use serde_json::{from_str, json, to_string, to_value};

    use super::{ContextWindow, Model, Provider, ProviderStatus, Surface};
    use crate::{AuthKind, ModelId, ModelTrack, ProtocolFamily, ProviderId, SurfaceId};

    #[test]
    fn provider_round_trip() {
        let provider = Provider {
            id: ProviderId("anthropic".to_string()),
            display_name: "Anthropic".to_string(),
            status: ProviderStatus::Active,
            auth: AuthKind::ApiKey,
            surfaces: vec![SurfaceId("conversation".to_string())],
        };

        let surface = Surface {
            id: SurfaceId("conversation".to_string()),
            provider_id: ProviderId("anthropic".to_string()),
            protocol: ProtocolFamily::AnthropicMessages,
            base_url: "https://api.anthropic.com".to_string(),
            auth: AuthKind::ApiKey,
        };

        let model = Model {
            id: ModelId("claude-opus-4-6".to_string()),
            surface: SurfaceId("conversation".to_string()),
            family: "claude-opus".to_string(),
            track: ModelTrack::Stable,
            context_window: ContextWindow {
                max_input_tokens: 200_000,
                max_output_tokens: 32_000,
                supports_1m: true,
            },
            aliases: vec!["opus".to_string()],
        };

        assert_eq!(to_string(&provider).unwrap(), "{\"id\":\"anthropic\",\"display_name\":\"Anthropic\",\"status\":\"active\",\"auth\":\"api_key\",\"surfaces\":[\"conversation\"]}");
        let decoded_provider: Provider = from_str(
            "{\"id\":\"anthropic\",\"display_name\":\"Anthropic\",\"status\":\"active\",\"auth\":\"api_key\",\"surfaces\":[\"conversation\"]}",
        )
        .unwrap();

        assert_eq!(decoded_provider, provider);
        assert_eq!(
            to_value(&surface).unwrap()["protocol"],
            json!("anthropic_messages")
        );
        assert_eq!(to_value(&model).unwrap()["track"], json!("stable"));
        assert_eq!(
            to_value(&model).unwrap()["context_window"]["supports_1m"],
            json!(true)
        );
    }
}
