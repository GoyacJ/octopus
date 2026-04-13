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
    let configured_limit = team.record.worker_concurrency_limit;
    let limit = if configured_limit == 0 {
        refs.len()
    } else {
        usize::try_from(configured_limit).unwrap_or(usize::MAX)
    };
    refs.into_iter().take(limit).collect()
}

pub(crate) fn worker_label(actor_ref: &str) -> String {
    actor_ref
        .split(':')
        .nth(1)
        .map(|value| value.replace('-', " "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| actor_ref.to_string())
}
