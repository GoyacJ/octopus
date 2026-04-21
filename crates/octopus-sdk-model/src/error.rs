//! Model error types land here in Task 3.

use thiserror::Error;

use crate::{AuthKind, ModelId, ProtocolFamily, ProviderId};

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("auth kind {kind} is not supported yet")]
    AuthUnsupported { kind: AuthKind },
    #[error("missing auth secret for provider {provider}")]
    AuthMissing { provider: ProviderId },
    #[error("upstream returned status {status}: {body_preview}")]
    UpstreamStatus { status: u16, body_preview: String },
    #[error("upstream request timed out")]
    UpstreamTimeout,
    #[error("upstream overloaded")]
    Overloaded { retry_after_ms: Option<u64> },
    #[error("prompt too long: estimated {estimated_tokens} tokens, max {max}")]
    PromptTooLong { estimated_tokens: u32, max: u32 },
    #[error("adapter for protocol family {family} is not implemented")]
    AdapterNotImplemented { family: ProtocolFamily },
    #[error("capability {capability} is not supported by model {model}")]
    CapabilityUnsupported { capability: String, model: ModelId },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("model {id} not found")]
    ModelNotFound { id: ModelId },
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ModelError;
    use crate::{AuthKind, ModelId, ProtocolFamily, ProviderId};

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn model_error_is_send_sync_and_displayable() {
        assert_send_sync::<ModelError>();

        let auth_error = ModelError::AuthUnsupported {
            kind: AuthKind::OAuth,
        };
        let missing_error = ModelError::AuthMissing {
            provider: ProviderId("anthropic".to_string()),
        };
        let adapter_error = ModelError::AdapterNotImplemented {
            family: ProtocolFamily::VendorNative,
        };
        let capability_error = ModelError::CapabilityUnsupported {
            capability: "tool_use".to_string(),
            model: ModelId("claude-opus-4-6".to_string()),
        };

        assert_eq!(auth_error.to_string(), "auth kind oauth is not supported yet");
        assert_eq!(missing_error.to_string(), "missing auth secret for provider anthropic");
        assert_eq!(
            adapter_error.to_string(),
            "adapter for protocol family vendor_native is not implemented"
        );
        assert_eq!(
            capability_error.to_string(),
            "capability tool_use is not supported by model claude-opus-4-6"
        );
    }
}
