use api::{
    build_http_client_or_default, AnthropicClient, AuthSource, InputContentBlock, InputMessage,
    MessageRequest, MessageResponse, OpenAiCompatClient, OpenAiCompatConfig, OutputContentBlock,
    ToolChoice, ToolDefinition, ToolResultContentBlock,
};
use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget};
use runtime::{AssistantEvent, ContentBlock, ConversationMessage, MessageRole, TokenUsage};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConversationRequest {
    pub system_prompt: Vec<String>,
    pub messages: Vec<ConversationMessage>,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResponse {
    pub content: String,
    pub request_id: Option<String>,
    pub total_tokens: Option<u32>,
}

#[async_trait]
pub trait RuntimeModelExecutor: Send + Sync {
    async fn execute_turn(
        &self,
        target: &ResolvedExecutionTarget,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ExecutionResponse, AppError>;

    async fn execute_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        let fallback_input = request
            .messages
            .iter()
            .rev()
            .find_map(|message| {
                if message.role != MessageRole::User {
                    return None;
                }
                Some(extract_text_content(message))
            })
            .unwrap_or_default();
        let system_prompt =
            (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n"));
        let response = self
            .execute_turn(target, fallback_input.as_str(), system_prompt.as_deref())
            .await?;
        let mut events = vec![AssistantEvent::TextDelta(response.content)];
        if let Some(total_tokens) = response.total_tokens {
            events.push(AssistantEvent::Usage(TokenUsage {
                input_tokens: 0,
                output_tokens: total_tokens,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }));
        }
        events.push(AssistantEvent::MessageStop);
        Ok(events)
    }
}

#[derive(Debug)]
pub struct LiveRuntimeModelExecutor {
    http: reqwest::Client,
}

impl LiveRuntimeModelExecutor {
    pub fn new() -> Self {
        Self {
            http: build_http_client_or_default(),
        }
    }
}

impl Default for LiveRuntimeModelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuntimeModelExecutor for LiveRuntimeModelExecutor {
    async fn execute_turn(
        &self,
        target: &ResolvedExecutionTarget,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        match target.protocol_family.as_str() {
            "anthropic_messages" => execute_anthropic_messages(target, input, system_prompt).await,
            "openai_chat" => execute_openai_chat(target, input, system_prompt).await,
            "openai_responses" => {
                execute_openai_responses(&self.http, target, input, system_prompt).await
            }
            "gemini_native" => {
                execute_gemini_native(&self.http, target, input, system_prompt).await
            }
            other => Err(AppError::runtime(format!(
                "runtime execution does not support protocol family `{other}` yet"
            ))),
        }
    }

    async fn execute_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        match target.protocol_family.as_str() {
            "anthropic_messages" => {
                execute_message_protocol_conversation(target, request, ProviderProtocol::Anthropic)
                    .await
            }
            "openai_chat" => {
                execute_message_protocol_conversation(target, request, ProviderProtocol::OpenAiChat)
                    .await
            }
            "openai_responses" | "gemini_native" if request.tools.is_empty() => {
                let input = request
                    .messages
                    .iter()
                    .rev()
                    .find_map(|message| {
                        if message.role != MessageRole::User {
                            return None;
                        }
                        Some(extract_text_content(message))
                    })
                    .unwrap_or_default();
                let system_prompt = (!request.system_prompt.is_empty())
                    .then(|| request.system_prompt.join("\n\n"));
                let response = self
                    .execute_turn(target, input.as_str(), system_prompt.as_deref())
                    .await?;
                let mut events = vec![AssistantEvent::TextDelta(response.content)];
                if let Some(total_tokens) = response.total_tokens {
                    events.push(AssistantEvent::Usage(TokenUsage {
                        input_tokens: 0,
                        output_tokens: total_tokens,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }));
                }
                events.push(AssistantEvent::MessageStop);
                Ok(events)
            }
            "openai_responses" | "gemini_native" => Err(AppError::runtime(format!(
                "runtime tool loop does not support protocol family `{}` with tool-enabled turns yet",
                target.protocol_family
            ))),
            other => Err(AppError::runtime(format!(
                "runtime execution does not support protocol family `{other}` yet"
            ))),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockRuntimeModelExecutor;

#[async_trait]
impl RuntimeModelExecutor for MockRuntimeModelExecutor {
    async fn execute_turn(
        &self,
        target: &ResolvedExecutionTarget,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        let prompt_prefix = system_prompt
            .map(|value| format!(" [{value}]"))
            .unwrap_or_default();
        Ok(ExecutionResponse {
            content: format!(
                "Mock assistant response via {}:{}{} -> {}",
                target.provider_id, target.surface, prompt_prefix, input
            ),
            request_id: Some("mock-request-id".into()),
            total_tokens: Some(32),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderProtocol {
    Anthropic,
    OpenAiChat,
}

async fn execute_anthropic_messages(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ExecutionResponse, AppError> {
    let api_key = resolve_api_key(target)?;
    let client = AnthropicClient::from_auth(AuthSource::ApiKey(api_key)).with_base_url(
        target
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".into()),
    );
    let request = message_request(target, input, system_prompt);
    let response: MessageResponse = client
        .send_message(&request)
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    Ok(ExecutionResponse {
        content: flatten_output_content(&response.content),
        request_id: response.request_id.clone(),
        total_tokens: Some(response.total_tokens()),
    })
}

async fn execute_message_protocol_conversation(
    target: &ResolvedExecutionTarget,
    request: &RuntimeConversationRequest,
    protocol: ProviderProtocol,
) -> Result<Vec<AssistantEvent>, AppError> {
    let api_key = resolve_api_key(target)?;
    let request = conversation_message_request(target, request);
    let response = match protocol {
        ProviderProtocol::Anthropic => AnthropicClient::from_auth(AuthSource::ApiKey(api_key))
            .with_base_url(
                target
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com".into()),
            )
            .send_message(&request)
            .await
            .map_err(|error| AppError::runtime(error.to_string()))?,
        ProviderProtocol::OpenAiChat => {
            let config = compat_config_for_provider(&target.provider_id);
            OpenAiCompatClient::new(api_key, config)
                .with_base_url(
                    target
                        .base_url
                        .clone()
                        .unwrap_or_else(|| config.default_base_url.to_string()),
                )
                .send_message(&request)
                .await
                .map_err(|error| AppError::runtime(error.to_string()))?
        }
    };
    Ok(response_to_events(response))
}

async fn execute_openai_chat(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ExecutionResponse, AppError> {
    let api_key = resolve_api_key(target)?;
    let config = compat_config_for_provider(&target.provider_id);
    let client = OpenAiCompatClient::new(api_key, config).with_base_url(
        target
            .base_url
            .clone()
            .unwrap_or_else(|| config.default_base_url.to_string()),
    );
    let request = message_request(target, input, system_prompt);
    let response: MessageResponse = client
        .send_message(&request)
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    Ok(ExecutionResponse {
        content: flatten_output_content(&response.content),
        request_id: response.request_id.clone(),
        total_tokens: Some(response.total_tokens()),
    })
}

async fn execute_openai_responses(
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ExecutionResponse, AppError> {
    let api_key = resolve_api_key(target)?;
    let base_url = target
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".into());
    let response = http
        .post(format!("{}/responses", base_url.trim_end_matches('/')))
        .bearer_auth(api_key)
        .json(&build_openai_responses_request_body(
            target,
            input,
            system_prompt,
        ))
        .send()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    if !status.is_success() {
        return Err(AppError::runtime(format!(
            "provider returned {} for responses request: {}",
            status, body
        )));
    }

    Ok(ExecutionResponse {
        content: extract_responses_output_text(&body),
        request_id,
        total_tokens: body
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    })
}

async fn execute_gemini_native(
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ExecutionResponse, AppError> {
    let api_key = resolve_api_key(target)?;
    let base_url = target
        .base_url
        .clone()
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".into());
    let response = http
        .post(format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            base_url.trim_end_matches('/'),
            target.model_id,
            api_key
        ))
        .json(&build_gemini_native_request_body(
            target,
            input,
            system_prompt,
        ))
        .send()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    if !status.is_success() {
        return Err(AppError::runtime(format!(
            "provider returned {} for gemini request: {}",
            status, body
        )));
    }

    let content = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    Ok(ExecutionResponse {
        content,
        request_id: None,
        total_tokens: body
            .get("usageMetadata")
            .and_then(|usage| usage.get("totalTokenCount"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    })
}

fn resolve_api_key(target: &ResolvedExecutionTarget) -> Result<String, AppError> {
    if let Some(reference) = target.credential_ref.as_deref() {
        if let Some(env_key) = reference.strip_prefix("env:") {
            return std::env::var(env_key).map_err(|_| {
                AppError::invalid_input(format!(
                    "missing configured credential env var `{env_key}` for provider `{}`",
                    target.provider_id
                ))
            });
        }
        if !reference.trim().is_empty() {
            return Ok(reference.to_string());
        }
    }

    let fallback_env = match target.provider_id.as_str() {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "xai" => "XAI_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "moonshot" => "MOONSHOT_API_KEY",
        "bigmodel" => "BIGMODEL_API_KEY",
        "qwen" => "DASHSCOPE_API_KEY",
        "ark" => "ARK_API_KEY",
        "google" => "GOOGLE_API_KEY",
        other => {
            return Err(AppError::invalid_input(format!(
                "no credential mapping for provider `{other}`"
            )))
        }
    };

    std::env::var(fallback_env).map_err(|_| {
        AppError::invalid_input(format!(
            "missing configured credential env var `{fallback_env}` for provider `{}`",
            target.provider_id
        ))
    })
}

fn compat_config_for_provider(provider_id: &str) -> OpenAiCompatConfig {
    match provider_id {
        "xai" => OpenAiCompatConfig::xai(),
        "openai" => OpenAiCompatConfig::openai(),
        "deepseek" => OpenAiCompatConfig {
            provider_name: "DeepSeek",
            api_key_env: "DEEPSEEK_API_KEY",
            base_url_env: "DEEPSEEK_BASE_URL",
            default_base_url: "https://api.deepseek.com",
        },
        "minimax" => OpenAiCompatConfig {
            provider_name: "MiniMax",
            api_key_env: "MINIMAX_API_KEY",
            base_url_env: "MINIMAX_BASE_URL",
            default_base_url: "https://api.minimaxi.com",
        },
        "moonshot" => OpenAiCompatConfig {
            provider_name: "Moonshot",
            api_key_env: "MOONSHOT_API_KEY",
            base_url_env: "MOONSHOT_BASE_URL",
            default_base_url: "https://api.moonshot.cn/v1",
        },
        "bigmodel" => OpenAiCompatConfig {
            provider_name: "BigModel",
            api_key_env: "BIGMODEL_API_KEY",
            base_url_env: "BIGMODEL_BASE_URL",
            default_base_url: "https://open.bigmodel.cn/api/paas/v4",
        },
        "qwen" => OpenAiCompatConfig {
            provider_name: "Qwen",
            api_key_env: "DASHSCOPE_API_KEY",
            base_url_env: "DASHSCOPE_BASE_URL",
            default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        },
        other => OpenAiCompatConfig {
            provider_name: "OpenAI-Compatible",
            api_key_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: match other {
                "ark" => "https://ark.cn-beijing.volces.com/api/v3",
                _ => "https://api.openai.com/v1",
            },
        },
    }
}

fn message_request(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> MessageRequest {
    MessageRequest {
        model: target.model_id.clone(),
        max_tokens: target_max_output_tokens(target),
        messages: vec![InputMessage::user_text(input)],
        system: system_prompt.map(|value| value.to_string()),
        tools: None,
        tool_choice: None,
        stream: false,
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: None,
        reasoning_effort: None,
    }
}

fn conversation_message_request(
    target: &ResolvedExecutionTarget,
    request: &RuntimeConversationRequest,
) -> MessageRequest {
    MessageRequest {
        model: target.model_id.clone(),
        max_tokens: target.max_output_tokens.unwrap_or(1024),
        messages: convert_messages(&request.messages),
        system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
        tools: (!request.tools.is_empty()).then(|| request.tools.clone()),
        tool_choice: (!request.tools.is_empty()).then_some(ToolChoice::Auto),
        stream: false,
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: None,
        reasoning_effort: None,
    }
}

fn build_openai_responses_request_body(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Value {
    json!({
        "model": target.model_id,
        "input": input,
        "instructions": system_prompt,
        "stream": false,
        "max_output_tokens": target_max_output_tokens(target),
    })
}

fn build_gemini_native_request_body(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Value {
    json!({
        "systemInstruction": system_prompt.map(|value| {
            json!({
                "parts": [{ "text": value }]
            })
        }),
        "contents": [{
            "role": "user",
            "parts": [{ "text": input }]
        }],
        "generationConfig": {
            "maxOutputTokens": target_max_output_tokens(target),
        }
    })
}

fn target_max_output_tokens(target: &ResolvedExecutionTarget) -> u32 {
    target.max_output_tokens.unwrap_or(2_048)
}

fn flatten_output_content(content: &[OutputContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            OutputContentBlock::Text { text } => Some(text.as_str()),
            OutputContentBlock::Thinking { thinking, .. } => Some(thinking.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn response_to_events(response: MessageResponse) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    for block in response.content {
        match block {
            OutputContentBlock::Text { text } => {
                if !text.is_empty() {
                    events.push(AssistantEvent::TextDelta(text));
                }
            }
            OutputContentBlock::ToolUse { id, name, input } => {
                events.push(AssistantEvent::ToolUse {
                    id,
                    name,
                    input: input.to_string(),
                });
            }
            OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
        }
    }
    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    events
}

fn extract_text_content(message: &ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn extract_responses_output_text(body: &Value) -> String {
    if let Some(output_text) = body.get("output_text").and_then(Value::as_str) {
        return output_text.to_string();
    }

    body.get("output")
        .and_then(Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .flat_map(|output| {
                    output
                        .get("content")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|item| {
                    item.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            item.get("output_text")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        })
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        build_gemini_native_request_body, build_openai_responses_request_body, message_request,
        ResolvedExecutionTarget,
    };

    fn target(protocol_family: &str, max_output_tokens: Option<u32>) -> ResolvedExecutionTarget {
        ResolvedExecutionTarget {
            configured_model_id: "configured".into(),
            configured_model_name: "Configured".into(),
            provider_id: "openai".into(),
            registry_model_id: "gpt-5".into(),
            model_id: "gpt-5".into(),
            surface: "conversation".into(),
            protocol_family: protocol_family.into(),
            credential_ref: Some("env:OPENAI_API_KEY".into()),
            base_url: Some("https://api.openai.com/v1".into()),
            max_output_tokens,
            capabilities: Vec::new(),
        }
    }

    #[test]
    fn message_request_prefers_target_max_output_tokens() {
        let request = message_request(&target("openai_chat", Some(4096)), "hello", Some("system"));

        assert_eq!(request.max_tokens, 4096);
    }

    #[test]
    fn message_request_falls_back_to_default_when_override_missing() {
        let request = message_request(&target("anthropic_messages", None), "hello", None);

        assert_eq!(request.max_tokens, 2048);
    }

    #[test]
    fn responses_payload_includes_max_output_tokens_override() {
        let body = build_openai_responses_request_body(
            &target("openai_responses", Some(3072)),
            "hello",
            Some("system"),
        );

        assert_eq!(body["max_output_tokens"], 3072);
    }

    #[test]
    fn gemini_payload_includes_generation_config_max_output_tokens() {
        let body = build_gemini_native_request_body(
            &target("gemini_native", Some(1024)),
            "hello",
            Some("system"),
        );

        assert_eq!(body["generationConfig"]["maxOutputTokens"], 1024);
    }
}
