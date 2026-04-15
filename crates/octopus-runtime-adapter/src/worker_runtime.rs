use super::*;

pub(crate) fn worker_actor_refs(team: &actor_manifest::CompiledTeamManifest) -> Vec<String> {
    let refs = if team.record.member_refs.is_empty() {
        if team.record.member_agent_ids.is_empty() {
            vec![team.record.leader_ref.clone()]
        } else {
            team.record
                .member_agent_ids
                .iter()
                .map(|agent_id| format!("agent:{agent_id}"))
                .collect()
        }
    } else {
        team.record.member_refs.clone()
    };
    let delegation_policy = &team.record.delegation_policy;
    if delegation_policy.mode == "disabled" || delegation_policy.max_worker_count == 0 {
        return Vec::new();
    }
    let policy_limit = usize::try_from(delegation_policy.max_worker_count).unwrap_or(usize::MAX);
    refs.into_iter().take(policy_limit).collect()
}

pub(crate) fn worker_concurrency_limit(team: &actor_manifest::CompiledTeamManifest) -> usize {
    let delegation_policy = &team.record.delegation_policy;
    if delegation_policy.mode == "disabled" || delegation_policy.max_worker_count == 0 {
        return 0;
    }

    let policy_limit = usize::try_from(delegation_policy.max_worker_count).unwrap_or(usize::MAX);
    let manifest_limit = if team.record.worker_concurrency_limit == 0 {
        policy_limit
    } else {
        usize::try_from(team.record.worker_concurrency_limit).unwrap_or(usize::MAX)
    };
    manifest_limit.min(policy_limit)
}

pub(crate) fn subrun_occupies_worker_slot(status: &str) -> bool {
    matches!(status, "running" | "waiting_approval" | "auth-required")
}

pub(crate) fn worker_label(actor_ref: &str) -> String {
    actor_ref
        .split(':')
        .nth(1)
        .map(|value| value.replace('-', " "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| actor_ref.to_string())
}
