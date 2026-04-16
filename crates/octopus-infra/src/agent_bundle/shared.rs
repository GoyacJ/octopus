use octopus_core::AppError;
#[cfg(test)]
use octopus_core::{
    capability_policy_from_sources, default_agent_asset_role, default_agent_delegation_policy,
    default_agent_memory_policy, default_agent_shared_capability_policy,
    default_approval_preference, default_asset_import_metadata, default_asset_trust_metadata,
    default_model_strategy, default_output_contract, default_permission_envelope,
    normalize_task_domains, timestamp_now, AgentRecord, ASSET_MANIFEST_REVISION_V2,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::catalog_hash_id;

pub(crate) fn ensure_import_source_tables(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS agent_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS skill_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                skill_slug TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS team_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                team_id TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn managed_skill_id(path_or_slug: &str) -> String {
    catalog_hash_id("skill", path_or_slug)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_imported_agent_record(
    workspace_id: &str,
    agent_id: &str,
    name: &str,
    department: &str,
    description: &str,
    personality: &str,
    prompt: &str,
    builtin_tool_keys: &[String],
    skill_ids: &[String],
) -> AgentRecord {
    let builtin_tool_keys = builtin_tool_keys.to_vec();
    let skill_ids = skill_ids.to_vec();
    let task_domains = normalize_task_domains(vec![department.to_string()]);
    AgentRecord {
        id: agent_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: None,
        scope: "workspace".into(),
        owner_user_id: None,
        asset_role: default_agent_asset_role(),
        name: name.trim().to_string(),
        avatar_path: None,
        avatar: None,
        personality: personality.trim().to_string(),
        tags: vec![department.to_string()],
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: Vec::new(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(&builtin_tool_keys, &skill_ids, &[]),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_agent_memory_policy(),
        delegation_policy: default_agent_delegation_policy(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_agent_shared_capability_policy(),
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

pub(crate) fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn short_hash(value: &str) -> String {
    hash_text(value).chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use octopus_core::ASSET_MANIFEST_REVISION_V2;

    use super::build_imported_agent_record;

    #[test]
    fn build_imported_agent_record_applies_runtime_defaults() {
        let record = build_imported_agent_record(
            "ws-local",
            "agent-import-1",
            "Research Agent",
            "research",
            "Find things",
            "Precise",
            "You are a researcher",
            &["read".into()],
            &["skill-research".into()],
        );

        assert_eq!(record.manifest_revision, ASSET_MANIFEST_REVISION_V2);
        assert_eq!(record.task_domains, vec!["research"]);
        assert_eq!(record.capability_policy.builtin_tool_keys, vec!["read"]);
        assert_eq!(record.capability_policy.skill_ids, vec!["skill-research"]);
        assert_eq!(record.import_metadata.translation_status, "native");
    }
}
