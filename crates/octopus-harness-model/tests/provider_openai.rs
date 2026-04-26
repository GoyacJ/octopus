#![cfg(feature = "openai")]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use chrono::Utc;
use futures::StreamExt;
use harness_contracts::{
    BudgetMetric, DeferPolicy, Message, MessageId, MessagePart, MessageRole, ModelError,
    OverflowAction, ProviderRestriction, ResultBudget, StopReason, ToolDescriptor, ToolGroup,
    ToolOrigin, ToolProperties, TrustLevel, UsageSnapshot,
};
use harness_model::{openai::OpenAiProvider, *};
use serde_json::{json, Value};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

fn message(role: MessageRole, parts: Vec<MessagePart>) -> Message {
    Message {
        id: MessageId::new(),
        role,
        parts,
        created_at: Utc::now(),
    }
}

fn request(stream: bool) -> ModelRequest {
    ModelRequest {
        model_id: "gpt-4o-mini".to_owned(),
        messages: vec![message(
            MessageRole::User,
            vec![MessagePart::Text("hello".to_owned())],
        )],
        tools: Some(vec![tool_descriptor()]),
        system: Some("You are precise.".to_owned()),
        temperature: Some(0.2),
        max_tokens: Some(128),
        stream,
        cache_breakpoints: Vec::new(),
        api_mode: ApiMode::ChatCompletions,
        extra: Value::Null,
    }
}

fn tool_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: "search".to_owned(),
        display_name: "Search".to_owned(),
        description: "Search docs".to_owned(),
        category: "search".to_owned(),
        group: ToolGroup::Search,
        version: "1.0.0".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"],
        }),
        output_schema: None,
        dynamic_schema: false,
        properties: ToolProperties {
            is_concurrency_safe: true,
            is_read_only: true,
            is_destructive: false,
            long_running: None,
            defer_policy: DeferPolicy::AlwaysLoad,
        },
        trust_level: TrustLevel::AdminTrusted,
        required_capabilities: Vec::new(),
        budget: ResultBudget {
            metric: BudgetMetric::Chars,
            limit: 4096,
            on_overflow: OverflowAction::Offload,
            preview_head_chars: 512,
            preview_tail_chars: 512,
        },
        provider_restriction: ProviderRestriction::All,
        origin: ToolOrigin::Builtin,
        search_hint: None,
    }
}

fn provider(server: &MockServer) -> OpenAiProvider {
    OpenAiProvider::from_api_key("test-key").with_base_url(server.uri())
}

#[test]
fn openai_provider_metadata_is_stable() {
    let provider = OpenAiProvider::from_api_key("test-key");

    assert_eq!(provider.provider_id(), "openai");
    assert_eq!(
        provider.prompt_cache_style(),
        PromptCacheStyle::OpenAi { auto: true }
    );
    assert!(provider.supports_tools());
    assert!(provider.supports_vision());
    assert!(!provider.supports_thinking());
    assert!(provider
        .supported_models()
        .iter()
        .any(|model| model.model_id == "gpt-4o-mini"));
}

#[tokio::test]
async fn openai_non_stream_request_posts_chat_completions_and_maps_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "world",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "search",
                            "arguments": "{\"query\":\"docs\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 7,
                "completion_tokens": 3,
                "prompt_tokens_details": { "cached_tokens": 2 }
            }
        })))
        .mount(&server)
        .await;

    let events = provider(&server)
        .infer(request(false), InferContext::for_test())
        .await
        .expect("request should succeed")
        .collect::<Vec<_>>()
        .await;

    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::Text("world".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 1,
        delta: ContentDelta::ToolUseStart {
            id: "call_1".to_owned(),
            name: "search".to_owned(),
        },
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 1,
        delta: ContentDelta::ToolUseInputJson("{\"query\":\"docs\"}".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        usage_delta: UsageSnapshot {
            input_tokens: 7,
            output_tokens: 3,
            cache_read_tokens: 2,
            cache_write_tokens: 0,
            cost_micros: 0,
        },
    }));
    assert!(events.contains(&ModelStreamEvent::MessageStop));

    let requests = server.received_requests().await.unwrap();
    let body: Value = requests[0].body_json().unwrap();
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["stream"], false);
    assert_eq!(body["max_tokens"], 128);
    assert!((body["temperature"].as_f64().unwrap() - 0.2).abs() < 0.0001);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][0]["content"], "You are precise.");
    assert_eq!(body["messages"][1]["role"], "user");
    assert_eq!(body["messages"][1]["content"], "hello");
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "search");
}

#[tokio::test]
async fn openai_stream_response_maps_text_tool_usage_and_done() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    concat!(
                        "data: {\"id\":\"chatcmpl_1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"hel\"},\"finish_reason\":null}],\"usage\":null}\n\n",
                        "data: {\"id\":\"chatcmpl_1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}],\"usage\":null}\n\n",
                        "data: {\"id\":\"chatcmpl_1\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"search\",\"arguments\":\"{\\\"query\\\":\"}}]},\"finish_reason\":null}],\"usage\":null}\n\n",
                        "data: {\"id\":\"chatcmpl_1\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"docs\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":4,\"prompt_tokens_details\":{\"cached_tokens\":1}}}\n\n",
                        "data: [DONE]\n\n",
                    ),
                    "text/event-stream",
                ),
        )
        .mount(&server)
        .await;

    let events = provider(&server)
        .infer(request(true), InferContext::for_test())
        .await
        .expect("stream request should start")
        .collect::<Vec<_>>()
        .await;

    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::Text("hel".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::Text("lo".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 1,
        delta: ContentDelta::ToolUseStart {
            id: "call_1".to_owned(),
            name: "search".to_owned(),
        },
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 1,
        delta: ContentDelta::ToolUseInputJson("{\"query\":".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 1,
        delta: ContentDelta::ToolUseInputJson("\"docs\"}".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        usage_delta: UsageSnapshot {
            input_tokens: 8,
            output_tokens: 4,
            cache_read_tokens: 1,
            cache_write_tokens: 0,
            cost_micros: 0,
        },
    }));
    assert!(events.contains(&ModelStreamEvent::MessageStop));

    let requests = server.received_requests().await.unwrap();
    let body: Value = requests[0].body_json().unwrap();
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn openai_retries_rate_limit_and_transient_errors_but_not_auth() {
    let server = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = attempts.clone();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |_req: &Request| {
            if seen.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "0")
                    .set_body_json(json!({ "error": { "message": "rate limited" } }))
            } else {
                ResponseTemplate::new(200).set_body_json(json!({
                    "id": "chatcmpl_1",
                    "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }],
                    "usage": {}
                }))
            }
        })
        .mount(&server)
        .await;

    let stream = provider(&server)
        .infer(request(false), InferContext::for_test())
        .await
        .expect("rate limit should be retried");
    drop(stream);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({ "error": { "message": "bad key" } })),
        )
        .mount(&auth_server)
        .await;

    let err = provider(&auth_server)
        .infer(request(false), InferContext::for_test())
        .await
        .err()
        .expect("auth failure should fail");
    assert!(matches!(err, ModelError::AuthExpired(_)));
    assert_eq!(auth_server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn openai_rejects_unsupported_request_shapes() {
    let provider = OpenAiProvider::from_api_key("test-key");

    let mut unsupported_mode = request(false);
    unsupported_mode.api_mode = ApiMode::Responses;
    assert!(matches!(
        provider
            .infer(unsupported_mode, InferContext::for_test())
            .await
            .err()
            .expect("unsupported mode should fail"),
        ModelError::InvalidRequest(_)
    ));

    let mut cache = request(false);
    cache.cache_breakpoints.push(CacheBreakpoint {
        after_message_id: cache.messages[0].id,
        reason: BreakpointReason::RecentMessage,
    });
    assert!(matches!(
        provider
            .infer(cache, InferContext::for_test())
            .await
            .err()
            .expect("cache breakpoints should fail"),
        ModelError::InvalidRequest(_)
    ));

    let mut thinking = request(false);
    thinking.messages[0].parts = vec![MessagePart::Thinking(harness_contracts::ThinkingBlock {
        text: Some("think".to_owned()),
        provider_id: "openai".to_owned(),
        provider_native: None,
        signature: None,
    })];
    assert!(matches!(
        provider
            .infer(thinking, InferContext::for_test())
            .await
            .err()
            .expect("thinking parts should fail"),
        ModelError::InvalidRequest(_)
    ));
}
