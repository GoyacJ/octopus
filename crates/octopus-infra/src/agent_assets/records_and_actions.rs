pub(crate) fn build_agent_record(
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    agent_id: &str,
    name: &str,
    avatar_path: Option<String>,
    description: &str,
    personality: &str,
    prompt: &str,
    tags: &[String],
    builtin_tool_keys: &[String],
    skill_ids: &[String],
    mcp_server_names: &[String],
) -> AgentRecord {
    let builtin_tool_keys = builtin_tool_keys.to_vec();
    let skill_ids = skill_ids.to_vec();
    let mcp_server_names = mcp_server_names.to_vec();
    let task_domains = normalize_task_domains(tags.to_vec());
    AgentRecord {
        id: agent_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: target.project_id().map(ToOwned::to_owned),
        scope: target.scope_label().into(),
        owner_user_id: None,
        asset_role: default_agent_asset_role(),
        name: name.trim().to_string(),
        avatar_path: avatar_path.clone(),
        avatar: agent_avatar(paths, avatar_path.as_deref()),
        personality: personality.trim().to_string(),
        tags: tags.to_vec(),
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: mcp_server_names.clone(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(
            &builtin_tool_keys,
            &skill_ids,
            &mcp_server_names,
        ),
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

pub(crate) fn build_team_record(
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    team_id: &str,
    name: &str,
    avatar_path: Option<String>,
    description: &str,
    personality: &str,
    prompt: &str,
    tags: &[String],
    builtin_tool_keys: &[String],
    skill_ids: &[String],
    mcp_server_names: &[String],
    leader_ref: String,
    member_refs: Vec<String>,
) -> TeamRecord {
    let builtin_tool_keys = builtin_tool_keys.to_vec();
    let skill_ids = skill_ids.to_vec();
    let mcp_server_names = mcp_server_names.to_vec();
    let task_domains = normalize_task_domains(tags.to_vec());
    let delegation_policy = default_team_delegation_policy();
    let leader_ref = crate::canonical_agent_ref(&leader_ref);
    let member_refs = crate::canonical_agent_refs(&member_refs);
    TeamRecord {
        id: team_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: target.project_id().map(ToOwned::to_owned),
        scope: target.scope_label().into(),
        name: name.trim().to_string(),
        avatar_path: avatar_path.clone(),
        avatar: agent_avatar(paths, avatar_path.as_deref()),
        personality: personality.trim().to_string(),
        tags: tags.to_vec(),
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.clone(),
        skill_ids: skill_ids.clone(),
        mcp_server_names: mcp_server_names.clone(),
        task_domains: task_domains.clone(),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(
            &builtin_tool_keys,
            &skill_ids,
            &mcp_server_names,
        ),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_team_memory_policy(),
        delegation_policy: delegation_policy.clone(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_team_shared_capability_policy(),
        leader_ref: leader_ref.clone(),
        member_refs: member_refs.clone(),
        team_topology: team_topology_from_refs(Some(leader_ref), member_refs.clone()),
        shared_memory_policy: default_shared_memory_policy(),
        mailbox_policy: default_mailbox_policy(),
        artifact_handoff_policy: default_artifact_handoff_policy(),
        workflow_affordance: workflow_affordance_from_task_domains(&task_domains, true, true),
        worker_concurrency_limit: delegation_policy.max_worker_count,
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

fn compute_agent_hash(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    record: &PlannedAgent,
    skill_ids: &[String],
) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": workspace_id,
        "projectId": target.project_id(),
        "scope": target.scope_label(),
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "model": record.model,
        "status": "active",
    }))?))
}

fn compute_existing_agent_hash(record: &AgentRecord) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": record.workspace_id,
        "projectId": record.project_id,
        "scope": record.scope,
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": record.skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "model": record.default_model_strategy.preferred_model_ref,
        "status": record.status,
    }))?))
}

fn compute_team_hash(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    record: &PlannedTeam,
    skill_ids: &[String],
    leader_ref: &str,
    member_refs: &[String],
) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": workspace_id,
        "projectId": target.project_id(),
        "scope": target.scope_label(),
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "leaderRef": leader_ref,
        "memberRefs": member_refs,
        "model": record.model,
        "status": "active",
    }))?))
}

fn compute_existing_team_hash(record: &TeamRecord) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(&json!({
        "workspaceId": record.workspace_id,
        "projectId": record.project_id,
        "scope": record.scope,
        "name": record.name,
        "description": record.description,
        "personality": record.personality,
        "prompt": record.prompt,
        "tags": record.tags,
        "builtinToolKeys": record.builtin_tool_keys,
        "skillIds": record.skill_ids,
        "mcpServerNames": record.mcp_server_names,
        "leaderRef": record.leader_ref,
        "memberRefs": record.member_refs,
        "model": record.default_model_strategy.preferred_model_ref,
        "status": record.status,
    }))?))
}

pub(crate) fn resolve_agent_action(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    existing_agents: &HashMap<String, AgentRecord>,
    planned: &PlannedAgent,
    skill_ids: &[String],
) -> Result<ImportAction, AppError> {
    let agent_id = planned
        .agent_id
        .clone()
        .unwrap_or_else(|| deterministic_asset_id("agent", target, &planned.source_id));
    let desired_hash = compute_agent_hash(workspace_id, target, planned, skill_ids)?;
    let Some(existing) = existing_agents.get(&agent_id) else {
        return Ok(ImportAction::Create);
    };
    if compute_existing_agent_hash(existing)? == desired_hash {
        Ok(ImportAction::Skip)
    } else {
        Ok(ImportAction::Update)
    }
}

pub(crate) fn resolve_team_action(
    workspace_id: &str,
    target: &AssetTargetScope<'_>,
    existing_teams: &HashMap<String, TeamRecord>,
    planned: &PlannedTeam,
    skill_ids: &[String],
    leader_ref: &str,
    member_refs: &[String],
) -> Result<ImportAction, AppError> {
    let team_id = planned
        .team_id
        .clone()
        .unwrap_or_else(|| deterministic_asset_id("team", target, &planned.source_id));
    let desired_hash = compute_team_hash(
        workspace_id,
        target,
        planned,
        skill_ids,
        leader_ref,
        member_refs,
    )?;
    let Some(existing) = existing_teams.get(&team_id) else {
        return Ok(ImportAction::Create);
    };
    if compute_existing_team_hash(existing)? == desired_hash {
        Ok(ImportAction::Skip)
    } else {
        Ok(ImportAction::Update)
    }
}

pub(crate) fn load_scoped_agents(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, AgentRecord>, AppError> {
    Ok(load_agents(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

pub(crate) fn load_scoped_teams(
    connection: &Connection,
    target: &AssetTargetScope<'_>,
) -> Result<HashMap<String, TeamRecord>, AppError> {
    Ok(load_teams(connection)?
        .into_iter()
        .filter(|record| record.project_id.as_deref() == target.project_id())
        .map(|record| (record.id.clone(), record))
        .collect())
}

pub(crate) fn load_existing_managed_skills(
    skills_root: &Path,
) -> Result<HashMap<String, ExistingManagedSkill>, AppError> {
    let mut skills = HashMap::new();
    if !skills_root.is_dir() {
        return Ok(skills);
    }
    for entry in fs::read_dir(skills_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let skill_file = path.join(SKILL_FRONTMATTER_FILE);
        if !skill_file.is_file() {
            continue;
        }
        let files = read_directory_files(&path)?;
        skills.insert(
            slug.clone(),
            ExistingManagedSkill {
                slug,
                content_hash: hash_bundle_files(&files),
            },
        );
    }
    Ok(skills)
}

