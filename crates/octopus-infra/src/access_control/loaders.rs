use super::*;

pub(crate) fn load_org_units(connection: &Connection) -> Result<Vec<OrgUnitRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT id, parent_id, code, name, status FROM org_units ORDER BY code ASC")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(OrgUnitRecord {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                code: row.get(2)?,
                name: row.get(3)?,
                status: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_positions(connection: &Connection) -> Result<Vec<PositionRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT id, code, name, status FROM positions ORDER BY code ASC")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PositionRecord {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_user_groups(connection: &Connection) -> Result<Vec<UserGroupRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT id, code, name, status FROM user_groups ORDER BY code ASC")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(UserGroupRecord {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_user_org_assignments(
    connection: &Connection,
) -> Result<Vec<UserOrgAssignmentRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT user_id, org_unit_id, is_primary, position_ids, user_group_ids
             FROM user_org_assignments
             ORDER BY user_id ASC, is_primary DESC, org_unit_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let position_ids_raw: String = row.get(3)?;
            let user_group_ids_raw: String = row.get(4)?;
            Ok(UserOrgAssignmentRecord {
                user_id: row.get(0)?,
                org_unit_id: row.get(1)?,
                is_primary: row.get::<_, i64>(2)? == 1,
                position_ids: read_json_vec(&position_ids_raw),
                user_group_ids: read_json_vec(&user_group_ids_raw),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_access_roles(
    connection: &Connection,
) -> Result<Vec<AccessRoleRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, code, name, description, status, permission_codes
             FROM access_roles
             ORDER BY code ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let role_code: String = row.get(1)?;
            let permission_codes_raw: String = row.get(5)?;
            Ok(AccessRoleRecord {
                id: row.get(0)?,
                code: role_code.clone(),
                name: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                permission_codes: read_json_vec(&permission_codes_raw),
                source: access_role_source(&role_code),
                editable: access_role_editable(&role_code),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_role_bindings(
    connection: &Connection,
) -> Result<Vec<RoleBindingRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, role_id, subject_type, subject_id, effect
             FROM role_bindings
             ORDER BY subject_type ASC, subject_id ASC, role_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RoleBindingRecord {
                id: row.get(0)?,
                role_id: row.get(1)?,
                subject_type: row.get(2)?,
                subject_id: row.get(3)?,
                effect: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_data_policies(
    connection: &Connection,
) -> Result<Vec<DataPolicyRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, name, subject_type, subject_id, resource_type, scope_type, project_ids, tags, classifications, effect
             FROM data_policies
             ORDER BY subject_type ASC, subject_id ASC, name ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let project_ids_raw: String = row.get(6)?;
            let tags_raw: String = row.get(7)?;
            Ok(DataPolicyRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                subject_type: row.get(2)?,
                subject_id: row.get(3)?,
                resource_type: row.get(4)?,
                scope_type: row.get(5)?,
                project_ids: read_json_vec(&project_ids_raw),
                tags: read_json_vec(&tags_raw),
                classifications: read_json_vec(&row.get::<_, String>(8)?),
                effect: row.get(9)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_resource_policies(
    connection: &Connection,
) -> Result<Vec<ResourcePolicyRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, subject_type, subject_id, resource_type, resource_id, action_name, effect
             FROM resource_policies
             ORDER BY subject_type ASC, subject_id ASC, resource_type ASC, resource_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ResourcePolicyRecord {
                id: row.get(0)?,
                subject_type: row.get(1)?,
                subject_id: row.get(2)?,
                resource_type: row.get(3)?,
                resource_id: row.get(4)?,
                action: row.get(5)?,
                effect: row.get(6)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_protected_resource_metadata(
    connection: &Connection,
) -> Result<Vec<ProtectedResourceDescriptor>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT resource_type, resource_id, resource_subtype, project_id, tags, classification, owner_subject_type, owner_subject_id
             FROM protected_resources
             ORDER BY resource_type ASC, resource_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let tags_raw: String = row.get(4)?;
            Ok(ProtectedResourceDescriptor {
                id: row.get(1)?,
                resource_type: row.get(0)?,
                resource_subtype: row.get(2)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                project_id: row.get(3)?,
                tags: read_json_vec(&tags_raw),
                classification: row.get(5)?,
                owner_subject_type: row.get(6)?,
                owner_subject_id: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_menu_policies(
    connection: &Connection,
) -> Result<Vec<MenuPolicyRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT menu_id, enabled, order_value, group_key, visibility
             FROM menu_policies
             ORDER BY order_value ASC, menu_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MenuPolicyRecord {
                menu_id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? == 1,
                order: row.get(2)?,
                group: row.get(3)?,
                visibility: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}
