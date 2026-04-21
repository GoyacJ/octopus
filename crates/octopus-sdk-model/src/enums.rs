//! Model-layer enums land here in Task 2.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolFamily {
    AnthropicMessages,
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
    GeminiNative,
    VendorNative,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTrack {
    Preview,
    Stable,
    LatestAlias,
    Deprecated,
    Sunset,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    ApiKey,
    XApiKey,
    #[serde(rename = "oauth")]
    OAuth,
    #[serde(rename = "aws_sigv4")]
    AwsSigV4,
    #[serde(rename = "gcp_adc")]
    GcpAdc,
    AzureAd,
    None,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    Main,
    Fast,
    Best,
    Plan,
    Compact,
    Vision,
    WebExtract,
    Embedding,
    Eval,
    SubagentDefault,
}

impl fmt::Display for ProtocolFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::AnthropicMessages => "anthropic_messages",
            Self::OpenAiChat => "openai_chat",
            Self::OpenAiResponses => "openai_responses",
            Self::GeminiNative => "gemini_native",
            Self::VendorNative => "vendor_native",
        };

        f.write_str(value)
    }
}

impl fmt::Display for AuthKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::ApiKey => "api_key",
            Self::XApiKey => "x_api_key",
            Self::OAuth => "oauth",
            Self::AwsSigV4 => "aws_sigv4",
            Self::GcpAdc => "gcp_adc",
            Self::AzureAd => "azure_ad",
            Self::None => "none",
        };

        f.write_str(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::to_string;

    use super::{AuthKind, ModelRole, ModelTrack, ProtocolFamily};

    #[test]
    fn model_role_serializes_in_snake_case() {
        assert_eq!(to_string(&ModelRole::Main).unwrap(), "\"main\"");
        assert_eq!(
            to_string(&ModelRole::SubagentDefault).unwrap(),
            "\"subagent_default\""
        );
    }

    #[test]
    fn protocol_and_auth_enums_use_expected_contract_strings() {
        assert_eq!(
            to_string(&ProtocolFamily::AnthropicMessages).unwrap(),
            "\"anthropic_messages\""
        );
        assert_eq!(
            to_string(&ProtocolFamily::OpenAiChat).unwrap(),
            "\"openai_chat\""
        );
        assert_eq!(
            to_string(&ModelTrack::LatestAlias).unwrap(),
            "\"latest_alias\""
        );
        assert_eq!(to_string(&AuthKind::OAuth).unwrap(), "\"oauth\"");
        assert_eq!(to_string(&AuthKind::AwsSigV4).unwrap(), "\"aws_sigv4\"");
        assert_eq!(to_string(&AuthKind::GcpAdc).unwrap(), "\"gcp_adc\"");
    }
}
