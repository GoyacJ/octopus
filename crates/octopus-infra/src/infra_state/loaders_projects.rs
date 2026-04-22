use super::*;

pub(crate) fn load_projects(connection: &Connection) -> Result<Vec<ProjectRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, name, status, description, resource_directory, leader_agent_id, manager_user_id, preset_code, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let leader_agent_id: Option<String> = row.get(6)?;
            let manager_user_id: Option<String> = row.get(7)?;
            let preset_code: Option<String> = row.get(8)?;
            let assignments_json: Option<String> = row.get(9)?;
            let assignments = assignments_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let owner_user_id: Option<String> = row.get(10)?;
            let member_user_ids_json: Option<String> = row.get(11)?;
            let member_user_ids = member_user_ids_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<Vec<String>>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        11,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_default();
            let permission_overrides_json: Option<String> = row.get(12)?;
            let permission_overrides = permission_overrides_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectPermissionOverrides>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        12,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_else(default_project_permission_overrides);
            let linked_workspace_assets_json: Option<String> = row.get(13)?;
            let linked_workspace_assets = linked_workspace_assets_json
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        13,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_else(empty_project_linked_workspace_assets);
            let owner_user_id = owner_user_id.unwrap_or_else(|| "user-owner".into());
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                resource_directory: row.get(5)?,
                leader_agent_id,
                manager_user_id,
                preset_code,
                owner_user_id: owner_user_id.clone(),
                member_user_ids: normalized_project_member_user_ids(
                    &owner_user_id,
                    member_user_ids,
                ),
                permission_overrides,
                linked_workspace_assets,
                assignments,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_project_promotion_requests(
    connection: &Connection,
) -> Result<Vec<ProjectPromotionRequest>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, asset_type, asset_id, requested_by_user_id, submitted_by_owner_user_id, required_workspace_capability, status, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at
             FROM project_promotion_requests
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectPromotionRequest {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                asset_type: row.get(3)?,
                asset_id: row.get(4)?,
                requested_by_user_id: row.get(5)?,
                submitted_by_owner_user_id: row.get(6)?,
                required_workspace_capability: row.get(7)?,
                status: row.get(8)?,
                reviewed_by_user_id: row.get(9)?,
                review_comment: row.get(10)?,
                created_at: row.get::<_, i64>(11)? as u64,
                updated_at: row.get::<_, i64>(12)? as u64,
                reviewed_at: row.get::<_, Option<i64>>(13)?.map(|value| value as u64),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_project_deletion_requests(
    connection: &Connection,
) -> Result<Vec<ProjectDeletionRequest>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, requested_by_user_id, status, reason, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at
             FROM project_deletion_requests
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectDeletionRequest {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                requested_by_user_id: row.get(3)?,
                status: row.get(4)?,
                reason: row.get(5)?,
                reviewed_by_user_id: row.get(6)?,
                review_comment: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
                updated_at: row.get::<_, i64>(9)? as u64,
                reviewed_at: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}
