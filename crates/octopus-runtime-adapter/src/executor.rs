use api::{
    AnthropicClient, AuthSource, InputMessage, MessageRequest, MessageResponse, OpenAiCompatClient,
    OpenAiCompatConfig, OutputContentBlock,
};
use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget};
use serde_json::{json, Value};

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
}

#[derive(Debug, Default)]
pub struct LiveRuntimeModelExecutor {
    http: reqwest::Client,
}

impl LiveRuntimeModelExecutor {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
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
        .json(&json!({
            "model": target.model_id,
            "input": input,
            "instructions": system_prompt,
            "stream": false,
        }))
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
        .json(&json!({
            "systemInstruction": system_prompt.map(|value| {
                json!({
                    "parts": [{ "text": value }]
                })
            }),
            "contents": [{
                "role": "user",
                "parts": [{ "text": input }]
            }]
        }))
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
        max_tokens: 2_048,
        messages: vec![InputMessage::user_text(input)],
        system: system_prompt.map(|value| value.to_string()),
        tools: None,
        tool_choice: None,
        stream: false,
    }
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
