use api::{build_http_client_or_default, ToolDefinition};
use async_trait::async_trait;
use octopus_core::{
    AppError, ResolvedExecutionTarget, ResolvedRequestPolicy, RuntimeExecutionSupport,
};
use runtime::{AssistantEvent, ContentBlock, ConversationMessage, MessageRole, TokenUsage};

use super::{
    driver_registry::ModelDriverRegistry, simple_completion::execute_simple_completion_request,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolDriverCapability {
    pub prompt: bool,
    pub conversation: bool,
    pub tool_loop: bool,
    pub streaming: bool,
    pub conversation_execution: bool,
    pub simple_completion: bool,
}

impl ProtocolDriverCapability {
    pub const fn runtime_support(self) -> RuntimeExecutionSupport {
        RuntimeExecutionSupport {
            prompt: self.prompt,
            conversation: self.conversation,
            tool_loop: self.tool_loop,
            streaming: self.streaming,
        }
    }
}

#[async_trait]
pub trait ProtocolDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
    fn capability(&self) -> ProtocolDriverCapability;

    fn runtime_support(&self) -> RuntimeExecutionSupport {
        self.capability().runtime_support()
    }

    async fn execute_prompt(
        &self,
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError>;

    async fn execute_conversation(
        &self,
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        Ok(self
            .execute_conversation_execution(http, target, request_policy, request)
            .await?
            .events)
    }

    async fn execute_conversation_execution(
        &self,
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        let capability = self.capability();

        if !request.tools.is_empty() && !capability.tool_loop {
            return Err(AppError::runtime(format!(
                "runtime tool loop does not support protocol family `{}` with tool-enabled turns yet",
                target.protocol_family
            )));
        }

        if capability.conversation_execution {
            return Err(AppError::runtime(format!(
                "protocol family `{}` requires an explicit conversation execution implementation",
                target.protocol_family
            )));
        }

        if capability.simple_completion {
            return execute_simple_completion_request(self, http, target, request_policy, request)
                .await;
        }

        Err(AppError::runtime(format!(
            "runtime execution does not support protocol family `{}` for conversation turns",
            target.protocol_family
        )))
    }
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
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        let fallback_input = request
            .messages
            .iter()
            .rev()
            .find_map(|message| {
                if message.role != MessageRole::User {
                    return None;
                }
                Some(extract_text_content(message))
            })
            .unwrap_or_default();
        let system_prompt =
            (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n"));
        let response = self
            .execute_prompt(
                target,
                request_policy,
                fallback_input.as_str(),
                system_prompt.as_deref(),
            )
            .await?;
        Ok(conversation_execution_from_response(response))
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
        self.registry
            .driver_for(target.protocol_family.as_str())?
            .execute_prompt(&self.http, target, request_policy, input, system_prompt)
            .await
    }

    async fn execute_conversation(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<Vec<AssistantEvent>, AppError> {
        self.registry
            .driver_for(target.protocol_family.as_str())?
            .execute_conversation(&self.http, target, request_policy, request)
            .await
    }

    async fn execute_conversation_execution(
        &self,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError> {
        self.registry
            .driver_for(target.protocol_family.as_str())?
            .execute_conversation_execution(&self.http, target, request_policy, request)
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
}

pub(crate) fn conversation_execution_from_response(
    response: ModelExecutionResult,
) -> RuntimeConversationExecution {
    let mut events = vec![AssistantEvent::TextDelta(response.content)];
    if let Some(total_tokens) = response.total_tokens {
        events.push(AssistantEvent::Usage(TokenUsage {
            input_tokens: 0,
            output_tokens: total_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }));
    }
    events.push(AssistantEvent::MessageStop);
    RuntimeConversationExecution {
        events,
        deliverables: response.deliverables,
    }
}

pub(crate) fn extract_text_content(message: &ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
