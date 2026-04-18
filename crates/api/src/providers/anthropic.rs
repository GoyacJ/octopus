use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use runtime::format_usd;
use runtime::{
    load_oauth_credentials, save_oauth_credentials, OAuthConfig, OAuthRefreshRequest,
    OAuthTokenExchangeRequest,
};
use serde_json::{Map, Value};
use telemetry::{AnalyticsEvent, AnthropicRequestProfile, ClientIdentity, SessionTracer};

use crate::error::ApiError;
use crate::http_client::build_http_client_or_default;
use crate::prompt_cache::{PromptCache, PromptCacheRecord, PromptCacheStats};

use super::provider_errors::{
    backoff_for_attempt, expect_anthropic_success, read_env_non_empty, request_id_from_headers,
    DEFAULT_INITIAL_BACKOFF, DEFAULT_MAX_BACKOFF, DEFAULT_MAX_RETRIES,
};
use super::request_assembly::{read_base_url_from_env, render_anthropic_request_body};
use super::response_normalization::attach_request_id_if_missing;
use super::{anthropic_missing_credentials, preflight_message_request, Provider, ProviderFuture};
use crate::sse::SseParser;
use crate::types::{MessageDeltaEvent, MessageRequest, MessageResponse, StreamEvent, Usage};

#[path = "anthropic_auth.rs"]
mod anthropic_auth;
#[path = "anthropic_oauth.rs"]
mod anthropic_oauth;
#[path = "anthropic_stream.rs"]
mod anthropic_stream;
#[cfg(test)]
#[path = "anthropic_tests.rs"]
mod anthropic_tests;

pub use anthropic_auth::AuthSource;
pub use anthropic_oauth::{
    oauth_token_is_expired, read_base_url, resolve_saved_oauth_token, resolve_startup_auth_source,
    OAuthTokenSet,
};
pub use anthropic_stream::MessageStream;

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

#[derive(Debug, Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    auth: AuthSource,
    base_url: String,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
    request_profile: AnthropicRequestProfile,
    session_tracer: Option<SessionTracer>,
    prompt_cache: Option<PromptCache>,
    last_prompt_cache_record: Arc<Mutex<Option<PromptCacheRecord>>>,
}

impl AnthropicClient {
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: build_http_client_or_default(),
            auth: AuthSource::ApiKey(api_key.into()),
            base_url: DEFAULT_BASE_URL.to_string(),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            request_profile: AnthropicRequestProfile::default(),
            session_tracer: None,
            prompt_cache: None,
            last_prompt_cache_record: Arc::new(Mutex::new(None)),
        }
    }

    #[must_use]
    pub fn from_auth(auth: AuthSource) -> Self {
        Self {
            http: build_http_client_or_default(),
            auth,
            base_url: DEFAULT_BASE_URL.to_string(),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            request_profile: AnthropicRequestProfile::default(),
            session_tracer: None,
            prompt_cache: None,
            last_prompt_cache_record: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_env() -> Result<Self, ApiError> {
        Ok(Self::from_auth(AuthSource::from_env_or_saved()?).with_base_url(read_base_url()))
    }

    #[must_use]
    pub fn with_auth_source(mut self, auth: AuthSource) -> Self {
        self.auth = auth;
        self
    }

    #[must_use]
    pub fn with_auth_token(mut self, auth_token: Option<String>) -> Self {
        match (
            self.auth.api_key().map(ToOwned::to_owned),
            auth_token.filter(|token| !token.is_empty()),
        ) {
            (Some(api_key), Some(bearer_token)) => {
                self.auth = AuthSource::ApiKeyAndBearer {
                    api_key,
                    bearer_token,
                };
            }
            (Some(api_key), None) => {
                self.auth = AuthSource::ApiKey(api_key);
            }
            (None, Some(bearer_token)) => {
                self.auth = AuthSource::BearerToken(bearer_token);
            }
            (None, None) => {
                self.auth = AuthSource::None;
            }
        }
        self
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    #[must_use]
    pub fn with_retry_policy(
        mut self,
        max_retries: u32,
        initial_backoff: Duration,
        max_backoff: Duration,
    ) -> Self {
        self.max_retries = max_retries;
        self.initial_backoff = initial_backoff;
        self.max_backoff = max_backoff;
        self
    }

    #[must_use]
    pub fn with_session_tracer(mut self, session_tracer: SessionTracer) -> Self {
        self.session_tracer = Some(session_tracer);
        self
    }

    #[must_use]
    pub fn with_client_identity(mut self, client_identity: ClientIdentity) -> Self {
        self.request_profile.client_identity = client_identity;
        self
    }

    #[must_use]
    pub fn with_beta(mut self, beta: impl Into<String>) -> Self {
        self.request_profile = self.request_profile.with_beta(beta);
        self
    }

    #[must_use]
    pub fn with_extra_body_param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.request_profile = self.request_profile.with_extra_body(key, value);
        self
    }

    #[must_use]
    pub fn with_prompt_cache(mut self, prompt_cache: PromptCache) -> Self {
        self.prompt_cache = Some(prompt_cache);
        self
    }

    #[must_use]
    pub fn prompt_cache_stats(&self) -> Option<PromptCacheStats> {
        self.prompt_cache.as_ref().map(PromptCache::stats)
    }

    #[must_use]
    pub fn request_profile(&self) -> &AnthropicRequestProfile {
        &self.request_profile
    }

    #[must_use]
    pub fn session_tracer(&self) -> Option<&SessionTracer> {
        self.session_tracer.as_ref()
    }

    #[must_use]
    pub fn prompt_cache(&self) -> Option<&PromptCache> {
        self.prompt_cache.as_ref()
    }

    #[must_use]
    pub fn take_last_prompt_cache_record(&self) -> Option<PromptCacheRecord> {
        self.last_prompt_cache_record
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    #[must_use]
    pub fn with_request_profile(mut self, request_profile: AnthropicRequestProfile) -> Self {
        self.request_profile = request_profile;
        self
    }

    #[must_use]
    pub fn auth_source(&self) -> &AuthSource {
        &self.auth
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        let request = MessageRequest {
            stream: false,
            ..request.clone()
        };

        if let Some(prompt_cache) = &self.prompt_cache {
            if let Some(response) = prompt_cache.lookup_completion(&request) {
                return Ok(response);
            }
        }

        preflight_message_request(&request)?;

        let response = self.send_with_retry(&request).await?;
        let request_id = request_id_from_headers(response.headers());
        let response = attach_request_id_if_missing(
            response
                .json::<MessageResponse>()
                .await
                .map_err(ApiError::from)?,
            request_id,
        );

        if let Some(prompt_cache) = &self.prompt_cache {
            let record = prompt_cache.record_response(&request, &response);
            self.store_last_prompt_cache_record(record);
        }
        if let Some(session_tracer) = &self.session_tracer {
            session_tracer.record_analytics(
                AnalyticsEvent::new("api", "message_usage")
                    .with_property(
                        "request_id",
                        response
                            .request_id
                            .clone()
                            .map_or(Value::Null, Value::String),
                    )
                    .with_property("total_tokens", Value::from(response.total_tokens()))
                    .with_property(
                        "estimated_cost_usd",
                        Value::String(format_usd(
                            response
                                .usage
                                .estimated_cost_usd(&response.model)
                                .total_cost_usd(),
                        )),
                    ),
            );
        }
        Ok(response)
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        preflight_message_request(request)?;
        let response = self
            .send_with_retry(&request.clone().with_streaming())
            .await?;
        Ok(MessageStream::new(
            response,
            request.clone(),
            self.prompt_cache.clone(),
            Arc::clone(&self.last_prompt_cache_record),
        ))
    }

    pub async fn exchange_oauth_code(
        &self,
        config: &OAuthConfig,
        request: &OAuthTokenExchangeRequest,
    ) -> Result<OAuthTokenSet, ApiError> {
        let response = self
            .http
            .post(&config.token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&request.form_params())
            .send()
            .await
            .map_err(ApiError::from)?;
        let response = expect_anthropic_success(response).await?;
        response
            .json::<OAuthTokenSet>()
            .await
            .map_err(ApiError::from)
    }

    pub async fn refresh_oauth_token(
        &self,
        config: &OAuthConfig,
        request: &OAuthRefreshRequest,
    ) -> Result<OAuthTokenSet, ApiError> {
        let response = self
            .http
            .post(&config.token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&request.form_params())
            .send()
            .await
            .map_err(ApiError::from)?;
        let response = expect_anthropic_success(response).await?;
        response
            .json::<OAuthTokenSet>()
            .await
            .map_err(ApiError::from)
    }

    async fn send_with_retry(
        &self,
        request: &MessageRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let mut attempts = 0;
        let mut last_error: Option<ApiError>;

        loop {
            attempts += 1;
            if let Some(session_tracer) = &self.session_tracer {
                session_tracer.record_http_request_started(
                    attempts,
                    "POST",
                    "/v1/messages",
                    Map::new(),
                );
            }
            match self.send_raw_request(request).await {
                Ok(response) => match expect_anthropic_success(response).await {
                    Ok(response) => {
                        if let Some(session_tracer) = &self.session_tracer {
                            session_tracer.record_http_request_succeeded(
                                attempts,
                                "POST",
                                "/v1/messages",
                                response.status().as_u16(),
                                request_id_from_headers(response.headers()),
                                Map::new(),
                            );
                        }
                        return Ok(response);
                    }
                    Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => {
                        self.record_request_failure(attempts, &error);
                        last_error = Some(error);
                    }
                    Err(error) => {
                        let error = anthropic_auth::enrich_bearer_auth_error(error, &self.auth);
                        self.record_request_failure(attempts, &error);
                        return Err(error);
                    }
                },
                Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => {
                    self.record_request_failure(attempts, &error);
                    last_error = Some(error);
                }
                Err(error) => {
                    self.record_request_failure(attempts, &error);
                    return Err(error);
                }
            }

            if attempts > self.max_retries {
                break;
            }

            tokio::time::sleep(self.backoff_for_attempt(attempts)?).await;
        }

        Err(ApiError::RetriesExhausted {
            attempts,
            last_error: Box::new(last_error.expect("retry loop must capture an error")),
        })
    }

    async fn send_raw_request(
        &self,
        request: &MessageRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let request_url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let request_builder = self
            .http
            .post(&request_url)
            .header("content-type", "application/json");
        let mut request_builder = self.auth.apply(request_builder);
        for (header_name, header_value) in self.request_profile.header_pairs() {
            request_builder = request_builder.header(header_name, header_value);
        }

        let request_body = self.render_request_body(request)?;
        request_builder = request_builder.json(&request_body);
        request_builder.send().await.map_err(ApiError::from)
    }

    fn render_request_body(&self, request: &MessageRequest) -> Result<Value, ApiError> {
        render_anthropic_request_body(|| {
            self.request_profile
                .render_json_body(request)
                .map_err(ApiError::from)
        })
    }

    fn record_request_failure(&self, attempt: u32, error: &ApiError) {
        if let Some(session_tracer) = &self.session_tracer {
            session_tracer.record_http_request_failed(
                attempt,
                "POST",
                "/v1/messages",
                error.to_string(),
                error.is_retryable(),
                Map::new(),
            );
        }
    }

    fn store_last_prompt_cache_record(&self, record: PromptCacheRecord) {
        *self
            .last_prompt_cache_record
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(record);
    }

    fn backoff_for_attempt(&self, attempt: u32) -> Result<Duration, ApiError> {
        backoff_for_attempt(attempt, self.initial_backoff, self.max_backoff)
    }
}

impl Provider for AnthropicClient {
    type Stream = MessageStream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse> {
        Box::pin(async move { self.send_message(request).await })
    }

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream> {
        Box::pin(async move { self.stream_message(request).await })
    }
}
