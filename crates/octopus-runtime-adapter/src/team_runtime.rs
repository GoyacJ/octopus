use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedSubrunState {
    pub(super) manifest_snapshot_ref: String,
    pub(super) session_policy_snapshot_ref: String,
    pub(super) run: RuntimeRunSnapshot,
}

fn subrun_current_step(status: &str) -> &'static str {
    match status {
        "completed" => "completed",
        "failed" => "failed",
        "running" => "running",
        "waiting_approval" => "waiting_approval",
        "auth-required" => "auth_required",
        _ => "queued",
    }
}

fn subrun_next_action(status: &str) -> &'static str {
    match status {
        "completed" | "failed" => "idle",
        _ => "resume_subrun",
    }
}

fn subrun_approval_state(status: &str) -> &'static str {
    match status {
        "waiting_approval" => "pending",
        "auth-required" => "auth-required",
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
            .filter(|subrun| subrun.status == "failed")
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

fn handoff_state_from_subrun_status(status: &str) -> &'static str {
    match status {
        "completed" => "delivered",
        "failed" => "failed",
        _ => "pending",
    }
}

fn workflow_status_from_subruns(subruns: &[RuntimeSubrunSummary], fallback_status: &str) -> String {
    if subruns.is_empty() {
        return fallback_status.to_string();
    }
    if subruns.iter().any(|subrun| subrun.status == "failed") {
        return "failed".into();
    }
    if subruns
        .iter()
        .any(|subrun| subrun.status == "waiting_approval")
    {
        return "waiting_approval".into();
    }
    if subruns
        .iter()
        .any(|subrun| subrun.status == "auth-required")
    {
        return "auth-required".into();
    }
    if subruns.iter().any(|subrun| subrun.status == "running") {
        return "running".into();
    }
    if subruns.iter().all(|subrun| subrun.status == "completed") {
        return "completed".into();
    }
    "queued".into()
}

fn workflow_step_from_subruns(
    subruns: &[RuntimeSubrunSummary],
    workflow_status: &str,
) -> (Option<String>, Option<String>) {
    if workflow_status == "completed" {
        return (
            Some("workflow-complete".into()),
            Some("Workflow complete".into()),
        );
    }

    if let Some(subrun) = subruns.iter().find(|subrun| subrun.status == "failed") {
        return (Some(subrun.run_id.clone()), Some(subrun.label.clone()));
    }

    if let Some(subrun) = subruns.iter().find(|subrun| subrun.status != "completed") {
        return (Some(subrun.run_id.clone()), Some(subrun.label.clone()));
    }

    (Some("leader-plan".into()), Some("Leader plan".into()))
}

fn apply_subrun_lineage_state(detail: &mut RuntimeSessionDetail, subruns: &[RuntimeSubrunSummary]) {
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

    let existing_handoffs = detail
        .handoffs
        .iter()
        .cloned()
        .map(|handoff| (handoff.handoff_ref.clone(), handoff))
        .collect::<BTreeMap<_, _>>();
    let mut handoffs = Vec::with_capacity(subruns.len());
    for subrun in subruns {
        let handoff_ref = subrun
            .handoff_ref
            .clone()
            .unwrap_or_else(|| format!("handoff-{}", subrun.run_id));
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
                    state: handoff_state_from_subrun_status(&subrun.status).into(),
                    artifact_refs: vec![format!("artifact-{}", subrun.run_id)],
                    updated_at: subrun.updated_at,
                });
        handoff.mailbox_ref = subrun
            .mailbox_ref
            .clone()
            .unwrap_or_else(|| mailbox_ref.clone());
        handoff.receiver_actor_ref = subrun.actor_ref.clone();
        handoff.state = handoff_state_from_subrun_status(&subrun.status).into();
        handoff.updated_at = subrun.updated_at;
        if handoff.artifact_refs.is_empty() {
            handoff.artifact_refs = vec![format!("artifact-{}", subrun.run_id)];
        }
        handoffs.push(handoff);
    }
    handoffs.sort_by(|left, right| {
        left.updated_at
            .cmp(&right.updated_at)
            .then_with(|| left.handoff_ref.cmp(&right.handoff_ref))
    });

    let pending_count = handoffs
        .iter()
        .filter(|handoff| handoff.state != "delivered")
        .count() as u64;
    let mailbox_status = if handoffs.iter().any(|handoff| handoff.state == "failed") {
        "failed"
    } else if pending_count == 0 {
        "completed"
    } else {
        "pending"
    };
    let mut mailbox = detail
        .pending_mailbox
        .clone()
        .unwrap_or(RuntimeMailboxSummary {
            mailbox_ref: mailbox_ref.clone(),
            channel: "team-mailbox".into(),
            status: mailbox_status.into(),
            pending_count,
            total_messages: handoffs.len() as u64,
            updated_at,
        });
    mailbox.mailbox_ref = mailbox_ref.clone();
    mailbox.status = mailbox_status.into();
    mailbox.pending_count = pending_count;
    mailbox.total_messages = handoffs.len() as u64;
    mailbox.updated_at = updated_at;

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
    let workflow_status = workflow_status_from_subruns(subruns, &detail.run.status);
    let total_steps = subruns.len() as u64 + 1;
    let completed_steps = if workflow_status == "completed" {
        total_steps
    } else if subruns.is_empty() {
        0
    } else {
        1 + subruns
            .iter()
            .filter(|subrun| subrun.status == "completed")
            .count() as u64
    };
    let (current_step_id, current_step_label) =
        workflow_step_from_subruns(subruns, &workflow_status);
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
    let workflow = RuntimeWorkflowSummary {
        workflow_run_id: workflow_run_id.clone(),
        label: workflow_label,
        status: workflow_status.clone(),
        total_steps,
        completed_steps,
        current_step_id: current_step_id.clone(),
        current_step_label: current_step_label.clone(),
        background_capable,
        updated_at,
    };
    let workflow_detail = RuntimeWorkflowRunDetail {
        workflow_run_id: workflow_run_id.clone(),
        status: workflow_status.clone(),
        current_step_id,
        current_step_label,
        total_steps,
        completed_steps,
        background_capable,
    };

    let background_run_id = detail
        .background_run
        .as_ref()
        .map(|background| background.run_id.clone())
        .unwrap_or_else(|| detail.run.id.clone());
    let background = RuntimeBackgroundRunSummary {
        run_id: background_run_id,
        workflow_run_id: Some(workflow_run_id.clone()),
        status: workflow_status.clone(),
        background_capable,
        updated_at,
    };

    detail.handoffs = handoffs;
    detail.pending_mailbox = Some(mailbox.clone());
    detail.workflow = Some(workflow);
    detail.background_run = Some(background.clone());
    detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
    detail.run.handoff_ref = detail
        .handoffs
        .iter()
        .find(|handoff| handoff.state != "delivered")
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
    apply_subrun_lineage_state(detail, &subruns);
    detail.subruns = subruns;
}

pub(crate) fn apply_team_runtime_projection(
    detail: &mut RuntimeSessionDetail,
    team: &actor_manifest::CompiledTeamManifest,
    now: u64,
) {
    let workflow_run_id = format!("workflow-{}", detail.run.id);
    let mailbox_ref = format!("mailbox-{}", detail.run.id);
    let (subruns, worker_dispatch) = subrun_orchestrator::build_subrun_projection(
        team,
        &detail.run,
        &workflow_run_id,
        &mailbox_ref,
        now,
    );
    let handoffs =
        handoff_runtime::build_handoff_projection(&detail.run, &subruns, &mailbox_ref, now);
    let mailbox = mailbox_runtime::build_mailbox_summary(
        &mailbox_ref,
        &detail.run.status,
        handoffs.len() as u64,
        now,
    );
    let (workflow, workflow_detail) = workflow_runtime::build_workflow_projection(
        team,
        &detail.run,
        &workflow_run_id,
        subruns.len() as u64,
        now,
    );
    let background = background_runtime::build_background_summary(
        &detail.run,
        &workflow_run_id,
        workflow.background_capable,
        now,
    );

    detail.subrun_count = subruns.len() as u64;
    detail.subruns = subruns;
    detail.handoffs = handoffs.clone();
    detail.workflow = Some(workflow.clone());
    detail.pending_mailbox = Some(mailbox.clone());
    detail.background_run = Some(background.clone());

    detail.run.workflow_run = Some(workflow_run_id);
    detail.run.workflow_run_detail = Some(workflow_detail);
    detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
    detail.run.handoff_ref = handoffs.first().map(|handoff| handoff.handoff_ref.clone());
    detail.run.background_state = Some(background.status.clone());
    detail.run.worker_dispatch = Some(worker_dispatch);
}

pub(crate) fn ensure_subrun_state_metadata(
    adapter: &RuntimeAdapter,
    aggregate: &mut RuntimeAggregate,
    run_context: &run_context::RunContext,
) -> Result<(), AppError> {
    for subrun in &aggregate.detail.subruns {
        let worker_manifest = adapter.compile_actor_manifest(&subrun.actor_ref)?;
        let manifest_snapshot_ref = format!("{}-manifest", subrun.run_id);
        adapter.persist_actor_manifest_snapshot(&manifest_snapshot_ref, &worker_manifest)?;
        let session_policy_snapshot_ref = format!("{}-policy", subrun.run_id);
        let mut session_policy = run_context.session_policy.clone();
        session_policy.selected_actor_ref = worker_manifest.actor_ref().into();
        session_policy.manifest_revision = worker_manifest.manifest_revision().into();
        session_policy.capability_policy = worker_manifest.capability_policy_value();
        session_policy.memory_policy = worker_manifest.memory_policy_value();
        session_policy.delegation_policy = worker_manifest.delegation_policy_value();
        session_policy.approval_preference = worker_manifest.approval_preference_value();
        session_policy.target_decisions = policy_compiler::compile_manifest_target_decisions(
            &worker_manifest,
            &session_policy.execution_permission_mode,
            session_policy.selected_configured_model_id.as_deref(),
        );
        session_policy.manifest_snapshot_ref = manifest_snapshot_ref.clone();
        session_policy.session_policy_snapshot_ref = session_policy_snapshot_ref.clone();
        adapter.persist_session_policy_snapshot(&session_policy_snapshot_ref, &session_policy)?;
        let capability_state_ref = format!("{}-capability-state", subrun.run_id);
        let capability_projection = adapter.project_capability_state(
            &worker_manifest,
            &session_policy,
            &session_policy.config_snapshot_id,
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
                run: RuntimeRunSnapshot {
                    id: subrun.run_id.clone(),
                    session_id: run_context.session_id.clone(),
                    conversation_id: run_context.conversation_id.clone(),
                    status: subrun.status.clone(),
                    current_step: subrun_current_step(&subrun.status).into(),
                    started_at: subrun.started_at,
                    updated_at: subrun.updated_at,
                    selected_memory: Vec::new(),
                    freshness_summary: Some(RuntimeMemoryFreshnessSummary::default()),
                    pending_memory_proposal: None,
                    memory_state_ref,
                    configured_model_id: session_policy.selected_configured_model_id.clone(),
                    configured_model_name: None,
                    model_id: None,
                    consumed_tokens: None,
                    next_action: Some(subrun_next_action(&subrun.status).into()),
                    config_snapshot_id: session_policy.config_snapshot_id.clone(),
                    effective_config_hash: session_policy.effective_config_hash.clone(),
                    started_from_scope_set: session_policy.started_from_scope_set.clone(),
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
                        &run_context.session_id,
                        Some(run_context.run_id.clone()),
                    ),
                    checkpoint: RuntimeRunCheckpoint {
                        approval_layer: None,
                        broker_decision: None,
                        capability_id: None,
                        checkpoint_artifact_ref: None,
                        serialized_session: json!({
                            "subrun": {
                                "parentRunId": run_context.run_id.clone(),
                                "actorRef": subrun.actor_ref.clone(),
                            }
                        }),
                        current_iteration_index: 0,
                        tool_name: None,
                        dispatch_kind: None,
                        concurrency_policy: None,
                        input: None,
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
    sync_subrun_state_metadata(aggregate, run_context.now);
    let active_subrun_ids = aggregate
        .detail
        .subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<BTreeSet<_>>();
    aggregate
        .metadata
        .subrun_states
        .retain(|run_id, _| active_subrun_ids.contains(run_id));

    Ok(())
}

pub(crate) fn sync_subrun_state_metadata(aggregate: &mut RuntimeAggregate, now: u64) {
    let active_subrun_ids = aggregate
        .detail
        .subruns
        .iter()
        .map(|subrun| subrun.run_id.clone())
        .collect::<BTreeSet<_>>();
    aggregate
        .metadata
        .subrun_states
        .retain(|run_id, _| active_subrun_ids.contains(run_id));

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
