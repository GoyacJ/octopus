use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SubrunSchedulerTick {
    pub(crate) runnable_run_ids: Vec<String>,
    pub(crate) promoted_run_ids: Vec<String>,
    pub(crate) occupied_slots: usize,
}

fn projected_subrun_status(run_status: &str) -> &'static str {
    match run_status {
        "failed" => "failed",
        _ => "queued",
    }
}

fn subrun_sort_key(state: &team_runtime::PersistedSubrunState) -> (u64, u64, String) {
    (
        state.run.started_at,
        state.run.updated_at,
        state.run.id.clone(),
    )
}

pub(crate) fn schedule_subrun_tick(
    subrun_states: &mut std::collections::BTreeMap<String, team_runtime::PersistedSubrunState>,
    concurrency_limit: usize,
    now: u64,
) -> SubrunSchedulerTick {
    if concurrency_limit == 0 {
        return SubrunSchedulerTick::default();
    }

    let mut running = subrun_states
        .values()
        .filter(|state| state.run.status == "running")
        .map(|state| state.run.id.clone())
        .collect::<Vec<_>>();
    running.sort_by(|left, right| {
        subrun_sort_key(&subrun_states[left]).cmp(&subrun_sort_key(&subrun_states[right]))
    });

    let occupied_slots = subrun_states
        .values()
        .filter(|state| worker_runtime::subrun_occupies_worker_slot(&state.run.status))
        .count();
    let available_slots = concurrency_limit.saturating_sub(occupied_slots);

    let mut queued = subrun_states
        .values()
        .filter(|state| state.run.status == "queued")
        .map(|state| state.run.id.clone())
        .collect::<Vec<_>>();
    queued.sort_by(|left, right| {
        subrun_sort_key(&subrun_states[left]).cmp(&subrun_sort_key(&subrun_states[right]))
    });

    let mut promoted_run_ids = Vec::new();
    for run_id in queued.into_iter().take(available_slots) {
        if let Some(state) = subrun_states.get_mut(&run_id) {
            state.run.status = "running".into();
            state.run.current_step = "running".into();
            state.run.next_action = Some("resume_subrun".into());
            state.run.approval_state = "not-required".into();
            state.run.updated_at = now;
            promoted_run_ids.push(run_id);
        }
    }

    let mut runnable_run_ids = running;
    runnable_run_ids.extend(promoted_run_ids.iter().cloned());

    SubrunSchedulerTick {
        occupied_slots: occupied_slots + promoted_run_ids.len(),
        runnable_run_ids,
        promoted_run_ids,
    }
}

pub(crate) fn planned_subruns(
    team: &actor_manifest::CompiledTeamManifest,
    run: &RuntimeRunSnapshot,
    workflow_run_id: &str,
    mailbox_ref: &str,
    now: u64,
) -> Vec<RuntimeSubrunSummary> {
    worker_runtime::worker_actor_refs(team)
        .into_iter()
        .enumerate()
        .map(|(index, actor_ref)| RuntimeSubrunSummary {
            run_id: format!("{}-subrun-{}", run.id, index + 1),
            parent_run_id: Some(run.id.clone()),
            actor_ref: actor_ref.clone(),
            label: worker_runtime::worker_label(&actor_ref),
            status: projected_subrun_status(&run.status).into(),
            run_kind: "subrun".into(),
            delegated_by_tool_call_id: Some(format!("team-dispatch-{}", index + 1)),
            workflow_run_id: Some(workflow_run_id.to_string()),
            mailbox_ref: Some(mailbox_ref.to_string()),
            handoff_ref: Some(format!("handoff-{}-{}", run.id, index + 1)),
            started_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
#[allow(clippy::large_stack_arrays)]
mod tests {
    use super::*;

    fn test_run(
        run_id: &str,
        status: &str,
        started_at: u64,
        updated_at: u64,
    ) -> RuntimeRunSnapshot {
        RuntimeRunSnapshot {
            id: run_id.into(),
            session_id: "session-test".into(),
            conversation_id: "conversation-test".into(),
            status: status.into(),
            current_step: status.into(),
            started_at,
            updated_at,
            selected_memory: Vec::new(),
            freshness_summary: Some(RuntimeMemoryFreshnessSummary::default()),
            pending_memory_proposal: None,
            memory_state_ref: "memory-state-test".into(),
            configured_model_id: None,
            configured_model_name: None,
            model_id: None,
            consumed_tokens: None,
            next_action: Some("resume_subrun".into()),
            config_snapshot_id: "config-snapshot-test".into(),
            effective_config_hash: "config-hash-test".into(),
            started_from_scope_set: Vec::new(),
            run_kind: "subrun".into(),
            parent_run_id: Some("parent-run".into()),
            actor_ref: format!("agent:{run_id}"),
            delegated_by_tool_call_id: Some(format!("tool-{run_id}")),
            workflow_run: Some("workflow-test".into()),
            workflow_run_detail: None,
            mailbox_ref: Some("mailbox-test".into()),
            handoff_ref: Some(format!("handoff-{run_id}")),
            background_state: None,
            worker_dispatch: None,
            approval_state: "not-required".into(),
            approval_target: None,
            auth_target: None,
            usage_summary: RuntimeUsageSummary::default(),
            artifact_refs: Vec::new(),
            deliverable_refs: Vec::new(),
            trace_context: trace_context::runtime_trace_context(
                "session-test",
                Some(run_id.into()),
            ),
            checkpoint: RuntimeRunCheckpoint {
                approval_layer: None,
                broker_decision: None,
                capability_id: None,
                checkpoint_artifact_ref: None,
                current_iteration_index: 0,
                tool_name: None,
                dispatch_kind: Some("team-subrun".into()),
                concurrency_policy: Some("parallel".into()),
                input: None,
                usage_summary: RuntimeUsageSummary::default(),
                pending_approval: None,
                pending_auth_challenge: None,
                pending_mediation: None,
                provider_key: None,
                reason: None,
                required_permission: None,
                requires_approval: None,
                requires_auth: None,
                target_kind: None,
                target_ref: None,
                capability_state_ref: Some(format!("capability-state-{run_id}")),
                capability_plan_summary: RuntimeCapabilityPlanSummary::default(),
                last_execution_outcome: None,
                last_mediation_outcome: None,
            },
            capability_plan_summary: RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            pending_mediation: None,
            capability_state_ref: Some(format!("capability-state-{run_id}")),
            last_execution_outcome: None,
            last_mediation_outcome: None,
            resolved_target: None,
            requested_actor_kind: Some("agent".into()),
            requested_actor_id: Some(format!("agent:{run_id}")),
            resolved_actor_kind: Some("agent".into()),
            resolved_actor_id: Some(format!("agent:{run_id}")),
            resolved_actor_label: Some(run_id.into()),
        }
    }

    fn test_subrun_state(
        run_id: &str,
        status: &str,
        started_at: u64,
        updated_at: u64,
    ) -> team_runtime::PersistedSubrunState {
        team_runtime::PersistedSubrunState {
            manifest_snapshot_ref: format!("manifest-{run_id}"),
            session_policy_snapshot_ref: format!("policy-{run_id}"),
            dispatch: team_runtime::PersistedSubrunDispatch::default(),
            serialized_session: json!({}),
            compaction_metadata: json!({}),
            run: test_run(run_id, status, started_at, updated_at),
        }
    }

    #[test]
    fn scheduler_tick_promotes_oldest_queued_subrun_within_available_slots() {
        let mut subrun_states = std::collections::BTreeMap::from([
            (
                "run-a".into(),
                test_subrun_state("run-a", "running", 10, 10),
            ),
            ("run-b".into(), test_subrun_state("run-b", "queued", 20, 20)),
            ("run-c".into(), test_subrun_state("run-c", "queued", 30, 30)),
        ]);

        let tick = schedule_subrun_tick(&mut subrun_states, 2, 40);

        assert_eq!(
            tick.runnable_run_ids,
            vec!["run-a".to_string(), "run-b".to_string()]
        );
        assert_eq!(tick.promoted_run_ids, vec!["run-b".to_string()]);
        assert_eq!(tick.occupied_slots, 2);
        assert_eq!(subrun_states["run-b"].run.status, "running");
        assert_eq!(subrun_states["run-c"].run.status, "queued");
    }

    #[test]
    fn scheduler_tick_treats_waiting_approval_and_auth_required_as_occupied_slots() {
        let mut subrun_states = std::collections::BTreeMap::from([
            (
                "run-approval".into(),
                test_subrun_state("run-approval", "waiting_approval", 10, 10),
            ),
            (
                "run-auth".into(),
                test_subrun_state("run-auth", "auth-required", 20, 20),
            ),
            (
                "run-queued".into(),
                test_subrun_state("run-queued", "queued", 30, 30),
            ),
        ]);

        let tick = schedule_subrun_tick(&mut subrun_states, 2, 40);

        assert!(tick.runnable_run_ids.is_empty());
        assert!(tick.promoted_run_ids.is_empty());
        assert_eq!(tick.occupied_slots, 2);
        assert_eq!(subrun_states["run-queued"].run.status, "queued");
    }
}
