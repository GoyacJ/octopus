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
    AnthropicMessagesAdapter, CacheControlStrategy, ModelId, ModelRequest, ModelRole,
    ProtocolAdapter, ResponseFormat, StreamBytes, ThinkingConfig, ToolSchema,
};

struct StaticVault;

#[async_trait::async_trait]
impl SecretVault for StaticVault {
    async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue::new("sk-ant-test"))
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Ok(())
    }
}

fn sample_request(stream: bool) -> ModelRequest {
    ModelRequest {
        model: ModelId("claude-opus-4-6".to_string()),
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
        role: ModelRole::Main,
        cache_breakpoints: vec![CacheBreakpoint {
            position: 0,
            ttl: CacheTtl::FiveMinutes,
        }],
        response_format: Some(ResponseFormat::Text),
        thinking: Some(ThinkingConfig {
            enabled: false,
            budget_tokens: None,
        }),
        cache_control: CacheControlStrategy::PromptCaching {
            breakpoints: vec!["system", "tools"],
        },
        max_tokens: Some(256),
        temperature: Some(0.2),
        stream,
    }
}

fn anthropic_provider() -> octopus_sdk_model::Provider {
    octopus_sdk_model::Provider {
        id: octopus_sdk_model::ProviderId("anthropic".to_string()),
        display_name: "Anthropic".to_string(),
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
    let adapter = AnthropicMessagesAdapter;
    let client = reqwest::Client::new();
    let headers = adapter
        .auth_headers(&StaticVault, &anthropic_provider())
        .await
        .expect("auth headers should resolve");
    let body = adapter
        .to_request(request)
        .expect("request body should serialize");
    let response = client
        .post(format!("{}/v1/messages", server.uri()))
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
async fn anthropic_non_stream_text_turn_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-test"))
        .and(body_partial_json(json!({
            "model": "claude-opus-4-6",
            "stream": false,
            "system": ["You are precise."],
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({
                    "id": "msg_1",
                    "content": [{ "type": "text", "text": "Hello from Anthropic" }],
                    "usage": {
                        "input_tokens": 12,
                        "output_tokens": 4,
                        "cache_creation_input_tokens": 0,
                        "cache_read_input_tokens": 0
                    },
                    "stop_reason": "end_turn"
                })),
        )
        .mount(&server)
        .await;

    let events = send_request(&server, &sample_request(false)).await;

    assert_eq!(
        events,
        vec![
            AssistantEvent::TextDelta("Hello from Anthropic".to_string()),
            AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                input_tokens: 12,
                output_tokens: 4,
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
async fn anthropic_stream_tool_use_turn_collapses_tool_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    concat!(
                        "event: message_start\n",
                        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_2\"}}\n\n",
                        "event: content_block_start\n",
                        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"search\",\"input\":{}}}\n\n",
                        "event: content_block_delta\n",
                        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\":\\\"docs\\\"}\"}}\n\n",
                        "event: content_block_stop\n",
                        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
                        "event: message_delta\n",
                        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"input_tokens\":16,\"output_tokens\":3,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}}\n\n",
                        "event: message_stop\n",
                        "data: {\"type\":\"message_stop\"}\n\n"
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
                id: octopus_sdk_contracts::ToolCallId("toolu_1".to_string()),
                name: "search".to_string(),
                input: json!({ "query": "docs" }),
            },
            AssistantEvent::Usage(octopus_sdk_contracts::Usage {
                input_tokens: 16,
                output_tokens: 3,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ]
    );
}

#[tokio::test]
async fn anthropic_stream_emits_prompt_cache_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    concat!(
                        "event: content_block_delta\n",
                        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"cached\"}}\n\n",
                        "event: message_delta\n",
                        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":20,\"output_tokens\":5,\"cache_creation_input_tokens\":10,\"cache_read_input_tokens\":2}}\n\n",
                        "event: message_stop\n",
                        "data: {\"type\":\"message_stop\"}\n\n"
                    ),
                    "text/event-stream",
                ),
        )
        .mount(&server)
        .await;

    let events = send_request(&server, &sample_request(true)).await;

    assert!(events.iter().any(|event| matches!(
        event,
        AssistantEvent::PromptCache(prompt)
            if prompt.cache_creation_input_tokens > 0 && prompt.cache_read_input_tokens > 0
    )));
}
