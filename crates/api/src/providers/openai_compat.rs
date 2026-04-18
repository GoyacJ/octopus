use std::collections::VecDeque;
use std::time::Duration;

use crate::error::ApiError;
use crate::http_client::build_http_client_or_default;
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

use super::provider_errors::{
    backoff_for_attempt, expect_openai_success, read_env_non_empty, request_id_from_headers,
    DEFAULT_INITIAL_BACKOFF, DEFAULT_MAX_BACKOFF, DEFAULT_MAX_RETRIES,
};
use super::request_assembly::{
    build_chat_completion_request, chat_completions_endpoint, read_base_url_from_env,
};
use super::response_normalization::{
    attach_request_id_if_missing, normalize_response, ChatCompletionResponse,
};
use super::stream_parsing::{OpenAiSseParser, StreamState};
use super::{preflight_message_request, Provider, ProviderFuture};

#[cfg(test)]
use super::request_assembly::openai_tool_choice;
#[cfg(test)]
use super::response_normalization::{normalize_finish_reason, parse_tool_arguments};
#[cfg(test)]
use super::stream_parsing::parse_sse_frame;

pub const DEFAULT_XAI_BASE_URL: &str = "https://api.x.ai/v1";
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_DASHSCOPE_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAiCompatConfig {
    pub provider_name: &'static str,
    pub api_key_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

const XAI_ENV_VARS: &[&str] = &["XAI_API_KEY"];
const OPENAI_ENV_VARS: &[&str] = &["OPENAI_API_KEY"];
const DASHSCOPE_ENV_VARS: &[&str] = &["DASHSCOPE_API_KEY"];

impl OpenAiCompatConfig {
    #[must_use]
    pub const fn xai() -> Self {
        Self {
            provider_name: "xAI",
            api_key_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: DEFAULT_XAI_BASE_URL,
        }
    }

    #[must_use]
    pub const fn openai() -> Self {
        Self {
            provider_name: "OpenAI",
            api_key_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: DEFAULT_OPENAI_BASE_URL,
        }
    }

    #[must_use]
    pub const fn dashscope() -> Self {
        Self {
            provider_name: "DashScope",
            api_key_env: "DASHSCOPE_API_KEY",
            base_url_env: "DASHSCOPE_BASE_URL",
            default_base_url: DEFAULT_DASHSCOPE_BASE_URL,
        }
    }

    #[must_use]
    pub fn credential_env_vars(self) -> &'static [&'static str] {
        match self.provider_name {
            "xAI" => XAI_ENV_VARS,
            "OpenAI" => OPENAI_ENV_VARS,
            "DashScope" => DASHSCOPE_ENV_VARS,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    http: reqwest::Client,
    api_key: String,
    config: OpenAiCompatConfig,
    base_url: String,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
}

impl OpenAiCompatClient {
    const fn config(&self) -> OpenAiCompatConfig {
        self.config
    }
    #[must_use]
    pub fn new(api_key: impl Into<String>, config: OpenAiCompatConfig) -> Self {
        Self {
            http: build_http_client_or_default(),
            api_key: api_key.into(),
            config,
            base_url: read_base_url(config),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
        }
    }

    pub fn from_env(config: OpenAiCompatConfig) -> Result<Self, ApiError> {
        let Some(api_key) = read_env_non_empty(config.api_key_env)? else {
            return Err(ApiError::missing_credentials(
                config.provider_name,
                config.credential_env_vars(),
            ));
        };
        Ok(Self::new(api_key, config))
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

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        let request = MessageRequest {
            stream: false,
            ..request.clone()
        };
        preflight_message_request(&request)?;
        let response = self.send_with_retry(&request).await?;
        let request_id = request_id_from_headers(response.headers());
        let body = response.text().await.map_err(ApiError::from)?;
        let payload = serde_json::from_str::<ChatCompletionResponse>(&body).map_err(|error| {
            ApiError::json_deserialize(self.config.provider_name, &request.model, &body, error)
        })?;
        Ok(attach_request_id_if_missing(
            normalize_response(&request.model, payload)?,
            request_id,
        ))
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        preflight_message_request(request)?;
        let response = self
            .send_with_retry(&request.clone().with_streaming())
            .await?;
        Ok(MessageStream {
            request_id: request_id_from_headers(response.headers()),
            response,
            parser: OpenAiSseParser::with_context(self.config.provider_name, request.model.clone()),
            pending: VecDeque::new(),
            done: false,
            state: StreamState::new(request.model.clone()),
        })
    }

    async fn send_with_retry(
        &self,
        request: &MessageRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let mut attempts = 0;

        let last_error = loop {
            attempts += 1;
            let retryable_error = match self.send_raw_request(request).await {
                Ok(response) => match expect_openai_success(response).await {
                    Ok(response) => return Ok(response),
                    Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => error,
                    Err(error) => return Err(error),
                },
                Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => error,
                Err(error) => return Err(error),
            };

            if attempts > self.max_retries {
                break retryable_error;
            }

            tokio::time::sleep(self.backoff_for_attempt(attempts)?).await;
        };

        Err(ApiError::RetriesExhausted {
            attempts,
            last_error: Box::new(last_error),
        })
    }

    async fn send_raw_request(
        &self,
        request: &MessageRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let request_url = chat_completions_endpoint(&self.base_url);
        self.http
            .post(&request_url)
            .header("content-type", "application/json")
            .bearer_auth(&self.api_key)
            .json(&build_chat_completion_request(request, self.config()))
            .send()
            .await
            .map_err(ApiError::from)
    }

    fn backoff_for_attempt(&self, attempt: u32) -> Result<Duration, ApiError> {
        backoff_for_attempt(attempt, self.initial_backoff, self.max_backoff)
    }
}

impl Provider for OpenAiCompatClient {
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

#[derive(Debug)]
pub struct MessageStream {
    request_id: Option<String>,
    response: reqwest::Response,
    parser: OpenAiSseParser,
    pending: VecDeque<StreamEvent>,
    done: bool,
    state: StreamState,
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            if self.done {
                self.pending.extend(self.state.finish()?);
                if let Some(event) = self.pending.pop_front() {
                    return Ok(Some(event));
                }
                return Ok(None);
            }

            match self.response.chunk().await? {
                Some(chunk) => {
                    for parsed in self.parser.push(&chunk)? {
                        self.pending.extend(self.state.ingest_chunk(parsed)?);
                    }
                }
                None => {
                    self.done = true;
                }
            }
        }
    }
}

#[must_use]
pub fn read_base_url(config: OpenAiCompatConfig) -> String {
    read_base_url_from_env(config.base_url_env, config.default_base_url)
}

#[cfg(test)]
mod tests {
    use super::{
        build_chat_completion_request, chat_completions_endpoint, normalize_finish_reason,
        openai_tool_choice, parse_sse_frame, parse_tool_arguments, OpenAiCompatClient,
        OpenAiCompatConfig,
    };
    use crate::error::ApiError;
    use crate::types::{
        InputContentBlock, InputMessage, MessageRequest, ToolChoice, ToolDefinition,
        ToolResultContentBlock,
    };
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};

    #[test]
    fn request_translation_uses_openai_compatible_shape() {
        let payload = build_chat_completion_request(
            &MessageRequest {
                model: "grok-3".to_string(),
                max_tokens: 64,
                messages: vec![InputMessage {
                    role: "user".to_string(),
                    content: vec![
                        InputContentBlock::Text {
                            text: "hello".to_string(),
                        },
                        InputContentBlock::ToolResult {
                            tool_use_id: "tool_1".to_string(),
                            content: vec![ToolResultContentBlock::Json {
                                value: json!({"ok": true}),
                            }],
                            is_error: false,
                        },
                    ],
                }],
                system: Some("be helpful".to_string()),
                tools: Some(vec![ToolDefinition {
                    name: "weather".to_string(),
                    description: Some("Get weather".to_string()),
                    input_schema: json!({"type": "object"}),
                }]),
                tool_choice: Some(ToolChoice::Auto),
                stream: false,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                reasoning_effort: None,
            },
            OpenAiCompatConfig::xai(),
        );

        assert_eq!(payload["messages"][0]["role"], json!("system"));
        assert_eq!(payload["messages"][1]["role"], json!("user"));
        assert_eq!(payload["messages"][2]["role"], json!("tool"));
        assert_eq!(payload["tools"][0]["type"], json!("function"));
        assert_eq!(payload["tool_choice"], json!("auto"));
    }

    #[test]
    fn openai_streaming_requests_include_usage_opt_in() {
        let payload = build_chat_completion_request(
            &MessageRequest {
                model: "gpt-5".to_string(),
                max_tokens: 64,
                messages: vec![InputMessage::user_text("hello")],
                system: None,
                tools: None,
                tool_choice: None,
                stream: true,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                reasoning_effort: None,
            },
            OpenAiCompatConfig::openai(),
        );

        assert_eq!(payload["stream_options"], json!({"include_usage": true}));
    }

    #[test]
    fn xai_streaming_requests_skip_openai_specific_usage_opt_in() {
        let payload = build_chat_completion_request(
            &MessageRequest {
                model: "grok-3".to_string(),
                max_tokens: 64,
                messages: vec![InputMessage::user_text("hello")],
                system: None,
                tools: None,
                tool_choice: None,
                stream: true,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                reasoning_effort: None,
            },
            OpenAiCompatConfig::xai(),
        );

        assert!(payload.get("stream_options").is_none());
    }

    #[test]
    fn tool_choice_translation_supports_required_function() {
        assert_eq!(openai_tool_choice(&ToolChoice::Any), json!("required"));
        assert_eq!(
            openai_tool_choice(&ToolChoice::Tool {
                name: "weather".to_string(),
            }),
            json!({"type": "function", "function": {"name": "weather"}})
        );
    }

    #[test]
    fn parses_tool_arguments_fallback() {
        assert_eq!(
            parse_tool_arguments("{\"city\":\"Paris\"}"),
            json!({"city": "Paris"})
        );
        assert_eq!(parse_tool_arguments("not-json"), json!({"raw": "not-json"}));
    }

    #[test]
    fn tool_definition_normalizes_object_schema_for_strict_endpoints() {
        let payload = build_chat_completion_request(
            &MessageRequest {
                model: "gpt-4o".to_string(),
                max_tokens: 64,
                messages: vec![InputMessage::user_text("hello")],
                system: None,
                tools: Some(vec![ToolDefinition {
                    name: "weather".to_string(),
                    description: Some("Get weather".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "object"
                            }
                        }
                    }),
                }]),
                tool_choice: Some(ToolChoice::Auto),
                stream: false,
                temperature: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop: None,
                reasoning_effort: None,
            },
            OpenAiCompatConfig::openai(),
        );

        let parameters = &payload["tools"][0]["function"]["parameters"];
        assert_eq!(parameters["additionalProperties"], json!(false));
        assert!(parameters["properties"].is_object());
        assert_eq!(
            parameters["properties"]["location"]["additionalProperties"],
            json!(false)
        );
        assert!(parameters["properties"]["location"]["properties"].is_object());
    }

    #[test]
    fn missing_xai_api_key_is_provider_specific() {
        let _lock = env_lock();
        std::env::remove_var("XAI_API_KEY");
        let error = OpenAiCompatClient::from_env(OpenAiCompatConfig::xai())
            .expect_err("missing key should error");
        assert!(matches!(
            error,
            ApiError::MissingCredentials {
                provider: "xAI",
                ..
            }
        ));
    }

    #[test]
    fn endpoint_builder_accepts_base_urls_and_full_endpoints() {
        assert_eq!(
            chat_completions_endpoint("https://api.x.ai/v1"),
            "https://api.x.ai/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_endpoint("https://api.x.ai/v1/"),
            "https://api.x.ai/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_endpoint("https://api.x.ai/v1/chat/completions"),
            "https://api.x.ai/v1/chat/completions"
        );
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    #[test]
    fn normalizes_stop_reasons() {
        assert_eq!(normalize_finish_reason("stop"), "end_turn");
        assert_eq!(normalize_finish_reason("tool_calls"), "tool_use");
    }

    #[test]
    fn tuning_params_included_in_payload_when_set() {
        let request: MessageRequest = serde_json::from_value(json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [],
            "stream": false,
            "temperature": 0.7,
            "top_p": 0.9,
            "frequency_penalty": 0.5,
            "presence_penalty": 0.3,
            "stop": ["\n"]
        }))
        .expect("request should deserialize");

        let payload = build_chat_completion_request(&request, OpenAiCompatConfig::openai());
        assert_eq!(payload["temperature"], json!(0.7));
        assert_eq!(payload["top_p"], json!(0.9));
        assert_eq!(payload["frequency_penalty"], json!(0.5));
        assert_eq!(payload["presence_penalty"], json!(0.3));
        assert_eq!(payload["stop"], json!(["\n"]));
    }

    #[test]
    fn reasoning_model_strips_tuning_params() {
        let request: MessageRequest = serde_json::from_value(json!({
            "model": "o1-mini",
            "max_tokens": 1024,
            "messages": [],
            "stream": false,
            "temperature": 0.7,
            "top_p": 0.9,
            "frequency_penalty": 0.5,
            "presence_penalty": 0.3,
            "stop": ["\n"]
        }))
        .expect("request should deserialize");

        let payload = build_chat_completion_request(&request, OpenAiCompatConfig::openai());
        assert!(
            payload.get("temperature").is_none(),
            "reasoning model should strip temperature"
        );
        assert!(
            payload.get("top_p").is_none(),
            "reasoning model should strip top_p"
        );
        assert!(payload.get("frequency_penalty").is_none());
        assert!(payload.get("presence_penalty").is_none());
        assert_eq!(payload["stop"], json!(["\n"]));
    }

    #[test]
    fn reasoning_effort_is_included_when_set() {
        let request: MessageRequest = serde_json::from_value(json!({
            "model": "o4-mini",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": [{"type": "text", "text": "think hard"}]}],
            "reasoning_effort": "high"
        }))
        .expect("request should deserialize");

        let payload = build_chat_completion_request(&request, OpenAiCompatConfig::openai());
        assert_eq!(payload["reasoning_effort"], json!("high"));
    }

    #[test]
    fn gpt5_uses_max_completion_tokens_not_max_tokens() {
        let request = MessageRequest {
            model: "gpt-5.2".to_string(),
            max_tokens: 512,
            messages: vec![],
            system: None,
            tools: None,
            tool_choice: None,
            stream: false,
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            reasoning_effort: None,
        };

        let payload = build_chat_completion_request(&request, OpenAiCompatConfig::openai());
        assert_eq!(
            payload["max_completion_tokens"],
            json!(512),
            "gpt-5.2 should emit max_completion_tokens"
        );
        assert!(
            payload.get("max_tokens").is_none(),
            "gpt-5.2 must not emit max_tokens"
        );
    }

    #[test]
    fn parse_sse_frame_reports_provider_and_model_on_invalid_json() {
        let error = parse_sse_frame("data: {not-json}\n\n", "OpenAI", "gpt-4o")
            .expect_err("invalid frame should fail");

        match error {
            ApiError::Json {
                provider, model, ..
            } => {
                assert_eq!(provider, "OpenAI");
                assert_eq!(model, "gpt-4o");
            }
            other => panic!("expected contextual json error, got {other:?}"),
        }
    }
}
