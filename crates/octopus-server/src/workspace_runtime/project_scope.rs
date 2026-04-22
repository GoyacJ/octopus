use super::*;

fn normalize_project_string_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !normalized.iter().any(|item| item == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn normalize_project_assignments(
    assignments: Option<octopus_core::ProjectWorkspaceAssignments>,
) -> Option<octopus_core::ProjectWorkspaceAssignments> {
    assignments.map(|mut assignments| {
        if let Some(models) = assignments.models.as_mut() {
            models.configured_model_ids =
                normalize_project_string_list(std::mem::take(&mut models.configured_model_ids));
            models.default_configured_model_id =
                models.default_configured_model_id.trim().to_string();
        }
        if let Some(tools) = assignments.tools.as_mut() {
            tools.source_keys =
                normalize_project_string_list(std::mem::take(&mut tools.source_keys));
            tools.excluded_source_keys =
                normalize_project_string_list(std::mem::take(&mut tools.excluded_source_keys));
        }
        if let Some(agents) = assignments.agents.as_mut() {
            agents.agent_ids = normalize_project_string_list(std::mem::take(&mut agents.agent_ids));
            agents.team_ids = normalize_project_string_list(std::mem::take(&mut agents.team_ids));
            agents.excluded_agent_ids =
                normalize_project_string_list(std::mem::take(&mut agents.excluded_agent_ids));
            agents.excluded_team_ids =
                normalize_project_string_list(std::mem::take(&mut agents.excluded_team_ids));
        }
        assignments
    })
}

fn project_workspace_assignments(
    document: &serde_json::Value,
) -> Option<octopus_core::ProjectWorkspaceAssignments> {
    let assignments = document
        .get("projectSettings")
        .and_then(|settings| settings.get("workspaceAssignments"))?;
    let assignments = serde_json::from_value(assignments.clone()).ok()?;
    normalize_project_assignments(Some(assignments))
}

#[derive(Debug, Default)]
pub(crate) struct ProjectGrantedScope {
    pub(crate) workspace_active_agent_ids: BTreeSet<String>,
    pub(crate) agents: Vec<octopus_core::AgentRecord>,
    pub(crate) teams: Vec<octopus_core::TeamRecord>,
    pub(crate) tool_source_keys: Vec<String>,
}

#[derive(Debug, Default)]
struct ProjectRuntimeDisables {
    agent_ids: BTreeSet<String>,
}

fn project_tool_assignments(
    assignments: Option<&octopus_core::ProjectWorkspaceAssignments>,
) -> Option<&octopus_core::ProjectToolAssignments> {
    assignments.and_then(|assignments| assignments.tools.as_ref())
}

fn project_agent_assignments(
    assignments: Option<&octopus_core::ProjectWorkspaceAssignments>,
) -> Option<&octopus_core::ProjectAgentAssignments> {
    assignments.and_then(|assignments| assignments.agents.as_ref())
}

pub(crate) async fn resolve_project_granted_scope(
    state: &ServerState,
    project: &ProjectRecord,
    runtime_document: &serde_json::Value,
) -> Result<ProjectGrantedScope, ApiError> {
    let assignments = project_workspace_assignments(runtime_document);
    let excluded_agent_ids = project_agent_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_agent_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let excluded_team_ids = project_agent_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_team_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let excluded_tool_source_keys = project_tool_assignments(assignments.as_ref())
        .map(|assignments| {
            assignments
                .excluded_source_keys
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let mut workspace_active_agent_ids = BTreeSet::new();
    let mut seen_agent_ids = BTreeSet::new();
    let mut agents = Vec::new();
    for record in state.services.workspace.list_agents().await? {
        if record.status != "active" || !agent_visible_in_generic_catalog(&record) {
            continue;
        }
        let is_project_owned = record.project_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = record.project_id.is_none();
        if is_workspace_inherited {
            workspace_active_agent_ids.insert(record.id.clone());
        }
        if (is_project_owned
            || (is_workspace_inherited && !excluded_agent_ids.contains(&record.id)))
            && seen_agent_ids.insert(record.id.clone())
        {
            agents.push(record);
        }
    }

    let mut seen_team_ids = BTreeSet::new();
    let mut teams = Vec::new();
    for record in state.services.workspace.list_teams().await? {
        if record.status != "active" {
            continue;
        }
        let is_project_owned = record.project_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = record.project_id.is_none();
        if (is_project_owned || (is_workspace_inherited && !excluded_team_ids.contains(&record.id)))
            && seen_team_ids.insert(record.id.clone())
        {
            teams.push(record);
        }
    }

    let mut tool_source_keys = BTreeSet::new();
    for asset in state
        .services
        .workspace
        .get_capability_management_projection()
        .await?
        .assets
    {
        if !asset.enabled {
            continue;
        }
        let is_project_owned = asset.owner_scope.as_deref() == Some("project")
            && asset.owner_id.as_deref() == Some(project.id.as_str());
        let is_workspace_inherited = asset.owner_scope.as_deref() != Some("project");
        if is_project_owned
            || (is_workspace_inherited && !excluded_tool_source_keys.contains(&asset.source_key))
        {
            tool_source_keys.insert(asset.source_key);
        }
    }

    Ok(ProjectGrantedScope {
        workspace_active_agent_ids,
        agents,
        teams,
        tool_source_keys: tool_source_keys.into_iter().collect(),
    })
}

fn merge_runtime_config_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match patch {
        serde_json::Value::Object(patch_map) => {
            if !target.is_object() {
                *target = serde_json::Value::Object(serde_json::Map::new());
            }
            let target_map = target
                .as_object_mut()
                .expect("target should be an object after initialization");
            for (key, value) in patch_map {
                if value.is_null() {
                    target_map.remove(key);
                    continue;
                }
                if let Some(existing) = target_map.get_mut(key) {
                    merge_runtime_config_patch(existing, value);
                } else {
                    target_map.insert(key.clone(), value.clone());
                }
            }
        }
        _ => *target = patch.clone(),
    }
}

pub(crate) async fn load_project_runtime_document(
    state: &ServerState,
    project: &ProjectRecord,
    patch: Option<&serde_json::Value>,
) -> Result<serde_json::Value, ApiError> {
    let config = state
        .services
        .runtime_config
        .get_project_config(&project.id, &project.owner_user_id)
        .await?;
    let mut document = config
        .sources
        .into_iter()
        .find(|source| source.scope == "project")
        .and_then(|source| source.document)
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    if let Some(patch) = patch {
        merge_runtime_config_patch(&mut document, patch);
    }
    Ok(document)
}

fn normalize_runtime_string_set(value: Option<&serde_json::Value>) -> Option<BTreeSet<String>> {
    let values = value.and_then(serde_json::Value::as_array)?;
    Some(
        values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn resolve_project_runtime_disables(
    document: &serde_json::Value,
    scope: &ProjectGrantedScope,
) -> ProjectRuntimeDisables {
    let project_settings = document
        .get("projectSettings")
        .and_then(serde_json::Value::as_object);
    let agents_object = project_settings
        .and_then(|settings| settings.get("agents"))
        .and_then(serde_json::Value::as_object);

    let granted_agent_ids = scope
        .agents
        .iter()
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();

    let disabled_agent_ids = normalize_runtime_string_set(
        agents_object.and_then(|settings| settings.get("disabledAgentIds")),
    )
    .map(|values| {
        values
            .into_iter()
            .filter(|value| granted_agent_ids.contains(value))
            .collect()
    })
    .or_else(|| {
        normalize_runtime_string_set(
            agents_object.and_then(|settings| settings.get("enabledAgentIds")),
        )
        .map(|enabled| {
            granted_agent_ids
                .iter()
                .filter(|value| !enabled.contains(*value))
                .cloned()
                .collect()
        })
    })
    .unwrap_or_default();

    ProjectRuntimeDisables {
        agent_ids: disabled_agent_ids,
    }
}

fn validate_project_leader_against_scope(
    leader_agent_id: Option<&str>,
    scope: &ProjectGrantedScope,
    runtime_disables: &ProjectRuntimeDisables,
) -> Result<(), ApiError> {
    let Some(leader_agent_id) = leader_agent_id else {
        return Ok(());
    };
    let granted_agent_ids = scope
        .agents
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    if !scope.workspace_active_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must reference an active workspace agent",
        )
        .into());
    }
    if !granted_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must remain in the effective project agent scope",
        )
        .into());
    }
    if runtime_disables.agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input("project leader must remain runtime enabled").into());
    }
    Ok(())
}

pub(crate) async fn validate_create_project_leader(
    state: &ServerState,
    request: &CreateProjectRequest,
) -> Result<(), ApiError> {
    let Some(leader_agent_id) = request.leader_agent_id.as_deref() else {
        return Ok(());
    };
    let workspace_active_agent_ids = state
        .services
        .workspace
        .list_agents()
        .await?
        .into_iter()
        .filter(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .map(|record| record.id)
        .collect::<BTreeSet<_>>();
    if !workspace_active_agent_ids.contains(leader_agent_id) {
        return Err(AppError::invalid_input(
            "project leader must reference a granted active workspace agent",
        )
        .into());
    }
    Ok(())
}

pub(crate) async fn validate_updated_project_leader(
    state: &ServerState,
    project: &ProjectRecord,
    request: &UpdateProjectRequest,
) -> Result<(), ApiError> {
    let runtime_document = load_project_runtime_document(state, project, None).await?;
    let scope = resolve_project_granted_scope(state, project, &runtime_document).await?;
    let runtime_disables = resolve_project_runtime_disables(&runtime_document, &scope);
    let leader_agent_id = request
        .leader_agent_id
        .as_deref()
        .or(project.leader_agent_id.as_deref());
    validate_project_leader_against_scope(leader_agent_id, &scope, &runtime_disables)
}

pub(crate) async fn validate_project_runtime_leader(
    state: &ServerState,
    project: &ProjectRecord,
    patch: &RuntimeConfigPatch,
) -> Result<(), ApiError> {
    let runtime_document =
        load_project_runtime_document(state, project, Some(&patch.patch)).await?;
    let scope = resolve_project_granted_scope(state, project, &runtime_document).await?;
    let runtime_disables = resolve_project_runtime_disables(&runtime_document, &scope);
    validate_project_leader_against_scope(
        project.leader_agent_id.as_deref(),
        &scope,
        &runtime_disables,
    )
}

pub(crate) async fn lookup_project(
    state: &ServerState,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    state
        .services
        .workspace
        .list_projects()
        .await?
        .into_iter()
        .find(|record| record.id == project_id)
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("project {project_id}"))))
}

pub(crate) async fn ensure_project_owner(
    state: &ServerState,
    session: &SessionRecord,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    let project = lookup_project(state, project_id).await?;
    if project.owner_user_id != session.user_id {
        return Err(ApiError::from(AppError::auth(
            "project owner access is required",
        )));
    }
    Ok(project)
}

pub(crate) async fn ensure_project_owner_session(
    state: &ServerState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectRecord, ApiError> {
    let session = authenticate_session(state, headers).await?;
    ensure_project_owner(state, &session, project_id).await
}
