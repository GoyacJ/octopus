use super::*;

pub(crate) fn ensure_project_assignment_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "projects",
        &[
            ("leader_agent_id", "TEXT"),
            ("manager_user_id", "TEXT"),
            ("preset_code", "TEXT"),
            ("assignments_json", "TEXT"),
            ("resource_directory", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("member_user_ids_json", "TEXT"),
            ("permission_overrides_json", "TEXT"),
            ("linked_workspace_assets_json", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_project_promotion_request_table(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_promotion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              asset_type TEXT NOT NULL,
              asset_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              submitted_by_owner_user_id TEXT NOT NULL,
              required_workspace_capability TEXT NOT NULL,
              status TEXT NOT NULL,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_deletion_request_table(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_deletion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              status TEXT NOT NULL,
              reason TEXT,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_agent_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_team_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn backfill_project_resource_directories(
    connection: &Connection,
    paths: &WorkspacePaths,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("SELECT id, resource_directory FROM projects")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (project_id, stored_directory) in rows {
        let resource_directory = stored_directory
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| paths.default_project_resource_directory(&project_id));
        connection
            .execute(
                "UPDATE projects SET resource_directory = ?2 WHERE id = ?1",
                params![project_id, resource_directory],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        fs::create_dir_all(paths.root.join(&resource_directory))?;
    }

    fs::create_dir_all(&paths.workspace_resources_dir)?;
    Ok(())
}

pub(crate) fn backfill_project_governance(
    connection: &Connection,
    workspace_owner_user_id: Option<&str>,
) -> Result<(), AppError> {
    let resolved_workspace_owner_user_id = workspace_owner_user_id
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let replace_bootstrap_placeholder = resolved_workspace_owner_user_id.is_some();
    let fallback_owner_user_id = resolved_workspace_owner_user_id
        .clone()
        .unwrap_or_else(|| BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID.to_string());
    let data_policies = load_data_policies(connection)?;
    let selected_project_members = data_policies
        .into_iter()
        .filter(|policy| {
            policy.subject_type == "user"
                && policy.resource_type == "project"
                && policy.scope_type == "selected-projects"
                && policy.effect == "allow"
        })
        .fold(
            std::collections::BTreeMap::<String, Vec<String>>::new(),
            |mut acc, policy| {
                for project_id in policy.project_ids {
                    acc.entry(project_id)
                        .or_default()
                        .push(policy.subject_id.clone());
                }
                acc
            },
        );

    let mut stmt = connection
        .prepare(
            "SELECT id, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (
        project_id,
        assignments_json,
        stored_owner_user_id,
        stored_member_user_ids_json,
        stored_permission_overrides_json,
        stored_linked_workspace_assets_json,
    ) in rows
    {
        let owner_user_id = stored_owner_user_id
            .filter(|value| !value.trim().is_empty())
            .filter(|value| {
                !(replace_bootstrap_placeholder && value == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .unwrap_or_else(|| fallback_owner_user_id.clone());
        let member_user_ids = stored_member_user_ids_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<Vec<String>>)
            .transpose()?
            .unwrap_or_else(|| {
                selected_project_members
                    .get(&project_id)
                    .cloned()
                    .unwrap_or_default()
            })
            .into_iter()
            .filter(|user_id| {
                !(replace_bootstrap_placeholder && user_id == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .collect::<Vec<_>>();
        let permission_overrides = stored_permission_overrides_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectPermissionOverrides>)
            .transpose()?
            .unwrap_or_else(default_project_permission_overrides);
        let linked_workspace_assets = stored_linked_workspace_assets_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
            .transpose()?
            .unwrap_or_else(|| {
                let assignments = assignments_json
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                    .transpose()
                    .ok()
                    .flatten();
                ProjectLinkedWorkspaceAssets {
                    agent_ids: assignments
                        .as_ref()
                        .and_then(|value| value.agents.as_ref())
                        .map(|value| value.agent_ids.clone())
                        .unwrap_or_default(),
                    resource_ids: Vec::new(),
                    tool_source_keys: assignments
                        .as_ref()
                        .and_then(|value| value.tools.as_ref())
                        .map(|value| value.source_keys.clone())
                        .unwrap_or_default(),
                    knowledge_ids: Vec::new(),
                }
            });
        let normalized_members =
            normalized_project_member_user_ids(&owner_user_id, member_user_ids);

        connection
            .execute(
                "UPDATE projects
                 SET owner_user_id = ?2,
                     member_user_ids_json = ?3,
                     permission_overrides_json = ?4,
                     linked_workspace_assets_json = ?5
                 WHERE id = ?1",
                params![
                    project_id,
                    owner_user_id,
                    serde_json::to_string(&normalized_members)?,
                    serde_json::to_string(&permission_overrides)?,
                    serde_json::to_string(&linked_workspace_assets)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(crate) fn backfill_default_project_assignments(
    connection: &Connection,
) -> Result<(), AppError> {
    let stored_assignments_json = connection
        .query_row(
            "SELECT assignments_json FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .flatten();
    let parsed_assignments = stored_assignments_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
        .transpose()?;
    let needs_model_backfill = parsed_assignments
        .as_ref()
        .and_then(|assignments| assignments.models.as_ref())
        .is_none_or(|models| {
            models.default_configured_model_id.trim().is_empty()
                || models.configured_model_ids.is_empty()
        });
    if !needs_model_backfill {
        return Ok(());
    }

    let next_assignments = match parsed_assignments {
        Some(mut assignments) => {
            assignments.models = Some(default_project_model_assignments());
            assignments
        }
        None => default_project_assignments(),
    };

    connection
        .execute(
            "UPDATE projects SET assignments_json = ?2 WHERE id = ?1",
            params![
                DEFAULT_PROJECT_ID,
                serde_json::to_string(&next_assignments)?,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}
