use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use harness_contracts::{
    Event, McpServerId, PermissionMode, RequestId, RunId, SamplingBudgetDimension,
    SamplingDenyReason, SamplingOutcome, SessionId, TrustLevel,
};
use harness_mcp::{
    AggregateBudget, McpEventSink, McpTimeouts, ModelAllowlist, SamplingAllow, SamplingBudget,
    SamplingCachePolicy, SamplingDecision, SamplingPolicy, SamplingProvider, SamplingRateLimit,
    SamplingRequest, SamplingResponse, SamplingUsageSnapshot, MCP_SAMPLING_BUDGET_EXCEEDED_CODE,
    MCP_SAMPLING_DENIED_CODE,
};
use parking_lot::Mutex;
use serde_json::json;

#[test]
fn denied_policy_rejects_and_emits_event() {
    let sink = Arc::new(CollectingSink::default());
    let decision = SamplingPolicy::denied().evaluate(
        sample_request(),
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        sink.clone(),
    );

    assert!(matches!(
        decision,
        SamplingDecision::Rejected { ref error, .. }
            if error.code == MCP_SAMPLING_DENIED_CODE
    ));
    assert!(matches!(
        sink.events().first(),
        Some(Event::McpSamplingRequested(event))
            if matches!(
                event.outcome,
                SamplingOutcome::Denied {
                    reason: SamplingDenyReason::PolicyDenied
                }
            )
    ));
}

#[test]
fn allow_auto_fails_closed_for_user_controlled_server() {
    let mut request = sample_request();
    request.server_trust = TrustLevel::UserControlled;
    let sink = Arc::new(CollectingSink::default());

    let decision = SamplingPolicy::allow_auto().evaluate(
        request,
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        sink.clone(),
    );

    assert!(matches!(
        decision,
        SamplingDecision::Rejected { ref error, .. }
            if error.code == MCP_SAMPLING_DENIED_CODE
    ));
    assert!(matches!(
        sink.events().first(),
        Some(Event::McpSamplingRequested(event))
            if matches!(
                event.outcome,
                SamplingOutcome::Denied {
                    reason: SamplingDenyReason::InlineUserSourceRefused
                }
            )
    ));
}

#[test]
fn permission_modes_downgrade_sampling_access() {
    let mut request = sample_request();
    request.permission_mode = PermissionMode::BypassPermissions;
    let denied = SamplingPolicy::allow_with_approval().evaluate(
        request,
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );
    assert!(matches!(
        denied,
        SamplingDecision::Rejected { ref error, .. }
            if error.code == MCP_SAMPLING_DENIED_CODE
    ));

    let mut plan_request = sample_request();
    plan_request.permission_mode = PermissionMode::Plan;
    let approval = SamplingPolicy::allow_auto().evaluate(
        plan_request,
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );
    assert!(matches!(
        approval,
        SamplingDecision::RequiresApproval { .. }
    ));
}

#[test]
fn per_request_budget_exceeded_returns_sampling_budget_error() {
    let policy = SamplingPolicy {
        allow: SamplingAllow::AllowAuto,
        per_request: SamplingBudget {
            max_input_tokens: 8,
            max_output_tokens: 4,
            max_tool_rounds: 0,
            timeout: Duration::from_secs(10),
        },
        ..SamplingPolicy::allow_auto()
    };
    let mut request = sample_request();
    request.input_tokens = 9;

    let decision = policy.evaluate(
        request,
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );

    assert!(matches!(
        decision,
        SamplingDecision::Rejected {
            ref error,
            outcome: SamplingOutcome::BudgetExceeded {
                dimension: SamplingBudgetDimension::PerRequestInputTokens
            },
        } if error.code == MCP_SAMPLING_BUDGET_EXCEEDED_CODE
    ));
}

#[test]
fn aggregate_and_rate_limits_are_enforced() {
    let aggregate_policy = SamplingPolicy {
        allow: SamplingAllow::AllowAuto,
        aggregate: AggregateBudget {
            per_server_session_input_tokens: 10,
            per_server_session_output_tokens: 100,
            per_session_input_tokens: 100,
            per_session_output_tokens: 100,
            lock_after_exceeded: true,
        },
        ..SamplingPolicy::allow_auto()
    };
    let aggregate_decision = aggregate_policy.evaluate(
        sample_request(),
        SamplingUsageSnapshot {
            per_server_session_input_tokens: 9,
            ..SamplingUsageSnapshot::default()
        },
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );
    assert!(matches!(
        aggregate_decision,
        SamplingDecision::Rejected {
            outcome: SamplingOutcome::BudgetExceeded {
                dimension: SamplingBudgetDimension::PerServerSessionInput
            },
            ..
        }
    ));

    let rate_policy = SamplingPolicy {
        allow: SamplingAllow::AllowAuto,
        rate_limit: SamplingRateLimit {
            per_server_rps: 1.0,
            per_session_rps: 10.0,
            burst: 10,
        },
        ..SamplingPolicy::allow_auto()
    };
    let rate_decision = rate_policy.evaluate(
        sample_request(),
        SamplingUsageSnapshot {
            current_per_server_rps: 1.0,
            ..SamplingUsageSnapshot::default()
        },
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );
    assert!(matches!(
        rate_decision,
        SamplingDecision::Rejected {
            ref error,
            outcome: SamplingOutcome::RateLimited,
        } if error.code == MCP_SAMPLING_BUDGET_EXCEEDED_CODE
    ));
}

#[test]
fn accepted_decision_uses_isolated_cache_and_effective_timeout() {
    let policy = SamplingPolicy {
        allow: SamplingAllow::AllowAuto,
        cache: SamplingCachePolicy::IsolatedNamespace {
            ttl: Duration::from_secs(300),
        },
        per_request: SamplingBudget {
            timeout: Duration::from_secs(20),
            ..SamplingBudget::default()
        },
        ..SamplingPolicy::allow_auto()
    };
    let timeouts = McpTimeouts {
        sampling: Duration::from_secs(5),
        ..McpTimeouts::default()
    };

    let decision = policy.evaluate(
        sample_request(),
        SamplingUsageSnapshot::default(),
        timeouts,
        Arc::new(CollectingSink::default()),
    );

    assert!(matches!(
        decision,
        SamplingDecision::Allowed {
            effective_timeout,
            ref prompt_cache_namespace,
            ..
        } if effective_timeout == Duration::from_secs(5)
            && prompt_cache_namespace == "mcp::sampling::github::00000000000000000000000001"
    ));
}

#[test]
fn model_allowlist_rejects_unlisted_model() {
    let policy = SamplingPolicy {
        allow: SamplingAllow::AllowAuto,
        allowed_models: ModelAllowlist::restricted(["claude-3-5-sonnet".to_owned()]),
        ..SamplingPolicy::allow_auto()
    };
    let mut request = sample_request();
    request.model_id = Some("unlisted".to_owned());

    let decision = policy.evaluate(
        request,
        SamplingUsageSnapshot::default(),
        McpTimeouts::default(),
        Arc::new(CollectingSink::default()),
    );

    assert!(matches!(
        decision,
        SamplingDecision::Rejected {
            outcome: SamplingOutcome::Denied {
                reason: SamplingDenyReason::ModelNotAllowed
            },
            ..
        }
    ));
}

#[test]
fn sampling_provider_is_object_safe() {
    let provider: Arc<dyn SamplingProvider> = Arc::new(EchoSamplingProvider);
    assert_eq!(Arc::strong_count(&provider), 1);
}

fn sample_request() -> SamplingRequest {
    SamplingRequest {
        session_id: SessionId::from_u128(1),
        run_id: Some(RunId::from_u128(2)),
        server_id: McpServerId("github".to_owned()),
        request_id: RequestId::from_u128(3),
        model_id: Some("claude-3-5-sonnet".to_owned()),
        input_tokens: 2,
        max_output_tokens: 4,
        tool_rounds: 0,
        requested_timeout: None,
        permission_mode: PermissionMode::Default,
        server_trust: TrustLevel::AdminTrusted,
        params: json!({ "messages": [] }),
    }
}

#[derive(Default)]
struct CollectingSink {
    events: Mutex<Vec<Event>>,
}

impl CollectingSink {
    fn events(&self) -> Vec<Event> {
        self.events.lock().clone()
    }
}

impl McpEventSink for CollectingSink {
    fn emit(&self, event: Event) {
        self.events.lock().push(event);
    }
}

struct EchoSamplingProvider;

#[async_trait]
impl SamplingProvider for EchoSamplingProvider {
    async fn create_message(
        &self,
        _request: SamplingRequest,
    ) -> Result<SamplingResponse, harness_mcp::McpError> {
        Ok(SamplingResponse {
            model_id: "mock".to_owned(),
            content: json!({ "text": "ok" }),
            input_tokens: 1,
            output_tokens: 1,
        })
    }
}
