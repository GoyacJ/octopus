use std::{collections::BTreeSet, sync::Arc, time::Duration};

use async_trait::async_trait;
use harness_contracts::{
    now, Event, McpSamplingRequestedEvent, McpServerId, PermissionMode, RequestId, RunId,
    SamplingBudgetDimension, SamplingDenyReason, SamplingOutcome, SessionId, TrustLevel,
};
use serde_json::{json, Value};

use crate::{JsonRpcError, McpError, McpEventSink, McpTimeouts};

pub const MCP_SAMPLING_DENIED_CODE: i32 = -32601;
pub const MCP_SAMPLING_BUDGET_EXCEEDED_CODE: i32 = -32029;

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingPolicy {
    pub allow: SamplingAllow,
    pub allowed_models: ModelAllowlist,
    pub per_request: SamplingBudget,
    pub aggregate: AggregateBudget,
    pub rate_limit: SamplingRateLimit,
    pub log_level: SamplingLogLevel,
    pub cache: SamplingCachePolicy,
}

impl SamplingPolicy {
    pub fn denied() -> Self {
        Self {
            allow: SamplingAllow::Denied,
            allowed_models: ModelAllowlist::default(),
            per_request: SamplingBudget::default(),
            aggregate: AggregateBudget::default(),
            rate_limit: SamplingRateLimit::default(),
            log_level: SamplingLogLevel::Summary,
            cache: SamplingCachePolicy::default(),
        }
    }

    pub fn allow_auto() -> Self {
        Self {
            allow: SamplingAllow::AllowAuto,
            ..Self::denied()
        }
    }

    pub fn allow_with_approval() -> Self {
        Self {
            allow: SamplingAllow::AllowWithApproval,
            ..Self::denied()
        }
    }

    pub fn is_denied(&self) -> bool {
        self.allow == SamplingAllow::Denied
    }

    pub fn evaluate(
        &self,
        request: SamplingRequest,
        usage: SamplingUsageSnapshot,
        timeouts: McpTimeouts,
        event_sink: Arc<dyn McpEventSink>,
    ) -> SamplingDecision {
        let effective_timeout = self.effective_timeout(&request, timeouts);
        let prompt_cache_namespace = self.cache.namespace(&request);

        match self.effective_allow(&request) {
            EffectiveSamplingAllow::Denied(reason) => {
                return self.reject(
                    request,
                    event_sink,
                    prompt_cache_namespace,
                    SamplingOutcome::Denied { reason },
                    MCP_SAMPLING_DENIED_CODE,
                    "sampling/createMessage denied",
                );
            }
            EffectiveSamplingAllow::RequiresApproval => {
                return SamplingDecision::RequiresApproval {
                    request,
                    effective_timeout,
                    prompt_cache_namespace,
                };
            }
            EffectiveSamplingAllow::Allowed => {}
        }

        if !self.allowed_models.allows(request.model_id.as_deref()) {
            return self.reject(
                request,
                event_sink,
                prompt_cache_namespace,
                SamplingOutcome::Denied {
                    reason: SamplingDenyReason::ModelNotAllowed,
                },
                MCP_SAMPLING_DENIED_CODE,
                "sampling model is not allowed",
            );
        }

        if let Some(dimension) = self.per_request.exceeded_by(&request) {
            return self.reject(
                request,
                event_sink,
                prompt_cache_namespace,
                SamplingOutcome::BudgetExceeded { dimension },
                MCP_SAMPLING_BUDGET_EXCEEDED_CODE,
                "sampling per-request budget exceeded",
            );
        }

        if let Some(dimension) = self.aggregate.exceeded_by(&request, &usage) {
            return self.reject(
                request,
                event_sink,
                prompt_cache_namespace,
                SamplingOutcome::BudgetExceeded { dimension },
                MCP_SAMPLING_BUDGET_EXCEEDED_CODE,
                "sampling aggregate budget exceeded",
            );
        }

        if self.rate_limit.exceeded_by(&usage) {
            return self.reject(
                request,
                event_sink,
                prompt_cache_namespace,
                SamplingOutcome::RateLimited,
                MCP_SAMPLING_BUDGET_EXCEEDED_CODE,
                "sampling rate limit exceeded",
            );
        }

        SamplingDecision::Allowed {
            request,
            effective_timeout,
            prompt_cache_namespace,
        }
    }

    fn effective_timeout(&self, request: &SamplingRequest, timeouts: McpTimeouts) -> Duration {
        let requested = request
            .requested_timeout
            .unwrap_or(self.per_request.timeout);
        requested
            .min(self.per_request.timeout)
            .min(timeouts.sampling)
    }

    fn effective_allow(&self, request: &SamplingRequest) -> EffectiveSamplingAllow {
        match self.allow {
            SamplingAllow::Denied => {
                EffectiveSamplingAllow::Denied(SamplingDenyReason::PolicyDenied)
            }
            SamplingAllow::AllowWithApproval
                if matches!(
                    request.permission_mode,
                    PermissionMode::BypassPermissions | PermissionMode::DontAsk
                ) =>
            {
                EffectiveSamplingAllow::Denied(SamplingDenyReason::PermissionModeBlocked)
            }
            SamplingAllow::AllowWithApproval => EffectiveSamplingAllow::RequiresApproval,
            SamplingAllow::AllowAuto if request.server_trust == TrustLevel::UserControlled => {
                EffectiveSamplingAllow::Denied(SamplingDenyReason::InlineUserSourceRefused)
            }
            SamplingAllow::AllowAuto if request.permission_mode == PermissionMode::Plan => {
                EffectiveSamplingAllow::RequiresApproval
            }
            SamplingAllow::AllowAuto => EffectiveSamplingAllow::Allowed,
        }
    }

    fn reject(
        &self,
        request: SamplingRequest,
        event_sink: Arc<dyn McpEventSink>,
        prompt_cache_namespace: String,
        outcome: SamplingOutcome,
        code: i32,
        message: &'static str,
    ) -> SamplingDecision {
        emit_sampling_event(
            &request,
            &prompt_cache_namespace,
            outcome.clone(),
            event_sink,
        );
        SamplingDecision::Rejected {
            error: JsonRpcError {
                code,
                message: message.to_owned(),
                data: Some(json!({ "server_id": request.server_id.0 })),
            },
            outcome,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingAllow {
    Denied,
    AllowWithApproval,
    AllowAuto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingBudget {
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_tool_rounds: u8,
    pub timeout: Duration,
}

impl Default for SamplingBudget {
    fn default() -> Self {
        Self {
            max_input_tokens: 8_192,
            max_output_tokens: 4_096,
            max_tool_rounds: 0,
            timeout: Duration::from_secs(30),
        }
    }
}

impl SamplingBudget {
    fn exceeded_by(&self, request: &SamplingRequest) -> Option<SamplingBudgetDimension> {
        if request.input_tokens > self.max_input_tokens {
            return Some(SamplingBudgetDimension::PerRequestInputTokens);
        }
        if request.max_output_tokens > self.max_output_tokens {
            return Some(SamplingBudgetDimension::PerRequestOutputTokens);
        }
        if request.tool_rounds > self.max_tool_rounds {
            return Some(SamplingBudgetDimension::PerRequestToolRounds);
        }
        if request
            .requested_timeout
            .is_some_and(|timeout| timeout > self.timeout)
        {
            return Some(SamplingBudgetDimension::PerRequestTimeout);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateBudget {
    pub per_server_session_input_tokens: u64,
    pub per_server_session_output_tokens: u64,
    pub per_session_input_tokens: u64,
    pub per_session_output_tokens: u64,
    pub lock_after_exceeded: bool,
}

impl Default for AggregateBudget {
    fn default() -> Self {
        Self {
            per_server_session_input_tokens: 64_000,
            per_server_session_output_tokens: 32_000,
            per_session_input_tokens: 256_000,
            per_session_output_tokens: 128_000,
            lock_after_exceeded: true,
        }
    }
}

impl AggregateBudget {
    fn exceeded_by(
        &self,
        request: &SamplingRequest,
        usage: &SamplingUsageSnapshot,
    ) -> Option<SamplingBudgetDimension> {
        if usage
            .per_server_session_input_tokens
            .saturating_add(request.input_tokens)
            > self.per_server_session_input_tokens
        {
            return Some(SamplingBudgetDimension::PerServerSessionInput);
        }
        if usage
            .per_server_session_output_tokens
            .saturating_add(request.max_output_tokens)
            > self.per_server_session_output_tokens
        {
            return Some(SamplingBudgetDimension::PerServerSessionOutput);
        }
        if usage
            .per_session_input_tokens
            .saturating_add(request.input_tokens)
            > self.per_session_input_tokens
        {
            return Some(SamplingBudgetDimension::PerSessionInput);
        }
        if usage
            .per_session_output_tokens
            .saturating_add(request.max_output_tokens)
            > self.per_session_output_tokens
        {
            return Some(SamplingBudgetDimension::PerSessionOutput);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingRateLimit {
    pub per_server_rps: f32,
    pub per_session_rps: f32,
    pub burst: u32,
}

impl Default for SamplingRateLimit {
    fn default() -> Self {
        Self {
            per_server_rps: 1.0,
            per_session_rps: 4.0,
            burst: 4,
        }
    }
}

impl SamplingRateLimit {
    fn exceeded_by(&self, usage: &SamplingUsageSnapshot) -> bool {
        usage.current_per_server_rps >= self.per_server_rps
            || usage.current_per_session_rps >= self.per_session_rps
            || usage.burst_used >= self.burst
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingLogLevel {
    None,
    Summary,
    FullPrompt,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamplingCachePolicy {
    IsolatedNamespace { ttl: Duration },
    SharedWithMainSession { namespace: String },
}

impl Default for SamplingCachePolicy {
    fn default() -> Self {
        Self::IsolatedNamespace {
            ttl: Duration::from_secs(300),
        }
    }
}

impl SamplingCachePolicy {
    pub fn namespace(&self, request: &SamplingRequest) -> String {
        match self {
            Self::IsolatedNamespace { .. } => {
                format!(
                    "mcp::sampling::{}::{}",
                    request.server_id.0, request.session_id
                )
            }
            Self::SharedWithMainSession { namespace } => namespace.clone(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ModelAllowlist {
    #[default]
    InheritSession,
    Restricted(BTreeSet<String>),
}

impl ModelAllowlist {
    pub fn restricted(models: impl IntoIterator<Item = String>) -> Self {
        Self::Restricted(models.into_iter().collect())
    }

    pub fn allows(&self, model_id: Option<&str>) -> bool {
        match self {
            Self::InheritSession => true,
            Self::Restricted(models) => model_id.is_some_and(|model_id| models.contains(model_id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingRequest {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub model_id: Option<String>,
    pub input_tokens: u64,
    pub max_output_tokens: u64,
    pub tool_rounds: u8,
    pub requested_timeout: Option<Duration>,
    pub permission_mode: PermissionMode,
    pub server_trust: TrustLevel,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingResponse {
    pub model_id: String,
    pub content: Value,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingUsageSnapshot {
    pub per_server_session_input_tokens: u64,
    pub per_server_session_output_tokens: u64,
    pub per_session_input_tokens: u64,
    pub per_session_output_tokens: u64,
    pub current_per_server_rps: f32,
    pub current_per_session_rps: f32,
    pub burst_used: u32,
}

impl Default for SamplingUsageSnapshot {
    fn default() -> Self {
        Self {
            per_server_session_input_tokens: 0,
            per_server_session_output_tokens: 0,
            per_session_input_tokens: 0,
            per_session_output_tokens: 0,
            current_per_server_rps: 0.0,
            current_per_session_rps: 0.0,
            burst_used: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SamplingDecision {
    Allowed {
        request: SamplingRequest,
        effective_timeout: Duration,
        prompt_cache_namespace: String,
    },
    RequiresApproval {
        request: SamplingRequest,
        effective_timeout: Duration,
        prompt_cache_namespace: String,
    },
    Rejected {
        error: JsonRpcError,
        outcome: SamplingOutcome,
    },
}

#[async_trait]
pub trait SamplingProvider: Send + Sync + 'static {
    async fn create_message(&self, request: SamplingRequest) -> Result<SamplingResponse, McpError>;
}

enum EffectiveSamplingAllow {
    Allowed,
    RequiresApproval,
    Denied(SamplingDenyReason),
}

fn emit_sampling_event(
    request: &SamplingRequest,
    prompt_cache_namespace: &str,
    outcome: SamplingOutcome,
    event_sink: Arc<dyn McpEventSink>,
) {
    event_sink.emit(Event::McpSamplingRequested(McpSamplingRequestedEvent {
        session_id: request.session_id,
        run_id: request.run_id,
        server_id: request.server_id.clone(),
        request_id: request.request_id,
        model_id: match outcome {
            SamplingOutcome::Completed => request.model_id.clone(),
            _ => None,
        },
        input_tokens: request.input_tokens,
        output_tokens: request.max_output_tokens,
        latency_ms: 0,
        outcome,
        prompt_cache_namespace: prompt_cache_namespace.to_owned(),
        at: now(),
    }));
}
