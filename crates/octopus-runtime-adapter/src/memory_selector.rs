use super::*;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMemorySelection {
    pub(crate) summary: RuntimeMemorySummary,
    pub(crate) selection_summary: RuntimeMemorySelectionSummary,
    pub(crate) freshness_summary: RuntimeMemoryFreshnessSummary,
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

impl RuntimeAdapter {
    pub(crate) fn select_runtime_memory(
        &self,
        session_policy: &session_policy::CompiledSessionPolicy,
        project_id: &str,
        run_id: &str,
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
                && record.proposal_state != "rejected"
                && record.proposal_state != "ignored"
        });
        let total_candidate_count = candidates.len() as u64;

        let mut selected_memory = if recall_mode == "skip" {
            Vec::new()
        } else {
            candidates
                .into_iter()
                .filter(|record| !ignored_memory_ids.contains(record.memory_id.as_str()))
                .filter(|record| !policy.freshness_required || is_fresh_memory(&record.freshness_state))
                .take(policy.max_selections as usize)
                .map(|record| RuntimeSelectedMemoryItem {
                    memory_id: record.memory_id,
                    title: record.title,
                    summary: record.summary,
                    kind: record.kind,
                    scope: record.scope,
                    owner_ref: record.owner_ref,
                    source_run_id: record.source_run_id,
                    freshness_state: record.freshness_state,
                    last_validated_at: record.last_validated_at,
                })
                .collect::<Vec<_>>()
        };

        selected_memory.sort_by(|left, right| left.memory_id.cmp(&right.memory_id));
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
            selected_memory,
            memory_state_ref: memory_runtime::runtime_memory_state_ref(run_id, now),
        })
    }
}
