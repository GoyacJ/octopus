use super::*;
use std::cmp::Reverse;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeMemoryLineageContext {
    pub(crate) current_run_id: String,
    pub(crate) related_run_ids: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMemorySelection {
    pub(crate) summary: RuntimeMemorySummary,
    pub(crate) selection_summary: RuntimeMemorySelectionSummary,
    pub(crate) freshness_summary: RuntimeMemoryFreshnessSummary,
    pub(crate) candidate_memory: Vec<RuntimeSelectedMemoryItem>,
    pub(crate) selected_memory: Vec<RuntimeSelectedMemoryItem>,
    pub(crate) memory_state_ref: String,
}

fn normalize_recall_mode(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some("skip") => "skip",
        Some("default") | None | Some("") => "default",
        _ => "default",
    }
}

fn is_fresh_memory(state: &str) -> bool {
    matches!(state, "fresh" | "revalidated")
}

fn canonical_memory_scope(scope: &str) -> &str {
    match scope.trim() {
        "user" | "user-private" => "user",
        "agent-private" => "agent-private",
        "project" | "project-shared" => "project",
        "workspace" | "workspace-shared" => "workspace",
        "team" | "team-shared" => "team",
        other => other,
    }
}

fn policy_allows_record_scope(policy_scopes: &[String], record_scope: &str) -> bool {
    let record_scope = canonical_memory_scope(record_scope);
    policy_scopes
        .iter()
        .any(|scope| canonical_memory_scope(scope) == record_scope)
}

fn supported_memory_kind(kind: &str) -> bool {
    matches!(kind.trim(), "user" | "feedback" | "project" | "reference")
}

fn tokenize_text(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

fn actor_can_access_record(
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    project_id: &str,
    record: &memory_runtime::PersistedRuntimeMemoryRecord,
) -> bool {
    let owner_ref = record.owner_ref.as_deref();
    let user_owner_ref = (!session_policy.user_id.trim().is_empty())
        .then(|| format!("user:{}", session_policy.user_id));
    let project_owner_ref = (!project_id.trim().is_empty()).then(|| format!("project:{project_id}"));
    match record.scope.trim() {
        "user" | "user-private" => owner_ref.is_none()
            || owner_ref == Some("user:runtime")
            || user_owner_ref.as_deref() == owner_ref,
        "agent-private" => matches!(actor_manifest, actor_manifest::CompiledActorManifest::Agent(manifest)
            if owner_ref.is_none() || owner_ref == Some(manifest.actor_ref.as_str())),
        "team" | "team-shared" => matches!(actor_manifest, actor_manifest::CompiledActorManifest::Team(manifest)
            if owner_ref.is_none() || owner_ref == Some(manifest.actor_ref.as_str())),
        "project" | "project-shared" => owner_ref.is_none()
            || project_owner_ref.as_deref() == owner_ref
            || record.project_id.as_deref() == Some(project_id),
        "workspace" | "workspace-shared" => record.project_id.is_none(),
        _ => false,
    }
}

fn scope_score(actor_manifest: &actor_manifest::CompiledActorManifest, scope: &str) -> i64 {
    match actor_manifest {
        actor_manifest::CompiledActorManifest::Agent(_) => match scope {
            "agent-private" => 240,
            "user-private" | "user" => 220,
            "project-shared" | "project" => 180,
            "workspace-shared" | "workspace" => 120,
            _ => 0,
        },
        actor_manifest::CompiledActorManifest::Team(_) => match scope {
            "team-shared" | "team" => 240,
            "project-shared" | "project" => 200,
            "workspace-shared" | "workspace" => 120,
            _ => 0,
        },
    }
}

fn owner_score(
    actor_manifest: &actor_manifest::CompiledActorManifest,
    session_policy: &session_policy::CompiledSessionPolicy,
    project_id: &str,
    owner_ref: Option<&str>,
) -> i64 {
    let Some(owner_ref) = owner_ref else {
        return 0;
    };
    if !session_policy.user_id.trim().is_empty()
        && owner_ref == format!("user:{}", session_policy.user_id)
    {
        return 80;
    }
    if !project_id.trim().is_empty() && owner_ref == format!("project:{project_id}") {
        return 60;
    }
    match actor_manifest {
        actor_manifest::CompiledActorManifest::Agent(manifest)
            if owner_ref == manifest.actor_ref =>
        {
            100
        }
        actor_manifest::CompiledActorManifest::Team(manifest)
            if owner_ref == manifest.actor_ref =>
        {
            100
        }
        _ => 0,
    }
}

fn freshness_score(state: &str) -> i64 {
    match state {
        "fresh" => 300,
        "revalidated" => 280,
        "unknown" => 40,
        "stale" => 0,
        _ => 0,
    }
}

fn project_like_scope(scope: &str) -> bool {
    matches!(canonical_memory_scope(scope), "project" | "team" | "workspace")
}

fn lineage_score(
    lineage: &RuntimeMemoryLineageContext,
    source_run_id: Option<&str>,
    scope: &str,
) -> i64 {
    let Some(source_run_id) = source_run_id.filter(|value| !value.trim().is_empty()) else {
        return 0;
    };
    if source_run_id == lineage.current_run_id {
        return 260;
    }
    if lineage.related_run_ids.contains(source_run_id) {
        return 180;
    }
    if !lineage.related_run_ids.is_empty() && project_like_scope(scope) {
        return -60;
    }
    0
}

impl RuntimeMemoryLineageContext {
    pub(crate) fn from_run_state(
        run: &RuntimeRunSnapshot,
        subruns: &[RuntimeSubrunSummary],
    ) -> Self {
        let mut related_run_ids = BTreeSet::new();

        if !run.id.trim().is_empty() {
            related_run_ids.insert(run.id.clone());
        }
        if let Some(parent_run_id) = run.parent_run_id.as_ref().filter(|value| !value.trim().is_empty())
        {
            related_run_ids.insert(parent_run_id.clone());
        }

        if let Some(workflow_detail) = run.workflow_run_detail.as_ref() {
            for step in &workflow_detail.steps {
                if let Some(run_id) = step.run_id.as_ref().filter(|value| !value.trim().is_empty()) {
                    related_run_ids.insert(run_id.clone());
                }
                if let Some(parent_run_id) = step
                    .parent_run_id
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    related_run_ids.insert(parent_run_id.clone());
                }
            }
        }

        for subrun in subruns {
            if !subrun.run_id.trim().is_empty() {
                related_run_ids.insert(subrun.run_id.clone());
            }
            if let Some(parent_run_id) = subrun
                .parent_run_id
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                related_run_ids.insert(parent_run_id.clone());
            }
        }

        Self {
            current_run_id: run.id.clone(),
            related_run_ids,
        }
    }
}

impl RuntimeAdapter {
    pub(crate) fn select_runtime_memory(
        &self,
        actor_manifest: &actor_manifest::CompiledActorManifest,
        session_policy: &session_policy::CompiledSessionPolicy,
        project_id: &str,
        run_id: &str,
        lineage: &RuntimeMemoryLineageContext,
        now: u64,
        input: &SubmitRuntimeTurnInput,
    ) -> Result<RuntimeMemorySelection, AppError> {
        let policy = memory_runtime::parse_memory_policy(&session_policy.memory_policy);
        let recall_mode = normalize_recall_mode(input.recall_mode.as_deref());
        let ignored_memory_ids = input
            .ignored_memory_ids
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .collect::<std::collections::BTreeSet<_>>();

        let mut candidates = self.load_runtime_memory_records(project_id)?;
        candidates.retain(|record| {
            policy_allows_record_scope(&policy.durable_scopes, &record.scope)
                && supported_memory_kind(&record.kind)
                && actor_can_access_record(actor_manifest, session_policy, project_id, record)
                && record.proposal_state != "rejected"
                && record.proposal_state != "ignored"
        });
        let total_candidate_count = candidates.len() as u64;
        let query_tokens = tokenize_text(&input.content);
        let intent = input.memory_intent.as_deref().map(str::trim).unwrap_or_default();

        let mut ranked_candidates = candidates
            .iter()
            .filter(|record| !ignored_memory_ids.contains(record.memory_id.as_str()))
            .map(|record| {
                let item = RuntimeSelectedMemoryItem {
                    memory_id: record.memory_id.clone(),
                    title: record.title.clone(),
                    summary: record.summary.clone(),
                    kind: record.kind.clone(),
                    scope: record.scope.clone(),
                    owner_ref: record.owner_ref.clone(),
                    source_run_id: record.source_run_id.clone(),
                    freshness_state: record.freshness_state.clone(),
                    last_validated_at: record.last_validated_at,
                };
                let overlap = query_tokens
                    .intersection(&tokenize_text(&format!("{} {}", record.title, record.summary)))
                    .count() as i64;
                let score = freshness_score(&record.freshness_state)
                    + scope_score(actor_manifest, &record.scope)
                    + owner_score(
                        actor_manifest,
                        session_policy,
                        project_id,
                        record.owner_ref.as_deref(),
                    )
                    + lineage_score(lineage, record.source_run_id.as_deref(), &record.scope)
                    + if !intent.is_empty() && record.kind == intent { 90 } else { 0 }
                    + overlap * 15;
                (item, score, record.updated_at, record.last_validated_at.unwrap_or(0))
            })
            .collect::<Vec<_>>();
        ranked_candidates.sort_by_key(|(item, score, updated_at, last_validated_at)| {
            (
                Reverse(*score),
                Reverse(*last_validated_at),
                Reverse(*updated_at),
                item.memory_id.clone(),
            )
        });
        let candidate_memory = ranked_candidates
            .iter()
            .map(|(item, _, _, _)| item.clone())
            .collect::<Vec<_>>();

        let selected_memory = if recall_mode == "skip" {
            Vec::new()
        } else {
            ranked_candidates
                .iter()
                .filter(|(record, _, _, _)| {
                    !policy.freshness_required || is_fresh_memory(&record.freshness_state)
                })
                .map(|(record, _, _, _)| record.clone())
                .take(policy.max_selections as usize)
                .collect::<Vec<_>>()
        };

        let (summary, selection_summary, freshness_summary) = memory_runtime::build_memory_summary(
            &selected_memory,
            total_candidate_count,
            ignored_memory_ids.len() as u64,
            recall_mode,
            policy.freshness_required,
        );

        Ok(RuntimeMemorySelection {
            summary,
            selection_summary,
            freshness_summary,
            candidate_memory,
            selected_memory,
            memory_state_ref: memory_runtime::runtime_memory_state_ref(run_id, now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workflow_step(run_id: &str, parent_run_id: &str) -> RuntimeWorkflowStepSummary {
        RuntimeWorkflowStepSummary {
            step_id: run_id.into(),
            node_kind: "worker".into(),
            label: "Worker".into(),
            actor_ref: "agent:worker".into(),
            run_id: Some(run_id.into()),
            parent_run_id: Some(parent_run_id.into()),
            delegated_by_tool_call_id: Some("tool-lineage".into()),
            mailbox_ref: Some("mailbox-lineage".into()),
            handoff_ref: Some("handoff-lineage".into()),
            status: "completed".into(),
            started_at: 1,
            updated_at: 2,
        }
    }

    fn run_snapshot() -> RuntimeRunSnapshot {
        RuntimeRunSnapshot {
            id: "run-current".into(),
            session_id: "session-1".into(),
            conversation_id: "conv-1".into(),
            status: "running".into(),
            current_step: "executing".into(),
            started_at: 1,
            updated_at: 2,
            selected_memory: Vec::new(),
            freshness_summary: None,
            pending_memory_proposal: None,
            memory_state_ref: "memory-state".into(),
            configured_model_id: None,
            configured_model_name: None,
            model_id: None,
            consumed_tokens: None,
            next_action: None,
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
            started_from_scope_set: Vec::new(),
            run_kind: "primary".into(),
            parent_run_id: Some("run-parent".into()),
            actor_ref: "agent:agent-project-delivery".into(),
            delegated_by_tool_call_id: Some("tool-root".into()),
            workflow_run: Some("workflow-1".into()),
            workflow_run_detail: Some(RuntimeWorkflowRunDetail {
                workflow_run_id: "workflow-1".into(),
                status: "running".into(),
                current_step_id: Some("run-subrun".into()),
                current_step_label: Some("Worker".into()),
                total_steps: 2,
                completed_steps: 1,
                background_capable: false,
                steps: vec![workflow_step("run-subrun", "run-current")],
                blocking: None,
            }),
            mailbox_ref: Some("mailbox-root".into()),
            handoff_ref: Some("handoff-root".into()),
            background_state: None,
            worker_dispatch: None,
            approval_state: "not-required".into(),
            approval_target: None,
            auth_target: None,
            usage_summary: RuntimeUsageSummary::default(),
            artifact_refs: Vec::new(),
            trace_context: RuntimeTraceContext::default(),
            checkpoint: RuntimeRunCheckpoint::default(),
            capability_plan_summary: RuntimeCapabilityPlanSummary::default(),
            provider_state_summary: Vec::new(),
            pending_mediation: None,
            capability_state_ref: None,
            last_execution_outcome: None,
            last_mediation_outcome: None,
            resolved_target: None,
            requested_actor_kind: None,
            requested_actor_id: None,
            resolved_actor_kind: None,
            resolved_actor_id: None,
            resolved_actor_label: None,
        }
    }

    #[test]
    fn runtime_memory_lineage_context_collects_parent_workflow_and_subrun_ids() {
        let lineage = RuntimeMemoryLineageContext::from_run_state(
            &run_snapshot(),
            &[RuntimeSubrunSummary {
                run_id: "run-subrun".into(),
                parent_run_id: Some("run-current".into()),
                actor_ref: "agent:worker".into(),
                label: "Worker".into(),
                status: "completed".into(),
                run_kind: "subrun".into(),
                delegated_by_tool_call_id: Some("tool-lineage".into()),
                workflow_run_id: Some("workflow-1".into()),
                mailbox_ref: Some("mailbox-lineage".into()),
                handoff_ref: Some("handoff-lineage".into()),
                started_at: 1,
                updated_at: 2,
            }],
        );

        assert!(lineage.related_run_ids.contains("run-current"));
        assert!(lineage.related_run_ids.contains("run-parent"));
        assert!(lineage.related_run_ids.contains("run-subrun"));
    }

    #[test]
    fn lineage_score_prefers_related_project_memory_and_penalizes_unrelated_branch_memory() {
        let lineage = RuntimeMemoryLineageContext::from_run_state(&run_snapshot(), &[]);

        assert!(
            lineage_score(&lineage, Some("run-subrun"), "project-shared")
                > lineage_score(&lineage, Some("run-unrelated"), "project-shared")
        );
        assert!(
            lineage_score(&lineage, Some("run-unrelated"), "project-shared") < 0
        );
    }
}
