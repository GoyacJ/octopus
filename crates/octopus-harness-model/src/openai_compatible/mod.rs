mod error;
mod responses;
mod streaming;

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    Message, MessagePart, MessageRole, ModelError, StopReason, ToolDescriptor, ToolResult,
    ToolResultPart, UsageSnapshot,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{Mutex, Semaphore};

use crate::{
    ApiMode, Backoff, ContentDelta, ContentType, ErrorClass, InferContext, ModelRequest,
    ModelStream, ModelStreamEvent,
};

use self::error::{map_response_error, map_transport_error, OpenAiCompatibleError};

const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Clone)]
pub(crate) struct OpenAiCompatibleClient {
    http: reqwest::Client,
    api_key: Option<SecretString>,
    base_url: String,
    path: String,
    api_mode: ApiMode,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    concurrency: Option<Arc<Semaphore>>,
}

#[allow(dead_code)]
impl OpenAiCompatibleClient {
    pub(crate) fn from_api_key(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::new(
            Some(api_key.into()),
            base_url,
            ApiMode::ChatCompletions,
            "/v1/chat/completions",
        )
    }

    pub(crate) fn without_api_key(base_url: impl Into<String>) -> Self {
        Self::new(
            None,
            base_url,
            ApiMode::ChatCompletions,
            "/v1/chat/completions",
        )
    }

    fn new(
        api_key: Option<String>,
        base_url: impl Into<String>,
        api_mode: ApiMode,
        path: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.map(SecretString::new),
            base_url: base_url.into(),
            path: path.into(),
            api_mode,
            cooldown_until: Arc::new(Mutex::new(None)),
            concurrency: None,
        }
    }

    #[must_use]
    pub(crate) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    #[must_use]
    pub(crate) fn with_chat_completions_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    #[must_use]
    pub(crate) fn with_responses_path(mut self, path: impl Into<String>) -> Self {
        self.api_mode = ApiMode::Responses;
        self.path = path.into();
        self
    }

    #[must_use]
    pub(crate) fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(SecretString::new(api_key.into()));
        self
    }

    #[must_use]
    pub(crate) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        self
    }

    #[must_use]
    pub(crate) fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.concurrency = (max_concurrency > 0).then(|| Arc::new(Semaphore::new(max_concurrency)));
        self
    }

    pub(crate) async fn infer(
        &self,
        req: ModelRequest,
        ctx: InferContext,
    ) -> Result<ModelStream, ModelError> {
        self.validate_request(&req)?;
        let body = self.request_body(&req)?;
        let max_attempts = ctx.retry_policy.max_attempts.max(1);
        let mut attempt = 0;

        loop {
            if ctx.cancel.is_cancelled() {
                return Err(ModelError::Cancelled);
            }
            if let Some(deadline) = ctx.deadline {
                if Instant::now() >= deadline {
                    return Err(ModelError::DeadlineExceeded(Duration::ZERO));
                }
            }
            self.wait_for_cooldown().await;

            let result = self.send_once(&body).await;
            match result {
                Ok(response) => {
                    if req.stream {
                        return Ok(match self.api_mode {
                            ApiMode::ChatCompletions => streaming::response_to_stream(response),
                            ApiMode::Responses => responses::response_to_stream(response),
                            _ => unreachable!("validated OpenAI-compatible API mode"),
                        });
                    }
                    let response = response
                        .json()
                        .await
                        .map_err(map_transport_error)
                        .map_err(|error| error.error)?;
                    return match self.api_mode {
                        ApiMode::ChatCompletions => chat_response_to_stream(response),
                        ApiMode::Responses => responses::json_response_to_stream(response),
                        _ => unreachable!("validated OpenAI-compatible API mode"),
                    };
                }
                Err(err) => {
                    if let Some(retry_after) = err.retry_after {
                        self.set_cooldown(retry_after).await;
                    }

                    attempt += 1;
                    let can_retry =
                        attempt < max_attempts && (ctx.retry_policy.retry_on)(&err.class);
                    if !can_retry {
                        return Err(err.error);
                    }

                    let delay = err
                        .retry_after
                        .unwrap_or_else(|| retry_delay(&ctx.retry_policy.backoff, attempt));
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }

    async fn send_once(&self, body: &Value) -> Result<reqwest::Response, OpenAiCompatibleError> {
        let _permit = match &self.concurrency {
            Some(semaphore) => Some(semaphore.clone().acquire_owned().await.map_err(|error| {
                OpenAiCompatibleError {
                    error: ModelError::ProviderUnavailable(error.to_string()),
                    class: ErrorClass::Transient,
                    retry_after: None,
                }
            })?),
            None => None,
        };
        let response = self
            .http
            .post(format!(
                "{}{}",
                self.base_url.trim_end_matches('/'),
                normalize_path(&self.path)
            ))
            .headers(self.headers()?)
            .json(body)
            .send()
            .await
            .map_err(map_transport_error)?;

        if !response.status().is_success() {
            return Err(map_response_error(response).await);
        }

        Ok(response)
    }

    fn headers(&self) -> Result<HeaderMap, OpenAiCompatibleError> {
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            let value = format!("Bearer {}", api_key.expose_secret());
            let auth = HeaderValue::from_str(&value).map_err(|error| OpenAiCompatibleError {
                error: ModelError::AuthExpired(error.to_string()),
                class: ErrorClass::AuthExpired,
                retry_after: None,
            })?;
            headers.insert(AUTHORIZATION, auth);
        }
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    fn validate_request(&self, req: &ModelRequest) -> Result<(), ModelError> {
        if req.api_mode != self.api_mode {
            return Err(ModelError::InvalidRequest(format!(
                "OpenAI-compatible provider expected {:?}, got {:?}",
                self.api_mode, req.api_mode
            )));
        }
        if !req.cache_breakpoints.is_empty() {
            return Err(ModelError::InvalidRequest(
                "OpenAI-compatible providers do not accept explicit cache breakpoints".to_owned(),
            ));
        }
        Ok(())
    }

    fn request_body(&self, req: &ModelRequest) -> Result<Value, ModelError> {
        match self.api_mode {
            ApiMode::ChatCompletions => chat_request_body(req),
            ApiMode::Responses => responses_request_body(req),
            _ => Err(ModelError::InvalidRequest(
                "unsupported OpenAI-compatible API mode".to_owned(),
            )),
        }
    }

    async fn wait_for_cooldown(&self) {
        let cooldown_until = *self.cooldown_until.lock().await;
        let delay = cooldown_until.and_then(|until| until.checked_duration_since(Instant::now()));
        if let Some(delay) = delay {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }
    }

    async fn set_cooldown(&self, delay: Duration) {
        *self.cooldown_until.lock().await = Some(Instant::now() + delay);
    }
}

fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    }
}

#[async_trait]
pub(crate) trait OpenAiCompatibleProviderExt: Send + Sync + 'static {
    fn client(&self) -> &OpenAiCompatibleClient;

    async fn infer_openai_compatible(
        &self,
        req: ModelRequest,
        ctx: InferContext,
    ) -> Result<ModelStream, ModelError> {
        self.client().infer(req, ctx).await
    }
}

fn chat_request_body(req: &ModelRequest) -> Result<Value, ModelError> {
    let mut messages = Vec::new();
    if let Some(system) = &req.system {
        messages.push(json!({
            "role": "system",
            "content": system,
        }));
    }
    for message in &req.messages {
        messages.push(chat_message(message)?);
    }

    let mut body = json!({
        "model": req.model_id,
        "messages": messages,
        "max_tokens": req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "stream": req.stream,
    });

    if req.stream {
        body["stream_options"] = json!({ "include_usage": true });
    }
    if let Some(temperature) = req.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(tools) = &req.tools {
        body["tools"] = Value::Array(tools.iter().map(openai_tool).collect());
    }

    Ok(body)
}

fn responses_request_body(req: &ModelRequest) -> Result<Value, ModelError> {
    let mut input = Vec::new();
    if let Some(system) = &req.system {
        input.push(json!({
            "role": "system",
            "content": system,
        }));
    }
    for message in &req.messages {
        input.push(chat_message(message)?);
    }

    let mut body = json!({
        "model": req.model_id,
        "input": input,
        "max_output_tokens": req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "stream": req.stream,
    });

    if let Some(temperature) = req.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(tools) = &req.tools {
        body["tools"] = Value::Array(tools.iter().map(responses_tool).collect());
    }

    Ok(body)
}

fn chat_message(message: &Message) -> Result<Value, ModelError> {
    match message.role {
        MessageRole::System => Ok(json!({
            "role": "system",
            "content": text_content(&message.parts)?,
        })),
        MessageRole::User => Ok(json!({
            "role": "user",
            "content": text_content(&message.parts)?,
        })),
        MessageRole::Assistant => assistant_message(&message.parts),
        MessageRole::Tool => tool_message(&message.parts),
        _ => Err(ModelError::InvalidRequest(
            "unknown message role is not supported by OpenAI-compatible providers".to_owned(),
        )),
    }
}

fn assistant_message(parts: &[MessagePart]) -> Result<Value, ModelError> {
    let mut text = Vec::new();
    let mut tool_calls = Vec::new();

    for part in parts {
        match part {
            MessagePart::Text(value) => text.push(value.clone()),
            MessagePart::ToolUse { id, name, input } => tool_calls.push(json!({
                "id": id.to_string(),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": input.to_string(),
                },
            })),
            MessagePart::Image { .. }
            | MessagePart::Thinking(_)
            | MessagePart::ToolResult { .. } => {
                return Err(ModelError::InvalidRequest(
                    "assistant messages only support text and tool use parts for OpenAI-compatible providers"
                        .to_owned(),
                ));
            }
            _ => {
                return Err(ModelError::InvalidRequest(
                    "unsupported assistant message part for OpenAI-compatible providers".to_owned(),
                ));
            }
        }
    }

    let mut message = json!({
        "role": "assistant",
        "content": if text.is_empty() {
            Value::Null
        } else {
            Value::String(text.join(""))
        },
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    Ok(message)
}

fn tool_message(parts: &[MessagePart]) -> Result<Value, ModelError> {
    let [MessagePart::ToolResult {
        tool_use_id,
        content,
    }] = parts
    else {
        return Err(ModelError::InvalidRequest(
            "tool messages must contain exactly one tool result part for OpenAI-compatible providers"
                .to_owned(),
        ));
    };

    Ok(json!({
        "role": "tool",
        "tool_call_id": tool_use_id.to_string(),
        "content": tool_result_content(content)?,
    }))
}

fn text_content(parts: &[MessagePart]) -> Result<String, ModelError> {
    let mut text = String::new();
    for part in parts {
        match part {
            MessagePart::Text(value) => text.push_str(value),
            MessagePart::Image { .. } => {
                return Err(ModelError::InvalidRequest(
                    "image message parts are not supported by OpenAI-compatible providers in M2-T04.5"
                        .to_owned(),
                ));
            }
            MessagePart::Thinking(_) => {
                return Err(ModelError::InvalidRequest(
                    "thinking message parts are not supported by OpenAI-compatible providers"
                        .to_owned(),
                ));
            }
            MessagePart::ToolUse { .. } | MessagePart::ToolResult { .. } => {
                return Err(ModelError::InvalidRequest(
                    "tool message parts must use assistant/tool roles for OpenAI-compatible providers"
                        .to_owned(),
                ));
            }
            _ => {
                return Err(ModelError::InvalidRequest(
                    "unsupported message part for OpenAI-compatible providers".to_owned(),
                ));
            }
        }
    }
    Ok(text)
}

fn tool_result_content(content: &ToolResult) -> Result<String, ModelError> {
    match content {
        ToolResult::Text(text) => Ok(text.clone()),
        ToolResult::Structured(value) => Ok(value.to_string()),
        ToolResult::Blob { .. } => Err(ModelError::InvalidRequest(
            "blob tool results are not supported by OpenAI-compatible providers in M2-T04.5"
                .to_owned(),
        )),
        ToolResult::Mixed(parts) => parts
            .iter()
            .map(tool_result_part_content)
            .collect::<Result<Vec<_>, _>>()
            .map(|parts| parts.join("")),
        _ => Err(ModelError::InvalidRequest(
            "unsupported tool result for OpenAI-compatible providers".to_owned(),
        )),
    }
}

fn tool_result_part_content(part: &ToolResultPart) -> Result<String, ModelError> {
    match part {
        ToolResultPart::Text { text } => Ok(text.clone()),
        ToolResultPart::Structured { value, .. } => Ok(value.to_string()),
        ToolResultPart::Code { text, .. } => Ok(text.clone()),
        ToolResultPart::Reference { summary, .. } => Ok(summary.clone().unwrap_or_default()),
        ToolResultPart::Blob { .. } => Err(ModelError::InvalidRequest(
            "blob tool result parts are not supported by OpenAI-compatible providers in M2-T04.5"
                .to_owned(),
        )),
        _ => Err(ModelError::InvalidRequest(
            "unsupported tool result part for OpenAI-compatible providers".to_owned(),
        )),
    }
}

fn openai_tool(tool: &ToolDescriptor) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        },
    })
}

fn responses_tool(tool: &ToolDescriptor) -> Value {
    json!({
        "type": "function",
        "name": tool.name,
        "description": tool.description,
        "parameters": tool.input_schema,
    })
}

fn chat_response_to_stream(response: Value) -> Result<ModelStream, ModelError> {
    let response: ChatCompletionResponse = serde_json::from_value(response).map_err(|error| {
        ModelError::UnexpectedResponse(format!("invalid OpenAI-compatible response: {error}"))
    })?;
    let usage = usage(response.usage.as_ref());
    let choice = response.choices.into_iter().next().ok_or_else(|| {
        ModelError::UnexpectedResponse("OpenAI-compatible response had no choices".to_owned())
    })?;
    let mut events = vec![ModelStreamEvent::MessageStart {
        message_id: response.id,
        usage: usage.clone(),
    }];
    let mut index = 0;

    if let Some(content) = choice.message.content {
        if !content.is_empty() {
            events.push(ModelStreamEvent::ContentBlockStart {
                index,
                content_type: ContentType::Text,
            });
            events.push(ModelStreamEvent::ContentBlockDelta {
                index,
                delta: ContentDelta::Text(content),
            });
            events.push(ModelStreamEvent::ContentBlockStop { index });
            index += 1;
        }
    }

    for tool_call in choice.message.tool_calls {
        events.push(ModelStreamEvent::ContentBlockStart {
            index,
            content_type: ContentType::ToolUse,
        });
        events.push(ModelStreamEvent::ContentBlockDelta {
            index,
            delta: ContentDelta::ToolUseStart {
                id: tool_call.id,
                name: tool_call.function.name,
            },
        });
        if !tool_call.function.arguments.is_empty() {
            events.push(ModelStreamEvent::ContentBlockDelta {
                index,
                delta: ContentDelta::ToolUseInputJson(tool_call.function.arguments),
            });
        }
        events.push(ModelStreamEvent::ContentBlockStop { index });
        index += 1;
    }

    events.push(ModelStreamEvent::MessageDelta {
        stop_reason: choice.finish_reason.as_deref().map(stop_reason),
        usage_delta: usage,
    });
    events.push(ModelStreamEvent::MessageStop);
    Ok(Box::pin(stream::iter(events)))
}

pub(crate) fn usage(value: Option<&OpenAiUsage>) -> UsageSnapshot {
    UsageSnapshot {
        input_tokens: value
            .and_then(|usage| usage.prompt_tokens)
            .unwrap_or_default(),
        output_tokens: value
            .and_then(|usage| usage.completion_tokens)
            .unwrap_or_default(),
        cache_read_tokens: value
            .and_then(|usage| usage.prompt_tokens_details.as_ref())
            .and_then(|details| details.cached_tokens)
            .unwrap_or_default(),
        cache_write_tokens: 0,
        cost_micros: 0,
    }
}

pub(crate) fn stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::EndTurn,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "length" => StopReason::MaxIterations,
        _ => StopReason::Error(reason.to_owned()),
    }
}

fn retry_delay(backoff: &Backoff, attempt: u32) -> Duration {
    match backoff {
        Backoff::Fixed(delay) => *delay,
        Backoff::Exponential {
            initial,
            factor,
            cap,
        } => {
            let multiplier = factor.powi(attempt.saturating_sub(1) as i32);
            initial.mul_f32(multiplier).min(*cap)
        }
    }
}

#[allow(unused_macros)]
macro_rules! openai_compatible_provider {
    (
        provider = $provider:ident,
        provider_id = $provider_id:literal,
        env = $env_name:ident => $env_value:literal,
        base_url = $base_url:literal,
        path = $path:literal,
        context_window = $context_window:literal,
        max_output_tokens = $max_output_tokens:literal,
        models = [$(($model_id:literal, $display_name:literal)),+ $(,)?]
    ) => {
        pub const $env_name: &str = $env_value;

        #[derive(Clone)]
        pub struct $provider {
            client: $crate::openai_compatible::OpenAiCompatibleClient,
        }

        impl $provider {
            pub fn from_api_key(api_key: impl Into<String>) -> Self {
                Self {
                    client: $crate::openai_compatible::OpenAiCompatibleClient::from_api_key(
                        api_key,
                        $base_url,
                    )
                    .with_chat_completions_path($path),
                }
            }

            #[must_use]
            pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
                self.client = self.client.with_base_url(base_url);
                self
            }
        }

        impl $crate::openai_compatible::OpenAiCompatibleProviderExt for $provider {
            fn client(&self) -> &$crate::openai_compatible::OpenAiCompatibleClient {
                &self.client
            }
        }

        #[async_trait::async_trait]
        impl $crate::ModelProvider for $provider {
            fn provider_id(&self) -> &str {
                $provider_id
            }

            fn supported_models(&self) -> Vec<$crate::ModelDescriptor> {
                vec![$(descriptor($model_id, $display_name)),+]
            }

            async fn infer(
                &self,
                req: $crate::ModelRequest,
                ctx: $crate::InferContext,
            ) -> Result<$crate::ModelStream, harness_contracts::ModelError> {
                $crate::openai_compatible::OpenAiCompatibleProviderExt::infer_openai_compatible(
                    self,
                    req,
                    ctx,
                )
                .await
            }

            fn supports_tools(&self) -> bool {
                true
            }
        }

        fn descriptor(model_id: &str, display_name: &str) -> $crate::ModelDescriptor {
            $crate::ModelDescriptor {
                provider_id: $provider_id.to_owned(),
                model_id: model_id.to_owned(),
                display_name: display_name.to_owned(),
                context_window: $context_window,
                max_output_tokens: $max_output_tokens,
                capabilities: $crate::ModelCapabilities {
                    supports_tools: true,
                    supports_vision: false,
                    supports_thinking: false,
                    supports_prompt_cache: false,
                    supports_tool_reference: false,
                    tool_reference_beta_header: None,
                },
                pricing: None,
            }
        }
    };
}

#[allow(unused_imports)]
pub(crate) use openai_compatible_provider;

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    #[serde(default)]
    choices: Vec<ChatCompletionChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatToolCall>,
}

#[derive(Debug, Deserialize)]
struct ChatToolCall {
    id: String,
    function: ChatToolCallFunction,
}

#[derive(Debug, Deserialize)]
struct ChatToolCallFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PromptTokensDetails {
    cached_tokens: Option<u64>,
}
