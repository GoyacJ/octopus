use std::sync::Arc;

use async_trait::async_trait;
use octopus_core::{
    AppError, CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestAuth,
    ResolvedRequestAuthMode, ResolvedRequestPolicy, ResolvedRequestPolicyInput,
};
use octopus_runtime_adapter::{
    LiveRuntimeModelDriver, ModelDriverRegistry, ModelExecutionResult, ProtocolDriver,
    ProtocolDriverCapability, RuntimeConversationRequest, RuntimeModelDriver,
};
use runtime::{AssistantEvent, ContentBlock, ConversationMessage, MessageRole, TokenUsage};

#[test]
fn responses_driver_refuses_tool_loop_but_supports_simple_completion() {
    let registry = ModelDriverRegistry::installed();
    let capability = registry
        .driver_for("openai_responses")
        .expect("responses driver")
        .capability();

    assert!(!capability.tool_loop);
    assert!(capability.simple_completion);
    assert!(!capability.conversation_execution);
}

#[tokio::test]
async fn simple_completion_projects_prompt_result_into_conversation_execution() {
    let registry = ModelDriverRegistry::new(vec![Arc::new(ScriptedSimpleCompletionDriver)]);
    let runtime_driver = LiveRuntimeModelDriver::with_registry(registry);

    let execution = runtime_driver
        .execute_conversation_execution(
            &target("scripted_simple_completion"),
            &request_policy(),
            &conversation_request(),
        )
        .await
        .expect("simple completion execution");

    assert_eq!(
        execution.events,
        vec![
            AssistantEvent::TextDelta("scripted prompt result".into()),
            AssistantEvent::Usage(TokenUsage {
                input_tokens: 0,
                output_tokens: 9,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            AssistantEvent::MessageStop,
        ]
    );
}

#[derive(Debug)]
struct ScriptedSimpleCompletionDriver;

#[async_trait]
impl ProtocolDriver for ScriptedSimpleCompletionDriver {
    fn protocol_family(&self) -> &'static str {
        "scripted_simple_completion"
    }

    fn capability(&self) -> ProtocolDriverCapability {
        ProtocolDriverCapability {
            prompt: true,
            conversation: true,
            tool_loop: false,
            streaming: false,
            conversation_execution: false,
            simple_completion: true,
        }
    }

    async fn execute_prompt(
        &self,
        _http: &reqwest::Client,
        _target: &ResolvedExecutionTarget,
        _request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        assert_eq!(input, "Say hello");
        assert_eq!(system_prompt, Some("Respond directly."));
        Ok(ModelExecutionResult {
            content: "scripted prompt result".into(),
            request_id: Some("scripted-request".into()),
            total_tokens: Some(9),
            deliverables: Vec::new(),
        })
    }
}

fn target(protocol_family: &str) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        configured_model_id: "configured".into(),
        configured_model_name: "Configured".into(),
        provider_id: "test".into(),
        registry_model_id: "test/model".into(),
        model_id: "test-model".into(),
        surface: "conversation".into(),
        protocol_family: protocol_family.into(),
        credential_ref: Some("env:TEST_API_KEY".into()),
        credential_source: "provider_inherited".into(),
        request_policy: ResolvedRequestPolicyInput {
            auth_strategy: "bearer".into(),
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

fn request_policy() -> ResolvedRequestPolicy {
    ResolvedRequestPolicy {
        base_url: "https://example.test".into(),
        headers: Default::default(),
        auth: ResolvedRequestAuth {
            mode: ResolvedRequestAuthMode::BearerToken,
            name: None,
            value: Some("test-key".into()),
        },
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
