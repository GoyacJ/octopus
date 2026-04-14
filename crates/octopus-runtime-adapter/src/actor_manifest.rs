use super::*;

use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum CompiledActorManifest {
    Agent(CompiledAgentManifest),
    Team(CompiledTeamManifest),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CompiledAgentManifest {
    pub(crate) actor_ref: String,
    pub(crate) record: AgentRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CompiledTeamManifest {
    pub(crate) actor_ref: String,
    pub(crate) record: TeamRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorRefKind {
    Agent,
    Team,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActorRef {
    pub(crate) kind: ActorRefKind,
    pub(crate) id: String,
}

fn merge_json_with_defaults(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let merged = merge_json_with_defaults(
                    base_map.remove(&key).unwrap_or(serde_json::Value::Null),
                    patch_value,
                );
                base_map.insert(key, merged);
            }
            serde_json::Value::Object(base_map)
        }
        (base, serde_json::Value::Null) => base,
        (_, patch) => patch,
    }
}

fn parse_json_or_default<T, F>(raw: &str, default: F) -> T
where
    T: DeserializeOwned + serde::Serialize,
    F: FnOnce() -> T,
{
    let default_value = default();
    let merged = serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|patch| {
            serde_json::to_value(&default_value)
                .ok()
                .map(|base| merge_json_with_defaults(base, patch))
        })
        .unwrap_or(serde_json::Value::Null);
    serde_json::from_value(merged).unwrap_or(default_value)
}

fn parse_actor_ref(actor_ref: &str) -> Result<ActorRef, AppError> {
    let mut parts = actor_ref.splitn(2, ':');
    let kind = parts
        .next()
        .ok_or_else(|| AppError::invalid_input("actor ref kind is missing"))?;
    let id = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid_input("actor ref id is missing"))?;
    let kind = match kind {
        "agent" => ActorRefKind::Agent,
        "team" => ActorRefKind::Team,
        other => {
            return Err(AppError::invalid_input(format!(
                "unsupported actor ref kind `{other}`"
            )))
        }
    };
    Ok(ActorRef {
        kind,
        id: id.to_string(),
    })
}

fn normalize_agent_actor_ref(actor_ref: &str) -> String {
    let trimmed = actor_ref.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("agent:{trimmed}")
    }
}

impl CompiledActorManifest {
    fn capability_policy(&self) -> &octopus_core::CapabilityPolicy {
        match self {
            Self::Agent(manifest) => &manifest.record.capability_policy,
            Self::Team(manifest) => &manifest.record.capability_policy,
        }
    }

    pub(crate) fn actor_ref(&self) -> &str {
        match self {
            Self::Agent(manifest) => &manifest.actor_ref,
            Self::Team(manifest) => &manifest.actor_ref,
        }
    }

    pub(crate) fn manifest_revision(&self) -> &str {
        match self {
            Self::Agent(manifest) => &manifest.record.manifest_revision,
            Self::Team(manifest) => &manifest.record.manifest_revision,
        }
    }

    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Agent(manifest) => &manifest.record.name,
            Self::Team(manifest) => &manifest.record.name,
        }
    }

    pub(crate) fn builtin_tool_keys(&self) -> &[String] {
        &self.capability_policy().builtin_tool_keys
    }

    pub(crate) fn skill_ids(&self) -> &[String] {
        &self.capability_policy().skill_ids
    }

    pub(crate) fn mcp_server_names(&self) -> &[String] {
        &self.capability_policy().mcp_server_names
    }

    pub(crate) fn plugin_capability_refs(&self) -> &[String] {
        &self.capability_policy().plugin_capability_refs
    }

    pub(crate) fn memory_summary(&self) -> RuntimeMemorySummary {
        let policy = match self {
            Self::Agent(manifest) => &manifest.record.memory_policy,
            Self::Team(manifest) => &manifest.record.memory_policy,
        };
        RuntimeMemorySummary {
            summary: format!(
                "{} durable memory scope(s) available{}.",
                policy.durable_scopes.len(),
                if policy.freshness_required {
                    "; freshness required"
                } else {
                    ""
                }
            ),
            durable_memory_count: policy.durable_scopes.len() as u64,
            selected_memory_ids: Vec::new(),
        }
    }

    pub(crate) fn default_model_ref(&self) -> Option<&str> {
        match self {
            Self::Agent(manifest) => manifest
                .record
                .default_model_strategy
                .preferred_model_ref
                .as_deref(),
            Self::Team(manifest) => manifest
                .record
                .default_model_strategy
                .preferred_model_ref
                .as_deref(),
        }
    }

    pub(crate) fn permission_ceiling(&self) -> &str {
        match self {
            Self::Agent(manifest) => &manifest.record.permission_envelope.max_mode,
            Self::Team(manifest) => &manifest.record.permission_envelope.max_mode,
        }
    }

    pub(crate) fn system_prompt(&self) -> String {
        match self {
            Self::Agent(manifest) => build_agent_system_prompt(&manifest.record),
            Self::Team(manifest) => build_team_system_prompt(&manifest.record),
        }
    }

    pub(crate) fn actor_kind_label(&self) -> &'static str {
        match self {
            Self::Agent(_) => "agent",
            Self::Team(_) => "team",
        }
    }

    pub(crate) fn capability_policy_value(&self) -> serde_json::Value {
        match self {
            Self::Agent(manifest) => serde_json::to_value(&manifest.record.capability_policy),
            Self::Team(manifest) => serde_json::to_value(&manifest.record.capability_policy),
        }
        .unwrap_or_else(|_| json!({}))
    }

    pub(crate) fn memory_policy_value(&self) -> serde_json::Value {
        match self {
            Self::Agent(manifest) => serde_json::to_value(&manifest.record.memory_policy),
            Self::Team(manifest) => serde_json::to_value(&manifest.record.memory_policy),
        }
        .unwrap_or_else(|_| json!({}))
    }

    pub(crate) fn delegation_policy_value(&self) -> serde_json::Value {
        match self {
            Self::Agent(manifest) => serde_json::to_value(&manifest.record.delegation_policy),
            Self::Team(manifest) => serde_json::to_value(&manifest.record.delegation_policy),
        }
        .unwrap_or_else(|_| json!({}))
    }

    pub(crate) fn approval_preference_value(&self) -> serde_json::Value {
        match self {
            Self::Agent(manifest) => serde_json::to_value(&manifest.record.approval_preference),
            Self::Team(manifest) => serde_json::to_value(&manifest.record.approval_preference),
        }
        .unwrap_or_else(|_| json!({}))
    }
}

fn build_agent_system_prompt(record: &AgentRecord) -> String {
    actor_context::build_actor_system_prompt([
        Some(format!("You are the agent `{}`.", record.name)),
        Some(format!("Actor ref: agent:{}.", record.id)),
        Some(format!("Personality: {}.", record.personality)),
        Some(format!("Task domains: {}.", record.task_domains.join(", "))),
        Some(format!("Instructions: {}.", record.prompt)),
    ])
    .unwrap_or_default()
}

fn build_team_system_prompt(record: &TeamRecord) -> String {
    actor_context::build_actor_system_prompt([
        Some(format!("You are the team `{}`.", record.name)),
        Some(format!("Actor ref: team:{}.", record.id)),
        Some(format!("Personality: {}.", record.personality)),
        Some(format!("Task domains: {}.", record.task_domains.join(", "))),
        Some(format!("Leader ref: {}.", record.leader_ref)),
        Some(format!("Members: {}.", record.member_refs.join(", "))),
        Some(format!("Instructions: {}.", record.prompt)),
    ])
    .unwrap_or_default()
}

fn fallback_builtin_agent_record(agent_id: &str) -> Option<AgentRecord> {
    let now = timestamp_now();
    match agent_id {
        "agent-orchestrator" => Some(AgentRecord {
            id: "agent-orchestrator".into(),
            workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            scope: "workspace".into(),
            name: "Workspace Orchestrator".into(),
            avatar_path: None,
            avatar: None,
            personality: "System coordinator".into(),
            tags: vec!["workspace".into(), "orchestration".into()],
            prompt: "Coordinate work across the workspace and keep execution aligned.".into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: Vec::new(),
            mcp_server_names: Vec::new(),
            task_domains: octopus_core::normalize_task_domains(vec![
                "workspace".into(),
                "orchestration".into(),
            ]),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: octopus_core::default_model_strategy(),
            capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
            permission_envelope: octopus_core::default_permission_envelope(),
            memory_policy: octopus_core::default_agent_memory_policy(),
            delegation_policy: octopus_core::default_agent_delegation_policy(),
            approval_preference: octopus_core::default_approval_preference(),
            output_contract: octopus_core::default_output_contract(),
            shared_capability_policy: octopus_core::default_agent_shared_capability_policy(),
            integration_source: None,
            trust_metadata: octopus_core::default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: octopus_core::default_asset_import_metadata(),
            description: "Coordinates projects, approvals, and execution policies.".into(),
            status: "active".into(),
            updated_at: now,
        }),
        "agent-project-delivery" => Some(AgentRecord {
            id: "agent-project-delivery".into(),
            workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
            scope: "project".into(),
            name: "Project Delivery Agent".into(),
            avatar_path: None,
            avatar: None,
            personality: "Delivery lead".into(),
            tags: vec!["project".into(), "delivery".into()],
            prompt: "Track project work, runtime sessions, and follow-up actions.".into(),
            builtin_tool_keys: Vec::new(),
            skill_ids: Vec::new(),
            mcp_server_names: Vec::new(),
            task_domains: octopus_core::normalize_task_domains(vec![
                "project".into(),
                "delivery".into(),
            ]),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: octopus_core::default_model_strategy(),
            capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
            permission_envelope: octopus_core::default_permission_envelope(),
            memory_policy: octopus_core::default_agent_memory_policy(),
            delegation_policy: octopus_core::default_agent_delegation_policy(),
            approval_preference: octopus_core::default_approval_preference(),
            output_contract: octopus_core::default_output_contract(),
            shared_capability_policy: octopus_core::default_agent_shared_capability_policy(),
            integration_source: None,
            trust_metadata: octopus_core::default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: octopus_core::default_asset_import_metadata(),
            description: "Tracks project work, runtime sessions, and follow-up actions.".into(),
            status: "active".into(),
            updated_at: now,
        }),
        _ => None,
    }
}

fn fallback_builtin_team_record(team_id: &str) -> Option<TeamRecord> {
    if team_id != "team-workspace-core" {
        return None;
    }
    let now = timestamp_now();
    Some(TeamRecord {
        id: "team-workspace-core".into(),
        workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
        project_id: None,
        scope: "workspace".into(),
        name: "Workspace Core".into(),
        avatar_path: None,
        avatar: None,
        personality: "Governance team".into(),
        tags: vec!["workspace".into(), "governance".into()],
        prompt: "Maintain workspace-wide standards and governance.".into(),
        builtin_tool_keys: Vec::new(),
        skill_ids: Vec::new(),
        mcp_server_names: Vec::new(),
        task_domains: octopus_core::normalize_task_domains(vec![
            "workspace".into(),
            "governance".into(),
        ]),
        manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: octopus_core::default_model_strategy(),
        capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
        permission_envelope: octopus_core::default_permission_envelope(),
        memory_policy: octopus_core::default_team_memory_policy(),
        delegation_policy: octopus_core::default_team_delegation_policy(),
        approval_preference: octopus_core::default_approval_preference(),
        output_contract: octopus_core::default_output_contract(),
        shared_capability_policy: octopus_core::default_team_shared_capability_policy(),
        leader_agent_id: Some("agent-orchestrator".into()),
        member_agent_ids: vec!["agent-orchestrator".into()],
        leader_ref: "agent:agent-orchestrator".into(),
        member_refs: vec!["agent:agent-orchestrator".into()],
        team_topology: octopus_core::team_topology_from_refs(
            Some("agent:agent-orchestrator".into()),
            vec!["agent:agent-orchestrator".into()],
        ),
        shared_memory_policy: octopus_core::default_shared_memory_policy(),
        mailbox_policy: octopus_core::default_mailbox_policy(),
        artifact_handoff_policy: octopus_core::default_artifact_handoff_policy(),
        workflow_affordance: octopus_core::workflow_affordance_from_task_domains(
            &octopus_core::normalize_task_domains(vec!["workspace".into(), "governance".into()]),
            true,
            true,
        ),
        worker_concurrency_limit: octopus_core::default_team_delegation_policy().max_worker_count,
        integration_source: None,
        trust_metadata: octopus_core::default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: octopus_core::default_asset_import_metadata(),
        description: "Maintains workspace-wide operating standards and governance.".into(),
        status: "active".into(),
        updated_at: now,
    })
}

impl RuntimeAdapter {
    pub(crate) fn manifest_snapshot_path(&self, manifest_snapshot_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{manifest_snapshot_ref}.json"))
    }

    pub(crate) fn persist_actor_manifest_snapshot(
        &self,
        manifest_snapshot_ref: &str,
        manifest: &CompiledActorManifest,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_vec_pretty(manifest)?;
        let path = self.manifest_snapshot_path(manifest_snapshot_ref);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, payload)?;
        Ok(())
    }

    pub(crate) fn load_actor_manifest_snapshot(
        &self,
        manifest_snapshot_ref: &str,
    ) -> Result<CompiledActorManifest, AppError> {
        let path = self.manifest_snapshot_path(manifest_snapshot_ref);
        let raw = fs::read(path)?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub(crate) fn compile_actor_manifest(
        &self,
        selected_actor_ref: &str,
    ) -> Result<CompiledActorManifest, AppError> {
        let actor_ref = parse_actor_ref(selected_actor_ref)?;
        let connection = self.open_db()?;
        match actor_ref.kind {
            ActorRefKind::Agent => Ok(CompiledActorManifest::Agent(CompiledAgentManifest {
                actor_ref: selected_actor_ref.to_string(),
                record: self.load_agent_record(&connection, &actor_ref.id)?,
            })),
            ActorRefKind::Team => Ok(CompiledActorManifest::Team(CompiledTeamManifest {
                actor_ref: selected_actor_ref.to_string(),
                record: self.load_team_record(&connection, &actor_ref.id)?,
            })),
        }
    }

    fn load_agent_record(
        &self,
        connection: &Connection,
        agent_id: &str,
    ) -> Result<AgentRecord, AppError> {
        connection
            .query_row(
                "SELECT
                    id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                    builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                    default_model_strategy_json, capability_policy_json, permission_envelope_json,
                    memory_policy_json, delegation_policy_json, approval_preference_json,
                    output_contract_json, shared_capability_policy_json, description, status, updated_at
                 FROM agents
                 WHERE id = ?1",
                [agent_id],
                |row| {
                    let tags_raw: String = row.get(7)?;
                    let builtin_tool_keys_raw: String = row.get(9)?;
                    let skill_ids_raw: String = row.get(10)?;
                    let mcp_server_names_raw: String = row.get(11)?;
                    let task_domains_raw: String = row.get(12)?;
                    let default_model_strategy_raw: String = row.get(14)?;
                    let capability_policy_raw: String = row.get(15)?;
                    let permission_envelope_raw: String = row.get(16)?;
                    let memory_policy_raw: String = row.get(17)?;
                    let delegation_policy_raw: String = row.get(18)?;
                    let approval_preference_raw: String = row.get(19)?;
                    let output_contract_raw: String = row.get(20)?;
                    let shared_capability_policy_raw: String = row.get(21)?;
                    let builtin_tool_keys: Vec<String> =
                        serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
                    let skill_ids: Vec<String> =
                        serde_json::from_str(&skill_ids_raw).unwrap_or_default();
                    let mcp_server_names: Vec<String> =
                        serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
                    Ok(AgentRecord {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        project_id: row.get(2)?,
                        scope: row.get(3)?,
                        name: row.get(4)?,
                        avatar_path: row.get(5)?,
                        avatar: None,
                        personality: row.get(6)?,
                        tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                        prompt: row.get(8)?,
                        builtin_tool_keys: builtin_tool_keys.clone(),
                        skill_ids: skill_ids.clone(),
                        mcp_server_names: mcp_server_names.clone(),
                        task_domains: parse_json_or_default(&task_domains_raw, || {
                            octopus_core::normalize_task_domains(Vec::new())
                        }),
                        manifest_revision: row.get(13)?,
                        default_model_strategy: parse_json_or_default(
                            &default_model_strategy_raw,
                            octopus_core::default_model_strategy,
                        ),
                        capability_policy: parse_json_or_default(&capability_policy_raw, || {
                            octopus_core::capability_policy_from_sources(
                                &builtin_tool_keys,
                                &skill_ids,
                                &mcp_server_names,
                            )
                        }),
                        permission_envelope: parse_json_or_default(
                            &permission_envelope_raw,
                            octopus_core::default_permission_envelope,
                        ),
                        memory_policy: parse_json_or_default(
                            &memory_policy_raw,
                            octopus_core::default_agent_memory_policy,
                        ),
                        delegation_policy: parse_json_or_default(
                            &delegation_policy_raw,
                            octopus_core::default_agent_delegation_policy,
                        ),
                        approval_preference: parse_json_or_default(
                            &approval_preference_raw,
                            octopus_core::default_approval_preference,
                        ),
                        output_contract: parse_json_or_default(
                            &output_contract_raw,
                            octopus_core::default_output_contract,
                        ),
                        shared_capability_policy: parse_json_or_default(
                            &shared_capability_policy_raw,
                            octopus_core::default_agent_shared_capability_policy,
                        ),
                        integration_source: None,
                        trust_metadata: octopus_core::default_asset_trust_metadata(),
                        dependency_resolution: Vec::new(),
                        import_metadata: octopus_core::default_asset_import_metadata(),
                        description: row.get(22)?,
                        status: row.get(23)?,
                        updated_at: row.get::<_, i64>(24)? as u64,
                    })
                },
            )
            .map_err(|error| {
                if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::not_found("agent")
                } else {
                    AppError::database(error.to_string())
                }
            })
            .or_else(|error| {
                if matches!(error, AppError::NotFound(_)) {
                    fallback_builtin_agent_record(agent_id)
                        .ok_or_else(|| AppError::not_found("agent"))
                } else {
                    Err(error)
                }
            })
    }

    fn load_team_record(
        &self,
        connection: &Connection,
        team_id: &str,
    ) -> Result<TeamRecord, AppError> {
        connection
            .query_row(
                "SELECT
                    id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                    builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                    default_model_strategy_json, capability_policy_json, permission_envelope_json,
                    memory_policy_json, delegation_policy_json, approval_preference_json,
                    output_contract_json, shared_capability_policy_json, leader_agent_id,
                    member_agent_ids, leader_ref, member_refs, team_topology_json,
                    shared_memory_policy_json, mailbox_policy_json, artifact_handoff_policy_json,
                    workflow_affordance_json, worker_concurrency_limit, description, status, updated_at
                 FROM teams
                 WHERE id = ?1",
                [team_id],
                |row| {
                    let tags_raw: String = row.get(7)?;
                    let builtin_tool_keys_raw: String = row.get(9)?;
                    let skill_ids_raw: String = row.get(10)?;
                    let mcp_server_names_raw: String = row.get(11)?;
                    let task_domains_raw: String = row.get(12)?;
                    let default_model_strategy_raw: String = row.get(14)?;
                    let capability_policy_raw: String = row.get(15)?;
                    let permission_envelope_raw: String = row.get(16)?;
                    let memory_policy_raw: String = row.get(17)?;
                    let delegation_policy_raw: String = row.get(18)?;
                    let approval_preference_raw: String = row.get(19)?;
                    let output_contract_raw: String = row.get(20)?;
                    let shared_capability_policy_raw: String = row.get(21)?;
                    let leader_agent_id: Option<String> = row.get(22)?;
                    let member_agent_ids_raw: String = row.get(23)?;
                    let leader_ref_raw: String = row.get(24)?;
                    let member_refs_raw: String = row.get(25)?;
                    let team_topology_raw: String = row.get(26)?;
                    let shared_memory_policy_raw: String = row.get(27)?;
                    let mailbox_policy_raw: String = row.get(28)?;
                    let artifact_handoff_policy_raw: String = row.get(29)?;
                    let workflow_affordance_raw: String = row.get(30)?;
                    let builtin_tool_keys: Vec<String> =
                        serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
                    let skill_ids: Vec<String> =
                        serde_json::from_str(&skill_ids_raw).unwrap_or_default();
                    let mcp_server_names: Vec<String> =
                        serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
                    let member_agent_ids: Vec<String> =
                        serde_json::from_str(&member_agent_ids_raw).unwrap_or_default();
                    let mut leader_ref = normalize_agent_actor_ref(&leader_ref_raw);
                    if leader_ref.is_empty() {
                        leader_ref = leader_agent_id
                            .as_deref()
                            .map(normalize_agent_actor_ref)
                            .or_else(|| {
                                member_agent_ids
                                    .first()
                                    .map(|agent_id| normalize_agent_actor_ref(agent_id))
                            })
                            .unwrap_or_default();
                    }
                    let parsed_member_refs: Vec<String> =
                        serde_json::from_str(&member_refs_raw).unwrap_or_default();
                    let legacy_member_refs = parsed_member_refs.is_empty() && !member_agent_ids.is_empty();
                    let mut member_refs = if parsed_member_refs.is_empty() {
                        member_agent_ids
                            .iter()
                            .map(|agent_id| normalize_agent_actor_ref(agent_id))
                            .collect::<Vec<_>>()
                    } else {
                        parsed_member_refs
                            .into_iter()
                            .map(|actor_ref| normalize_agent_actor_ref(&actor_ref))
                            .filter(|actor_ref| !actor_ref.is_empty())
                            .collect::<Vec<_>>()
                    };
                    if member_refs.is_empty() && !leader_ref.is_empty() {
                        member_refs.push(leader_ref.clone());
                    }
                    let worker_concurrency_limit = row.get::<_, i64>(31)? as u64;
                    let worker_concurrency_limit = if legacy_member_refs
                        && worker_concurrency_limit <= 1
                        && member_refs.len() > 1
                    {
                        member_refs.len() as u64
                    } else if worker_concurrency_limit == 0 {
                        member_refs.len().max(1) as u64
                    } else {
                        worker_concurrency_limit
                    };
                    Ok(TeamRecord {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        project_id: row.get(2)?,
                        scope: row.get(3)?,
                        name: row.get(4)?,
                        avatar_path: row.get(5)?,
                        avatar: None,
                        personality: row.get(6)?,
                        tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                        prompt: row.get(8)?,
                        builtin_tool_keys: builtin_tool_keys.clone(),
                        skill_ids: skill_ids.clone(),
                        mcp_server_names: mcp_server_names.clone(),
                        task_domains: parse_json_or_default(&task_domains_raw, || {
                            octopus_core::normalize_task_domains(Vec::new())
                        }),
                        manifest_revision: row.get(13)?,
                        default_model_strategy: parse_json_or_default(
                            &default_model_strategy_raw,
                            octopus_core::default_model_strategy,
                        ),
                        capability_policy: parse_json_or_default(&capability_policy_raw, || {
                            octopus_core::capability_policy_from_sources(
                                &builtin_tool_keys,
                                &skill_ids,
                                &mcp_server_names,
                            )
                        }),
                        permission_envelope: parse_json_or_default(
                            &permission_envelope_raw,
                            octopus_core::default_permission_envelope,
                        ),
                        memory_policy: parse_json_or_default(
                            &memory_policy_raw,
                            octopus_core::default_team_memory_policy,
                        ),
                        delegation_policy: parse_json_or_default(
                            &delegation_policy_raw,
                            octopus_core::default_team_delegation_policy,
                        ),
                        approval_preference: parse_json_or_default(
                            &approval_preference_raw,
                            octopus_core::default_approval_preference,
                        ),
                        output_contract: parse_json_or_default(
                            &output_contract_raw,
                            octopus_core::default_output_contract,
                        ),
                        shared_capability_policy: parse_json_or_default(
                            &shared_capability_policy_raw,
                            octopus_core::default_team_shared_capability_policy,
                        ),
                        leader_agent_id,
                        member_agent_ids,
                        leader_ref: leader_ref.clone(),
                        member_refs: member_refs.clone(),
                        team_topology: parse_json_or_default(&team_topology_raw, || {
                            octopus_core::team_topology_from_refs(
                                Some(leader_ref.clone()),
                                member_refs.clone(),
                            )
                        }),
                        shared_memory_policy: parse_json_or_default(
                            &shared_memory_policy_raw,
                            octopus_core::default_shared_memory_policy,
                        ),
                        mailbox_policy: parse_json_or_default(
                            &mailbox_policy_raw,
                            octopus_core::default_mailbox_policy,
                        ),
                        artifact_handoff_policy: parse_json_or_default(
                            &artifact_handoff_policy_raw,
                            octopus_core::default_artifact_handoff_policy,
                        ),
                        workflow_affordance: parse_json_or_default(&workflow_affordance_raw, || {
                            octopus_core::workflow_affordance_from_task_domains(
                                &Vec::new(),
                                true,
                                true,
                            )
                        }),
                        worker_concurrency_limit,
                        integration_source: None,
                        trust_metadata: octopus_core::default_asset_trust_metadata(),
                        dependency_resolution: Vec::new(),
                        import_metadata: octopus_core::default_asset_import_metadata(),
                        description: row.get(32)?,
                        status: row.get(33)?,
                        updated_at: row.get::<_, i64>(34)? as u64,
                    })
                },
            )
            .map_err(|error| {
                if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::not_found("team")
                } else {
                    AppError::database(error.to_string())
                }
            })
            .or_else(|error| {
                if matches!(error, AppError::NotFound(_)) {
                    fallback_builtin_team_record(team_id)
                        .ok_or_else(|| AppError::not_found("team"))
                } else {
                    Err(error)
                }
            })
    }
}
