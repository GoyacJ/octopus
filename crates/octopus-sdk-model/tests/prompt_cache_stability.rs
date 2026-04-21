use bytes::Bytes;
use futures::{stream, StreamExt};
use serde_json::{json, Value};

use octopus_sdk_contracts::{AssistantEvent, CacheBreakpoint, CacheTtl, ContentBlock, Message, Role, ToolSchema, Usage};
use octopus_sdk_model::{
    AnthropicMessagesAdapter, CacheControlStrategy, ModelId, ModelRequest, ModelRole,
    ProtocolAdapter, ResponseFormat, ThinkingConfig,
};

fn request_with_last_user(last_user: &str) -> ModelRequest {
    ModelRequest {
        model: ModelId("claude-opus-4-6".to_string()),
        system_prompt: vec![
            "You are precise.".to_string(),
            "Keep tool order stable.".to_string(),
        ],
        messages: vec![
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Summarize the design doc.".to_string(),
                }],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: last_user.to_string(),
                }],
            },
        ],
        tools: vec![
            ToolSchema {
                name: "search".to_string(),
                description: "Search docs".to_string(),
                input_schema: json!({"type": "object", "properties": {"query": {"type": "string"}}}),
            },
            ToolSchema {
                name: "bash".to_string(),
                description: "Run shell".to_string(),
                input_schema: json!({"type": "object", "properties": {"cmd": {"type": "string"}}}),
            },
        ],
        role: ModelRole::Main,
        cache_breakpoints: vec![CacheBreakpoint {
            position: 0,
            ttl: CacheTtl::FiveMinutes,
        }],
        response_format: Some(ResponseFormat::Json {
            schema: json!({"type": "object", "properties": {"answer": {"type": "string"}}}),
        }),
        thinking: Some(ThinkingConfig {
            enabled: true,
            budget_tokens: Some(512),
        }),
        cache_control: CacheControlStrategy::PromptCaching {
            breakpoints: vec!["system", "tools", "first_user"],
        },
        max_tokens: Some(2048),
        temperature: Some(0.1),
        stream: false,
    }
}

fn parse_usage(events: &[AssistantEvent]) -> Usage {
    events
        .iter()
        .find_map(|event| match event {
            AssistantEvent::Usage(usage) => Some(usage.clone()),
            _ => None,
        })
        .expect("usage event should be present")
}

async fn parse_response(
    adapter: &AnthropicMessagesAdapter,
    cache_read_input_tokens: u32,
) -> Vec<AssistantEvent> {
    let payload = json!({
        "content": [{"type": "text", "text": "ok"}],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 20,
            "output_tokens": 5,
            "cache_creation_input_tokens": 120,
            "cache_read_input_tokens": cache_read_input_tokens
        }
    });
    let raw = Box::pin(stream::iter(vec![Ok(Bytes::from(
        serde_json::to_vec(&payload).expect("payload should serialize"),
    ))]));
    let stream = adapter
        .parse_stream(raw)
        .expect("anthropic parser should accept canonical JSON");

    stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("response should parse cleanly")
}

#[tokio::test]
async fn anthropic_prompt_cache_inputs_stay_stable_across_turns() {
    let adapter = AnthropicMessagesAdapter;
    let requests = [
        request_with_last_user("Draft a two-line summary."),
        request_with_last_user("Draft a three-line summary."),
        request_with_last_user("Draft a bullet summary."),
    ];

    let tool_fingerprints = requests
        .iter()
        .map(ModelRequest::tools_fingerprint)
        .collect::<Vec<_>>();
    assert!(
        tool_fingerprints.windows(2).all(|window| window[0] == window[1]),
        "tools fingerprint drifted across otherwise stable requests: {tool_fingerprints:?}"
    );

    let request_bodies = requests
        .iter()
        .map(|request| {
            adapter
                .to_request(request)
                .expect("anthropic request should serialize")
        })
        .collect::<Vec<Value>>();

    let system_bytes = request_bodies
        .iter()
        .map(|body| serde_json::to_vec(&body["system"]).expect("system field should serialize"))
        .collect::<Vec<_>>();
    assert!(
        system_bytes.windows(2).all(|window| window[0] == window[1]),
        "system field drifted across cache-stable requests"
    );

    let tools_bytes = request_bodies
        .iter()
        .map(|body| serde_json::to_vec(&body["tools"]).expect("tools field should serialize"))
        .collect::<Vec<_>>();
    assert!(
        tools_bytes.windows(2).all(|window| window[0] == window[1]),
        "tools field drifted across cache-stable requests"
    );

    let first_usage = parse_usage(&parse_response(&adapter, 0).await);
    let second_usage = parse_usage(&parse_response(&adapter, 120).await);
    let third_usage = parse_usage(&parse_response(&adapter, 240).await);

    assert!(
        first_usage.cache_read_input_tokens <= second_usage.cache_read_input_tokens
            && second_usage.cache_read_input_tokens <= third_usage.cache_read_input_tokens,
        "cache_read_input_tokens must be monotonic, got [{}, {}, {}]",
        first_usage.cache_read_input_tokens,
        second_usage.cache_read_input_tokens,
        third_usage.cache_read_input_tokens
    );
}
