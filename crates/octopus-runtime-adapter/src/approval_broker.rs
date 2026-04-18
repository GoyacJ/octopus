use super::*;

#[derive(Debug, Clone)]
pub(crate) struct MediationRequest {
    pub(crate) session_id: String,
    pub(crate) conversation_id: String,
    pub(crate) run_id: String,
    pub(crate) tool_name: String,
    pub(crate) summary: String,
    pub(crate) detail: String,
    pub(crate) mediation_kind: String,
    pub(crate) approval_layer: String,
    pub(crate) target_kind: String,
    pub(crate) target_ref: String,
    pub(crate) capability_id: Option<String>,
    pub(crate) dispatch_kind: String,
    pub(crate) provider_key: Option<String>,
    pub(crate) concurrency_policy: String,
    pub(crate) input: Value,
    pub(crate) required_permission: Option<String>,
    pub(crate) escalation_reason: Option<String>,
    pub(crate) requires_approval: bool,
    pub(crate) requires_auth: bool,
    pub(crate) created_at: u64,
    pub(crate) risk_level: String,
    pub(crate) checkpoint_ref: Option<String>,
    pub(crate) policy_action: Option<String>,
    pub(crate) pending_state: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BrokerDecision {
    pub(crate) state: String,
    pub(crate) pending_mediation: Option<RuntimePendingMediationSummary>,
    pub(crate) approval: Option<ApprovalRequestRecord>,
    pub(crate) auth_challenge: Option<RuntimeAuthChallengeSummary>,
    pub(crate) execution_outcome: RuntimeCapabilityExecutionOutcome,
    pub(crate) mediation_outcome: Option<RuntimeMediationOutcome>,
}

fn execution_outcome(
    request: &MediationRequest,
    outcome: &str,
    detail: Option<String>,
) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: request.capability_id.clone(),
        tool_name: Some(request.tool_name.clone()),
        provider_key: request.provider_key.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        outcome: outcome.into(),
        detail,
        requires_approval: request.requires_approval,
        requires_auth: request.requires_auth,
        concurrency_policy: Some(request.concurrency_policy.clone()),
    }
}

pub(crate) fn allow(request: &MediationRequest) -> BrokerDecision {
    BrokerDecision {
        state: "allow".into(),
        pending_mediation: None,
        approval: None,
        auth_challenge: None,
        execution_outcome: execution_outcome(request, "allow", None),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail: Some(request.detail.clone()),
            mediation_id: None,
            mediation_kind: request.mediation_kind.clone(),
            outcome: "allow".into(),
            provider_key: request.provider_key.clone(),
            reason: request.escalation_reason.clone(),
            requires_approval: request.requires_approval,
            requires_auth: request.requires_auth,
            resolved_at: Some(request.created_at),
            target_kind: request.target_kind.clone(),
            target_ref: request.target_ref.clone(),
            tool_name: Some(request.tool_name.clone()),
        }),
    }
}

pub(crate) fn deny(request: &MediationRequest) -> BrokerDecision {
    BrokerDecision {
        state: "deny".into(),
        pending_mediation: None,
        approval: None,
        auth_challenge: None,
        execution_outcome: execution_outcome(
            request,
            "deny",
            Some(
                request
                    .escalation_reason
                    .clone()
                    .unwrap_or_else(|| request.detail.clone()),
            ),
        ),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail: Some(request.detail.clone()),
            mediation_id: None,
            mediation_kind: request.mediation_kind.clone(),
            outcome: "deny".into(),
            provider_key: request.provider_key.clone(),
            reason: request.escalation_reason.clone(),
            requires_approval: request.requires_approval,
            requires_auth: request.requires_auth,
            resolved_at: Some(request.created_at),
            target_kind: request.target_kind.clone(),
            target_ref: request.target_ref.clone(),
            tool_name: Some(request.tool_name.clone()),
        }),
    }
}

pub(crate) fn require_approval(request: &MediationRequest) -> BrokerDecision {
    let approval_id = format!("approval-{}", Uuid::new_v4());
    let mediation_id = format!("mediation-{}", Uuid::new_v4());
    let approval = ApprovalRequestRecord {
        id: approval_id.clone(),
        session_id: request.session_id.clone(),
        conversation_id: request.conversation_id.clone(),
        run_id: request.run_id.clone(),
        tool_name: request.tool_name.clone(),
        summary: request.summary.clone(),
        detail: request.detail.clone(),
        risk_level: request.risk_level.clone(),
        created_at: request.created_at,
        status: "pending".into(),
        approval_layer: Some(request.approval_layer.clone()),
        capability_id: request.capability_id.clone(),
        checkpoint_ref: request.checkpoint_ref.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        provider_key: request.provider_key.clone(),
        concurrency_policy: Some(request.concurrency_policy.clone()),
        input: Some(request.input.clone()),
        required_permission: request.required_permission.clone(),
        requires_approval: true,
        requires_auth: request.requires_auth,
        target_kind: Some(request.target_kind.clone()),
        target_ref: Some(request.target_ref.clone()),
        escalation_reason: request.escalation_reason.clone(),
    };
    let pending_mediation = RuntimePendingMediationSummary {
        approval_id: Some(approval_id),
        approval_layer: Some(request.approval_layer.clone()),
        auth_challenge_id: None,
        capability_id: request.capability_id.clone(),
        checkpoint_ref: request.checkpoint_ref.clone(),
        detail: Some(request.detail.clone()),
        escalation_reason: request.escalation_reason.clone(),
        mediation_id: Some(mediation_id.clone()),
        mediation_kind: request.mediation_kind.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        provider_key: request.provider_key.clone(),
        concurrency_policy: Some(request.concurrency_policy.clone()),
        input: Some(request.input.clone()),
        reason: request.escalation_reason.clone(),
        required_permission: request.required_permission.clone(),
        requires_approval: true,
        requires_auth: request.requires_auth,
        state: request
            .pending_state
            .clone()
            .unwrap_or_else(|| "pending".into()),
        summary: Some(request.summary.clone()),
        target_kind: request.target_kind.clone(),
        target_ref: request.target_ref.clone(),
        tool_name: Some(request.tool_name.clone()),
    };

    BrokerDecision {
        state: "requireApproval".into(),
        pending_mediation: Some(pending_mediation),
        approval: Some(approval),
        auth_challenge: None,
        execution_outcome: execution_outcome(
            request,
            "require_approval",
            Some(request.detail.clone()),
        ),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail: Some(request.detail.clone()),
            mediation_id: Some(mediation_id),
            mediation_kind: request.mediation_kind.clone(),
            outcome: "pending".into(),
            provider_key: request.provider_key.clone(),
            reason: request.escalation_reason.clone(),
            requires_approval: request.requires_approval,
            requires_auth: request.requires_auth,
            resolved_at: None,
            target_kind: request.target_kind.clone(),
            target_ref: request.target_ref.clone(),
            tool_name: Some(request.tool_name.clone()),
        }),
    }
}

pub(crate) fn require_auth(request: &MediationRequest) -> BrokerDecision {
    let challenge_id = format!("auth-{}", Uuid::new_v4());
    let mediation_id = format!("mediation-{}", Uuid::new_v4());
    let challenge = RuntimeAuthChallengeSummary {
        approval_layer: request.approval_layer.clone(),
        capability_id: request.capability_id.clone(),
        checkpoint_ref: request.checkpoint_ref.clone(),
        conversation_id: request.conversation_id.clone(),
        created_at: request.created_at,
        detail: request.detail.clone(),
        escalation_reason: request.escalation_reason.clone().unwrap_or_default(),
        id: challenge_id.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        provider_key: request.provider_key.clone(),
        concurrency_policy: Some(request.concurrency_policy.clone()),
        input: Some(request.input.clone()),
        required_permission: request.required_permission.clone(),
        requires_approval: request.requires_approval,
        requires_auth: true,
        resolution: None,
        run_id: request.run_id.clone(),
        session_id: request.session_id.clone(),
        status: "pending".into(),
        summary: request.summary.clone(),
        target_kind: request.target_kind.clone(),
        target_ref: request.target_ref.clone(),
        tool_name: Some(request.tool_name.clone()),
    };
    let pending_mediation = RuntimePendingMediationSummary {
        approval_id: None,
        approval_layer: Some(request.approval_layer.clone()),
        auth_challenge_id: Some(challenge_id),
        capability_id: request.capability_id.clone(),
        checkpoint_ref: request.checkpoint_ref.clone(),
        detail: Some(request.detail.clone()),
        escalation_reason: request.escalation_reason.clone(),
        mediation_id: Some(mediation_id.clone()),
        mediation_kind: request.mediation_kind.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        provider_key: request.provider_key.clone(),
        concurrency_policy: Some(request.concurrency_policy.clone()),
        input: Some(request.input.clone()),
        reason: request.escalation_reason.clone(),
        required_permission: request.required_permission.clone(),
        requires_approval: request.requires_approval,
        requires_auth: true,
        state: request
            .pending_state
            .clone()
            .unwrap_or_else(|| "pending".into()),
        summary: Some(request.summary.clone()),
        target_kind: request.target_kind.clone(),
        target_ref: request.target_ref.clone(),
        tool_name: Some(request.tool_name.clone()),
    };

    BrokerDecision {
        state: "requireAuth".into(),
        pending_mediation: Some(pending_mediation),
        approval: None,
        auth_challenge: Some(challenge),
        execution_outcome: execution_outcome(request, "require_auth", Some(request.detail.clone())),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail: Some(request.detail.clone()),
            mediation_id: Some(mediation_id),
            mediation_kind: request.mediation_kind.clone(),
            outcome: "pending".into(),
            provider_key: request.provider_key.clone(),
            reason: request.escalation_reason.clone(),
            requires_approval: request.requires_approval,
            requires_auth: request.requires_auth,
            resolved_at: None,
            target_kind: request.target_kind.clone(),
            target_ref: request.target_ref.clone(),
            tool_name: Some(request.tool_name.clone()),
        }),
    }
}

pub(crate) fn defer(request: &MediationRequest) -> BrokerDecision {
    let mediation_id = request
        .checkpoint_ref
        .clone()
        .unwrap_or_else(|| format!("mediation-{}", Uuid::new_v4()));
    let pending_mediation = RuntimePendingMediationSummary {
        approval_id: None,
        approval_layer: Some(request.approval_layer.clone()),
        auth_challenge_id: None,
        capability_id: request.capability_id.clone(),
        checkpoint_ref: request.checkpoint_ref.clone(),
        detail: Some(request.detail.clone()),
        escalation_reason: request.escalation_reason.clone(),
        mediation_id: Some(mediation_id.clone()),
        mediation_kind: request.mediation_kind.clone(),
        dispatch_kind: Some(request.dispatch_kind.clone()),
        provider_key: request.provider_key.clone(),
        concurrency_policy: Some(request.concurrency_policy.clone()),
        input: Some(request.input.clone()),
        reason: request.escalation_reason.clone(),
        required_permission: request.required_permission.clone(),
        requires_approval: request.requires_approval,
        requires_auth: request.requires_auth,
        state: request
            .pending_state
            .clone()
            .unwrap_or_else(|| "pending".into()),
        summary: Some(request.summary.clone()),
        target_kind: request.target_kind.clone(),
        target_ref: request.target_ref.clone(),
        tool_name: Some(request.tool_name.clone()),
    };

    BrokerDecision {
        state: "defer".into(),
        pending_mediation: Some(pending_mediation),
        approval: None,
        auth_challenge: None,
        execution_outcome: execution_outcome(request, "defer", Some(request.detail.clone())),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail: Some(request.detail.clone()),
            mediation_id: Some(mediation_id),
            mediation_kind: request.mediation_kind.clone(),
            outcome: "pending".into(),
            provider_key: request.provider_key.clone(),
            reason: request.escalation_reason.clone(),
            requires_approval: request.requires_approval,
            requires_auth: request.requires_auth,
            resolved_at: None,
            target_kind: request.target_kind.clone(),
            target_ref: request.target_ref.clone(),
            tool_name: Some(request.tool_name.clone()),
        }),
    }
}

pub(crate) fn mediate(request: &MediationRequest) -> BrokerDecision {
    match request.policy_action.as_deref() {
        Some("deny") => deny(request),
        Some("defer") => defer(request),
        Some("allow") => allow(request),
        Some("requireApproval") => require_approval(request),
        Some("requireAuth") => require_auth(request),
        Some(_) | None => {
            if request.requires_auth {
                require_auth(request)
            } else if request.requires_approval {
                require_approval(request)
            } else {
                allow(request)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mediate, MediationRequest};

    #[test]
    fn routes_all_mediation_through_one_entrypoint() {
        let base_request = MediationRequest {
            session_id: "rt-1".into(),
            conversation_id: "conv-1".into(),
            run_id: "run-1".into(),
            tool_name: "workspace-api".into(),
            summary: "Workspace API call requires mediation".into(),
            detail: "Review before the tool call can continue.".into(),
            mediation_kind: "approval".into(),
            approval_layer: "capability-call".into(),
            target_kind: "capability-call".into(),
            target_ref: "capability-call:run-1:tool-use-1".into(),
            capability_id: Some("cap-1".into()),
            dispatch_kind: "runtime_capability".into(),
            provider_key: None,
            concurrency_policy: "serialized".into(),
            input: serde_json::json!({ "path": "." }),
            required_permission: Some("workspace-write".into()),
            escalation_reason: Some("approval required".into()),
            requires_approval: true,
            requires_auth: false,
            created_at: 1,
            risk_level: "high".into(),
            checkpoint_ref: None,
            policy_action: None,
            pending_state: None,
        };

        let approval = mediate(&base_request);
        assert_eq!(approval.state, "requireApproval");
        assert!(approval.pending_mediation.is_some());
        assert!(approval.approval.is_some());

        let auth = mediate(&MediationRequest {
            mediation_kind: "auth".into(),
            approval_layer: "provider-auth".into(),
            target_kind: "provider-auth".into(),
            target_ref: "mcp-server".into(),
            provider_key: Some("mcp-server".into()),
            escalation_reason: Some("auth required".into()),
            requires_approval: false,
            requires_auth: true,
            policy_action: None,
            ..base_request.clone()
        });
        assert_eq!(auth.state, "requireAuth");
        assert!(auth.pending_mediation.is_some());
        assert!(auth.auth_challenge.is_some());

        let allowed = mediate(&MediationRequest {
            requires_approval: false,
            requires_auth: false,
            escalation_reason: None,
            policy_action: Some("allow".into()),
            ..base_request.clone()
        });
        assert_eq!(allowed.state, "allow");
        assert!(allowed.pending_mediation.is_none());

        let denied = mediate(&MediationRequest {
            requires_approval: false,
            requires_auth: false,
            policy_action: Some("deny".into()),
            escalation_reason: Some("policy denied".into()),
            ..base_request.clone()
        });
        assert_eq!(denied.state, "deny");
        assert!(denied.pending_mediation.is_none());
        assert_eq!(denied.execution_outcome.outcome, "deny");

        let deferred = mediate(&MediationRequest {
            mediation_kind: "memory".into(),
            approval_layer: "memory-review".into(),
            target_kind: "memory-write".into(),
            target_ref: "proposal-1".into(),
            tool_name: "Agent".into(),
            summary: "Memory proposal pending review".into(),
            detail: "Durable memory stays proposal-only until review.".into(),
            requires_approval: false,
            requires_auth: false,
            policy_action: Some("defer".into()),
            pending_state: Some("pending_review".into()),
            ..base_request
        });
        assert_eq!(deferred.state, "defer");
        assert_eq!(
            deferred
                .pending_mediation
                .as_ref()
                .map(|mediation| mediation.state.as_str()),
            Some("pending_review")
        );
        assert!(deferred.approval.is_none());
        assert!(deferred.auth_challenge.is_none());
    }
}
