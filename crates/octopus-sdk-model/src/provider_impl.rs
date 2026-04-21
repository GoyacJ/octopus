use std::{
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use futures::stream;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use octopus_sdk_contracts::SecretVault;

use crate::{
    CacheControlStrategy, FallbackPolicy, ModelCatalog, ModelError, ModelRequest, ModelStream, ProtocolAdapter,
    ProtocolFamily, ProviderDescriptor,
};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError>;
    fn describe(&self) -> ProviderDescriptor;
}

pub struct DefaultModelProvider {
    pub catalog: Arc<ModelCatalog>,
    pub adapters: HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>>,
    pub http_client: reqwest::Client,
    pub secret_vault: Arc<dyn SecretVault>,
}

impl DefaultModelProvider {
    #[must_use]
    pub fn new(
        catalog: Arc<ModelCatalog>,
        adapters: HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>>,
        http_client: reqwest::Client,
        secret_vault: Arc<dyn SecretVault>,
    ) -> Self {
        Self {
            catalog,
            adapters,
            http_client,
            secret_vault,
        }
    }

    pub async fn complete_with_fallback(
        &self,
        req: ModelRequest,
        policy: &FallbackPolicy,
    ) -> Result<ModelStream, ModelError> {
        match self.complete(req.clone()).await {
            Ok(stream) => Ok(stream),
            Err(err) => {
                if policy.should_fallback(&err).is_none() {
                    return Err(err);
                }

                let Some(next_model) = policy.next_model(&req.model).cloned() else {
                    return Err(err);
                };

                let mut retry_req = req;
                retry_req.model = next_model;
                self.complete(retry_req).await
            }
        }
    }
}

#[async_trait]
impl ModelProvider for DefaultModelProvider {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
        let resolved = self
            .catalog
            .resolve(&req.model.0)
            .ok_or_else(|| ModelError::ModelNotFound {
                id: req.model.clone(),
            })?;
        let adapter = self.adapters.get(&resolved.surface.protocol).ok_or_else(|| {
            ModelError::AdapterNotImplemented {
                family: resolved.surface.protocol.clone(),
            }
        })?;
        let mut headers = adapter
            .auth_headers(self.secret_vault.as_ref(), &resolved.provider)
            .await?;
        append_request_headers(&mut headers, &resolved.surface.protocol, &req);
        let body = adapter.to_request(&req)?;
        let request = build_request(
            &self.http_client,
            &resolved.surface.protocol,
            &resolved.surface.base_url,
            headers,
            &body,
        )?;
        let response = self.http_client.execute(request).await.map_err(map_transport_error)?;
        let response = map_response(response).await?;
        let raw = response_byte_stream(response);

        adapter.parse_stream(raw)
    }

    fn describe(&self) -> ProviderDescriptor {
        let mut supported_families = self.adapters.keys().cloned().collect::<Vec<_>>();
        supported_families.sort_by_key(|family| family.to_string());

        ProviderDescriptor {
            id: crate::ProviderId("builtin".to_string()),
            supported_families,
            catalog_version: "builtin-2026-04-02".to_string(),
        }
    }
}

fn append_request_headers(
    headers: &mut Vec<(HeaderName, HeaderValue)>,
    family: &ProtocolFamily,
    req: &ModelRequest,
) {
    if *family != ProtocolFamily::AnthropicMessages {
        return;
    }
    if matches!(req.cache_control, CacheControlStrategy::None) {
        return;
    }

    headers.push((
        HeaderName::from_static("anthropic-beta"),
        HeaderValue::from_static("prompt-caching-2024-07-31"),
    ));
}

fn build_request(
    client: &reqwest::Client,
    family: &ProtocolFamily,
    base_url: &str,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: &serde_json::Value,
) -> Result<reqwest::Request, ModelError> {
    let mut header_map = HeaderMap::new();
    for (name, value) in headers {
        header_map.insert(name, value);
    }

    client
        .post(request_url(base_url, family))
        .headers(header_map)
        .json(body)
        .build()
        .map_err(ModelError::Transport)
}

fn request_url(base_url: &str, family: &ProtocolFamily) -> String {
    let trimmed = base_url.trim_end_matches('/');
    match family {
        ProtocolFamily::AnthropicMessages => format!("{trimmed}/v1/messages"),
        ProtocolFamily::OpenAiChat => format!("{trimmed}/chat/completions"),
        ProtocolFamily::OpenAiResponses => format!("{trimmed}/responses"),
        _ => trimmed.to_string(),
    }
}

fn map_transport_error(error: reqwest::Error) -> ModelError {
    if error.is_timeout() {
        ModelError::UpstreamTimeout
    } else {
        ModelError::Transport(error)
    }
}

async fn map_response(response: reqwest::Response) -> Result<reqwest::Response, ModelError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let retry_after_ms = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds * 1000);
    let body_preview = response.text().await.unwrap_or_default();

    if status.as_u16() == 429 || status.as_u16() == 503 || status.as_u16() == 529 {
        return Err(ModelError::Overloaded { retry_after_ms });
    }

    Err(ModelError::UpstreamStatus {
        status: status.as_u16(),
        body_preview,
    })
}

fn response_byte_stream(response: reqwest::Response) -> crate::StreamBytes {
    Box::pin(stream::try_unfold(response, |mut response| async move {
        match response.chunk().await {
            Ok(Some(chunk)) => Ok(Some((chunk, response))),
            Ok(None) => Ok(None),
            Err(error) => Err(map_transport_error(error)),
        }
    }))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use futures::{stream, StreamExt};
    use reqwest::header::{HeaderName, HeaderValue};
    use serde_json::json;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use octopus_sdk_contracts::{
        AssistantEvent, CacheBreakpoint, ContentBlock, Message, Role, SecretValue, SecretVault,
        StopReason, ToolSchema, VaultError,
    };

    use super::{append_request_headers, DefaultModelProvider, ModelProvider};
    use crate::{
        AuthKind, CacheControlStrategy, ContextWindow, FallbackPolicy, Model, ModelCatalog,
        ModelError, ModelId, ModelRequest, ModelRole, ModelStream, ModelTrack, ProtocolAdapter,
        ProtocolFamily, Provider, ProviderDescriptor, ProviderId, ProviderStatus, ResponseFormat,
        Surface, SurfaceId, ThinkingConfig,
    };

    struct StaticVault;

    #[async_trait]
    impl SecretVault for StaticVault {
        async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
            Ok(SecretValue::new("secret"))
        }

        async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct MockAdapter {
        outcomes: Arc<Mutex<HashMap<String, Vec<Result<(), ModelError>>>>>,
        current_model: Arc<Mutex<Option<String>>>,
    }

    impl MockAdapter {
        fn with_outcome(self, model: &str, outcome: Result<(), ModelError>) -> Self {
            self.outcomes
                .lock()
                .unwrap()
                .entry(model.to_string())
                .or_default()
                .push(outcome);
            self
        }
    }

    #[async_trait]
    impl ProtocolAdapter for MockAdapter {
        fn family(&self) -> ProtocolFamily {
            ProtocolFamily::AnthropicMessages
        }

        fn to_request(&self, req: &ModelRequest) -> Result<serde_json::Value, ModelError> {
            *self.current_model.lock().unwrap() = Some(req.model.0.clone());
            Ok(json!({ "model": req.model.0 }))
        }

        fn parse_stream(
            &self,
            _raw: crate::StreamBytes,
        ) -> Result<ModelStream, ModelError> {
            let model = self.current_model.lock().unwrap().clone().unwrap();
            let outcome = self
                .outcomes
                .lock()
                .unwrap()
                .get_mut(&model)
                .and_then(|outcomes| {
                    if outcomes.is_empty() {
                        None
                    } else {
                        Some(outcomes.remove(0))
                    }
                })
                .unwrap_or(Ok(()));

            match outcome {
                Ok(()) => Ok(Box::pin(stream::iter(vec![Ok(AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                })]))),
                Err(err) => Err(err),
            }
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

    fn sample_request(model: &str) -> ModelRequest {
        ModelRequest {
            model: ModelId(model.to_string()),
            system_prompt: vec!["You are precise.".to_string()],
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                }],
            }],
            tools: vec![ToolSchema {
                name: "search".to_string(),
                description: "Search docs".to_string(),
                input_schema: json!({"type": "object"}),
            }],
            role: ModelRole::Main,
            cache_breakpoints: vec![CacheBreakpoint {
                position: 0,
                ttl: octopus_sdk_contracts::CacheTtl::FiveMinutes,
            }],
            response_format: Some(ResponseFormat::Text),
            thinking: Some(ThinkingConfig {
                enabled: false,
                budget_tokens: None,
            }),
            cache_control: CacheControlStrategy::None,
            max_tokens: Some(256),
            temperature: None,
            stream: true,
        }
    }

    fn test_catalog(base_url: &str) -> ModelCatalog {
        let providers = vec![
            Provider {
                id: ProviderId("anthropic".to_string()),
                display_name: "Anthropic".to_string(),
                status: ProviderStatus::Active,
                auth: AuthKind::ApiKey,
                surfaces: vec![SurfaceId("anthropic.conversation".to_string())],
            },
            Provider {
                id: ProviderId("openai".to_string()),
                display_name: "OpenAI".to_string(),
                status: ProviderStatus::Active,
                auth: AuthKind::ApiKey,
                surfaces: vec![SurfaceId("openai.responses".to_string())],
            },
            Provider {
                id: ProviderId("google".to_string()),
                display_name: "Google".to_string(),
                status: ProviderStatus::Active,
                auth: AuthKind::XApiKey,
                surfaces: vec![SurfaceId("google.conversation".to_string())],
            },
        ];
        let surfaces = vec![
            Surface {
                id: SurfaceId("anthropic.conversation".to_string()),
                provider_id: ProviderId("anthropic".to_string()),
                protocol: ProtocolFamily::AnthropicMessages,
                base_url: base_url.to_string(),
                auth: AuthKind::ApiKey,
            },
            Surface {
                id: SurfaceId("openai.responses".to_string()),
                provider_id: ProviderId("openai".to_string()),
                protocol: ProtocolFamily::OpenAiResponses,
                base_url: base_url.to_string(),
                auth: AuthKind::ApiKey,
            },
            Surface {
                id: SurfaceId("google.conversation".to_string()),
                provider_id: ProviderId("google".to_string()),
                protocol: ProtocolFamily::GeminiNative,
                base_url: base_url.to_string(),
                auth: AuthKind::XApiKey,
            },
        ];
        let models = vec![
            Model {
                id: ModelId("claude-opus-4-6".to_string()),
                surface: SurfaceId("anthropic.conversation".to_string()),
                family: "claude-opus".to_string(),
                track: ModelTrack::Stable,
                context_window: ContextWindow {
                    max_input_tokens: 200_000,
                    max_output_tokens: 32_000,
                    supports_1m: true,
                },
                aliases: vec!["opus".to_string()],
            },
            Model {
                id: ModelId("gpt-5.4".to_string()),
                surface: SurfaceId("openai.responses".to_string()),
                family: "gpt-5.4".to_string(),
                track: ModelTrack::Stable,
                context_window: ContextWindow {
                    max_input_tokens: 200_000,
                    max_output_tokens: 32_000,
                    supports_1m: false,
                },
                aliases: vec!["gpt-5".to_string()],
            },
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
        ];
        let aliases = HashMap::from([
            (
                "claude-opus-4-6".to_string(),
                ModelId("claude-opus-4-6".to_string()),
            ),
            ("opus".to_string(), ModelId("claude-opus-4-6".to_string())),
            ("gpt-5.4".to_string(), ModelId("gpt-5.4".to_string())),
            ("gpt-5".to_string(), ModelId("gpt-5.4".to_string())),
            (
                "gemini-2.5-pro".to_string(),
                ModelId("gemini-2.5-pro".to_string()),
            ),
            (
                "gemini-pro".to_string(),
                ModelId("gemini-2.5-pro".to_string()),
            ),
        ]);

        ModelCatalog::from_parts(providers, surfaces, models, aliases)
    }

    async fn mount_ok(server: &MockServer, route: &str) {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_raw("{}", "application/json"),
            )
            .mount(server)
            .await;
    }

    #[test]
    fn provider_descriptor_is_serializable() {
        let descriptor = ProviderDescriptor {
            id: crate::ProviderId("builtin".to_string()),
            supported_families: vec![ProtocolFamily::AnthropicMessages],
            catalog_version: "builtin-2026-04-02".to_string(),
        };

        assert_eq!(serde_json::to_value(&descriptor).unwrap()["catalog_version"], "builtin-2026-04-02");
    }

    #[test]
    fn append_request_headers_adds_anthropic_beta_only_for_prompt_caching() {
        let mut headers = Vec::new();
        let mut request = sample_request("claude-opus-4-6");
        request.cache_control = CacheControlStrategy::PromptCaching {
            breakpoints: vec!["system", "tools"],
        };

        append_request_headers(
            &mut headers,
            &ProtocolFamily::AnthropicMessages,
            &request,
        );

        assert!(headers.iter().any(|(name, value)| {
            name == &HeaderName::from_static("anthropic-beta")
                && value == &HeaderValue::from_static("prompt-caching-2024-07-31")
        }));
    }

    #[test]
    fn append_request_headers_skips_anthropic_beta_without_prompt_caching() {
        let mut headers = Vec::new();

        append_request_headers(
            &mut headers,
            &ProtocolFamily::AnthropicMessages,
            &sample_request("claude-opus-4-6"),
        );
        append_request_headers(
            &mut headers,
            &ProtocolFamily::OpenAiChat,
            &ModelRequest {
                cache_control: CacheControlStrategy::PromptCaching {
                    breakpoints: vec!["system"],
                },
                ..sample_request("gpt-5.4")
            },
        );

        assert!(headers.is_empty());
    }

    #[tokio::test]
    async fn default_provider_returns_adapter_not_implemented_for_gemini() {
        let server = MockServer::start().await;
        let provider = DefaultModelProvider::new(
            Arc::new(test_catalog(&server.uri())),
            HashMap::from([(
                ProtocolFamily::AnthropicMessages,
                Arc::new(MockAdapter::default()) as Arc<dyn ProtocolAdapter>,
            )]),
            reqwest::Client::new(),
            Arc::new(StaticVault),
        );

        let error = match provider.complete(sample_request("gemini-2.5-pro")).await {
            Ok(_) => panic!("gemini request should not succeed without adapter"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ModelError::AdapterNotImplemented {
                family: ProtocolFamily::GeminiNative
            }
        ));
    }

    #[tokio::test]
    async fn fallback_triggers_on_overloaded_then_succeeds() {
        let server = MockServer::start().await;
        mount_ok(&server, "/v1/messages").await;
        mount_ok(&server, "/responses").await;
        let adapter = MockAdapter::default()
            .with_outcome(
                "claude-opus-4-6",
                Err(ModelError::Overloaded {
                    retry_after_ms: Some(50),
                }),
            )
            .with_outcome("gpt-5.4", Ok(()));
        let provider = DefaultModelProvider::new(
            Arc::new(test_catalog(&server.uri())),
            HashMap::from([
                (
                    ProtocolFamily::AnthropicMessages,
                    Arc::new(adapter.clone()) as Arc<dyn ProtocolAdapter>,
                ),
                (
                    ProtocolFamily::OpenAiResponses,
                    Arc::new(adapter) as Arc<dyn ProtocolAdapter>,
                ),
            ]),
            reqwest::Client::new(),
            Arc::new(StaticVault),
        );
        let policy = FallbackPolicy::default().with_route(
            ModelId("claude-opus-4-6".to_string()),
            ModelId("gpt-5.4".to_string()),
        );
        let mut stream = provider
            .complete_with_fallback(sample_request("claude-opus-4-6"), &policy)
            .await
            .unwrap();

        assert!(matches!(
            stream.next().await.unwrap().unwrap(),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn
            }
        ));
    }

    #[tokio::test]
    async fn fallback_does_not_trigger_on_unrelated_error() {
        let server = MockServer::start().await;
        mount_ok(&server, "/v1/messages").await;
        let adapter = MockAdapter::default().with_outcome(
            "claude-opus-4-6",
            Err(ModelError::CapabilityUnsupported {
                capability: "tool_use".to_string(),
                model: ModelId("claude-opus-4-6".to_string()),
            }),
        );
        let provider = DefaultModelProvider::new(
            Arc::new(test_catalog(&server.uri())),
            HashMap::from([(
                ProtocolFamily::AnthropicMessages,
                Arc::new(adapter) as Arc<dyn ProtocolAdapter>,
            )]),
            reqwest::Client::new(),
            Arc::new(StaticVault),
        );
        let policy = FallbackPolicy::default().with_route(
            ModelId("claude-opus-4-6".to_string()),
            ModelId("gpt-5.4".to_string()),
        );
        let error = match provider
            .complete_with_fallback(sample_request("claude-opus-4-6"), &policy)
            .await
        {
            Ok(_) => panic!("unrelated capability error must not fallback to success"),
            Err(error) => error,
        };

        assert!(matches!(error, ModelError::CapabilityUnsupported { .. }));
    }

    #[tokio::test]
    async fn fallback_exhausted_after_one_retry() {
        let server = MockServer::start().await;
        mount_ok(&server, "/v1/messages").await;
        let adapter = MockAdapter::default().with_outcome(
            "claude-opus-4-6",
            Err(ModelError::Overloaded { retry_after_ms: None }),
        );
        let provider = DefaultModelProvider::new(
            Arc::new(test_catalog(&server.uri())),
            HashMap::from([(
                ProtocolFamily::AnthropicMessages,
                Arc::new(adapter) as Arc<dyn ProtocolAdapter>,
            )]),
            reqwest::Client::new(),
            Arc::new(StaticVault),
        );
        let policy = FallbackPolicy::default();
        let error = match provider
            .complete_with_fallback(sample_request("claude-opus-4-6"), &policy)
            .await
        {
            Ok(_) => panic!("fallback without next model should not succeed"),
            Err(error) => error,
        };

        assert!(matches!(error, ModelError::Overloaded { .. }));
    }
}
