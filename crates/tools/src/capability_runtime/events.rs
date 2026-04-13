use runtime::PermissionMode;
use serde_json::Value;

use super::executor::CapabilityDispatchKind;
use super::provider::CapabilityConcurrencyPolicy;

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityExecutionRequest {
    pub tool_name: String,
    pub capability_id: String,
    pub dispatch_kind: CapabilityDispatchKind,
    pub required_permission: PermissionMode,
    pub concurrency_policy: CapabilityConcurrencyPolicy,
    pub requires_auth: bool,
    pub requires_approval: bool,
    pub input: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityExecutionPhase {
    Started,
    Completed,
    Failed,
    BlockedApproval,
    BlockedAuth,
    Denied,
    Cancelled,
    Interrupted,
    Degraded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityExecutionEvent {
    pub phase: CapabilityExecutionPhase,
    pub tool_name: String,
    pub capability_id: String,
    pub dispatch_kind: CapabilityDispatchKind,
    pub required_permission: PermissionMode,
    pub concurrency_policy: CapabilityConcurrencyPolicy,
    pub requires_auth: bool,
    pub requires_approval: bool,
    pub detail: Option<String>,
}

impl CapabilityExecutionEvent {
    pub(crate) fn from_request(
        request: &CapabilityExecutionRequest,
        phase: CapabilityExecutionPhase,
        detail: Option<String>,
    ) -> Self {
        Self {
            phase,
            tool_name: request.tool_name.clone(),
            capability_id: request.capability_id.clone(),
            dispatch_kind: request.dispatch_kind,
            required_permission: request.required_permission,
            concurrency_policy: request.concurrency_policy,
            requires_auth: request.requires_auth,
            requires_approval: request.requires_approval,
            detail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityMediationDecision {
    Allow,
    RequireApproval(Option<String>),
    RequireAuth(Option<String>),
    Deny(String),
}
