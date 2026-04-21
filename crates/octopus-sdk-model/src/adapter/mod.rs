//! Protocol adapter traits and implementations land here in later W2 tasks.

use std::{io, pin::Pin};

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use reqwest::header::{HeaderName, HeaderValue};

use octopus_sdk_contracts::{PromptCacheEvent, SecretValue, SecretVault, StopReason, Usage};

use crate::{ModelError, ModelRequest, ModelStream, ProtocolFamily, Provider};

mod anthropic_messages;
mod openai_chat;
mod sse;
mod stubs;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use openai_chat::OpenAiChatAdapter;
pub use stubs::{GeminiNativeAdapter, OpenAiResponsesAdapter, VendorNativeAdapter};

pub type StreamBytes = Pin<Box<dyn Stream<Item = Result<Bytes, ModelError>> + Send>>;

#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    fn family(&self) -> ProtocolFamily;
    fn to_request(&self, req: &ModelRequest) -> Result<serde_json::Value, ModelError>;
    fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError>;
    async fn auth_headers(
        &self,
        vault: &dyn SecretVault,
        provider: &Provider,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError>;
}

pub(crate) fn json_error(message: impl Into<String>) -> ModelError {
    ModelError::Serialization(serde_json::Error::io(io::Error::new(
        io::ErrorKind::InvalidData,
        message.into(),
    )))
}

pub(crate) fn secret_to_string(secret: SecretValue) -> Result<String, ModelError> {
    String::from_utf8(secret.as_bytes().to_vec())
        .map_err(|_| json_error("secret contained invalid UTF-8"))
}

pub(crate) fn header_value_from_secret(secret: SecretValue) -> Result<HeaderValue, ModelError> {
    HeaderValue::from_str(&secret_to_string(secret)?)
        .map_err(|_| json_error("secret could not be encoded as an HTTP header"))
}

pub(crate) fn map_stop_reason(value: Option<&str>) -> StopReason {
    match value.unwrap_or("end_turn") {
        "tool_use" | "tool_calls" => StopReason::ToolUse,
        "max_tokens" | "length" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        _ => StopReason::EndTurn,
    }
}

pub(crate) fn prompt_cache_event(usage: &Usage) -> Option<PromptCacheEvent> {
    let breakpoint_count = u32::from(usage.cache_creation_input_tokens > 0)
        + u32::from(usage.cache_read_input_tokens > 0);
    (breakpoint_count > 0).then_some(PromptCacheEvent {
        cache_read_input_tokens: usage.cache_read_input_tokens,
        cache_creation_input_tokens: usage.cache_creation_input_tokens,
        breakpoint_count,
    })
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use bytes::Bytes;
    use futures::stream;
    use reqwest::header::{HeaderName, HeaderValue};

    use octopus_sdk_contracts::{SecretValue, SecretVault, VaultError};

    use super::{ProtocolAdapter, StreamBytes};
    use crate::{ModelError, ModelRequest, ModelStream, ProtocolFamily, Provider};

    struct NullVault;

    #[async_trait]
    impl SecretVault for NullVault {
        async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
            Ok(SecretValue::new("secret"))
        }

        async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
            Ok(())
        }
    }

    struct DummyAdapter;

    #[async_trait]
    impl ProtocolAdapter for DummyAdapter {
        fn family(&self) -> ProtocolFamily {
            ProtocolFamily::VendorNative
        }

        fn to_request(&self, _req: &ModelRequest) -> Result<serde_json::Value, ModelError> {
            Ok(serde_json::json!({"ok": true}))
        }

        fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError> {
            let _ = raw;
            Ok(Box::pin(stream::empty()))
        }

        async fn auth_headers(
            &self,
            _vault: &dyn SecretVault,
            _provider: &Provider,
        ) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError> {
            Ok(vec![(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_static("secret"),
            )])
        }
    }

    fn accepts_trait_object(_adapter: &dyn ProtocolAdapter) {}

    #[tokio::test]
    async fn protocol_adapter_is_object_safe_and_returns_auth_headers() {
        let adapter = DummyAdapter;
        let vault = NullVault;
        let raw: StreamBytes = Box::pin(stream::iter(vec![Ok(Bytes::from_static(b"data"))]));
        let provider = Provider {
            id: crate::ProviderId("vendor".to_string()),
            display_name: "Vendor".to_string(),
            status: crate::ProviderStatus::Experimental,
            auth: crate::AuthKind::XApiKey,
            surfaces: vec![],
        };

        accepts_trait_object(&adapter);
        assert_eq!(adapter.family(), ProtocolFamily::VendorNative);
        assert_eq!(
            adapter.auth_headers(&vault, &provider).await.unwrap().len(),
            1
        );
        assert!(adapter.parse_stream(raw).is_ok());
    }
}
