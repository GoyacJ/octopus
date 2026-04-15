use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedSubrunInput {
    pub(super) message_id: Option<String>,
    pub(super) content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedSubrunDispatch {
    pub(super) dispatch_key: String,
    pub(super) parent_actor_ref: String,
    pub(super) worker_actor_ref: String,
    pub(super) worker_input: PersistedSubrunInput,
    pub(super) workflow_run_id: Option<String>,
    pub(super) workflow_step_ref: Option<String>,
    pub(super) mailbox_ref: String,
    pub(super) mailbox_policy: octopus_core::MailboxPolicy,
    pub(super) handoff_ref: String,
    pub(super) artifact_handoff_policy: octopus_core::ArtifactHandoffPolicy,
    pub(super) workflow_affordance: octopus_core::WorkflowAffordance,
    pub(super) worker_concurrency_ceiling: u64,
}

impl Default for PersistedSubrunDispatch {
    fn default() -> Self {
        Self {
            dispatch_key: String::new(),
            parent_actor_ref: String::new(),
            worker_actor_ref: String::new(),
            worker_input: PersistedSubrunInput::default(),
            workflow_run_id: None,
            workflow_step_ref: None,
            mailbox_ref: String::new(),
            mailbox_policy: octopus_core::default_mailbox_policy(),
            handoff_ref: String::new(),
            artifact_handoff_policy: octopus_core::default_artifact_handoff_policy(),
            workflow_affordance: octopus_core::WorkflowAffordance {
                supported_task_kinds: Vec::new(),
                background_capable: false,
                automation_capable: false,
            },
            worker_concurrency_ceiling: 0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedSubrunState {
    pub(super) manifest_snapshot_ref: String,
    pub(super) session_policy_snapshot_ref: String,
    #[serde(default)]
    pub(super) dispatch: PersistedSubrunDispatch,
    pub(super) run: RuntimeRunSnapshot,
}

fn subrun_current_step(status: &str) -> &'static str {
    match status {
        "completed" => "completed",
        "cancelled" => "cancelled",
        "failed" => "failed",
        "running" => "running",
        "waiting_approval" => "awaiting_approval",
        "auth-required" => "awaiting_auth",
        _ => "queued",
    }
}

fn subrun_next_action(status: &str) -> &'static str {
    match status {
        "completed" | "cancelled" | "failed" => "idle",
        _ => "resume_subrun",
    }
}

fn subrun_approval_state(status: &str) -> &'static str {
    match status {
        "waiting_approval" => "pending",
        "auth-required" => "auth-required",
        "cancelled" => "cancelled",
        _ => "not-required",
    }
}

pub(crate) fn build_worker_dispatch_summary(
    subruns: &[RuntimeSubrunSummary],
) -> RuntimeWorkerDispatchSummary {
    RuntimeWorkerDispatchSummary {
        total_subruns: subruns.len() as u64,
        active_subruns: subruns
            .iter()
            .filter(|subrun| subrun.status == "running")
            .count() as u64,
        completed_subruns: subruns
            .iter()
            .filter(|subrun| subrun.status == "completed")
            .count() as u64,
        failed_subruns: subruns
            .iter()
            .filter(|subrun| matches!(subrun.status.as_str(), "failed" | "cancelled"))
            .count() as u64,
    }
}

fn subrun_summary_from_state(state: &PersistedSubrunState) -> RuntimeSubrunSummary {
    RuntimeSubrunSummary {
        run_id: state.run.id.clone(),
        parent_run_id: state.run.parent_run_id.clone(),
        actor_ref: state.run.actor_ref.clone(),
        label: state
            .run
            .resolved_actor_label
            .clone()
            .unwrap_or_else(|| worker_runtime::worker_label(&state.run.actor_ref)),
        status: state.run.status.clone(),
        run_kind: state.run.run_kind.clone(),
        delegated_by_tool_call_id: state.run.delegated_by_tool_call_id.clone(),
        workflow_run_id: state.run.workflow_run.clone(),
        mailbox_ref: state.run.mailbox_ref.clone(),
        handoff_ref: state.run.handoff_ref.clone(),
        started_at: state.run.started_at,
        updated_at: state.run.updated_at,
    }
}

fn primary_run_has_own_blocking_mediation(detail: &RuntimeSessionDetail) -> bool {
    detail
        .run
        .checkpoint
        .pending_approval
        .as_ref()
        .is_some_and(|approval| approval.run_id == detail.run.id && approval.status == "pending")
        || detail
            .run
            .checkpoint
            .pending_auth_challenge
            .as_ref()
            .is_some_and(|challenge| {
                challenge.run_id == detail.run.id && challenge.status == "pending"
            })
        || detail.pending_approval.as_ref().is_some_and(|approval| {
            approval.run_id == detail.run.id && approval.status == "pending"
        })
        || detail.run.auth_target.as_ref().is_some_and(|challenge| {
            challenge.run_id == detail.run.id && challenge.status == "pending"
        })
}

fn blocked_subrun_state<'a>(
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &'a BTreeMap<String, PersistedSubrunState>,
    status: &str,
) -> Option<&'a PersistedSubrunState> {
    subruns.iter().find_map(|subrun| {
        let state = subrun_states.get(&subrun.run_id)?;
        (state.run.status == status).then_some(state)
    })
}

fn pending_subrun_approval(
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
) -> Option<ApprovalRequestRecord> {
    blocked_subrun_state(subruns, subrun_states, "waiting_approval").and_then(|state| {
        state
            .run
            .approval_target
            .clone()
            .or_else(|| state.run.checkpoint.pending_approval.clone())
    })
}

fn pending_subrun_auth_challenge(
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
) -> Option<RuntimeAuthChallengeSummary> {
    blocked_subrun_state(subruns, subrun_states, "auth-required").and_then(|state| {
        state
            .run
            .auth_target
            .clone()
            .or_else(|| state.run.checkpoint.pending_auth_challenge.clone())
    })
}

fn pending_subrun_mediation(
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
) -> Option<RuntimePendingMediationSummary> {
    blocked_subrun_state(subruns, subrun_states, "waiting_approval")
        .or_else(|| blocked_subrun_state(subruns, subrun_states, "auth-required"))
        .and_then(|state| {
            state
                .run
                .pending_mediation
                .clone()
                .or_else(|| state.run.checkpoint.pending_mediation.clone())
        })
}

fn apply_subrun_lineage_state(
    detail: &mut RuntimeSessionDetail,
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
) {
    let mailbox_channel = detail
        .pending_mailbox
        .as_ref()
        .map(|mailbox| mailbox.channel.clone())
        .filter(|channel| !channel.trim().is_empty())
        .or_else(|| {
            subrun_states.values().find_map(|state| {
                let mode = state.dispatch.mailbox_policy.mode.trim();
                (!mode.is_empty()).then(|| mode.to_string())
            })
        })
        .unwrap_or_else(|| octopus_core::default_mailbox_policy().mode);
    let mailbox_ref = detail
        .pending_mailbox
        .as_ref()
        .map(|mailbox| mailbox.mailbox_ref.clone())
        .or_else(|| detail.run.mailbox_ref.clone())
        .or_else(|| subruns.iter().find_map(|subrun| subrun.mailbox_ref.clone()))
        .unwrap_or_else(|| format!("mailbox-{}", detail.run.id));
    let updated_at = subruns
        .iter()
        .map(|subrun| subrun.updated_at)
        .max()
        .unwrap_or(detail.run.updated_at);
    let primary_run_blocked = primary_run_has_own_blocking_mediation(detail);

    let existing_handoffs = detail
        .handoffs
        .iter()
        .cloned()
        .map(|handoff| (handoff.handoff_ref.clone(), handoff))
        .collect::<BTreeMap<_, _>>();
    let mut handoffs = Vec::with_capacity(subruns.len());
    for subrun in subruns {
        let runtime_artifact_refs = subrun_states
            .get(&subrun.run_id)
            .map(|state| state.run.artifact_refs.clone())
            .unwrap_or_default();
        let handoff_ref = subrun
            .handoff_ref
            .clone()
            .unwrap_or_else(|| format!("handoff-{}", subrun.run_id));
        let handoff_state = handoff_runtime::handoff_state_for_subrun(
            subruns,
            primary_run_blocked,
            &subrun.status,
        );
        let mut handoff =
            existing_handoffs
                .get(&handoff_ref)
                .cloned()
                .unwrap_or(RuntimeHandoffSummary {
                    handoff_ref: handoff_ref.clone(),
                    mailbox_ref: subrun
                        .mailbox_ref
                        .clone()
                        .unwrap_or_else(|| mailbox_ref.clone()),
                    sender_actor_ref: detail.run.actor_ref.clone(),
                    receiver_actor_ref: subrun.actor_ref.clone(),
                    state: handoff_state.into(),
                    artifact_refs: runtime_artifact_refs.clone(),
                    updated_at: subrun.updated_at,
                });
        let (sender_actor_ref, receiver_actor_ref) = handoff_runtime::handoff_actor_refs(
            &detail.run.actor_ref,
            &subrun.actor_ref,
            handoff_state,
        );
        handoff.mailbox_ref = subrun
            .mailbox_ref
            .clone()
            .unwrap_or_else(|| mailbox_ref.clone());
        handoff.sender_actor_ref = sender_actor_ref.to_string();
        handoff.receiver_actor_ref = receiver_actor_ref.to_string();
        handoff.state = handoff_state.into();
        handoff.updated_at = subrun.updated_at;
        if !runtime_artifact_refs.is_empty() {
            handoff.artifact_refs = runtime_artifact_refs;
        }
        handoffs.push(handoff);
    }
    handoffs.sort_by(|left, right| {
        left.updated_at
            .cmp(&right.updated_at)
            .then_with(|| left.handoff_ref.cmp(&right.handoff_ref))
    });

    let mailbox = mailbox_runtime::summarize_handoffs(
        &mailbox_ref,
        &mailbox_channel,
        &handoffs,
        updated_at,
    );

    let workflow_run_id = detail
        .workflow
        .as_ref()
        .map(|workflow| workflow.workflow_run_id.clone())
        .or_else(|| detail.run.workflow_run.clone())
        .or_else(|| {
            subruns
                .iter()
                .find_map(|subrun| subrun.workflow_run_id.clone())
        })
        .unwrap_or_else(|| format!("workflow-{}", detail.run.id));
    let background_capable = detail
        .workflow
        .as_ref()
        .map(|workflow| workflow.background_capable)
        .or_else(|| {
            detail
                .background_run
                .as_ref()
                .map(|background| background.background_capable)
        })
        .unwrap_or(false);
    let workflow_label = detail
        .workflow
        .as_ref()
        .map(|workflow| workflow.label.clone())
        .unwrap_or_else(|| format!("{} workflow", detail.summary.title));

    let background_run_id = detail
        .background_run
        .as_ref()
        .map(|background| background.run_id.clone())
        .unwrap_or_else(|| detail.run.id.clone());

    if !primary_run_has_own_blocking_mediation(detail) {
        let pending_approval = pending_subrun_approval(subruns, subrun_states);
        let pending_auth_challenge = pending_subrun_auth_challenge(subruns, subrun_states);
        let pending_mediation = pending_subrun_mediation(subruns, subrun_states);

        detail.pending_approval = pending_approval.clone();
        detail.pending_mediation = pending_mediation.clone();
        detail.run.approval_target = pending_approval.clone();
        detail.run.auth_target = pending_auth_challenge.clone();
        detail.run.pending_mediation = pending_mediation.clone();

        if pending_approval.is_some() {
            detail.run.status = "waiting_approval".into();
            detail.run.current_step = "awaiting_approval".into();
            detail.run.next_action = Some("approval".into());
            detail.run.approval_state = "pending".into();
        } else if pending_auth_challenge.is_some() {
            detail.run.status = "auth-required".into();
            detail.run.current_step = "awaiting_auth".into();
            detail.run.next_action = Some("auth".into());
            detail.run.approval_state = "auth-required".into();
        } else if subruns
            .iter()
            .any(|subrun| matches!(subrun.status.as_str(), "failed" | "cancelled"))
        {
            detail.run.status = "failed".into();
            detail.run.current_step = "failed".into();
            detail.run.next_action = Some("blocked".into());
            detail.run.approval_state = "not-required".into();
        } else if matches!(
            detail.run.status.as_str(),
            "waiting_approval" | "auth-required"
        ) {
            detail.run.status = "completed".into();
            detail.run.current_step = "completed".into();
            detail.run.next_action = Some("idle".into());
            detail.run.approval_state = "not-required".into();
        }
    }

    let existing_workflow_detail = detail.run.workflow_run_detail.clone();
    let (workflow, workflow_detail, background) = workflow_runtime::build_workflow_projection(
        &workflow_label,
        &detail.run,
        &workflow_run_id,
        &background_run_id,
        subruns,
        subrun_states,
        existing_workflow_detail.as_ref(),
        background_capable,
        updated_at,
    );

    detail.handoffs = handoffs;
    detail.pending_mailbox = Some(mailbox.clone());
    detail.workflow = Some(workflow);
    detail.background_run = Some(background.clone());
    detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
    detail.run.handoff_ref = detail
        .handoffs
        .iter()
        .find(|handoff| {
            !matches!(
                handoff.state.as_str(),
                "acknowledged" | "failed" | "cancelled"
            )
        })
        .map(|handoff| handoff.handoff_ref.clone())
        .or_else(|| {
            detail
                .handoffs
                .first()
                .map(|handoff| handoff.handoff_ref.clone())
        });
    detail.run.workflow_run = Some(workflow_run_id);
    detail.run.workflow_run_detail = Some(workflow_detail);
    detail.run.background_state = Some(background.status);
}

pub(crate) fn apply_subrun_state_projection(
    detail: &mut RuntimeSessionDetail,
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
) {
    if subrun_states.is_empty() {
        return;
    }

    let mut subruns = detail.subruns.clone();
    let existing_ids = subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<BTreeSet<_>>();

    for subrun in &mut subruns {
        if let Some(state) = subrun_states.get(&subrun.run_id) {
            *subrun = subrun_summary_from_state(state);
        }
    }

    for (run_id, state) in subrun_states {
        if existing_ids.contains(run_id) {
            continue;
        }
        subruns.push(subrun_summary_from_state(state));
    }

    subruns.sort_by(|left, right| {
        left.started_at
            .cmp(&right.started_at)
            .then_with(|| left.run_id.cmp(&right.run_id))
    });

    detail.subrun_count = subruns.len() as u64;
    detail.run.worker_dispatch = Some(build_worker_dispatch_summary(&subruns));
    apply_subrun_lineage_state(detail, &subruns, subrun_states);
    detail.subruns = subruns;
}

fn apply_team_runtime_scaffold(
    detail: &mut RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    now: u64,
) {
    let workflow_run_id = format!("workflow-{}", detail.run.id);
    let mailbox_ref = format!("mailbox-{}", detail.run.id);
    let mailbox_channel = mailbox_runtime::mailbox_channel(team);
    let mailbox = mailbox_runtime::build_mailbox_summary(
        &mailbox_ref,
        &mailbox_channel,
        &detail.run.status,
        0,
        now,
    );
    let empty_subruns = Vec::new();
    let empty_subrun_states = BTreeMap::new();
    let existing_workflow_detail = detail.run.workflow_run_detail.clone();
    let (workflow, workflow_detail, background) = workflow_runtime::build_workflow_projection(
        &format!("{} workflow", team.record.name),
        &detail.run,
        &workflow_run_id,
        &detail.run.id,
        &empty_subruns,
        &empty_subrun_states,
        existing_workflow_detail.as_ref(),
        team.record.workflow_affordance.background_capable,
        now,
    );

    detail.subrun_count = 0;
    detail.subruns.clear();
    detail.handoffs.clear();
    detail.pending_mailbox = Some(mailbox.clone());
    detail.workflow = Some(workflow.clone());
    detail.background_run = Some(background.clone());

    detail.run.workflow_run = Some(workflow_run_id);
    detail.run.workflow_run_detail = Some(workflow_detail);
    detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
    detail.run.handoff_ref = None;
    detail.run.background_state = Some(background.status.clone());
    detail.run.worker_dispatch = Some(build_worker_dispatch_summary(&[]));
}

pub(crate) fn apply_team_runtime_state(
    detail: &mut RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    subrun_states: &BTreeMap<String, PersistedSubrunState>,
    now: u64,
) {
    apply_team_runtime_scaffold(detail, team, now);
    if !subrun_states.is_empty() {
        apply_subrun_state_projection(detail, subrun_states);
    }
}

fn worker_session_policy(
    adapter: &RuntimeAdapter,
    session_policy: &session_policy::CompiledSessionPolicy,
    subrun: &RuntimeSubrunSummary,
) -> Result<
    (
        actor_manifest::CompiledActorManifest,
        session_policy::CompiledSessionPolicy,
    ),
    AppError,
> {
    let worker_manifest = adapter.compile_actor_manifest(&subrun.actor_ref)?;
    let mut worker_policy = session_policy.clone();
    worker_policy.selected_actor_ref = worker_manifest.actor_ref().into();
    worker_policy.manifest_revision = worker_manifest.manifest_revision().into();
    worker_policy.capability_policy = worker_manifest.capability_policy_value();
    worker_policy.memory_policy = worker_manifest.memory_policy_value();
    worker_policy.delegation_policy = worker_manifest.delegation_policy_value();
    worker_policy.approval_preference = worker_manifest.approval_preference_value();
    worker_policy.target_decisions = policy_compiler::compile_manifest_target_decisions(
        &worker_manifest,
        &worker_policy.execution_permission_mode,
        worker_policy.selected_configured_model_id.as_deref(),
    );
    Ok((worker_manifest, worker_policy))
}

fn latest_subrun_input(detail: &RuntimeSessionDetail) -> PersistedSubrunInput {
    detail
        .messages
        .iter()
        .rev()
        .find(|message| message.sender_type == "user" && !message.content.trim().is_empty())
        .map(|message| PersistedSubrunInput {
            message_id: Some(message.id.clone()),
            content: message.content.clone(),
        })
        .unwrap_or_else(|| PersistedSubrunInput {
            message_id: None,
            content: detail
                .summary
                .last_message_preview
                .clone()
                .unwrap_or_default(),
        })
}

fn merge_frozen_subrun_dispatch(
    existing: &mut PersistedSubrunDispatch,
    generated: PersistedSubrunDispatch,
) {
    if existing.dispatch_key.trim().is_empty() {
        existing.dispatch_key = generated.dispatch_key;
    }
    if existing.parent_actor_ref.trim().is_empty() {
        existing.parent_actor_ref = generated.parent_actor_ref;
    }
    if existing.worker_actor_ref.trim().is_empty() {
        existing.worker_actor_ref = generated.worker_actor_ref;
    }
    if existing.worker_input.content.trim().is_empty() {
        existing.worker_input = generated.worker_input;
    }
    if existing.workflow_run_id.is_none() {
        existing.workflow_run_id = generated.workflow_run_id;
    }
    if existing.workflow_step_ref.is_none() {
        existing.workflow_step_ref = generated.workflow_step_ref;
    }
    if existing.mailbox_ref.trim().is_empty() {
        existing.mailbox_ref = generated.mailbox_ref;
    }
    if existing.mailbox_policy.mode.trim().is_empty() {
        existing.mailbox_policy = generated.mailbox_policy;
    }
    if existing.handoff_ref.trim().is_empty() {
        existing.handoff_ref = generated.handoff_ref;
    }
    if existing.artifact_handoff_policy.mode.trim().is_empty() {
        existing.artifact_handoff_policy = generated.artifact_handoff_policy;
    }
    if existing.workflow_affordance.supported_task_kinds.is_empty()
        && !existing.workflow_affordance.background_capable
        && !existing.workflow_affordance.automation_capable
    {
        existing.workflow_affordance = generated.workflow_affordance;
    }
    if existing.worker_concurrency_ceiling == 0 {
        existing.worker_concurrency_ceiling = generated.worker_concurrency_ceiling;
    }
}

fn subrun_dispatch(
    detail: &RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    subrun: &RuntimeSubrunSummary,
) -> PersistedSubrunDispatch {
    PersistedSubrunDispatch {
        dispatch_key: subrun
            .delegated_by_tool_call_id
            .clone()
            .unwrap_or_else(|| format!("dispatch-{}", subrun.run_id)),
        parent_actor_ref: detail.run.actor_ref.clone(),
        worker_actor_ref: subrun.actor_ref.clone(),
        worker_input: latest_subrun_input(detail),
        workflow_run_id: subrun.workflow_run_id.clone(),
        workflow_step_ref: Some(subrun.run_id.clone()),
        mailbox_ref: subrun
            .mailbox_ref
            .clone()
            .unwrap_or_else(|| format!("mailbox-{}", detail.run.id)),
        mailbox_policy: team.record.mailbox_policy.clone(),
        handoff_ref: subrun
            .handoff_ref
            .clone()
            .unwrap_or_else(|| format!("handoff-{}", subrun.run_id)),
        artifact_handoff_policy: team.record.artifact_handoff_policy.clone(),
        workflow_affordance: team.record.workflow_affordance.clone(),
        worker_concurrency_ceiling: worker_runtime::worker_concurrency_limit(team) as u64,
    }
}

fn planned_subruns_for_session(
    detail: &RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    now: u64,
) -> Vec<RuntimeSubrunSummary> {
    let workflow_run_id = detail
        .run
        .workflow_run
        .clone()
        .unwrap_or_else(|| format!("workflow-{}", detail.run.id));
    let mailbox_ref = detail
        .run
        .mailbox_ref
        .clone()
        .or_else(|| {
            detail
                .pending_mailbox
                .as_ref()
                .map(|mailbox| mailbox.mailbox_ref.clone())
        })
        .unwrap_or_else(|| format!("mailbox-{}", detail.run.id));
    subrun_orchestrator::planned_subruns(team, &detail.run, &workflow_run_id, &mailbox_ref, now)
}

fn subrun_serialized_session(
    input: &PersistedSubrunInput,
    trace_context: &RuntimeTraceContext,
) -> Value {
    json!({
        "content": input.content,
        "pendingContent": input.content,
        "traceContext": trace_context,
        "pendingToolUses": [],
        "session": {
            "messages": [{
                "role": "user",
                "blocks": [{
                    "type": "text",
                    "text": input.content,
                }],
                "usage": Value::Null,
            }],
        }
    })
}

fn subrun_checkpoint_missing_serialized_messages(serialized_session: &Value) -> bool {
    !serialized_session
        .get("session")
        .and_then(|session| session.get("messages"))
        .and_then(Value::as_array)
        .is_some_and(|messages| !messages.is_empty())
}

fn seed_subrun_checkpoint_from_dispatch(
    run: &mut RuntimeRunSnapshot,
    dispatch: &PersistedSubrunDispatch,
) {
    if subrun_checkpoint_missing_serialized_messages(&run.checkpoint.serialized_session) {
        run.checkpoint.serialized_session =
            subrun_serialized_session(&dispatch.worker_input, &run.trace_context);
    }
    if run.checkpoint.input.is_none() && !dispatch.worker_input.content.trim().is_empty() {
        run.checkpoint.input = Some(json!({
            "content": dispatch.worker_input.content,
        }));
    }
    if run.checkpoint.dispatch_kind.is_none() {
        run.checkpoint.dispatch_kind = Some("team-subrun".into());
    }
    if run.checkpoint.concurrency_policy.is_none() {
        run.checkpoint.concurrency_policy = Some(if dispatch.worker_concurrency_ceiling <= 1 {
            "serialized".into()
        } else {
            "parallel".into()
        });
    }
}

pub(crate) fn ensure_subrun_state_metadata_for_session(
    adapter: &RuntimeAdapter,
    aggregate: &mut RuntimeAggregate,
    team: &actor_manifest::CompiledTeamManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    now: u64,
) -> Result<(), AppError> {
    let planned_subruns = planned_subruns_for_session(&aggregate.detail, team, now);
    let active_subrun_ids = planned_subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<BTreeSet<_>>();

    for subrun in &planned_subruns {
        let dispatch = subrun_dispatch(&aggregate.detail, team, subrun);
        if let Some(existing) = aggregate.metadata.subrun_states.get_mut(&subrun.run_id) {
            if existing.run.status.trim().is_empty() {
                existing.run.status = subrun.status.clone();
            }
            existing.run.current_step = subrun_current_step(&existing.run.status).into();
            existing.run.updated_at = now;
            existing.run.next_action = Some(subrun_next_action(&existing.run.status).into());
            existing.run.run_kind = subrun.run_kind.clone();
            existing.run.parent_run_id = subrun.parent_run_id.clone();
            existing.run.actor_ref = subrun.actor_ref.clone();
            existing.run.delegated_by_tool_call_id = subrun.delegated_by_tool_call_id.clone();
            existing.run.workflow_run = subrun.workflow_run_id.clone();
            existing.run.mailbox_ref = subrun.mailbox_ref.clone();
            existing.run.handoff_ref = subrun.handoff_ref.clone();
            existing.run.approval_state = subrun_approval_state(&existing.run.status).into();
            merge_frozen_subrun_dispatch(&mut existing.dispatch, dispatch);
            seed_subrun_checkpoint_from_dispatch(&mut existing.run, &existing.dispatch);
            continue;
        }

        let (worker_manifest, mut worker_policy) =
            worker_session_policy(adapter, session_policy, subrun)?;
        let manifest_snapshot_ref = format!("{}-manifest", subrun.run_id);
        adapter.persist_actor_manifest_snapshot(&manifest_snapshot_ref, &worker_manifest)?;
        let session_policy_snapshot_ref = format!("{}-policy", subrun.run_id);
        worker_policy.manifest_snapshot_ref = manifest_snapshot_ref.clone();
        worker_policy.session_policy_snapshot_ref = session_policy_snapshot_ref.clone();
        adapter.persist_session_policy_snapshot(&session_policy_snapshot_ref, &worker_policy)?;
        let capability_state_ref = format!("{}-capability-state", subrun.run_id);
        let capability_projection = adapter.project_capability_state(
            &worker_manifest,
            &worker_policy,
            &worker_policy.config_snapshot_id,
            capability_state_ref.clone(),
            &tools::SessionCapabilityStore::default(),
        )?;
        let memory_state_ref =
            memory_runtime::runtime_memory_state_ref(&subrun.run_id, subrun.updated_at);

        aggregate.metadata.subrun_states.insert(
            subrun.run_id.clone(),
            PersistedSubrunState {
                manifest_snapshot_ref,
                session_policy_snapshot_ref,
                dispatch: dispatch.clone(),
                run: RuntimeRunSnapshot {
                    id: subrun.run_id.clone(),
                    session_id: aggregate.detail.summary.id.clone(),
                    conversation_id: aggregate.detail.summary.conversation_id.clone(),
                    status: subrun.status.clone(),
                    current_step: subrun_current_step(&subrun.status).into(),
                    started_at: subrun.started_at,
                    updated_at: subrun.updated_at,
                    selected_memory: Vec::new(),
                    freshness_summary: Some(RuntimeMemoryFreshnessSummary::default()),
                    pending_memory_proposal: None,
                    memory_state_ref,
                    configured_model_id: worker_policy.selected_configured_model_id.clone(),
                    configured_model_name: None,
                    model_id: None,
                    consumed_tokens: None,
                    next_action: Some(subrun_next_action(&subrun.status).into()),
                    config_snapshot_id: worker_policy.config_snapshot_id.clone(),
                    effective_config_hash: worker_policy.effective_config_hash.clone(),
                    started_from_scope_set: worker_policy.started_from_scope_set.clone(),
                    run_kind: subrun.run_kind.clone(),
                    parent_run_id: subrun.parent_run_id.clone(),
                    actor_ref: subrun.actor_ref.clone(),
                    delegated_by_tool_call_id: subrun.delegated_by_tool_call_id.clone(),
                    workflow_run: subrun.workflow_run_id.clone(),
                    workflow_run_detail: None,
                    mailbox_ref: subrun.mailbox_ref.clone(),
                    handoff_ref: subrun.handoff_ref.clone(),
                    background_state: None,
                    worker_dispatch: None,
                    approval_state: subrun_approval_state(&subrun.status).into(),
                    approval_target: None,
                    auth_target: None,
                    usage_summary: RuntimeUsageSummary::default(),
                    artifact_refs: Vec::new(),
                    trace_context: trace_context::runtime_trace_context(
                        &aggregate.detail.summary.id,
                        Some(subrun.run_id.clone()),
                    ),
                    checkpoint: RuntimeRunCheckpoint {
                        approval_layer: None,
                        broker_decision: None,
                        capability_id: None,
                        checkpoint_artifact_ref: None,
                        serialized_session: subrun_serialized_session(
                            &dispatch.worker_input,
                            &trace_context::runtime_trace_context(
                                &aggregate.detail.summary.id,
                                Some(subrun.run_id.clone()),
                            ),
                        ),
                        current_iteration_index: 0,
                        tool_name: None,
                        dispatch_kind: Some("team-subrun".into()),
                        concurrency_policy: Some(if dispatch.worker_concurrency_ceiling <= 1 {
                            "serialized".into()
                        } else {
                            "parallel".into()
                        }),
                        input: Some(json!({
                            "content": dispatch.worker_input.content,
                        })),
                        usage_summary: RuntimeUsageSummary::default(),
                        pending_approval: None,
                        pending_auth_challenge: None,
                        compaction_metadata: json!({}),
                        pending_mediation: None,
                        provider_key: None,
                        reason: Some("team-worker-subrun".into()),
                        required_permission: None,
                        requires_approval: None,
                        requires_auth: None,
                        target_kind: None,
                        target_ref: None,
                        capability_state_ref: Some(capability_state_ref.clone()),
                        capability_plan_summary: capability_projection.plan_summary.clone(),
                        last_execution_outcome: None,
                        last_mediation_outcome: None,
                    },
                    capability_plan_summary: capability_projection.plan_summary,
                    provider_state_summary: capability_projection.provider_state_summary,
                    pending_mediation: None,
                    capability_state_ref: Some(capability_state_ref),
                    last_execution_outcome: None,
                    last_mediation_outcome: None,
                    resolved_target: None,
                    requested_actor_kind: Some(worker_manifest.actor_kind_label().into()),
                    requested_actor_id: Some(worker_manifest.actor_ref().into()),
                    resolved_actor_kind: Some(worker_manifest.actor_kind_label().into()),
                    resolved_actor_id: Some(worker_manifest.actor_ref().into()),
                    resolved_actor_label: Some(worker_manifest.label().into()),
                },
            },
        );
    }
    sync_subrun_state_metadata(aggregate, now);
    aggregate
        .metadata
        .subrun_states
        .retain(|run_id, _| active_subrun_ids.contains(run_id));
    apply_team_runtime_state(
        &mut aggregate.detail,
        team,
        &aggregate.metadata.subrun_states,
        now,
    );

    Ok(())
}

pub(crate) fn ensure_subrun_state_metadata(
    adapter: &RuntimeAdapter,
    aggregate: &mut RuntimeAggregate,
    run_context: &run_context::RunContext,
) -> Result<(), AppError> {
    let actor_manifest::CompiledActorManifest::Team(team) = &run_context.actor_manifest else {
        return Ok(());
    };
    ensure_subrun_state_metadata_for_session(
        adapter,
        aggregate,
        team,
        &run_context.session_policy,
        run_context.now,
    )
}

pub(crate) fn sync_subrun_state_metadata(aggregate: &mut RuntimeAggregate, now: u64) {
    for subrun in &aggregate.detail.subruns {
        if let Some(state) = aggregate.metadata.subrun_states.get_mut(&subrun.run_id) {
            state.run.status = subrun.status.clone();
            state.run.current_step = subrun_current_step(&subrun.status).into();
            state.run.updated_at = now;
            state.run.next_action = Some(subrun_next_action(&subrun.status).into());
            state.run.run_kind = subrun.run_kind.clone();
            state.run.parent_run_id = subrun.parent_run_id.clone();
            state.run.actor_ref = subrun.actor_ref.clone();
            state.run.delegated_by_tool_call_id = subrun.delegated_by_tool_call_id.clone();
            state.run.workflow_run = subrun.workflow_run_id.clone();
            state.run.mailbox_ref = subrun.mailbox_ref.clone();
            state.run.handoff_ref = subrun.handoff_ref.clone();
            state.run.approval_state = subrun_approval_state(&subrun.status).into();
        }
    }
}
