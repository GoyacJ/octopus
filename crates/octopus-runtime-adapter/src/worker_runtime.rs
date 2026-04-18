use super::*;

pub(crate) fn worker_actor_refs(team: &actor_manifest::CompiledTeamManifest) -> Vec<String> {
    let refs = if team.record.member_refs.is_empty() {
        (!team.record.leader_ref.trim().is_empty())
            .then(|| vec![team.record.leader_ref.clone()])
            .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled_team_manifest() -> actor_manifest::CompiledTeamManifest {
        let mut delegation_policy = octopus_core::default_team_delegation_policy();
        delegation_policy.mode = "leader-orchestrated".into();
        delegation_policy.max_worker_count = 4;

        actor_manifest::CompiledTeamManifest {
            actor_ref: "team:test-team".into(),
            record: octopus_core::TeamRecord {
                id: "test-team".into(),
                workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                scope: "project".into(),
                name: "Test Team".into(),
                avatar_path: None,
                avatar: None,
                personality: "Coordinated".into(),
                tags: Vec::new(),
                prompt: "Coordinate.".into(),
                builtin_tool_keys: Vec::new(),
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: Vec::new(),
                manifest_revision: "test".into(),
                default_model_strategy: octopus_core::default_model_strategy(),
                capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
                permission_envelope: octopus_core::default_permission_envelope(),
                memory_policy: octopus_core::default_team_memory_policy(),
                delegation_policy,
                approval_preference: octopus_core::default_approval_preference(),
                output_contract: octopus_core::default_output_contract(),
                shared_capability_policy: octopus_core::default_team_shared_capability_policy(),
                leader_ref: "agent:team-leader".into(),
                member_refs: Vec::new(),
                team_topology: octopus_core::team_topology_from_refs(
                    Some("agent:team-leader".into()),
                    Vec::new(),
                ),
                shared_memory_policy: octopus_core::default_shared_memory_policy(),
                mailbox_policy: octopus_core::default_mailbox_policy(),
                artifact_handoff_policy: octopus_core::default_artifact_handoff_policy(),
                workflow_affordance: octopus_core::workflow_affordance_from_task_domains(
                    &Vec::new(),
                    true,
                    false,
                ),
                worker_concurrency_limit: 1,
                integration_source: None,
                trust_metadata: octopus_core::default_asset_trust_metadata(),
                dependency_resolution: Vec::new(),
                import_metadata: octopus_core::default_asset_import_metadata(),
                description: "Runtime test team".into(),
                status: "active".into(),
                updated_at: 1,
            },
        }
    }

    #[test]
    fn worker_actor_refs_prefer_member_refs_over_legacy_payloads() {
        let team = compiled_team_manifest();

        assert_eq!(worker_actor_refs(&team), vec!["agent:team-leader"]);
    }
}
