use std::collections::BTreeMap;

use octopus_core::{
    AppError, ResolvedExecutionTarget, ResolvedRequestAuth, ResolvedRequestAuthMode,
    ResolvedRequestPolicy, ResolvedRequestPolicyInput,
};

use super::ResolvedModelAuth;

pub fn resolve_request_policy(
    target: &ResolvedExecutionTarget,
    auth: &ResolvedModelAuth,
) -> Result<ResolvedRequestPolicy, AppError> {
    Ok(ResolvedRequestPolicy {
        base_url: resolve_request_base_url(
            &target.request_policy,
            &target.provider_id,
            &target.protocol_family,
        )?,
        headers: BTreeMap::new(),
        auth: resolve_request_auth(target, auth)?,
        timeout_ms: None,
    })
}

pub(crate) fn resolve_request_base_url(
    request_policy: &ResolvedRequestPolicyInput,
    provider_id: &str,
    protocol_family: &str,
) -> Result<String, AppError> {
    let default_base_url = request_policy.default_base_url.trim();
    if default_base_url.is_empty() {
        return Err(AppError::invalid_input(format!(
            "provider `{provider_id}` protocol `{protocol_family}` is missing a default base URL"
        )));
    }

    let configured_base_url = request_policy
        .configured_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider_base_url = request_policy
        .provider_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let base_url = match request_policy.base_url_policy.trim() {
        "" | "allow_override" => configured_base_url
            .or(provider_base_url)
            .unwrap_or(default_base_url),
        "fixed" => default_base_url,
        "credential_only" => provider_base_url.unwrap_or(default_base_url),
        other => {
            return Err(AppError::invalid_input(format!(
                "provider `{provider_id}` protocol `{protocol_family}` uses unsupported base URL policy `{other}`"
            )))
        }
    };

    Ok(normalize_request_base_url(
        provider_id,
        protocol_family,
        base_url,
    ))
}

fn resolve_request_auth(
    target: &ResolvedExecutionTarget,
    auth: &ResolvedModelAuth,
) -> Result<ResolvedRequestAuth, AppError> {
    let credential = auth.credential.trim();
    let auth_strategy = target.request_policy.auth_strategy.trim();
    if auth_strategy != "none" && credential.is_empty() {
        return Err(AppError::invalid_input(format!(
            "resolved execution target `{}` is missing an executable credential",
            target.configured_model_id
        )));
    }

    match auth_strategy {
        "" | "bearer" => Ok(ResolvedRequestAuth {
            mode: ResolvedRequestAuthMode::BearerToken,
            name: Some("Authorization".into()),
            value: Some(credential.to_string()),
        }),
        "x_api_key" => Ok(ResolvedRequestAuth {
            mode: ResolvedRequestAuthMode::Header,
            name: Some("x-api-key".into()),
            value: Some(credential.to_string()),
        }),
        "api_key" => Ok(ResolvedRequestAuth {
            mode: ResolvedRequestAuthMode::QueryParam,
            name: Some("key".into()),
            value: Some(credential.to_string()),
        }),
        "none" => Ok(ResolvedRequestAuth {
            mode: ResolvedRequestAuthMode::None,
            name: None,
            value: None,
        }),
        other => Err(AppError::invalid_input(format!(
            "provider `{}` protocol `{}` uses unsupported auth strategy `{other}`",
            target.provider_id, target.protocol_family
        ))),
    }
}

fn normalize_request_base_url(provider_id: &str, protocol_family: &str, base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/').to_string();
    if provider_id == "minimax"
        && protocol_family == "anthropic_messages"
        && normalized == "https://api.minimaxi.com"
    {
        return "https://api.minimaxi.com/anthropic".to_string();
    }
    normalized
}
