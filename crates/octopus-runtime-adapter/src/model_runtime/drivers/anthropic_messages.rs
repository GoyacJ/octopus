use api::{AnthropicClient, MessageResponse};
use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};

use super::{
    auth_source_from_request_policy, execute_message_protocol_streaming_conversation,
    flatten_output_content, message_request, ProviderProtocol,
};
use crate::model_runtime::{
    ConversationModelDriver, ConversationModelDriverCapability, GenerationModelDriver,
    GenerationModelDriverCapability, ModelExecutionResult, RuntimeConversationExecution,
    RuntimeConversationRequest,
};

#[derive(Debug)]
pub(crate) struct AnthropicMessagesDriver;

#[async_trait]
impl GenerationModelDriver for AnthropicMessagesDriver {
    fn protocol_family(&self) -> &'static str {
        "anthropic_messages"
    }

    fn capability(&self) -> GenerationModelDriverCapability {
        GenerationModelDriverCapability { prompt: true }
    }

    async fn execute_prompt(
        &self,
        _http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        let client = AnthropicClient::from_auth(auth_source_from_request_policy(
            request_policy,
            &target.protocol_family,
        )?)
        .with_base_url(request_policy.base_url.clone());
        let request = message_request(target, input, system_prompt);
        let response: MessageResponse = client
            .send_message(&request)
            .await
            .map_err(|error| AppError::runtime(error.to_string()))?;
        Ok(ModelExecutionResult {
            content: flatten_output_content(&response.content),
            request_id: response.request_id.clone(),
            total_tokens: Some(response.total_tokens()),
            deliverables: Vec::new(),
        })
    }
}

#[async_trait]
impl ConversationModelDriver for AnthropicMessagesDriver {
    fn protocol_family(&self) -> &'static str {
        "anthropic_messages"
    }

    fn capability(&self) -> ConversationModelDriverCapability {
        ConversationModelDriverCapability {
            tool_loop: true,
            upstream_streaming: true,
        }
    }

    async fn execute_conversation(
        &self,
        _http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        execute_message_protocol_streaming_conversation(
            target,
            request_policy,
            request,
            ProviderProtocol::Anthropic,
        )
        .await
    }
}
