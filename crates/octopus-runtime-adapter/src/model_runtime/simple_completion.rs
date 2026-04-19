use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};

use super::{GenerationModelDriver, ModelExecutionResult};

pub(crate) async fn execute_simple_completion<D: GenerationModelDriver + ?Sized>(
    driver: &D,
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ModelExecutionResult, AppError> {
    let capability = driver.capability();
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
