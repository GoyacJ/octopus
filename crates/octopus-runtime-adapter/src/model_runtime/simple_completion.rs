use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};

use super::driver::{
    conversation_execution_from_response, extract_text_content, ModelExecutionResult,
    ProtocolDriver, RuntimeConversationExecution, RuntimeConversationRequest,
};

pub(crate) async fn execute_simple_completion<D: ProtocolDriver + ?Sized>(
    driver: &D,
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ModelExecutionResult, AppError> {
    let capability = driver.capability();
    if !capability.simple_completion {
        return Err(AppError::runtime(format!(
            "protocol family `{}` does not support simple completion",
            target.protocol_family
        )));
    }
    if !capability.prompt {
        return Err(AppError::runtime(format!(
            "protocol family `{}` does not support prompt execution",
            target.protocol_family
        )));
    }

    driver
        .execute_prompt(http, target, request_policy, input, system_prompt)
        .await
}

pub(crate) async fn execute_simple_completion_request<D: ProtocolDriver + ?Sized>(
    driver: &D,
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    request: &RuntimeConversationRequest,
) -> Result<RuntimeConversationExecution, AppError> {
    let fallback_input = request
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            if message.role != runtime::MessageRole::User {
                return None;
            }
            Some(extract_text_content(message))
        })
        .unwrap_or_default();
    let system_prompt =
        (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n"));
    let response = execute_simple_completion(
        driver,
        http,
        target,
        request_policy,
        fallback_input.as_str(),
        system_prompt.as_deref(),
    )
    .await?;
    Ok(conversation_execution_from_response(response))
}
