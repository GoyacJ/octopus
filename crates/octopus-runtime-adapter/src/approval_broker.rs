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
    pub(crate) provider_key: Option<String>,
    pub(crate) required_permission: Option<String>,
    pub(crate) escalation_reason: Option<String>,
    pub(crate) requires_approval: bool,
    pub(crate) requires_auth: bool,
    pub(crate) created_at: u64,
    pub(crate) risk_level: String,
    pub(crate) checkpoint_ref: Option<String>,
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

fn execution_outcome(request: &MediationRequest, outcome: &str, detail: Option<String>) -> RuntimeCapabilityExecutionOutcome {
    RuntimeCapabilityExecutionOutcome {
        capability_id: request.capability_id.clone(),
        tool_name: Some(request.tool_name.clone()),
        provider_key: request.provider_key.clone(),
        dispatch_kind: Some(request.target_kind.clone()),
        outcome: outcome.into(),
        detail,
        requires_approval: request.requires_approval,
        requires_auth: request.requires_auth,
        concurrency_policy: Some("serialized".into()),
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

pub(crate) fn deny(request: &MediationRequest, detail: Option<String>) -> BrokerDecision {
    BrokerDecision {
        state: "deny".into(),
        pending_mediation: None,
        approval: None,
        auth_challenge: None,
        execution_outcome: execution_outcome(request, "deny", detail.clone()),
        mediation_outcome: Some(RuntimeMediationOutcome {
            approval_layer: Some(request.approval_layer.clone()),
            capability_id: request.capability_id.clone(),
            checkpoint_ref: request.checkpoint_ref.clone(),
            detail,
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
        provider_key: request.provider_key.clone(),
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
        provider_key: request.provider_key.clone(),
        reason: request.escalation_reason.clone(),
        required_permission: request.required_permission.clone(),
        requires_approval: true,
        requires_auth: request.requires_auth,
        state: "pending".into(),
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
        provider_key: request.provider_key.clone(),
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
        provider_key: request.provider_key.clone(),
        reason: request.escalation_reason.clone(),
        required_permission: request.required_permission.clone(),
        requires_approval: request.requires_approval,
        requires_auth: true,
        state: "pending".into(),
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
