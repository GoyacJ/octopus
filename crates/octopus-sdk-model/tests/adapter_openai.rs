use futures::TryStreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::json;
use wiremock::{
    matchers::{body_partial_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use octopus_sdk_contracts::{
    AssistantEvent, CacheBreakpoint, CacheTtl, ContentBlock, Message, Role, SecretValue,
    SecretVault, StopReason, VaultError,
};
use octopus_sdk_model::{
    CacheControlStrategy, ModelId, ModelRequest, ModelRole, OpenAiChatAdapter, ProtocolAdapter,
    ResponseFormat, StreamBytes, ThinkingConfig, ToolSchema,
};

struct StaticVault;

#[async_trait::async_trait]
impl SecretVault for StaticVault {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new("sk-openai-test"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

fn sample_request(stream: bool) -> ModelRequest {
    ModelRequest {
        model: ModelId("deepseek-chat".to_string()),
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
            input_schema: json!({
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }),
        }],
        role: ModelRole::Fast,
        cache_breakpoints: vec![CacheBreakpoint {
            position: 0,
            ttl: CacheTtl::FiveMinutes,
        }],
        response_format: Some(ResponseFormat::Text),
        thinking: Some(ThinkingConfig {
            enabled: false,
            budget_tokens: None,
        }),
        cache_control: CacheControlStrategy::None,
        max_tokens: Some(256),
        temperature: Some(0.2),
        stream,
    }
}

fn compat_provider() -> octopus_sdk_model::Provider {
    octopus_sdk_model::Provider {
        id: octopus_sdk_model::ProviderId("deepseek".to_string()),
        display_name: "DeepSeek".to_string(),
        status: octopus_sdk_model::ProviderStatus::Active,
        auth: octopus_sdk_model::AuthKind::ApiKey,
        surfaces: vec![],
    }
}

fn header_map(headers: Vec<(HeaderName, HeaderValue)>) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        map.insert(name, value);
    }
    map
}

fn response_byte_stream(response: reqwest::Response) -> StreamBytes {
    Box::pin(futures::stream::try_unfold(
        response,
        |mut response| async move {
            match response.chunk().await {
                Ok(Some(chunk)) => Ok(Some((chunk, response))),
                Ok(None) => Ok(None),
                Err(error) => Err(octopus_sdk_model::ModelError::Transport(error)),
            }
        },
    ))
}

async fn send_request(server: &MockServer, request: &ModelRequest) -> Vec<AssistantEvent> {
    let adapter = OpenAiChatAdapter;
    let client = reqwest::Client::new();
    let headers = adapter
        .auth_headers(&StaticVault, &compat_provider())
        .await
        .expect("auth headers should resolve");
    let body = adapter
        .to_request(request)
        .expect("request body should serialize");
    let response = client
        .post(format!("{}/chat/completions", server.uri()))
        .headers(header_map(headers))
        .json(&body)
        .send()
        .await
        .expect("request should succeed");
    let raw = response_byte_stream(response);
    adapter
        .parse_stream(raw)
        .expect("stream should parse")
        .try_collect::<Vec<_>>()
        .await
        .expect("assistant events should decode")
}

#[tokio::test]
async fn openai_non_stream_text_turn_normalizes_usage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer sk-openai-test"))
        .and(body_partial_json(json!({
            "model": "deepseek-chat",
            "stream": false,
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({
                    "id": "chatcmpl_1",
                    "model": "deepseek-chat",
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "Hello from OpenAI compat"
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 11,
                        "completion_tokens": 5
                    }
                })),
        )
        .mount(&server)
        .await;

    let events = send_request(&server, &sample_request(false)).await;

    assert_eq!(
        events,
        vec![
            AssistantEvent::TextDelta("Hello from OpenAI compat".to_string()),
            AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                input_tokens: 11,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ]
    );
}

#[tokio::test]
async fn openai_stream_tool_call_turn_aggregates_arguments() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    concat!(
                        "data: {\"id\":\"chatcmpl_2\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"search\",\"arguments\":\"{\\\"query\\\":\\\"do\"}}]}}]}\n\n",
                        "data: {\"id\":\"chatcmpl_2\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"cs\\\"}\"}}],\"finish_reason\":\"tool_calls\"}}]}\n\n",
                        "data: [DONE]\n\n"
                    ),
                    "text/event-stream",
                ),
        )
        .mount(&server)
        .await;

    let events = send_request(&server, &sample_request(true)).await;

    assert_eq!(
        events,
        vec![
            AssistantEvent::ToolUse {
                id: octopus_sdk_contracts::ToolCallId("call_1".to_string()),
                name: "search".to_string(),
                input: json!({ "query": "docs" }),
            },
            AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ]
    );
}

#[tokio::test]
async fn openai_stream_usage_maps_missing_cache_fields_to_zero() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    concat!(
                        "data: {\"id\":\"chatcmpl_3\",\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
                        "data: {\"id\":\"chatcmpl_3\",\"choices\":[],\"usage\":{\"prompt_tokens\":9,\"completion_tokens\":2}}\n\n",
                        "data: {\"id\":\"chatcmpl_3\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                        "data: [DONE]\n\n"
                    ),
                    "text/event-stream",
                ),
        )
        .mount(&server)
        .await;

    let events = send_request(&server, &sample_request(true)).await;

    assert!(events.contains(&AssistantEvent::TextDelta("Hi".to_string())));
    assert!(
        events.contains(&AssistantEvent::Usage(octopus_sdk_contracts::Usage {
            input_tokens: 9,
            output_tokens: 2,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }))
    );
    assert!(events.contains(&AssistantEvent::MessageStop {
        stop_reason: StopReason::EndTurn,
    }));
}
