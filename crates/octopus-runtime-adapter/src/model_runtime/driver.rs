use api::{build_http_client_or_default, ToolDefinition};
use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};
use runtime::{AssistantEvent, ConversationMessage};

use super::{driver_registry::ModelDriverRegistry, simple_completion::execute_simple_completion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConversationRequest {
    pub system_prompt: Vec<String>,
    pub messages: Vec<ConversationMessage>,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConversationExecution {
    pub events: Vec<AssistantEvent>,
    pub deliverables: Vec<ModelExecutionDeliverable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExecutionResult {
    pub content: String,
    pub request_id: Option<String>,
    pub total_tokens: Option<u32>,
    pub deliverables: Vec<ModelExecutionDeliverable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExecutionDeliverable {
    pub title: Option<String>,
    pub preview_kind: String,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub text_content: Option<String>,
    pub data_base64: Option<String>,
}

#[async_trait]
pub trait RuntimeModelDriver: Send + Sync {
    async fn execute_prompt(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError>;

    async fn execute_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        Ok(self
            .execute_conversation_execution(target, request_policy, request)
            .await?
            .events)
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        _request_policy: &ResolvedRequestPolicy,
        _request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        Err(AppError::runtime(format!(
            "runtime execution does not support protocol family `{}` for conversation turns",
            target.protocol_family
        )))
    }
}

#[derive(Clone)]
pub struct LiveRuntimeModelDriver {
    http: reqwest::Client,
    registry: ModelDriverRegistry,
}

impl LiveRuntimeModelDriver {
    pub fn new() -> Self {
        Self::with_registry(ModelDriverRegistry::installed())
    }

    pub fn with_registry(registry: ModelDriverRegistry) -> Self {
        Self {
            http: build_http_client_or_default(),
            registry,
        }
    }
}

impl Default for LiveRuntimeModelDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuntimeModelDriver for LiveRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        let driver = self
            .registry
            .generation_driver_for(target.protocol_family.as_str())?;
        execute_simple_completion(
            driver.as_ref(),
            &self.http,
            target,
            request_policy,
            input,
            system_prompt,
        )
        .await
    }

    async fn execute_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        Ok(self
            .execute_conversation_execution(target, request_policy, request)
            .await?
            .events)
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        self.registry
            .conversation_driver_for(target.protocol_family.as_str())?
            .execute_conversation(&self.http, target, request_policy, request)
            .await
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockRuntimeModelDriver;

#[async_trait]
impl RuntimeModelDriver for MockRuntimeModelDriver {
    async fn execute_prompt(
        &self,
        target: &ResolvedExecutionTarget,
        _request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        let prompt_prefix = system_prompt
            .map(|value| format!(" [{value}]"))
            .unwrap_or_default();
        Ok(ModelExecutionResult {
            content: format!(
                "Mock assistant response via {}:{}{} -> {}",
                target.provider_id, target.surface, prompt_prefix, input
            ),
            request_id: Some("mock-request-id".into()),
            total_tokens: Some(32),
            deliverables: Vec::new(),
        })
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        let input = request
            .messages
            .iter()
            .rev()
            .find_map(|message| {
                if message.role != runtime::MessageRole::User {
                    return None;
                }
                message.blocks.iter().find_map(|block| match block {
                    runtime::ContentBlock::Text { text } if !text.trim().is_empty() => {
                        Some(text.as_str())
                    }
                    _ => None,
                })
            })
            .unwrap_or_default();
        let system_prompt =
            (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n"));
        let response = self
            .execute_prompt(target, request_policy, input, system_prompt.as_deref())
            .await?;
        let mut events = vec![AssistantEvent::TextDelta(response.content)];
        if let Some(total_tokens) = response.total_tokens {
            events.push(AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 0,
                output_tokens: total_tokens,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }));
        }
        events.push(AssistantEvent::MessageStop);
        Ok(RuntimeConversationExecution {
            events,
            deliverables: response.deliverables,
        })
    }
}
