use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};
use serde_json::{json, Value};

use super::{apply_request_policy, target_max_output_tokens};
use crate::model_runtime::driver::{
    ModelExecutionResult, ProtocolDriver, ProtocolDriverCapability,
};

#[derive(Debug)]
pub(crate) struct OpenAiResponsesDriver;

#[async_trait]
impl ProtocolDriver for OpenAiResponsesDriver {
    fn protocol_family(&self) -> &'static str {
        "openai_responses"
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
        http: &reqwest::Client,
        target: &ResolvedExecutionTarget,
        request_policy: &ResolvedRequestPolicy,
        input: &str,
        system_prompt: Option<&str>,
    ) -> Result<ModelExecutionResult, AppError> {
        execute_openai_responses(http, target, request_policy, input, system_prompt).await
    }
}

async fn execute_openai_responses(
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ModelExecutionResult, AppError> {
    let response = apply_request_policy(
        http.post(format!(
            "{}/responses",
            request_policy.base_url.trim_end_matches('/')
        )),
        request_policy,
        &target.protocol_family,
    )?
    .json(&build_openai_responses_request_body(
        target,
        input,
        system_prompt,
    ))
    .send()
    .await
    .map_err(|error| AppError::runtime(error.to_string()))?;
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    if !status.is_success() {
        return Err(AppError::runtime(format!(
            "provider returned {} for responses request: {}",
            status, body
        )));
    }

    Ok(ModelExecutionResult {
        content: extract_responses_output_text(&body),
        request_id,
        total_tokens: body
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        deliverables: Vec::new(),
    })
}

fn build_openai_responses_request_body(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Value {
    json!({
        "model": target.model_id,
        "input": input,
        "instructions": system_prompt,
        "stream": false,
        "max_output_tokens": target_max_output_tokens(target),
    })
}

fn extract_responses_output_text(body: &Value) -> String {
    if let Some(output_text) = body.get("output_text").and_then(Value::as_str) {
        return output_text.to_string();
    }

    body.get("output")
        .and_then(Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .flat_map(|output| {
                    output
                        .get("content")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|item| {
                    item.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            item.get("output_text")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        })
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::build_openai_responses_request_body;
    use octopus_core::{CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestPolicyInput};

    fn target(max_output_tokens: Option<u32>) -> ResolvedExecutionTarget {
        ResolvedExecutionTarget {
            configured_model_id: "configured".into(),
            configured_model_name: "Configured".into(),
            provider_id: "openai".into(),
            registry_model_id: "gpt-5".into(),
            model_id: "gpt-5".into(),
            surface: "conversation".into(),
            protocol_family: "openai_responses".into(),
            credential_ref: Some("env:OPENAI_API_KEY".into()),
            credential_source: "provider_inherited".into(),
            request_policy: ResolvedRequestPolicyInput {
                auth_strategy: "bearer".into(),
                base_url_policy: "allow_override".into(),
                default_base_url: "https://api.openai.com/v1".into(),
                provider_base_url: None,
                configured_base_url: None,
            },
            base_url: Some("https://api.openai.com/v1".into()),
            max_output_tokens,
            capabilities: Vec::<CapabilityDescriptor>::new(),
        }
    }

    #[test]
    fn responses_payload_includes_max_output_tokens_override() {
        let body =
            build_openai_responses_request_body(&target(Some(3072)), "hello", Some("system"));

        assert_eq!(body["max_output_tokens"], 3072);
    }
}
