mod anthropic_messages;
mod gemini_native;
mod openai_chat;
mod openai_responses;

use std::time::Duration;

use api::{
    AnthropicClient, AuthSource, InputContentBlock, InputMessage, MessageRequest,
    OpenAiCompatClient, OpenAiCompatConfig, OutputContentBlock, ToolChoice, ToolResultContentBlock,
};
use octopus_core::{
    AppError, ResolvedExecutionTarget, ResolvedRequestAuthMode, ResolvedRequestPolicy,
};
use runtime::{ContentBlock, ConversationMessage, MessageRole};

use super::{
    stream_bridge::{runtime_conversation_execution_from_turn_events, ProviderStreamBridge},
    RuntimeConversationExecution, RuntimeConversationRequest,
};

pub(crate) use anthropic_messages::AnthropicMessagesDriver;
pub(crate) use gemini_native::GeminiNativeDriver;
pub(crate) use openai_chat::OpenAiChatDriver;
pub(crate) use openai_responses::OpenAiResponsesDriver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderProtocol {
    Anthropic,
    OpenAiChat,
}

pub(crate) async fn execute_message_protocol_streaming_conversation(
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    request: &RuntimeConversationRequest,
    protocol: ProviderProtocol,
) -> Result<RuntimeConversationExecution, AppError> {
    let request = conversation_message_request(target, request).with_streaming();
    let mut bridge = ProviderStreamBridge::default();
    let mut turn_events = Vec::new();

    match protocol {
        ProviderProtocol::Anthropic => {
            let mut stream = AnthropicClient::from_auth(auth_source_from_request_policy(
                request_policy,
                &target.protocol_family,
            )?)
            .with_base_url(request_policy.base_url.clone())
            .stream_message(&request)
            .await
            .map_err(|error| AppError::runtime(error.to_string()))?;
            if let Some(event) = bridge.request_metadata(stream.request_id().map(ToOwned::to_owned))
            {
                turn_events.push(event);
            }
            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| AppError::runtime(error.to_string()))?
            {
                turn_events.extend(bridge.ingest(event)?);
            }
        }
        ProviderProtocol::OpenAiChat => {
            let config = compat_config_for_provider(&target.provider_id);
            let mut stream = OpenAiCompatClient::new(
                bearer_token_from_request_policy(request_policy, &target.protocol_family)?,
                config,
            )
            .with_base_url(request_policy.base_url.clone())
            .stream_message(&request)
            .await
            .map_err(|error| AppError::runtime(error.to_string()))?;
            if let Some(event) = bridge.request_metadata(stream.request_id().map(ToOwned::to_owned))
            {
                turn_events.push(event);
            }
            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| AppError::runtime(error.to_string()))?
            {
                turn_events.extend(bridge.ingest(event)?);
            }
        }
    }

    Ok(runtime_conversation_execution_from_turn_events(
        turn_events,
        Vec::new(),
    ))
}

pub(crate) fn auth_source_from_request_policy(
    request_policy: &ResolvedRequestPolicy,
    protocol_family: &str,
) -> Result<AuthSource, AppError> {
    match request_policy.auth.mode {
        ResolvedRequestAuthMode::None => Ok(AuthSource::None),
        ResolvedRequestAuthMode::BearerToken => Ok(AuthSource::BearerToken(
            auth_value(request_policy, protocol_family)?.to_string(),
        )),
        ResolvedRequestAuthMode::Header => match request_policy.auth.name.as_deref() {
            Some("x-api-key") => Ok(AuthSource::ApiKey(
                auth_value(request_policy, protocol_family)?.to_string(),
            )),
            Some(header) => Err(AppError::invalid_input(format!(
                "protocol family `{protocol_family}` does not support auth header `{header}`"
            ))),
            None => Err(AppError::invalid_input(format!(
                "protocol family `{protocol_family}` is missing an auth header name"
            ))),
        },
        ResolvedRequestAuthMode::QueryParam => Err(AppError::invalid_input(format!(
            "protocol family `{protocol_family}` does not support query auth transport"
        ))),
    }
}

pub(crate) fn bearer_token_from_request_policy(
    request_policy: &ResolvedRequestPolicy,
    protocol_family: &str,
) -> Result<String, AppError> {
    if request_policy.auth.mode != ResolvedRequestAuthMode::BearerToken {
        return Err(AppError::invalid_input(format!(
            "protocol family `{protocol_family}` requires bearer auth transport"
        )));
    }
    Ok(auth_value(request_policy, protocol_family)?.to_string())
}

pub(crate) fn apply_request_policy(
    mut builder: reqwest::RequestBuilder,
    request_policy: &ResolvedRequestPolicy,
    protocol_family: &str,
) -> Result<reqwest::RequestBuilder, AppError> {
    if let Some(timeout_ms) = request_policy.timeout_ms {
        builder = builder.timeout(Duration::from_millis(timeout_ms));
    }

    for (name, value) in &request_policy.headers {
        builder = builder.header(name, value);
    }

    match request_policy.auth.mode {
        ResolvedRequestAuthMode::None => {}
        ResolvedRequestAuthMode::BearerToken => {
            builder = builder.bearer_auth(auth_value(request_policy, protocol_family)?);
        }
        ResolvedRequestAuthMode::Header => {
            let name = request_policy.auth.name.as_deref().ok_or_else(|| {
                AppError::invalid_input(format!(
                    "protocol family `{protocol_family}` is missing an auth header name"
                ))
            })?;
            builder = builder.header(name, auth_value(request_policy, protocol_family)?);
        }
        ResolvedRequestAuthMode::QueryParam => {
            let name = request_policy.auth.name.as_deref().ok_or_else(|| {
                AppError::invalid_input(format!(
                    "protocol family `{protocol_family}` is missing an auth query parameter name"
                ))
            })?;
            builder = builder.query(&[(name, auth_value(request_policy, protocol_family)?)]);
        }
    }

    Ok(builder)
}

pub(crate) fn compat_config_for_provider(provider_id: &str) -> OpenAiCompatConfig {
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

pub(crate) fn message_request(
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

pub(crate) fn target_max_output_tokens(target: &ResolvedExecutionTarget) -> u32 {
    target.max_output_tokens.unwrap_or(2_048)
}

pub(crate) fn flatten_output_content(content: &[OutputContentBlock]) -> String {
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

fn auth_value<'a>(
    request_policy: &'a ResolvedRequestPolicy,
    protocol_family: &str,
) -> Result<&'a str, AppError> {
    request_policy
        .auth
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "protocol family `{protocol_family}` is missing an auth credential value"
            ))
        })
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
