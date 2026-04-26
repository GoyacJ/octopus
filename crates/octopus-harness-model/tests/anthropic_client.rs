#![cfg(feature = "anthropic")]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use chrono::Utc;
use futures::StreamExt;
use harness_contracts::{
    Message, MessageId, MessagePart, MessageRole, ModelError, StopReason, ToolDescriptor,
};
use harness_model::{anthropic::AnthropicProvider, *};
use serde_json::{json, Value};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

fn sample_request(stream: bool) -> ModelRequest {
    ModelRequest {
        model_id: "claude-3-5-sonnet-20241022".to_owned(),
        messages: vec![Message {
            id: MessageId::new(),
            role: MessageRole::User,
            parts: vec![MessagePart::Text("hello".to_owned())],
            created_at: Utc::now(),
        }],
        tools: Some(vec![tool_descriptor()]),
        system: Some("You are precise.".to_owned()),
        temperature: Some(0.2),
        max_tokens: Some(128),
        stream,
        cache_breakpoints: Vec::new(),
        api_mode: ApiMode::Messages,
        extra: Value::Null,
    }
}

fn tool_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        name: "search".to_owned(),
        display_name: "Search".to_owned(),
        description: "Search docs".to_owned(),
        category: "search".to_owned(),
        group: harness_contracts::ToolGroup::Search,
        version: "1.0.0".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"],
        }),
        output_schema: None,
        dynamic_schema: false,
        properties: harness_contracts::ToolProperties {
            is_concurrency_safe: true,
            is_read_only: true,
            is_destructive: false,
            long_running: None,
            defer_policy: harness_contracts::DeferPolicy::AlwaysLoad,
        },
        trust_level: harness_contracts::TrustLevel::AdminTrusted,
        required_capabilities: Vec::new(),
        budget: harness_contracts::ResultBudget {
            metric: harness_contracts::BudgetMetric::Chars,
            limit: 4096,
            on_overflow: harness_contracts::OverflowAction::Offload,
            preview_head_chars: 512,
            preview_tail_chars: 512,
        },
        provider_restriction: harness_contracts::ProviderRestriction::All,
        origin: harness_contracts::ToolOrigin::Builtin,
        search_hint: None,
    }
}

fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "content": [
            { "type": "text", "text": "world" }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 7,
            "output_tokens": 3
        }
    }))
}

fn provider(server: &MockServer) -> AnthropicProvider {
    AnthropicProvider::from_api_key("test-key").with_base_url(server.uri())
}

#[test]
fn anthropic_provider_exports_required_models() {
    let provider = AnthropicProvider::from_api_key("test-key");
    let models = provider
        .supported_models()
        .into_iter()
        .map(|model| model.model_id)
        .collect::<Vec<_>>();

    assert!(models.contains(&"claude-3-5-sonnet-20241022".to_owned()));
    assert!(models.contains(&"claude-3-7-sonnet-20250219".to_owned()));
}

#[tokio::test]
async fn anthropic_non_stream_request_posts_messages() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ok_response())
        .mount(&server)
        .await;

    let events = provider(&server)
        .infer(sample_request(false), InferContext::for_test())
        .await
        .expect("request should succeed")
        .collect::<Vec<_>>()
        .await;

    assert!(events.contains(&ModelStreamEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::Text("world".to_owned()),
    }));
    assert!(events.contains(&ModelStreamEvent::MessageDelta {
        stop_reason: Some(StopReason::EndTurn),
        usage_delta: harness_contracts::UsageSnapshot {
            input_tokens: 7,
            output_tokens: 3,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_micros: 0,
        },
    }));

    let requests = server.received_requests().await.unwrap();
    let body: Value = requests[0].body_json().unwrap();
    assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
    assert_eq!(body["system"], "You are precise.");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["text"], "hello");
    assert_eq!(body["tools"][0]["name"], "search");
    assert_eq!(body["max_tokens"], 128);
    assert_eq!(body["stream"], false);
}

#[tokio::test]
async fn anthropic_retries_transient_status() {
    let server = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = attempts.clone();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |_req: &Request| {
            if seen.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(503).set_body_json(json!({ "error": { "message": "busy" } }))
            } else {
                ok_response()
            }
        })
        .mount(&server)
        .await;

    let stream = provider(&server)
        .infer(sample_request(false), InferContext::for_test())
        .await
        .expect("transient failure should be retried");
    drop(stream);

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn anthropic_rate_limit_retries_with_retry_after() {
    let server = MockServer::start().await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = attempts.clone();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |_req: &Request| {
            if seen.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "0")
                    .set_body_json(json!({ "error": { "message": "rate limited" } }))
            } else {
                ok_response()
            }
        })
        .mount(&server)
        .await;

    let stream = provider(&server)
        .infer(sample_request(false), InferContext::for_test())
        .await
        .expect("rate limit should be retried");
    drop(stream);

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn anthropic_auth_failure_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({ "error": { "message": "bad key" } })),
        )
        .mount(&server)
        .await;

    let err = provider(&server)
        .infer(sample_request(false), InferContext::for_test())
        .await
        .err()
        .expect("auth failure should fail");

    assert!(matches!(err, ModelError::AuthExpired(_)));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn anthropic_stream_request_posts_stream_true() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
                    "text/event-stream",
                ),
        )
        .mount(&server)
        .await;

    let events = provider(&server)
        .infer(sample_request(true), InferContext::for_test())
        .await
        .expect("stream request should succeed")
        .collect::<Vec<_>>()
        .await;

    assert_eq!(events, vec![ModelStreamEvent::MessageStop]);

    let requests = server.received_requests().await.unwrap();
    let body: Value = requests[0].body_json().unwrap();
    assert_eq!(body["stream"], true);
}
