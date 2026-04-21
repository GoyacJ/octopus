use async_trait::async_trait;
use reqwest::header::{HeaderName, HeaderValue};

use octopus_sdk_contracts::SecretVault;

use crate::{ModelError, ModelRequest, ModelStream, ProtocolAdapter, ProtocolFamily, Provider};

use super::{header_value_from_secret, StreamBytes};

#[derive(Debug, Default)]
pub struct OpenAiResponsesAdapter;

#[derive(Debug, Default)]
pub struct GeminiNativeAdapter;

#[derive(Debug, Default)]
pub struct VendorNativeAdapter;

macro_rules! impl_stub_adapter {
    ($name:ident, $family:expr) => {
        #[async_trait]
        impl ProtocolAdapter for $name {
            fn family(&self) -> ProtocolFamily {
                $family
            }

            fn to_request(&self, _req: &ModelRequest) -> Result<serde_json::Value, ModelError> {
                Err(ModelError::AdapterNotImplemented {
                    family: self.family(),
                })
            }

            fn parse_stream(&self, _raw: StreamBytes) -> Result<ModelStream, ModelError> {
                Err(ModelError::AdapterNotImplemented {
                    family: self.family(),
                })
            }

            async fn auth_headers(
                &self,
                vault: &dyn SecretVault,
                provider: &Provider,
            ) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError> {
                match &provider.auth {
                    crate::AuthKind::ApiKey => Ok(vec![(
                        HeaderName::from_static("authorization"),
                        HeaderValue::from_str(&format!(
                            "Bearer {}",
                            super::secret_to_string(
                                vault.get(&format!("{}_api_key", provider.id.0)).await.map_err(
                                    |_| ModelError::AuthMissing {
                                        provider: provider.id.clone(),
                                    },
                                )?,
                            )?,
                        ))
                        .map_err(|_| super::json_error("invalid bearer token"))?,
                    )]),
                    crate::AuthKind::XApiKey => Ok(vec![(
                        HeaderName::from_static("x-api-key"),
                        header_value_from_secret(
                            vault.get(&format!("{}_api_key", provider.id.0)).await.map_err(|_| {
                                ModelError::AuthMissing {
                                    provider: provider.id.clone(),
                                }
                            })?,
                        )?,
                    )]),
                    kind => Err(ModelError::AuthUnsupported { kind: kind.clone() }),
                }
            }
        }
    };
}

impl_stub_adapter!(OpenAiResponsesAdapter, ProtocolFamily::OpenAiResponses);
impl_stub_adapter!(GeminiNativeAdapter, ProtocolFamily::GeminiNative);
impl_stub_adapter!(VendorNativeAdapter, ProtocolFamily::VendorNative);

#[cfg(test)]
mod tests {
    use super::{GeminiNativeAdapter, OpenAiResponsesAdapter, VendorNativeAdapter};
    use crate::{
        CacheControlStrategy, ModelError, ModelId, ModelRequest, ModelRole, ProtocolAdapter,
    };

    fn request() -> ModelRequest {
        ModelRequest {
            model: ModelId("stub".to_string()),
            system_prompt: vec![],
            messages: vec![],
            tools: vec![],
            role: ModelRole::Main,
            cache_breakpoints: vec![],
            response_format: None,
            thinking: None,
            cache_control: CacheControlStrategy::None,
            max_tokens: None,
            temperature: None,
            stream: true,
        }
    }

    #[test]
    fn stubs_return_adapter_not_implemented() {
        for (adapter, family) in [
            (&OpenAiResponsesAdapter as &dyn ProtocolAdapter, crate::ProtocolFamily::OpenAiResponses),
            (&GeminiNativeAdapter as &dyn ProtocolAdapter, crate::ProtocolFamily::GeminiNative),
            (&VendorNativeAdapter as &dyn ProtocolAdapter, crate::ProtocolFamily::VendorNative),
        ] {
            assert!(matches!(
                adapter.to_request(&request()),
                Err(ModelError::AdapterNotImplemented { family: returned }) if returned == family
            ));
        }
    }
}
