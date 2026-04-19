use octopus_core::{
    CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestAuthMode, RuntimeExecutionProfile,
};
use octopus_runtime_adapter::{resolve_request_policy, ResolvedModelAuth, ResolvedModelAuthMode};

fn test_target(
    provider_id: &str,
    protocol_family: &str,
    auth_strategy: &str,
    configured_base_url: Option<&str>,
    provider_base_url: Option<&str>,
    default_base_url: &str,
    base_url_policy: &str,
) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        configured_model_id: "configured-model".into(),
        configured_model_name: "Configured Model".into(),
        provider_id: provider_id.into(),
        registry_model_id: "registry-model".into(),
        model_id: "runtime-model".into(),
        surface: "conversation".into(),
        protocol_family: protocol_family.into(),
        execution_profile: RuntimeExecutionProfile::default(),
        credential_ref: Some("secret-ref:test".into()),
        credential_source: "configured_model_override".into(),
        request_policy: octopus_core::ResolvedRequestPolicyInput {
            auth_strategy: auth_strategy.into(),
            base_url_policy: base_url_policy.into(),
            default_base_url: default_base_url.into(),
            provider_base_url: provider_base_url.map(ToOwned::to_owned),
            configured_base_url: configured_base_url.map(ToOwned::to_owned),
        },
        base_url: None,
        max_output_tokens: Some(4096),
        capabilities: vec![CapabilityDescriptor {
            capability_id: "reasoning".into(),
            label: "reasoning".into(),
        }],
    }
}

fn test_auth() -> ResolvedModelAuth {
    ResolvedModelAuth {
        mode: ResolvedModelAuthMode::BearerToken,
        credential: "sk-test-credential".into(),
        source: "configured_model_override".into(),
        reference_kind: "managed_secret".into(),
    }
}

#[test]
fn request_policy_prefers_configured_base_url_then_surface_default() {
    let policy = resolve_request_policy(
        &test_target(
            "minimax",
            "anthropic_messages",
            "x_api_key",
            Some("https://api.minimaxi.com"),
            None,
            "https://api.minimaxi.com/anthropic",
            "allow_override",
        ),
        &test_auth(),
    )
    .expect("request policy should resolve");

    assert_eq!(policy.base_url, "https://api.minimaxi.com/anthropic");
}

#[test]
fn request_policy_uses_provider_base_url_when_configured_override_is_absent() {
    let policy = resolve_request_policy(
        &test_target(
            "openai",
            "openai_responses",
            "bearer",
            None,
            Some("https://proxy.workspace.internal/v1"),
            "https://api.openai.com/v1",
            "allow_override",
        ),
        &test_auth(),
    )
    .expect("request policy should resolve");

    assert_eq!(policy.base_url, "https://proxy.workspace.internal/v1");
}

#[test]
fn request_policy_maps_x_api_key_strategy_to_header_transport() {
    let policy = resolve_request_policy(
        &test_target(
            "anthropic",
            "anthropic_messages",
            "x_api_key",
            None,
            None,
            "https://api.anthropic.com",
            "allow_override",
        ),
        &test_auth(),
    )
    .expect("request policy should resolve");

    assert_eq!(policy.auth.mode, ResolvedRequestAuthMode::Header);
    assert_eq!(policy.auth.name.as_deref(), Some("x-api-key"));
    assert_eq!(policy.auth.value.as_deref(), Some("sk-test-credential"));
}

#[test]
fn request_policy_maps_api_key_strategy_to_query_transport() {
    let policy = resolve_request_policy(
        &test_target(
            "google",
            "gemini_native",
            "api_key",
            None,
            None,
            "https://generativelanguage.googleapis.com",
            "allow_override",
        ),
        &test_auth(),
    )
    .expect("request policy should resolve");

    assert_eq!(policy.auth.mode, ResolvedRequestAuthMode::QueryParam);
    assert_eq!(policy.auth.name.as_deref(), Some("key"));
    assert_eq!(policy.auth.value.as_deref(), Some("sk-test-credential"));
}
