use octopus_sdk_model::{FallbackPolicy, FallbackTrigger, ModelError, ModelId};

#[test]
fn overloaded_errors_trigger_fallback() {
    let policy = FallbackPolicy::default().with_route(
        ModelId("claude-opus-4-6".to_string()),
        ModelId("gpt-4o".to_string()),
    );

    assert_eq!(
        policy.should_fallback(&ModelError::Overloaded {
            retry_after_ms: Some(250),
        }),
        Some(FallbackTrigger::Overloaded)
    );
}

#[test]
fn next_model_uses_second_model_in_route() {
    let policy = FallbackPolicy::default().with_route(
        ModelId("claude-opus-4-6".to_string()),
        ModelId("gpt-4o".to_string()),
    );

    assert_eq!(
        policy.next_model(&ModelId("claude-opus-4-6".to_string())),
        Some(&ModelId("gpt-4o".to_string()))
    );
}

#[test]
fn model_not_found_does_not_trigger_fallback() {
    let policy = FallbackPolicy::default().with_route(
        ModelId("claude-opus-4-6".to_string()),
        ModelId("gpt-4o".to_string()),
    );

    assert_eq!(
        policy.should_fallback(&ModelError::ModelNotFound {
            id: ModelId("missing".to_string()),
        }),
        None
    );
}

#[test]
fn upstream_5xx_and_prompt_too_long_are_supported_triggers() {
    let policy = FallbackPolicy::default();

    assert_eq!(
        policy.should_fallback(&ModelError::UpstreamStatus {
            status: 503,
            body_preview: "busy".to_string(),
        }),
        Some(FallbackTrigger::Upstream5xx)
    );
    assert_eq!(
        policy.should_fallback(&ModelError::PromptTooLong {
            estimated_tokens: 4097,
            max: 4096,
        }),
        Some(FallbackTrigger::PromptTooLong)
    );
}
