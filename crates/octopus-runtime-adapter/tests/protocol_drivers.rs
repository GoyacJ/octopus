use std::{collections::HashMap, sync::Arc};

use octopus_core::{
    CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestAuth, ResolvedRequestAuthMode,
    ResolvedRequestPolicy, ResolvedRequestPolicyInput, RuntimeExecutionSupport,
};
use octopus_runtime_adapter::{ModelDriverRegistry, RuntimeConversationRequest};
use runtime::{AssistantEvent, ContentBlock, ConversationMessage, MessageRole};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
};

#[test]
fn driver_registry_rejects_unknown_protocol_family() {
    let registry = ModelDriverRegistry::new(vec![]);
    let result = registry.driver_for("unknown_protocol");
    assert!(result.is_err());
}

#[tokio::test]
async fn anthropic_messages_driver_executes_conversation_and_normalizes_events() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let server = spawn_server(
        state.clone(),
        vec![http_response(
            "200 OK",
            "application/json",
            concat!(
                "{",
                "\"id\":\"msg_anthropic\",",
                "\"type\":\"message\",",
                "\"role\":\"assistant\",",
                "\"content\":[{\"type\":\"text\",\"text\":\"Hello from Anthropic\"}],",
                "\"model\":\"claude-sonnet-4-5\",",
                "\"stop_reason\":\"end_turn\",",
                "\"stop_sequence\":null,",
                "\"usage\":{\"input_tokens\":12,\"output_tokens\":8}",
                "}"
            ),
        )],
    )
    .await;

    let registry = ModelDriverRegistry::installed();
    let driver = registry
        .driver_for("anthropic_messages")
        .expect("anthropic driver");
    let execution = driver
        .execute_conversation_execution(
            &reqwest::Client::new(),
            &target("anthropic", "anthropic_messages"),
            &request_policy(
                server.base_url(),
                ResolvedRequestAuth {
                    mode: ResolvedRequestAuthMode::Header,
                    name: Some("x-api-key".into()),
                    value: Some("anthropic-test-key".into()),
                },
            ),
            &conversation_request(),
        )
        .await
        .expect("anthropic conversation execution");

    assert_eq!(
        execution.events,
        vec![
            AssistantEvent::TextDelta("Hello from Anthropic".into()),
            AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 12,
                output_tokens: 8,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ]
    );
    assert!(execution.deliverables.is_empty());

    let captured = state.lock().await;
    let request = captured.first().expect("captured request");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/messages");
    assert_eq!(
        request.headers.get("x-api-key").map(String::as_str),
        Some("anthropic-test-key")
    );
}

#[tokio::test]
async fn openai_chat_driver_executes_prompt_and_exposes_runtime_support() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let server = spawn_server(
        state.clone(),
        vec![http_response(
            "200 OK",
            "application/json",
            concat!(
                "{",
                "\"id\":\"chatcmpl_test\",",
                "\"object\":\"chat.completion\",",
                "\"created\":1744934400,",
                "\"model\":\"gpt-5.4\",",
                "\"choices\":[{",
                "\"index\":0,",
                "\"message\":{\"role\":\"assistant\",\"content\":\"Hello from OpenAI Chat\"},",
                "\"finish_reason\":\"stop\"",
                "}],",
                "\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":7,\"total_tokens\":12}",
                "}"
            ),
        )],
    )
    .await;

    let registry = ModelDriverRegistry::installed();
    let driver = registry
        .driver_for("openai_chat")
        .expect("openai chat driver");
    assert_eq!(
        driver.runtime_support(),
        RuntimeExecutionSupport {
            prompt: true,
            conversation: true,
            tool_loop: true,
            streaming: false,
        }
    );

    let result = driver
        .execute_prompt(
            &reqwest::Client::new(),
            &target("openai", "openai_chat"),
            &request_policy(
                server.base_url(),
                ResolvedRequestAuth {
                    mode: ResolvedRequestAuthMode::BearerToken,
                    name: None,
                    value: Some("openai-test-key".into()),
                },
            ),
            "Say hello",
            Some("Be concise."),
        )
        .await
        .expect("openai prompt execution");

    assert_eq!(result.content, "Hello from OpenAI Chat");
    assert_eq!(result.total_tokens, Some(12));
    assert!(result.deliverables.is_empty());

    let captured = state.lock().await;
    let request = captured.first().expect("captured request");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/chat/completions");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer openai-test-key")
    );
}

#[tokio::test]
async fn openai_responses_driver_executes_prompt() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let server = spawn_server(
        state.clone(),
        vec![http_response(
            "200 OK",
            "application/json",
            concat!(
                "{",
                "\"output_text\":\"Hello from Responses\",",
                "\"usage\":{\"total_tokens\":18}",
                "}"
            ),
        )],
    )
    .await;

    let registry = ModelDriverRegistry::installed();
    let driver = registry
        .driver_for("openai_responses")
        .expect("openai responses driver");

    let result = driver
        .execute_prompt(
            &reqwest::Client::new(),
            &target("openai", "openai_responses"),
            &request_policy(
                server.base_url(),
                ResolvedRequestAuth {
                    mode: ResolvedRequestAuthMode::BearerToken,
                    name: None,
                    value: Some("responses-test-key".into()),
                },
            ),
            "Say hello",
            Some("Be concise."),
        )
        .await
        .expect("responses prompt execution");

    assert_eq!(result.content, "Hello from Responses");
    assert_eq!(result.total_tokens, Some(18));
    assert!(result.deliverables.is_empty());

    let captured = state.lock().await;
    let request = captured.first().expect("captured request");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/responses");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer responses-test-key")
    );
}

#[tokio::test]
async fn gemini_native_driver_executes_prompt_with_query_param_auth() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let server = spawn_server(
        state.clone(),
        vec![http_response(
            "200 OK",
            "application/json",
            concat!(
                "{",
                "\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hello from Gemini\"}]}}],",
                "\"usageMetadata\":{\"totalTokenCount\":21}",
                "}"
            ),
        )],
    )
    .await;

    let registry = ModelDriverRegistry::installed();
    let driver = registry
        .driver_for("gemini_native")
        .expect("gemini native driver");

    let result = driver
        .execute_prompt(
            &reqwest::Client::new(),
            &target("google", "gemini_native"),
            &request_policy(
                server.base_url(),
                ResolvedRequestAuth {
                    mode: ResolvedRequestAuthMode::QueryParam,
                    name: Some("key".into()),
                    value: Some("gemini-test-key".into()),
                },
            ),
            "Say hello",
            Some("Be concise."),
        )
        .await
        .expect("gemini prompt execution");

    assert_eq!(result.content, "Hello from Gemini");
    assert_eq!(result.total_tokens, Some(21));
    assert!(result.deliverables.is_empty());

    let captured = state.lock().await;
    let request = captured.first().expect("captured request");
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1beta/models/gemini-2.5-pro:generateContent?key=gemini-test-key"
    );
}

fn target(provider_id: &str, protocol_family: &str) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        configured_model_id: format!("{provider_id}-configured"),
        configured_model_name: format!("{provider_id} Configured"),
        provider_id: provider_id.into(),
        registry_model_id: format!("{provider_id}/test-model"),
        model_id: match protocol_family {
            "anthropic_messages" => "claude-sonnet-4-5".into(),
            "gemini_native" => "gemini-2.5-pro".into(),
            _ => "gpt-5.4".into(),
        },
        surface: "conversation".into(),
        protocol_family: protocol_family.into(),
        credential_ref: Some("secret://runtime/test".into()),
        credential_source: "provider_inherited".into(),
        request_policy: ResolvedRequestPolicyInput {
            auth_strategy: match protocol_family {
                "anthropic_messages" => "x_api_key".into(),
                "gemini_native" => "api_key".into(),
                _ => "bearer".into(),
            },
            base_url_policy: "allow_override".into(),
            default_base_url: "https://example.test".into(),
            provider_base_url: None,
            configured_base_url: None,
        },
        base_url: Some("https://example.test".into()),
        max_output_tokens: Some(1024),
        capabilities: Vec::<CapabilityDescriptor>::new(),
    }
}

fn request_policy(base_url: String, auth: ResolvedRequestAuth) -> ResolvedRequestPolicy {
    ResolvedRequestPolicy {
        base_url,
        headers: Default::default(),
        auth,
        timeout_ms: None,
    }
}

fn conversation_request() -> RuntimeConversationRequest {
    RuntimeConversationRequest {
        system_prompt: vec!["Respond directly.".into()],
        messages: vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Say hello".into(),
            }],
            usage: None,
        }],
        tools: Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

struct TestServer {
    base_url: String,
    join_handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    fn base_url(&self) -> String {
        self.base_url.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

async fn spawn_server(
    state: Arc<Mutex<Vec<CapturedRequest>>>,
    responses: Vec<String>,
) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener
        .local_addr()
        .expect("listener should have local addr");
    let join_handle = tokio::spawn(async move {
        for response in responses {
            let (mut socket, _) = listener.accept().await.expect("server should accept");
            let mut buffer = Vec::new();
            let mut header_end = None;

            loop {
                let mut chunk = [0_u8; 1024];
                let read = socket
                    .read(&mut chunk)
                    .await
                    .expect("request read should succeed");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(position) = find_header_end(&buffer) {
                    header_end = Some(position);
                    break;
                }
            }

            let header_end = header_end.expect("request should include headers");
            let (header_bytes, remaining) = buffer.split_at(header_end);
            let header_text =
                String::from_utf8(header_bytes.to_vec()).expect("headers should be utf8");
            let mut lines = header_text.split("\r\n");
            let request_line = lines.next().expect("request line should exist");
            let mut parts = request_line.split_whitespace();
            let method = parts.next().expect("method should exist").to_string();
            let path = parts.next().expect("path should exist").to_string();
            let mut headers = HashMap::new();
            let mut content_length = 0_usize;
            for line in lines {
                if line.is_empty() {
                    continue;
                }
                let (name, value) = line.split_once(':').expect("header should have colon");
                let value = value.trim().to_string();
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.parse().expect("content length should parse");
                }
                headers.insert(name.to_ascii_lowercase(), value);
            }

            let mut body = remaining[4..].to_vec();
            while body.len() < content_length {
                let mut chunk = vec![0_u8; content_length - body.len()];
                let read = socket
                    .read(&mut chunk)
                    .await
                    .expect("body read should succeed");
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..read]);
            }

            state.lock().await.push(CapturedRequest {
                method,
                path,
                headers,
                body: String::from_utf8(body).expect("body should be utf8"),
            });

            socket
                .write_all(response.as_bytes())
                .await
                .expect("response write should succeed");
        }
    });

    TestServer {
        base_url: format!("http://{address}"),
        join_handle,
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}
