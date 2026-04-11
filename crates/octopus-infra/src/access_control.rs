use super::*;
use octopus_core::{
    AccessRoleRecord, AccessUserRecord, AccessUserUpsertRequest, DataPolicyRecord,
    DataPolicyUpsertRequest, MenuPolicyRecord, MenuPolicyUpsertRequest, OrgUnitRecord,
    OrgUnitUpsertRequest, PositionRecord, PositionUpsertRequest, ProtectedResourceDescriptor,
    ProtectedResourceMetadataUpsertRequest, ResourcePolicyRecord, ResourcePolicyUpsertRequest,
    RoleBindingRecord, RoleBindingUpsertRequest, RoleUpsertRequest, UserGroupRecord,
    UserGroupUpsertRequest, UserOrgAssignmentRecord, UserOrgAssignmentUpsertRequest,
};
use std::collections::{BTreeSet, HashMap};

fn read_json_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn bool_to_sql(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(super) fn load_org_units(connection: &Connection) -> Result<Vec<OrgUnitRecord>, AppError> {
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

fn normalize_subject_type(value: &str) -> &str {
    match value {
        "org_unit" | "org-unit" => "org-unit",
        "user_group" | "user-group" => "user-group",
        other => other,
    }
}

pub(super) fn org_unit_ancestor_ids(
    units: &[OrgUnitRecord],
    org_unit_id: &str,
) -> BTreeSet<String> {
    let parent_by_id = units
        .iter()
        .map(|unit| (unit.id.as_str(), unit.parent_id.as_deref()))
        .collect::<HashMap<_, _>>();
    let mut ancestors = BTreeSet::new();
    let mut cursor = Some(org_unit_id);
    while let Some(current) = cursor {
        if !ancestors.insert(current.to_string()) {
            break;
        }
        cursor = parent_by_id.get(current).copied().flatten();
    }
    ancestors
}

fn load_positions(connection: &Connection) -> Result<Vec<PositionRecord>, AppError> {
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

fn load_user_groups(connection: &Connection) -> Result<Vec<UserGroupRecord>, AppError> {
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

pub(super) fn load_user_org_assignments(
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

fn load_access_roles(connection: &Connection) -> Result<Vec<AccessRoleRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, code, name, description, status, permission_codes
             FROM access_roles
             ORDER BY code ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let permission_codes_raw: String = row.get(5)?;
            Ok(AccessRoleRecord {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                permission_codes: read_json_vec(&permission_codes_raw),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_role_bindings(
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

pub(super) fn load_data_policies(
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

pub(super) fn load_resource_policies(
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

fn load_protected_resource_metadata(
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

pub(super) fn resolve_subject_resource_policies(
    connection: &Connection,
    user_id: &str,
) -> Result<Vec<ResourcePolicyRecord>, AppError> {
    let org_units = load_org_units(connection)?;
    let assignments = assignments_for_user(&load_user_org_assignments(connection)?, user_id);
    let position_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.position_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let user_group_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.user_group_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let org_unit_ids = assignments
        .iter()
        .flat_map(|assignment| org_unit_ancestor_ids(&org_units, &assignment.org_unit_id))
        .collect::<BTreeSet<_>>();

    Ok(load_resource_policies(connection)?
        .into_iter()
        .filter(
            |policy| match normalize_subject_type(&policy.subject_type) {
                "user" => policy.subject_id == user_id,
                "org-unit" => org_unit_ids.contains(&policy.subject_id),
                "position" => position_ids.contains(&policy.subject_id),
                "user-group" => user_group_ids.contains(&policy.subject_id),
                _ => false,
            },
        )
        .collect())
}

fn load_menu_policies(connection: &Connection) -> Result<Vec<MenuPolicyRecord>, AppError> {
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

pub(super) fn assignments_for_user(
    assignments: &[UserOrgAssignmentRecord],
    user_id: &str,
) -> Vec<UserOrgAssignmentRecord> {
    assignments
        .iter()
        .filter(|assignment| assignment.user_id == user_id)
        .cloned()
        .collect()
}

pub(super) fn resolve_subject_role_bindings(
    connection: &Connection,
    user_id: &str,
) -> Result<Vec<RoleBindingRecord>, AppError> {
    let org_units = load_org_units(connection)?;
    let assignments = assignments_for_user(&load_user_org_assignments(connection)?, user_id);
    let position_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.position_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let user_group_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.user_group_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let org_unit_ids = assignments
        .iter()
        .flat_map(|assignment| org_unit_ancestor_ids(&org_units, &assignment.org_unit_id))
        .collect::<BTreeSet<_>>();

    Ok(load_role_bindings(connection)?
        .into_iter()
        .filter(
            |binding| match normalize_subject_type(&binding.subject_type) {
                "user" => binding.subject_id == user_id,
                "org-unit" => org_unit_ids.contains(&binding.subject_id),
                "position" => position_ids.contains(&binding.subject_id),
                "user-group" => user_group_ids.contains(&binding.subject_id),
                _ => false,
            },
        )
        .collect())
}

pub(super) fn resolve_subject_data_policies(
    connection: &Connection,
    user_id: &str,
) -> Result<Vec<DataPolicyRecord>, AppError> {
    let org_units = load_org_units(connection)?;
    let assignments = assignments_for_user(&load_user_org_assignments(connection)?, user_id);
    let position_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.position_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let user_group_ids = assignments
        .iter()
        .flat_map(|assignment| assignment.user_group_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let org_unit_ids = assignments
        .iter()
        .flat_map(|assignment| org_unit_ancestor_ids(&org_units, &assignment.org_unit_id))
        .collect::<BTreeSet<_>>();

    Ok(load_data_policies(connection)?
        .into_iter()
        .filter(
            |policy| match normalize_subject_type(&policy.subject_type) {
                "user" => policy.subject_id == user_id,
                "org-unit" => org_unit_ids.contains(&policy.subject_id),
                "position" => position_ids.contains(&policy.subject_id),
                "user-group" => user_group_ids.contains(&policy.subject_id),
                _ => false,
            },
        )
        .collect())
}

pub(super) fn resolve_effective_role_ids(
    connection: &Connection,
    user_id: &str,
) -> Result<(Vec<String>, Vec<RoleBindingRecord>), AppError> {
    let bindings = resolve_subject_role_bindings(connection, user_id)?;
    let deny_role_ids = bindings
        .iter()
        .filter(|binding| binding.effect == "deny")
        .map(|binding| binding.role_id.clone())
        .collect::<BTreeSet<_>>();
    let effective_role_ids = bindings
        .iter()
        .filter(|binding| binding.effect == "allow" && !deny_role_ids.contains(&binding.role_id))
        .map(|binding| binding.role_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok((effective_role_ids, bindings))
}

pub(super) fn resolve_effective_permission_codes(
    connection: &Connection,
    user_id: &str,
) -> Result<(Vec<String>, Vec<RoleBindingRecord>), AppError> {
    let (role_ids, bindings) = resolve_effective_role_ids(connection, user_id)?;
    let roles_by_id = load_access_roles(connection)?
        .into_iter()
        .map(|role| (role.id.clone(), role))
        .collect::<HashMap<_, _>>();
    let permission_codes = role_ids
        .into_iter()
        .filter_map(|role_id| roles_by_id.get(&role_id).cloned())
        .flat_map(|role| role.permission_codes.into_iter())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok((permission_codes, bindings))
}

pub(crate) fn default_owner_permission_codes() -> Vec<String> {
    vec![
        "workspace.overview.read",
        "project.view",
        "project.manage",
        "team.view",
        "team.manage",
        "team.import",
        "access.users.read",
        "access.users.manage",
        "access.org.read",
        "access.org.manage",
        "access.roles.read",
        "access.roles.manage",
        "access.policies.read",
        "access.policies.manage",
        "access.menus.read",
        "access.menus.manage",
        "access.sessions.read",
        "access.sessions.manage",
        "agent.view",
        "agent.run",
        "agent.debug",
        "agent.edit",
        "agent.publish",
        "agent.delete",
        "agent.grant",
        "agent.import",
        "agent.export",
        "resource.view",
        "resource.upload",
        "resource.update",
        "resource.delete",
        "resource.export",
        "resource.grant",
        "knowledge.view",
        "knowledge.create",
        "knowledge.edit",
        "knowledge.publish",
        "knowledge.delete",
        "knowledge.retrieve",
        "knowledge.grant",
        "tool.catalog.view",
        "tool.catalog.manage",
        "provider-credential.view",
        "provider-credential.manage",
        "tool.builtin.view",
        "tool.builtin.enable",
        "tool.builtin.configure",
        "tool.builtin.delete",
        "tool.builtin.invoke",
        "tool.builtin.grant",
        "tool.skill.view",
        "tool.skill.enable",
        "tool.skill.configure",
        "tool.skill.publish",
        "tool.skill.delete",
        "tool.skill.invoke",
        "tool.skill.grant",
        "tool.mcp.view",
        "tool.mcp.enable",
        "tool.mcp.configure",
        "tool.mcp.delete",
        "tool.mcp.invoke",
        "tool.mcp.bind-credential",
        "tool.mcp.publish",
        "tool.mcp.grant",
        "automation.view",
        "automation.manage",
        "pet.view",
        "pet.manage",
        "artifact.view",
        "inbox.view",
        "runtime.config.workspace.read",
        "runtime.config.workspace.manage",
        "runtime.config.project.read",
        "runtime.config.project.manage",
        "runtime.config.user.read",
        "runtime.config.user.manage",
        "runtime.session.read",
        "runtime.approval.resolve",
        "audit.read",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn map_access_user_record_from_parts(
    _connection: &Connection,
    user: &StoredUser,
) -> Result<AccessUserRecord, AppError> {
    Ok(AccessUserRecord {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        status: user.record.status.clone(),
        password_state: user.record.password_state.clone(),
    })
}

fn default_user_password_state(
    password: Option<&str>,
    confirm_password: Option<&str>,
    reset_password: bool,
) -> Result<(String, String), AppError> {
    if reset_password || password.is_none() {
        return Ok((hash_password("changeme"), "reset-required".into()));
    }
    let password = password.unwrap_or_default();
    if password.len() < 8 {
        return Err(AppError::invalid_input(
            "password must be at least 8 characters",
        ));
    }
    if password != confirm_password.unwrap_or_default() {
        return Err(AppError::invalid_input(
            "password confirmation does not match",
        ));
    }
    Ok((hash_password(password), "set".into()))
}

fn validate_username_unique(
    users: &[StoredUser],
    username: &str,
    exclude_user_id: Option<&str>,
) -> Result<(), AppError> {
    let normalized = username.trim();
    if normalized.is_empty() {
        return Err(AppError::invalid_input("username is required"));
    }
    let exists = users.iter().any(|user| {
        if let Some(excluded_id) = exclude_user_id {
            if user.record.id == excluded_id {
                return false;
            }
        }
        user.record.username == normalized
    });
    if exists {
        return Err(AppError::conflict("username already exists"));
    }
    Ok(())
}

#[async_trait]
impl AccessControlService for InfraAccessControlService {
    async fn list_users(&self) -> Result<Vec<AccessUserRecord>, AppError> {
        let connection = self.state.open_db()?;
        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        users
            .iter()
            .map(|user| map_access_user_record_from_parts(&connection, user))
            .collect()
    }

    async fn create_user(
        &self,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        validate_username_unique(&users, &request.username, None)?;
        let user_id = format!("user-{}", Uuid::new_v4());
        let now = timestamp_now();
        let (password_hash, password_state) = default_user_password_state(
            request.password.as_deref(),
            request.confirm_password.as_deref(),
            false,
        )?;
        let stored_user = StoredUser {
            record: UserRecord {
                id: user_id.clone(),
                username: request.username.trim().into(),
                display_name: request.display_name.trim().into(),
                avatar_path: None,
                avatar_content_type: None,
                avatar_byte_size: None,
                avatar_content_hash: None,
                status: request.status.clone(),
                password_state: password_state.clone(),
                created_at: now,
                updated_at: now,
            },
            password_hash: password_hash.clone(),
        };

        let connection = self.state.open_db()?;
        connection.execute(
            "INSERT INTO users (id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, ?4, ?5, ?6, ?7, ?8)",
            params![
                stored_user.record.id,
                stored_user.record.username,
                stored_user.record.display_name,
                stored_user.record.status,
                password_hash,
                password_state,
                now as i64,
                now as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        users.push(stored_user);
        let created = users
            .iter()
            .find(|user| user.record.id == user_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("created user"))?;
        map_access_user_record_from_parts(&connection, &created)
    }

    async fn update_user(
        &self,
        user_id: &str,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        validate_username_unique(&users, &request.username, Some(user_id))?;
        let user = users
            .iter_mut()
            .find(|user| user.record.id == user_id)
            .ok_or_else(|| AppError::not_found("access user"))?;

        let (password_hash, password_state) = if request.reset_password.unwrap_or(false)
            || request.password.is_some()
            || request.confirm_password.is_some()
        {
            default_user_password_state(
                request.password.as_deref(),
                request.confirm_password.as_deref(),
                request.reset_password.unwrap_or(false),
            )?
        } else {
            (
                user.password_hash.clone(),
                user.record.password_state.clone(),
            )
        };

        let now = timestamp_now();
        let connection = self.state.open_db()?;
        connection.execute(
            "UPDATE users
             SET username = ?2, display_name = ?3, status = ?4, password_hash = ?5, password_state = ?6, updated_at = ?7
             WHERE id = ?1",
            params![
                user_id,
                request.username.trim(),
                request.display_name.trim(),
                request.status,
                password_hash,
                password_state,
                now as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        user.record.username = request.username.trim().into();
        user.record.display_name = request.display_name.trim().into();
        user.record.status = request.status;
        user.record.password_state = password_state;
        user.record.updated_at = now;
        user.password_hash = password_hash;

        map_access_user_record_from_parts(&connection, user)
    }

    async fn delete_user(&self, user_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM users WHERE id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        let connection = self.state.open_db()?;
        connection
            .execute(
                "DELETE FROM user_org_assignments WHERE user_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM role_bindings WHERE subject_type = 'user' AND subject_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM data_policies WHERE subject_type = 'user' AND subject_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .retain(|user| user.record.id != user_id);
        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .retain(|session| session.user_id != user_id);
        Ok(())
    }

    async fn list_org_units(&self) -> Result<Vec<OrgUnitRecord>, AppError> {
        load_org_units(&self.state.open_db()?)
    }

    async fn create_org_unit(
        &self,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        let record = OrgUnitRecord {
            id: format!("org-{}", Uuid::new_v4()),
            parent_id: request.parent_id,
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state.open_db()?.execute(
            "INSERT INTO org_units (id, parent_id, code, name, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.parent_id, record.code, record.name, record.status],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_org_unit(
        &self,
        org_unit_id: &str,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE org_units SET parent_id = ?2, code = ?3, name = ?4, status = ?5 WHERE id = ?1",
            params![org_unit_id, request.parent_id, request.code, request.name, request.status],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(OrgUnitRecord {
            id: org_unit_id.into(),
            parent_id: request.parent_id,
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    async fn delete_org_unit(&self, org_unit_id: &str) -> Result<(), AppError> {
        if org_unit_id == "org-root" {
            return Err(AppError::invalid_input("org-root cannot be deleted"));
        }
        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM org_units WHERE id = ?1", params![org_unit_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM user_org_assignments WHERE org_unit_id = ?1",
                params![org_unit_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_positions(&self) -> Result<Vec<PositionRecord>, AppError> {
        load_positions(&self.state.open_db()?)
    }

    async fn create_position(
        &self,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        let record = PositionRecord {
            id: format!("position-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO positions (id, code, name, status) VALUES (?1, ?2, ?3, ?4)",
                params![record.id, record.code, record.name, record.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_position(
        &self,
        position_id: &str,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE positions SET code = ?2, name = ?3, status = ?4 WHERE id = ?1",
                params![position_id, request.code, request.name, request.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(PositionRecord {
            id: position_id.into(),
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    async fn delete_position(&self, position_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM positions WHERE id = ?1", params![position_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_user_groups(&self) -> Result<Vec<UserGroupRecord>, AppError> {
        load_user_groups(&self.state.open_db()?)
    }

    async fn create_user_group(
        &self,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        let record = UserGroupRecord {
            id: format!("group-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO user_groups (id, code, name, status) VALUES (?1, ?2, ?3, ?4)",
                params![record.id, record.code, record.name, record.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_user_group(
        &self,
        group_id: &str,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE user_groups SET code = ?2, name = ?3, status = ?4 WHERE id = ?1",
                params![group_id, request.code, request.name, request.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(UserGroupRecord {
            id: group_id.into(),
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    async fn delete_user_group(&self, group_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM user_groups WHERE id = ?1", params![group_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_user_org_assignments(&self) -> Result<Vec<UserOrgAssignmentRecord>, AppError> {
        load_user_org_assignments(&self.state.open_db()?)
    }

    async fn upsert_user_org_assignment(
        &self,
        request: UserOrgAssignmentUpsertRequest,
    ) -> Result<UserOrgAssignmentRecord, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO user_org_assignments (user_id, org_unit_id, is_primary, position_ids, user_group_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.user_id,
                request.org_unit_id,
                bool_to_sql(request.is_primary),
                serde_json::to_string(&request.position_ids)?,
                serde_json::to_string(&request.user_group_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(UserOrgAssignmentRecord {
            user_id: request.user_id,
            org_unit_id: request.org_unit_id,
            is_primary: request.is_primary,
            position_ids: request.position_ids,
            user_group_ids: request.user_group_ids,
        })
    }

    async fn delete_user_org_assignment(
        &self,
        user_id: &str,
        org_unit_id: &str,
    ) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM user_org_assignments WHERE user_id = ?1 AND org_unit_id = ?2",
                params![user_id, org_unit_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_roles(&self) -> Result<Vec<AccessRoleRecord>, AppError> {
        load_access_roles(&self.state.open_db()?)
    }

    async fn create_role(&self, request: RoleUpsertRequest) -> Result<AccessRoleRecord, AppError> {
        let record = AccessRoleRecord {
            id: format!("role-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            permission_codes: request.permission_codes,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO access_roles (id, code, name, description, status, permission_codes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.id,
                    record.code,
                    record.name,
                    record.description,
                    record.status,
                    serde_json::to_string(&record.permission_codes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_role(
        &self,
        role_id: &str,
        request: RoleUpsertRequest,
    ) -> Result<AccessRoleRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE access_roles SET code = ?2, name = ?3, description = ?4, status = ?5, permission_codes = ?6 WHERE id = ?1",
            params![
                role_id,
                request.code,
                request.name,
                request.description,
                request.status,
                serde_json::to_string(&request.permission_codes)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(AccessRoleRecord {
            id: role_id.into(),
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            permission_codes: request.permission_codes,
        })
    }

    async fn delete_role(&self, role_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM access_roles WHERE id = ?1", params![role_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .open_db()?
            .execute(
                "DELETE FROM role_bindings WHERE role_id = ?1",
                params![role_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_role_bindings(&self) -> Result<Vec<RoleBindingRecord>, AppError> {
        load_role_bindings(&self.state.open_db()?)
    }

    async fn create_role_binding(
        &self,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        let record = RoleBindingRecord {
            id: format!("binding-{}", Uuid::new_v4()),
            role_id: request.role_id,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO role_bindings (id, role_id, subject_type, subject_id, effect) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.role_id, record.subject_type, record.subject_id, record.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_role_binding(
        &self,
        binding_id: &str,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE role_bindings SET role_id = ?2, subject_type = ?3, subject_id = ?4, effect = ?5 WHERE id = ?1",
            params![binding_id, request.role_id, request.subject_type, request.subject_id, request.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(RoleBindingRecord {
            id: binding_id.into(),
            role_id: request.role_id,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            effect: request.effect,
        })
    }

    async fn delete_role_binding(&self, binding_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM role_bindings WHERE id = ?1",
                params![binding_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_data_policies(&self) -> Result<Vec<DataPolicyRecord>, AppError> {
        load_data_policies(&self.state.open_db()?)
    }

    async fn create_data_policy(
        &self,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        let record = DataPolicyRecord {
            id: format!("data-policy-{}", Uuid::new_v4()),
            name: request.name,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            scope_type: request.scope_type,
            project_ids: request.project_ids,
            tags: request.tags,
            classifications: request.classifications,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO data_policies (id, name, subject_type, subject_id, resource_type, scope_type, project_ids, tags, classifications, effect)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.id,
                record.name,
                record.subject_type,
                record.subject_id,
                record.resource_type,
                record.scope_type,
                serde_json::to_string(&record.project_ids)?,
                serde_json::to_string(&record.tags)?,
                serde_json::to_string(&record.classifications)?,
                record.effect,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_data_policy(
        &self,
        policy_id: &str,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE data_policies
             SET name = ?2, subject_type = ?3, subject_id = ?4, resource_type = ?5, scope_type = ?6, project_ids = ?7, tags = ?8, classifications = ?9, effect = ?10
             WHERE id = ?1",
            params![
                policy_id,
                request.name,
                request.subject_type,
                request.subject_id,
                request.resource_type,
                request.scope_type,
                serde_json::to_string(&request.project_ids)?,
                serde_json::to_string(&request.tags)?,
                serde_json::to_string(&request.classifications)?,
                request.effect,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(DataPolicyRecord {
            id: policy_id.into(),
            name: request.name,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            scope_type: request.scope_type,
            project_ids: request.project_ids,
            tags: request.tags,
            classifications: request.classifications,
            effect: request.effect,
        })
    }

    async fn delete_data_policy(&self, policy_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM data_policies WHERE id = ?1",
                params![policy_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_resource_policies(&self) -> Result<Vec<ResourcePolicyRecord>, AppError> {
        load_resource_policies(&self.state.open_db()?)
    }

    async fn create_resource_policy(
        &self,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        let record = ResourcePolicyRecord {
            id: format!("resource-policy-{}", Uuid::new_v4()),
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            resource_id: request.resource_id,
            action: request.action,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO resource_policies (id, subject_type, subject_id, resource_type, resource_id, action_name, effect)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![record.id, record.subject_type, record.subject_id, record.resource_type, record.resource_id, record.action, record.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    async fn update_resource_policy(
        &self,
        policy_id: &str,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE resource_policies
             SET subject_type = ?2, subject_id = ?3, resource_type = ?4, resource_id = ?5, action_name = ?6, effect = ?7
             WHERE id = ?1",
            params![policy_id, request.subject_type, request.subject_id, request.resource_type, request.resource_id, request.action, request.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(ResourcePolicyRecord {
            id: policy_id.into(),
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            resource_id: request.resource_id,
            action: request.action,
            effect: request.effect,
        })
    }

    async fn delete_resource_policy(&self, policy_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM resource_policies WHERE id = ?1",
                params![policy_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_menu_policies(&self) -> Result<Vec<MenuPolicyRecord>, AppError> {
        load_menu_policies(&self.state.open_db()?)
    }

    async fn upsert_menu_policy(
        &self,
        menu_id: &str,
        request: MenuPolicyUpsertRequest,
    ) -> Result<MenuPolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO menu_policies (menu_id, enabled, order_value, group_key, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![menu_id, bool_to_sql(request.enabled), request.order, request.group, request.visibility],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(MenuPolicyRecord {
            menu_id: menu_id.into(),
            enabled: request.enabled,
            order: request.order,
            group: request.group,
            visibility: request.visibility,
        })
    }

    async fn delete_menu_policy(&self, menu_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM menu_policies WHERE menu_id = ?1",
                params![menu_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    async fn list_protected_resources(&self) -> Result<Vec<ProtectedResourceDescriptor>, AppError> {
        load_protected_resource_metadata(&self.state.open_db()?)
    }

    async fn upsert_protected_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        request: ProtectedResourceMetadataUpsertRequest,
    ) -> Result<ProtectedResourceDescriptor, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO protected_resources
             (resource_type, resource_id, resource_subtype, project_id, tags, classification, owner_subject_type, owner_subject_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                resource_type,
                resource_id,
                request.resource_subtype,
                request.project_id,
                serde_json::to_string(&request.tags)?,
                request.classification,
                request.owner_subject_type,
                request.owner_subject_id,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(ProtectedResourceDescriptor {
            id: resource_id.into(),
            resource_type: resource_type.into(),
            resource_subtype: request.resource_subtype,
            name: resource_id.into(),
            project_id: request.project_id,
            tags: request.tags,
            classification: request.classification,
            owner_subject_type: request.owner_subject_type,
            owner_subject_id: request.owner_subject_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::default_owner_permission_codes;

    #[test]
    fn owner_permission_catalog_includes_enterprise_access_matrix() {
        let permissions = default_owner_permission_codes();

        for code in [
            "agent.debug",
            "agent.publish",
            "agent.grant",
            "resource.export",
            "resource.grant",
            "knowledge.create",
            "knowledge.edit",
            "knowledge.publish",
            "knowledge.delete",
            "knowledge.grant",
            "tool.builtin.enable",
            "tool.builtin.grant",
            "tool.skill.enable",
            "tool.skill.publish",
            "tool.skill.grant",
            "tool.mcp.enable",
            "tool.mcp.bind-credential",
            "tool.mcp.publish",
            "tool.mcp.grant",
        ] {
            assert!(
                permissions.iter().any(|permission| permission == code),
                "missing owner permission code: {code}"
            );
        }
    }
}
