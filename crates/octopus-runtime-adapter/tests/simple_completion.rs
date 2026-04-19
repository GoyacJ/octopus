use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use octopus_core::{
    AppError, CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestAuth,
    ResolvedRequestAuthMode, ResolvedRequestPolicy, ResolvedRequestPolicyInput,
    RuntimeExecutionClass, RuntimeExecutionProfile,
};
use octopus_runtime_adapter::{
    GenerationModelDriver, GenerationModelDriverCapability, LiveRuntimeModelDriver,
    ModelDriverRegistry, ModelExecutionResult, RuntimeConversationRequest, RuntimeModelDriver,
};
use runtime::{ContentBlock, ConversationMessage, MessageRole};

#[test]
fn responses_driver_is_generation_only() {
    let registry = ModelDriverRegistry::installed();
    assert!(registry.generation_driver_for("openai_responses").is_ok());
    assert!(registry
        .conversation_driver_for("openai_responses")
        .is_err());
    assert_eq!(
        registry.execution_profile_for("openai_responses"),
        RuntimeExecutionProfile {
            execution_class: RuntimeExecutionClass::SingleShotGeneration,
            tool_loop: false,
            upstream_streaming: false,
        }
    );
}

#[tokio::test]
async fn generation_only_driver_cannot_execute_conversation_execution() {
    let registry = ModelDriverRegistry::new(vec![], vec![Arc::new(ScriptedGenerationDriver)]);
    let runtime_driver = LiveRuntimeModelDriver::with_registry(registry);

    let error = runtime_driver
        .execute_conversation_execution(
            &target("scripted_simple_completion"),
            &request_policy(),
            &conversation_request(),
        )
        .await
        .expect_err("generation-only driver should reject conversation execution");

    assert!(
        error
            .to_string()
            .contains("runtime execution does not support protocol family `scripted_simple_completion` for conversation turns")
    );
}

#[derive(Debug)]
struct ScriptedGenerationDriver;

#[async_trait]
impl GenerationModelDriver for ScriptedGenerationDriver {
    fn protocol_family(&self) -> &'static str {
        "scripted_simple_completion"
    }

    fn capability(&self) -> GenerationModelDriverCapability {
        GenerationModelDriverCapability { prompt: true }
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
        execution_profile: RuntimeExecutionProfile::default(),
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
        headers: BTreeMap::default(),
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
