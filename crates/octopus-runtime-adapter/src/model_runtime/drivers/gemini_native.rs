use async_trait::async_trait;
use octopus_core::{AppError, ResolvedExecutionTarget, ResolvedRequestPolicy};
use serde_json::{json, Value};

use super::{apply_request_policy, target_max_output_tokens};
use crate::model_runtime::driver::{
    ModelExecutionResult, ProtocolDriver, ProtocolDriverCapability,
};

#[derive(Debug)]
pub(crate) struct GeminiNativeDriver;

#[async_trait]
impl ProtocolDriver for GeminiNativeDriver {
    fn protocol_family(&self) -> &'static str {
        "gemini_native"
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
        execute_gemini_native(http, target, request_policy, input, system_prompt).await
    }
}

async fn execute_gemini_native(
    http: &reqwest::Client,
    target: &ResolvedExecutionTarget,
    request_policy: &ResolvedRequestPolicy,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ModelExecutionResult, AppError> {
    let response = apply_request_policy(
        http.post(format!(
            "{}/v1beta/models/{}:generateContent",
            request_policy.base_url.trim_end_matches('/'),
            target.model_id
        )),
        request_policy,
        &target.protocol_family,
    )?
    .json(&build_gemini_native_request_body(
        target,
        input,
        system_prompt,
    ))
    .send()
    .await
    .map_err(|error| AppError::runtime(error.to_string()))?;
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    if !status.is_success() {
        return Err(AppError::runtime(format!(
            "provider returned {} for gemini request: {}",
            status, body
        )));
    }

    let content = body
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    Ok(ModelExecutionResult {
        content,
        request_id: None,
        total_tokens: body
            .get("usageMetadata")
            .and_then(|usage| usage.get("totalTokenCount"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        deliverables: Vec::new(),
    })
}

fn build_gemini_native_request_body(
    target: &ResolvedExecutionTarget,
    input: &str,
    system_prompt: Option<&str>,
) -> Value {
    json!({
        "systemInstruction": system_prompt.map(|value| {
            json!({
                "parts": [{ "text": value }]
            })
        }),
        "contents": [{
            "role": "user",
            "parts": [{ "text": input }]
        }],
        "generationConfig": {
            "maxOutputTokens": target_max_output_tokens(target),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::build_gemini_native_request_body;
    use octopus_core::{CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestPolicyInput};

    fn target(max_output_tokens: Option<u32>) -> ResolvedExecutionTarget {
        ResolvedExecutionTarget {
            configured_model_id: "configured".into(),
            configured_model_name: "Configured".into(),
            provider_id: "google".into(),
            registry_model_id: "gemini-2.5-pro".into(),
            model_id: "gemini-2.5-pro".into(),
            surface: "conversation".into(),
            protocol_family: "gemini_native".into(),
            credential_ref: Some("env:GEMINI_API_KEY".into()),
            credential_source: "provider_inherited".into(),
            request_policy: ResolvedRequestPolicyInput {
                auth_strategy: "api_key".into(),
                base_url_policy: "allow_override".into(),
                default_base_url: "https://generativelanguage.googleapis.com".into(),
                provider_base_url: None,
                configured_base_url: None,
            },
            base_url: Some("https://generativelanguage.googleapis.com".into()),
            max_output_tokens,
            capabilities: Vec::<CapabilityDescriptor>::new(),
        }
    }

    #[test]
    fn gemini_payload_includes_generation_config_max_output_tokens() {
        let body = build_gemini_native_request_body(&target(Some(1024)), "hello", Some("system"));

        assert_eq!(body["generationConfig"]["maxOutputTokens"], 1024);
    }
}
