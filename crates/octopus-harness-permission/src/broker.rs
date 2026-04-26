use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use harness_contracts::{
    Decision, DecisionId, DecisionScope, FallbackPolicy, InteractivityLevel, PermissionError,
    PermissionMode, PermissionSubject, RequestId, SessionId, Severity, TenantId, TimeoutPolicy,
    ToolUseId,
};

use crate::rule::{OverrideDecision, RuleSnapshot};

#[async_trait]
pub trait PermissionBroker: Send + Sync + 'static {
    async fn decide(&self, request: PermissionRequest, ctx: PermissionContext) -> Decision;

    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    pub request_id: RequestId,
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub subject: PermissionSubject,
    pub severity: Severity,
    pub scope_hint: DecisionScope,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionContext {
    pub permission_mode: PermissionMode,
    pub previous_mode: Option<PermissionMode>,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub interactivity: InteractivityLevel,
    pub timeout_policy: Option<TimeoutPolicy>,
    pub fallback_policy: FallbackPolicy,
    pub rule_snapshot: Arc<RuleSnapshot>,
    pub hook_overrides: Vec<OverrideDecision>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionCheck {
    Allowed,
    Denied {
        reason: String,
    },
    AskUser {
        subject: PermissionSubject,
        scope: DecisionScope,
    },
    DangerousCommand {
        pattern: String,
        severity: Severity,
    },
}
