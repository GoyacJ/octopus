use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn blocking_target_kind(
    pending: Option<&RuntimePendingMediationSummary>,
    approval_target_kind: Option<&str>,
    auth_target_kind: Option<&str>,
) -> String {
    pending
        .map(|mediation| mediation.target_kind.clone())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            approval_target_kind
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            auth_target_kind
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "capability-call".into())
}

fn blocking_state_for_run(
    run_id: &str,
    actor_ref: &str,
    status: &str,
    approval: Option<&ApprovalRequestRecord>,
    auth: Option<&RuntimeAuthChallengeSummary>,
    pending: Option<&RuntimePendingMediationSummary>,
) -> Option<RuntimeWorkflowBlockingSummary> {
    if let Some(approval) = approval.filter(|approval| approval.status == "pending") {
        return Some(RuntimeWorkflowBlockingSummary {
            run_id: run_id.to_string(),
            actor_ref: actor_ref.to_string(),
            mediation_kind: pending
                .map(|mediation| mediation.mediation_kind.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "approval".into()),
            state: pending
                .map(|mediation| mediation.state.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| approval.status.clone()),
            target_kind: blocking_target_kind(
                pending,
                approval.target_kind.as_deref(),
                auth.map(|challenge| challenge.target_kind.as_str()),
            ),
        });
    }
    if let Some(auth) = auth.filter(|challenge| challenge.status == "pending") {
        return Some(RuntimeWorkflowBlockingSummary {
            run_id: run_id.to_string(),
            actor_ref: actor_ref.to_string(),
            mediation_kind: pending
                .map(|mediation| mediation.mediation_kind.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "auth".into()),
            state: pending
                .map(|mediation| mediation.state.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| auth.status.clone()),
            target_kind: blocking_target_kind(
                pending,
                approval.and_then(|record| {
                    record
                        .target_kind
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                }),
                Some(auth.target_kind.as_str()),
            ),
        });
    }
    pending
        .filter(|mediation| !mediation.state.trim().is_empty() || !status.trim().is_empty())
        .map(|mediation| RuntimeWorkflowBlockingSummary {
            run_id: run_id.to_string(),
            actor_ref: actor_ref.to_string(),
            mediation_kind: if mediation.mediation_kind.trim().is_empty() {
                status.to_string()
            } else {
                mediation.mediation_kind.clone()
            },
            state: if mediation.state.trim().is_empty() {
                status.to_string()
            } else {
                mediation.state.clone()
            },
            target_kind: if mediation.target_kind.trim().is_empty() {
                "capability-call".into()
            } else {
                mediation.target_kind.clone()
            },
        })
}

fn primary_workflow_blocking(run: &RuntimeRunSnapshot) -> Option<RuntimeWorkflowBlockingSummary> {
    if let Some(approval) = run
        .approval_target
        .as_ref()
        .filter(|approval| approval.run_id == run.id && approval.status == "pending")
    {
        return blocking_state_for_run(
            &run.id,
            &run.actor_ref,
            &run.status,
            Some(approval),
            run.auth_target
                .as_ref()
                .filter(|challenge| challenge.run_id == run.id),
            run.pending_mediation.as_ref(),
        );
    }
    if let Some(auth) = run
        .auth_target
        .as_ref()
        .filter(|challenge| challenge.run_id == run.id && challenge.status == "pending")
    {
        return blocking_state_for_run(
            &run.id,
            &run.actor_ref,
            &run.status,
            run.approval_target
                .as_ref()
                .filter(|approval| approval.run_id == run.id),
            Some(auth),
            run.pending_mediation.as_ref(),
        );
    }
    None
}

fn subrun_workflow_blocking(
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, team_runtime::PersistedSubrunState>,
) -> Option<RuntimeWorkflowBlockingSummary> {
    subruns.iter().find_map(|subrun| {
        let state = subrun_states.get(&subrun.run_id)?;
        if !matches!(
            state.run.status.as_str(),
            "waiting_approval" | "auth-required"
        ) {
            return None;
        }
        blocking_state_for_run(
            &state.run.id,
            &state.run.actor_ref,
            &state.run.status,
            state
                .run
                .approval_target
                .as_ref()
                .or(state.run.checkpoint.pending_approval.as_ref()),
            state
                .run
                .auth_target
                .as_ref()
                .or(state.run.checkpoint.pending_auth_challenge.as_ref()),
            state.run.pending_mediation.as_ref().or(state
                .run
                .checkpoint
                .pending_mediation
                .as_ref()),
        )
    })
}

fn leader_step_status(run: &RuntimeRunSnapshot, subruns: &[RuntimeSubrunSummary]) -> String {
    if primary_workflow_blocking(run).is_some() {
        return run.status.clone();
    }
    if !subruns.is_empty() {
        return "completed".into();
    }
    run.status.clone()
}

fn merge_workflow_step(
    live_step: RuntimeWorkflowStepSummary,
    persisted_steps: &BTreeMap<&str, &RuntimeWorkflowStepSummary>,
) -> RuntimeWorkflowStepSummary {
    let Some(persisted) = persisted_steps.get(live_step.step_id.as_str()) else {
        return live_step;
    };
    let mut merged = live_step;
    if !persisted.label.trim().is_empty() {
        merged.label = persisted.label.clone();
    }
    if !persisted.node_kind.trim().is_empty() {
        merged.node_kind = persisted.node_kind.clone();
    }
    if !persisted.actor_ref.trim().is_empty() {
        merged.actor_ref = persisted.actor_ref.clone();
    }
    merged
}

fn workflow_steps(
    run: &RuntimeRunSnapshot,
    subruns: &[RuntimeSubrunSummary],
    persisted_detail: Option<&RuntimeWorkflowRunDetail>,
) -> Vec<RuntimeWorkflowStepSummary> {
    let persisted_steps = persisted_detail
        .map(|detail| {
            detail
                .steps
                .iter()
                .map(|step| (step.step_id.as_str(), step))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut seen_step_ids = BTreeSet::new();
    let mut steps = vec![merge_workflow_step(
        RuntimeWorkflowStepSummary {
            step_id: run.id.clone(),
            node_kind: "leader".into(),
            label: "Leader plan".into(),
            actor_ref: run.actor_ref.clone(),
            run_id: Some(run.id.clone()),
            parent_run_id: run.parent_run_id.clone(),
            delegated_by_tool_call_id: run.delegated_by_tool_call_id.clone(),
            mailbox_ref: run.mailbox_ref.clone(),
            handoff_ref: run.handoff_ref.clone(),
            status: leader_step_status(run, subruns),
            started_at: run.started_at,
            updated_at: run.updated_at,
        },
        &persisted_steps,
    )];
    seen_step_ids.insert(run.id.as_str());
    steps.extend(subruns.iter().map(|subrun| {
        seen_step_ids.insert(subrun.run_id.as_str());
        merge_workflow_step(
            RuntimeWorkflowStepSummary {
                step_id: subrun.run_id.clone(),
                node_kind: "worker".into(),
                label: subrun.label.clone(),
                actor_ref: subrun.actor_ref.clone(),
                run_id: Some(subrun.run_id.clone()),
                parent_run_id: subrun.parent_run_id.clone(),
                delegated_by_tool_call_id: subrun.delegated_by_tool_call_id.clone(),
                mailbox_ref: subrun.mailbox_ref.clone(),
                handoff_ref: subrun.handoff_ref.clone(),
                status: subrun.status.clone(),
                started_at: subrun.started_at,
                updated_at: subrun.updated_at,
            },
            &persisted_steps,
        )
    }));

    if let Some(detail) = persisted_detail {
        steps.extend(
            detail
                .steps
                .iter()
                .filter(|step| !seen_step_ids.contains(step.step_id.as_str()))
                .cloned(),
        );
    }
    steps
}

fn workflow_status_from_steps(
    steps: &[RuntimeWorkflowStepSummary],
    fallback_status: &str,
) -> String {
    if steps.is_empty() {
        return fallback_status.to_string();
    }
    if steps
        .iter()
        .any(|step| matches!(step.status.as_str(), "failed" | "cancelled"))
    {
        return "failed".into();
    }
    if steps.iter().any(|step| step.status == "waiting_approval") {
        return "waiting_approval".into();
    }
    if steps.iter().any(|step| step.status == "auth-required") {
        return "auth-required".into();
    }
    if steps.iter().any(|step| step.status == "running") {
        return "running".into();
    }
    if steps.iter().any(|step| step.status == "queued") {
        return "queued".into();
    }
    if steps.iter().all(|step| step.status == "completed") {
        return "completed".into();
    }
    fallback_status.to_string()
}

fn workflow_current_step(steps: &[RuntimeWorkflowStepSummary]) -> (Option<String>, Option<String>) {
    steps
        .iter()
        .find(|step| step.status != "completed")
        .map(|step| (Some(step.step_id.clone()), Some(step.label.clone())))
        .or_else(|| {
            steps
                .last()
                .map(|step| (Some(step.step_id.clone()), Some(step.label.clone())))
        })
        .unwrap_or((None, None))
}

pub(crate) fn build_workflow_projection(
    workflow_label: &str,
    run: &RuntimeRunSnapshot,
    workflow_run_id: &str,
    background_run_id: &str,
    subruns: &[RuntimeSubrunSummary],
    subrun_states: &BTreeMap<String, team_runtime::PersistedSubrunState>,
    persisted_detail: Option<&RuntimeWorkflowRunDetail>,
    background_capable: bool,
    now: u64,
) -> (
    RuntimeWorkflowSummary,
    RuntimeWorkflowRunDetail,
    RuntimeBackgroundRunSummary,
) {
    let steps = workflow_steps(run, subruns, persisted_detail);
    let workflow_status = workflow_status_from_steps(&steps, &run.status);
    let completed_steps = steps
        .iter()
        .filter(|step| step.status == "completed")
        .count() as u64;
    let total_steps = steps.len() as u64;
    let (current_step_id, current_step_label) = workflow_current_step(&steps);
    let blocking =
        primary_workflow_blocking(run).or_else(|| subrun_workflow_blocking(subruns, subrun_states));

    let summary = RuntimeWorkflowSummary {
        workflow_run_id: workflow_run_id.to_string(),
        label: workflow_label.to_string(),
        status: workflow_status.clone(),
        total_steps,
        completed_steps,
        current_step_id: current_step_id.clone(),
        current_step_label: current_step_label.clone(),
        background_capable,
        updated_at: now,
    };
    let detail = RuntimeWorkflowRunDetail {
        workflow_run_id: workflow_run_id.to_string(),
        status: workflow_status.clone(),
        current_step_id,
        current_step_label,
        total_steps,
        completed_steps,
        background_capable,
        steps,
        blocking: blocking.clone(),
    };
    let background = background_runtime::build_background_summary(
        background_run_id,
        workflow_run_id,
        &workflow_status,
        background_capable,
        blocking,
        now,
    );
    (summary, detail, background)
}
