use super::*;
use octopus_core::{
    AccessExperienceCounts, AccessExperienceSnapshot, AccessMemberSummary, AccessRoleRecord,
    AccessUserPresetUpdateRequest, AccessUserRecord, AccessUserUpsertRequest, DataPolicyRecord,
    DataPolicyUpsertRequest, MenuPolicyRecord, MenuPolicyUpsertRequest, OrgUnitRecord,
    OrgUnitUpsertRequest, PositionRecord, PositionUpsertRequest, ProtectedResourceDescriptor,
    ProtectedResourceMetadataUpsertRequest, ResourcePolicyRecord, ResourcePolicyUpsertRequest,
    RoleBindingRecord, RoleBindingUpsertRequest, RoleUpsertRequest, UserGroupRecord,
    UserGroupUpsertRequest, UserOrgAssignmentRecord, UserOrgAssignmentUpsertRequest,
};
use std::collections::{BTreeSet, HashMap};

pub(crate) const SYSTEM_ROLE_NAMESPACE_PREFIX: &str = "system.";
pub(crate) const SYSTEM_OWNER_ROLE_ID: &str = "system.owner";
pub(crate) const SYSTEM_ADMIN_ROLE_ID: &str = "system.admin";
pub(crate) const SYSTEM_MEMBER_ROLE_ID: &str = "system.member";
pub(crate) const SYSTEM_VIEWER_ROLE_ID: &str = "system.viewer";
pub(crate) const SYSTEM_AUDITOR_ROLE_ID: &str = "system.auditor";
const LEGACY_OWNER_ROLE_ID: &str = "owner";
const ROOT_ORG_UNIT_ID: &str = "org-root";
const CUSTOM_ACCESS_CODE: &str = "custom";
const CUSTOM_ACCESS_NAME: &str = "Custom access";
const MIXED_ACCESS_CODE: &str = "mixed";
const MIXED_ACCESS_NAME: &str = "Mixed access";
const NO_PRESET_ASSIGNED_NAME: &str = "No preset assigned";

#[derive(Debug, Clone)]
struct SystemRoleDefinition {
    code: &'static str,
    name: &'static str,
    description: &'static str,
    permission_codes: Vec<String>,
}

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

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn merge_permission_codes(
    existing_codes: impl IntoIterator<Item = String>,
    required_codes: impl IntoIterator<Item = String>,
) -> Vec<String> {
    existing_codes
        .into_iter()
        .chain(required_codes)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn is_system_role_code(code: &str) -> bool {
    code.starts_with(SYSTEM_ROLE_NAMESPACE_PREFIX)
}

pub(crate) fn access_role_source(code: &str) -> String {
    if is_system_role_code(code) {
        "system".into()
    } else {
        "custom".into()
    }
}

pub(crate) fn access_role_editable(code: &str) -> bool {
    !is_system_role_code(code)
}

fn system_role_definitions() -> Vec<SystemRoleDefinition> {
    vec![
        SystemRoleDefinition {
            code: SYSTEM_OWNER_ROLE_ID,
            name: "Owner",
            description: "Full workspace ownership across members, governance, and operations.",
            permission_codes: default_owner_permission_codes(),
        },
        SystemRoleDefinition {
            code: SYSTEM_ADMIN_ROLE_ID,
            name: "Admin",
            description: "Manage members, presets, governance configuration, and workspace operations.",
            permission_codes: default_admin_permission_codes(),
        },
        SystemRoleDefinition {
            code: SYSTEM_MEMBER_ROLE_ID,
            name: "Member",
            description: "Participate in day-to-day workspace projects, resources, and runtime work.",
            permission_codes: default_member_permission_codes(),
        },
        SystemRoleDefinition {
            code: SYSTEM_VIEWER_ROLE_ID,
            name: "Viewer",
            description: "Read workspace activity, project context, and published resources.",
            permission_codes: default_viewer_permission_codes(),
        },
        SystemRoleDefinition {
            code: SYSTEM_AUDITOR_ROLE_ID,
            name: "Auditor",
            description: "Inspect members, policies, sessions, and audit records without editing them.",
            permission_codes: default_auditor_permission_codes(),
        },
    ]
}

fn preset_code_to_system_role_code(preset_code: &str) -> Option<&'static str> {
    match preset_code.trim() {
        "owner" => Some(SYSTEM_OWNER_ROLE_ID),
        "admin" => Some(SYSTEM_ADMIN_ROLE_ID),
        "member" => Some(SYSTEM_MEMBER_ROLE_ID),
        "viewer" => Some(SYSTEM_VIEWER_ROLE_ID),
        "auditor" => Some(SYSTEM_AUDITOR_ROLE_ID),
        _ => None,
    }
}

fn system_role_code_to_preset_code(role_code: &str) -> Option<&'static str> {
    match role_code {
        SYSTEM_OWNER_ROLE_ID => Some("owner"),
        SYSTEM_ADMIN_ROLE_ID => Some("admin"),
        SYSTEM_MEMBER_ROLE_ID => Some("member"),
        SYSTEM_VIEWER_ROLE_ID => Some("viewer"),
        SYSTEM_AUDITOR_ROLE_ID => Some("auditor"),
        _ => None,
    }
}

fn preset_display_name(code: &str) -> &'static str {
    match code {
        "owner" => "Owner",
        "admin" => "Admin",
        "member" => "Member",
        "viewer" => "Viewer",
        "auditor" => "Auditor",
        CUSTOM_ACCESS_CODE => CUSTOM_ACCESS_NAME,
        MIXED_ACCESS_CODE => MIXED_ACCESS_NAME,
        _ => NO_PRESET_ASSIGNED_NAME,
    }
}

fn ensure_system_role(
    connection: &Connection,
    definition: &SystemRoleDefinition,
) -> Result<(), AppError> {
    let matching_roles = {
        let mut stmt = connection
            .prepare(
                "SELECT id, permission_codes
                 FROM access_roles
                 WHERE id = ?1 OR code = ?1
                 ORDER BY CASE WHEN id = ?1 THEN 0 ELSE 1 END, id ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = stmt
            .query_map(params![definition.code], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| AppError::database(error.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))?
    };

    let merged_permissions = merge_permission_codes(
        matching_roles
            .iter()
            .flat_map(|(_, permission_codes)| read_json_vec(permission_codes)),
        definition.permission_codes.clone(),
    );

    if matching_roles.is_empty() {
        connection
            .execute(
                "INSERT INTO access_roles (id, code, name, description, status, permission_codes)
                 VALUES (?1, ?1, ?2, ?3, 'active', ?4)",
                params![
                    definition.code,
                    definition.name,
                    definition.description,
                    serde_json::to_string(&merged_permissions)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        return Ok(());
    }

    let canonical_id = matching_roles
        .iter()
        .find(|(role_id, _)| role_id == definition.code)
        .map(|(role_id, _)| role_id.clone())
        .unwrap_or_else(|| matching_roles[0].0.clone());

    if canonical_id != definition.code {
        connection
            .execute(
                "UPDATE role_bindings SET role_id = ?2 WHERE role_id = ?1",
                params![canonical_id, definition.code],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "UPDATE access_roles
                 SET id = ?2, code = ?2, name = ?3, description = ?4, status = 'active', permission_codes = ?5
                 WHERE id = ?1",
                params![
                    canonical_id,
                    definition.code,
                    definition.name,
                    definition.description,
                    serde_json::to_string(&merged_permissions)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    } else {
        connection
            .execute(
                "UPDATE access_roles
                 SET code = ?2, name = ?3, description = ?4, status = 'active', permission_codes = ?5
                 WHERE id = ?1",
                params![
                    definition.code,
                    definition.code,
                    definition.name,
                    definition.description,
                    serde_json::to_string(&merged_permissions)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    for (role_id, _) in matching_roles {
        if role_id == canonical_id {
            continue;
        }
        connection
            .execute(
                "UPDATE role_bindings SET role_id = ?2 WHERE role_id = ?1",
                params![role_id, definition.code],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute("DELETE FROM access_roles WHERE id = ?1", params![role_id])
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

fn migrate_legacy_owner_role(connection: &Connection) -> Result<(), AppError> {
    let legacy_owner = connection
        .query_row(
            "SELECT id, permission_codes
             FROM access_roles
             WHERE id = ?1 OR code = ?1
             ORDER BY CASE WHEN id = ?1 THEN 0 ELSE 1 END, id ASC
             LIMIT 1",
            params![LEGACY_OWNER_ROLE_ID],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    let system_owner = connection
        .query_row(
            "SELECT permission_codes FROM access_roles WHERE id = ?1 LIMIT 1",
            params![SYSTEM_OWNER_ROLE_ID],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "UPDATE role_bindings SET role_id = ?2 WHERE role_id = ?1",
            params![LEGACY_OWNER_ROLE_ID, SYSTEM_OWNER_ROLE_ID],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    match (legacy_owner, system_owner) {
        (Some((legacy_role_id, legacy_permissions_raw)), Some(system_permissions_raw)) => {
            let merged_permissions = merge_permission_codes(
                read_json_vec(&system_permissions_raw),
                read_json_vec(&legacy_permissions_raw),
            );
            connection
                .execute(
                    "UPDATE access_roles
                     SET code = ?2, name = 'Owner', description = ?3, status = 'active', permission_codes = ?4
                     WHERE id = ?1",
                    params![
                        SYSTEM_OWNER_ROLE_ID,
                        SYSTEM_OWNER_ROLE_ID,
                        "Full workspace ownership across members, governance, and operations.",
                        serde_json::to_string(&merged_permissions)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
            if legacy_role_id != SYSTEM_OWNER_ROLE_ID {
                connection
                    .execute("DELETE FROM access_roles WHERE id = ?1", params![legacy_role_id])
                    .map_err(|error| AppError::database(error.to_string()))?;
            }
        }
        (Some((legacy_role_id, legacy_permissions_raw)), None) => {
            let merged_permissions =
                merge_permission_codes(read_json_vec(&legacy_permissions_raw), Vec::new());
            connection
                .execute(
                    "UPDATE access_roles
                     SET id = ?2, code = ?2, name = 'Owner', description = ?3, status = 'active', permission_codes = ?4
                     WHERE id = ?1",
                    params![
                        legacy_role_id,
                        SYSTEM_OWNER_ROLE_ID,
                        "Full workspace ownership across members, governance, and operations.",
                        serde_json::to_string(&merged_permissions)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
        _ => {}
    }

    Ok(())
}

pub(crate) fn ensure_system_roles(connection: &Connection) -> Result<(), AppError> {
    migrate_legacy_owner_role(connection)?;
    for definition in system_role_definitions() {
        ensure_system_role(connection, &definition)?;
    }
    Ok(())
}

fn reject_reserved_system_role_code(code: &str) -> Result<(), AppError> {
    if is_system_role_code(code) {
        return Err(AppError::invalid_input(
            "system.* role codes are reserved for platform-managed roles",
        ));
    }
    Ok(())
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

pub(super) fn ensure_default_owner_role_permissions(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_system_roles(connection)
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
        "resource.publish",
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
        "runtime.submit_turn",
        "runtime.approval.resolve",
        "audit.read",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn default_admin_permission_codes() -> Vec<String> {
    string_vec(&[
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
        "agent.import",
        "agent.export",
        "resource.view",
        "resource.upload",
        "resource.update",
        "resource.delete",
        "resource.publish",
        "resource.export",
        "knowledge.view",
        "knowledge.create",
        "knowledge.edit",
        "knowledge.publish",
        "knowledge.retrieve",
        "tool.builtin.enable",
        "tool.skill.enable",
        "tool.mcp.enable",
        "runtime.session.read",
        "runtime.submit_turn",
        "audit.read",
    ])
}

pub(crate) fn default_member_permission_codes() -> Vec<String> {
    string_vec(&[
        "workspace.overview.read",
        "project.view",
        "team.view",
        "agent.view",
        "agent.run",
        "resource.view",
        "resource.upload",
        "knowledge.view",
        "knowledge.retrieve",
        "tool.builtin.enable",
        "tool.skill.enable",
        "tool.mcp.enable",
        "runtime.session.read",
        "runtime.submit_turn",
    ])
}

pub(crate) fn default_viewer_permission_codes() -> Vec<String> {
    string_vec(&[
        "workspace.overview.read",
        "project.view",
        "team.view",
        "agent.view",
        "resource.view",
        "knowledge.view",
        "runtime.session.read",
    ])
}

pub(crate) fn default_auditor_permission_codes() -> Vec<String> {
    string_vec(&[
        "workspace.overview.read",
        "access.users.read",
        "access.org.read",
        "access.roles.read",
        "access.policies.read",
        "access.menus.read",
        "access.sessions.read",
        "resource.view",
        "knowledge.view",
        "audit.read",
    ])
}

fn query_count(connection: &Connection, sql: &str) -> Result<u32, AppError> {
    let count: i64 = connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(count.max(0) as u32)
}

fn query_exists(connection: &Connection, sql: &str) -> Result<bool, AppError> {
    let exists: i64 = connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(exists != 0)
}

fn has_meaningful_org_assignment(assignment: &UserOrgAssignmentRecord) -> bool {
    assignment.org_unit_id != ROOT_ORG_UNIT_ID
        || !assignment.position_ids.is_empty()
        || !assignment.user_group_ids.is_empty()
}

fn resolve_effective_bindings_for_user(
    org_units: &[OrgUnitRecord],
    assignments: &[UserOrgAssignmentRecord],
    bindings: &[RoleBindingRecord],
    user_id: &str,
) -> Vec<RoleBindingRecord> {
    let assignments = assignments_for_user(assignments, user_id);
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
        .flat_map(|assignment| org_unit_ancestor_ids(org_units, &assignment.org_unit_id))
        .collect::<BTreeSet<_>>();

    bindings
        .iter()
        .filter(|binding| match normalize_subject_type(&binding.subject_type) {
            "user" => binding.subject_id == user_id,
            "org-unit" => org_unit_ids.contains(&binding.subject_id),
            "position" => position_ids.contains(&binding.subject_id),
            "user-group" => user_group_ids.contains(&binding.subject_id),
            _ => false,
        })
        .cloned()
        .collect()
}

fn effective_role_ids_from_bindings(bindings: &[RoleBindingRecord]) -> BTreeSet<String> {
    let deny_role_ids = bindings
        .iter()
        .filter(|binding| binding.effect == "deny")
        .map(|binding| binding.role_id.clone())
        .collect::<BTreeSet<_>>();

    bindings
        .iter()
        .filter(|binding| binding.effect == "allow" && !deny_role_ids.contains(&binding.role_id))
        .map(|binding| binding.role_id.clone())
        .collect()
}

fn build_access_experience_snapshot(connection: &Connection) -> Result<AccessExperienceSnapshot, AppError> {
    let member_count = query_count(connection, "SELECT COUNT(*) FROM users")?;
    let custom_role_count = query_count(
        connection,
        "SELECT COUNT(*) FROM access_roles WHERE code NOT LIKE 'system.%'",
    )?;
    let org_unit_count = query_count(
        connection,
        "SELECT COUNT(*) FROM org_units WHERE id != 'org-root'",
    )?;
    let position_count = query_count(connection, "SELECT COUNT(*) FROM positions")?;
    let user_group_count = query_count(connection, "SELECT COUNT(*) FROM user_groups")?;
    let data_policy_count = query_count(connection, "SELECT COUNT(*) FROM data_policies")?;
    let resource_policy_count = query_count(connection, "SELECT COUNT(*) FROM resource_policies")?;
    let menu_policy_count = query_count(connection, "SELECT COUNT(*) FROM menu_policies")?;
    let protected_resource_count = query_count(connection, "SELECT COUNT(*) FROM protected_resources")?;
    let session_count = query_count(connection, "SELECT COUNT(*) FROM sessions")?;
    let audit_event_count = query_count(connection, "SELECT COUNT(*) FROM audit_records")?;
    let has_meaningful_org_assignments = query_exists(
        connection,
        "SELECT EXISTS(
            SELECT 1
            FROM user_org_assignments
            WHERE org_unit_id != 'org-root'
               OR position_ids != '[]'
               OR user_group_ids != '[]'
        )",
    )?;
    let has_advanced_policies = query_exists(
        connection,
        "SELECT EXISTS(
            SELECT 1
            FROM data_policies
            WHERE resource_type != 'project'
               OR scope_type != 'selected-projects'
               OR effect != 'allow'
        )",
    )?;

    let has_org_structure =
        org_unit_count > 0 || position_count > 0 || user_group_count > 0 || has_meaningful_org_assignments;
    let has_custom_roles = custom_role_count > 0;
    let has_menu_governance = menu_policy_count > 0;
    let has_resource_governance = resource_policy_count > 0 || protected_resource_count > 0;
    let experience_level = if has_org_structure
        || has_custom_roles
        || has_advanced_policies
        || has_menu_governance
        || has_resource_governance
    {
        "enterprise"
    } else if member_count > 1 {
        "team"
    } else {
        "personal"
    };

    Ok(AccessExperienceSnapshot {
        experience_level: experience_level.into(),
        member_count,
        has_org_structure,
        has_custom_roles,
        has_advanced_policies,
        has_menu_governance,
        has_resource_governance,
        counts: AccessExperienceCounts {
            custom_role_count,
            org_unit_count,
            data_policy_count,
            resource_policy_count,
            menu_policy_count,
            protected_resource_count,
            session_count,
            audit_event_count,
        },
    })
}

fn build_access_member_summary(
    connection: &Connection,
    user: &StoredUser,
    roles_by_id: &HashMap<String, AccessRoleRecord>,
    org_units: &[OrgUnitRecord],
    assignments: &[UserOrgAssignmentRecord],
    bindings: &[RoleBindingRecord],
) -> Result<AccessMemberSummary, AppError> {
    let access_user = map_access_user_record_from_parts(connection, user)?;
    let user_assignments = assignments_for_user(assignments, &user.record.id);
    let direct_user_bindings = bindings
        .iter()
        .filter(|binding| {
            normalize_subject_type(&binding.subject_type) == "user" && binding.subject_id == user.record.id
        })
        .cloned()
        .collect::<Vec<_>>();
    let effective_bindings =
        resolve_effective_bindings_for_user(org_units, assignments, bindings, &user.record.id);
    let effective_role_ids = effective_role_ids_from_bindings(&effective_bindings);

    let effective_role_names = effective_role_ids
        .iter()
        .filter_map(|role_id| roles_by_id.get(role_id))
        .map(|role| role.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let direct_system_role_ids = direct_user_bindings
        .iter()
        .filter(|binding| binding.effect == "allow")
        .filter_map(|binding| {
            roles_by_id
                .get(&binding.role_id)
                .and_then(|role| system_role_code_to_preset_code(&role.code))
                .or_else(|| system_role_code_to_preset_code(&binding.role_id))
                .map(|_| binding.role_id.clone())
        })
        .collect::<BTreeSet<_>>();
    let direct_preset_codes = direct_user_bindings
        .iter()
        .filter(|binding| binding.effect == "allow")
        .filter_map(|binding| {
            roles_by_id
                .get(&binding.role_id)
                .and_then(|role| system_role_code_to_preset_code(&role.code))
                .or_else(|| system_role_code_to_preset_code(&binding.role_id))
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    let (primary_preset_code, primary_preset_name) = if direct_preset_codes.len() == 1
        && direct_system_role_ids.len() == 1
        && effective_role_ids.len() == 1
        && effective_role_ids
            .iter()
            .next()
            .is_some_and(|role_id| direct_system_role_ids.contains(role_id))
    {
        let code = direct_preset_codes
            .iter()
            .next()
            .cloned()
            .unwrap_or_default();
        (Some(code.clone()), preset_display_name(&code).to_string())
    } else if !effective_role_ids.is_empty() {
        if !direct_preset_codes.is_empty() {
            (
                Some(MIXED_ACCESS_CODE.into()),
                MIXED_ACCESS_NAME.to_string(),
            )
        } else {
            (
                Some(CUSTOM_ACCESS_CODE.into()),
                CUSTOM_ACCESS_NAME.to_string(),
            )
        }
    } else {
        (None, NO_PRESET_ASSIGNED_NAME.to_string())
    };

    Ok(AccessMemberSummary {
        user: access_user,
        primary_preset_code,
        primary_preset_name,
        effective_role_names,
        has_org_assignments: user_assignments.iter().any(has_meaningful_org_assignment),
    })
}

fn build_access_member_summaries(
    connection: &Connection,
    users: &[StoredUser],
) -> Result<Vec<AccessMemberSummary>, AppError> {
    let org_units = load_org_units(connection)?;
    let assignments = load_user_org_assignments(connection)?;
    let bindings = load_role_bindings(connection)?;
    let roles_by_id = load_access_roles(connection)?
        .into_iter()
        .map(|role| (role.id.clone(), role))
        .collect::<HashMap<_, _>>();

    let mut summaries = users
        .iter()
        .map(|user| {
            build_access_member_summary(connection, user, &roles_by_id, &org_units, &assignments, &bindings)
        })
        .collect::<Result<Vec<_>, _>>()?;
    summaries.sort_by(|left, right| {
        left.user
            .display_name
            .cmp(&right.user.display_name)
            .then_with(|| left.user.username.cmp(&right.user.username))
    });
    Ok(summaries)
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

    async fn get_experience_snapshot(&self) -> Result<AccessExperienceSnapshot, AppError> {
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        build_access_experience_snapshot(&connection)
    }

    async fn list_member_summaries(&self) -> Result<Vec<AccessMemberSummary>, AppError> {
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        build_access_member_summaries(&connection, &users)
    }

    async fn assign_user_preset(
        &self,
        user_id: &str,
        request: AccessUserPresetUpdateRequest,
    ) -> Result<AccessMemberSummary, AppError> {
        let role_id = preset_code_to_system_role_code(&request.preset_code).ok_or_else(|| {
            AppError::invalid_input(format!("unknown access preset: {}", request.preset_code))
        })?;

        {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if !users.iter().any(|user| user.record.id == user_id) {
                return Err(AppError::not_found("access user"));
            }
        }

        let mut connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        let tx = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        tx.execute(
            "DELETE FROM role_bindings
             WHERE subject_type = 'user'
               AND subject_id = ?1
               AND role_id LIKE 'system.%'",
            params![user_id],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        tx.execute(
            "INSERT INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, ?2, 'user', ?3, 'allow')",
            params![
                format!("binding-user-{user_id}-{}", role_id.replace('.', "-")),
                role_id,
                user_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        tx.commit()
            .map_err(|error| AppError::database(error.to_string()))?;

        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        let summaries = build_access_member_summaries(&connection, &users)?;
        summaries
            .into_iter()
            .find(|summary| summary.user.id == user_id)
            .ok_or_else(|| AppError::not_found("access user"))
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
        ensure_personal_pet_for_user(&connection, &self.state.workspace_id()?, &user_id)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = load_agents(&connection)?;
        *self
            .state
            .pet_extensions
            .lock()
            .map_err(|_| AppError::runtime("pet extensions mutex poisoned"))? =
            load_pet_agent_extensions(&connection)?;

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
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        load_access_roles(&connection)
    }

    async fn create_role(&self, request: RoleUpsertRequest) -> Result<AccessRoleRecord, AppError> {
        reject_reserved_system_role_code(&request.code)?;
        let record = AccessRoleRecord {
            id: format!("role-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            permission_codes: request.permission_codes,
            source: "custom".into(),
            editable: true,
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
        let connection = self.state.open_db()?;
        let existing_role = load_access_roles(&connection)?
            .into_iter()
            .find(|role| role.id == role_id)
            .ok_or_else(|| AppError::not_found("role"))?;
        if !existing_role.editable {
            return Err(AppError::invalid_input(
                "system roles are managed by platform and cannot be edited",
            ));
        }
        reject_reserved_system_role_code(&request.code)?;
        connection.execute(
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
            source: "custom".into(),
            editable: true,
        })
    }

    async fn delete_role(&self, role_id: &str) -> Result<(), AppError> {
        let connection = self.state.open_db()?;
        let existing_role = load_access_roles(&connection)?
            .into_iter()
            .find(|role| role.id == role_id)
            .ok_or_else(|| AppError::not_found("role"))?;
        if !existing_role.editable {
            return Err(AppError::invalid_input(
                "system roles are managed by platform and cannot be deleted",
            ));
        }
        connection
            .execute("DELETE FROM access_roles WHERE id = ?1", params![role_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
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
    use crate::build_infra_bundle;
    use octopus_core::{
        AccessUserPresetUpdateRequest, AccessUserUpsertRequest, AvatarUploadPayload,
        DataPolicyUpsertRequest, OrgUnitUpsertRequest, RegisterBootstrapAdminRequest,
        ResourcePolicyUpsertRequest, RoleBindingUpsertRequest, RoleUpsertRequest,
        UserOrgAssignmentUpsertRequest,
    };
    use octopus_platform::{AccessControlService, AuthService};

    fn avatar_payload() -> AvatarUploadPayload {
        AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

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
            "runtime.submit_turn",
        ] {
            assert!(
                permissions.iter().any(|permission| permission == code),
                "missing owner permission code: {code}"
            );
        }
    }

    #[test]
    fn access_control_system_roles_are_seeded_with_system_namespace_and_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let roles = runtime
            .block_on(bundle.access_control.list_roles())
            .expect("list roles");

        for (role_code, role_name) in [
            ("system.owner", "Owner"),
            ("system.admin", "Admin"),
            ("system.member", "Member"),
            ("system.viewer", "Viewer"),
            ("system.auditor", "Auditor"),
        ] {
            let role = roles
                .iter()
                .find(|record| record.code == role_code)
                .unwrap_or_else(|| panic!("missing system role {role_code}"));
            assert_eq!(role.id, role_code);
            assert_eq!(role.name, role_name);
            assert_eq!(role.source, "system");
            assert!(!role.editable, "system role should not be editable");
        }

        assert!(
            roles.iter().all(|role| role.code != "owner"),
            "legacy owner role should be migrated away"
        );
    }

    #[test]
    fn access_control_reloads_legacy_owner_role_as_system_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let session = runtime
            .block_on(bundle.auth.register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("bootstrap admin")
            .session;

        let db = bundle.access_control.state.open_db().expect("db");
        db.execute(
            "UPDATE role_bindings SET role_id = 'owner', id = ?1 WHERE role_id = 'system.owner'",
            rusqlite::params![format!("binding-user-{}-owner", session.user_id)],
        )
        .expect("rewrite role binding");
        db.execute(
            "UPDATE access_roles SET id = 'owner', code = 'owner' WHERE id = 'system.owner'",
            [],
        )
        .expect("rewrite owner role");
        drop(db);
        drop(bundle);

        let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
        let roles = runtime
            .block_on(reloaded.access_control.list_roles())
            .expect("list roles");
        let owner_role = roles
            .iter()
            .find(|record| record.id == "system.owner")
            .expect("system owner role");
        assert_eq!(owner_role.code, "system.owner");
        assert_eq!(owner_role.source, "system");

        let reloaded_db = reloaded.access_control.state.open_db().expect("reloaded db");
        let binding_role_id: String = reloaded_db
            .query_row(
                "SELECT role_id FROM role_bindings WHERE subject_type = 'user' AND subject_id = ?1 LIMIT 1",
                rusqlite::params![session.user_id],
                |row| row.get(0),
            )
            .expect("load migrated role binding");
        assert_eq!(binding_role_id, "system.owner");
        let legacy_owner_count: i64 = reloaded_db
            .query_row(
                "SELECT COUNT(*) FROM access_roles WHERE id = 'owner' OR code = 'owner'",
                [],
                |row| row.get(0),
            )
            .expect("count legacy owner role");
        assert_eq!(legacy_owner_count, 0);
    }

    #[test]
    fn access_control_custom_role_flows_reject_reserved_system_namespace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let create_error = runtime
            .block_on(bundle.access_control.create_role(RoleUpsertRequest {
                code: "system.custom".into(),
                name: "System Custom".into(),
                description: "should be rejected".into(),
                status: "active".into(),
                permission_codes: vec!["workspace.overview.read".into()],
            }))
            .expect_err("system.* create should fail");
        assert!(
            create_error.to_string().contains("system."),
            "unexpected create error: {create_error}"
        );

        let custom_role = runtime
            .block_on(bundle.access_control.create_role(RoleUpsertRequest {
                code: "custom.member-helper".into(),
                name: "Member Helper".into(),
                description: "custom role".into(),
                status: "active".into(),
                permission_codes: vec!["access.users.read".into()],
            }))
            .expect("create custom role");

        let update_error = runtime
            .block_on(bundle.access_control.update_role(
                &custom_role.id,
                RoleUpsertRequest {
                    code: "system.viewer".into(),
                    name: "Should Fail".into(),
                    description: "reserved".into(),
                    status: "active".into(),
                    permission_codes: vec!["access.users.read".into()],
                },
            ))
            .expect_err("system.* update should fail");
        assert!(
            update_error.to_string().contains("system."),
            "unexpected update error: {update_error}"
        );
    }

    #[test]
    fn access_control_experience_snapshot_distinguishes_advanced_and_resource_governance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let session = runtime
            .block_on(bundle.auth.register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("bootstrap admin")
            .session;

        runtime
            .block_on(bundle.access_control.create_resource_policy(
                ResourcePolicyUpsertRequest {
                    subject_type: "user".into(),
                    subject_id: session.user_id.clone(),
                    resource_type: "resource".into(),
                    resource_id: "res-confidential".into(),
                    action: "view".into(),
                    effect: "allow".into(),
                },
            ))
            .expect("create resource policy");

        let snapshot = runtime
            .block_on(bundle.access_control.get_experience_snapshot())
            .expect("experience snapshot");
        assert!(
            !snapshot.has_advanced_policies,
            "resource governance alone should not trigger advanced policies"
        );
        assert!(
            snapshot.has_resource_governance,
            "resource policy should trigger resource governance"
        );

        runtime
            .block_on(bundle.access_control.create_data_policy(
                DataPolicyUpsertRequest {
                    name: "confidential".into(),
                    subject_type: "user".into(),
                    subject_id: session.user_id,
                    resource_type: "resource".into(),
                    scope_type: "tag-match".into(),
                    project_ids: Vec::new(),
                    tags: vec!["confidential".into()],
                    classifications: Vec::new(),
                    effect: "allow".into(),
                },
            ))
            .expect("create advanced data policy");

        let advanced_snapshot = runtime
            .block_on(bundle.access_control.get_experience_snapshot())
            .expect("advanced experience snapshot");
        assert!(advanced_snapshot.has_advanced_policies);
    }

    #[test]
    fn access_control_experience_snapshot_ignores_basic_project_access_policies_for_advanced_governance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let session = runtime
            .block_on(bundle.auth.register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("bootstrap admin")
            .session;

        runtime
            .block_on(bundle.access_control.create_data_policy(
                DataPolicyUpsertRequest {
                    name: "owner project access".into(),
                    subject_type: "user".into(),
                    subject_id: session.user_id,
                    resource_type: "project".into(),
                    scope_type: "selected-projects".into(),
                    project_ids: vec!["proj-redesign".into()],
                    tags: Vec::new(),
                    classifications: Vec::new(),
                    effect: "allow".into(),
                },
            ))
            .expect("create basic project policy");

        let snapshot = runtime
            .block_on(bundle.access_control.get_experience_snapshot())
            .expect("experience snapshot");
        assert!(
            !snapshot.has_advanced_policies,
            "basic project access policy should not trigger advanced policies"
        );
        assert_eq!(snapshot.experience_level, "personal");
    }

    #[test]
    fn access_control_member_summaries_report_primary_presets_and_org_participation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime
            .block_on(bundle.auth.register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("bootstrap admin");

        let member = runtime
            .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
                username: "member".into(),
                display_name: "Member".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            }))
            .expect("create member");

        runtime
            .block_on(bundle.access_control.create_org_unit(OrgUnitUpsertRequest {
                parent_id: Some("org-root".into()),
                code: "engineering".into(),
                name: "Engineering".into(),
                status: "active".into(),
            }))
            .expect("create org unit");
        runtime
            .block_on(bundle.access_control.upsert_user_org_assignment(
                UserOrgAssignmentUpsertRequest {
                    user_id: member.id.clone(),
                    org_unit_id: "org-engineering".into(),
                    is_primary: true,
                    position_ids: Vec::new(),
                    user_group_ids: Vec::new(),
                },
            ))
            .expect("assign member org unit");
        runtime
            .block_on(bundle.access_control.create_role_binding(RoleBindingUpsertRequest {
                role_id: "system.member".into(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            }))
            .expect("bind member preset");

        let summaries = runtime
            .block_on(bundle.access_control.list_member_summaries())
            .expect("member summaries");
        let member_summary = summaries
            .iter()
            .find(|record| record.user.id == member.id)
            .expect("member summary");
        assert_eq!(member_summary.primary_preset_code, Some("member".into()));
        assert_eq!(member_summary.primary_preset_name, "Member");
        assert!(member_summary.effective_role_names.iter().any(|name| name == "Member"));
        assert!(member_summary.has_org_assignments);
    }

    #[test]
    fn access_control_assign_user_preset_replaces_direct_system_bindings_but_preserves_custom_and_inherited_roles(
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime
            .block_on(bundle.auth.register_bootstrap_admin(RegisterBootstrapAdminRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                password: "password123".into(),
                confirm_password: "password123".into(),
                avatar: avatar_payload(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("bootstrap admin");

        let member = runtime
            .block_on(bundle.access_control.create_user(AccessUserUpsertRequest {
                username: "operator".into(),
                display_name: "Operator".into(),
                status: "active".into(),
                password: Some("password123".into()),
                confirm_password: Some("password123".into()),
                reset_password: Some(false),
            }))
            .expect("create operator");
        let custom_role = runtime
            .block_on(bundle.access_control.create_role(RoleUpsertRequest {
                code: "custom.member-helper".into(),
                name: "Member Helper".into(),
                description: "custom role".into(),
                status: "active".into(),
                permission_codes: vec!["access.users.read".into()],
            }))
            .expect("create custom role");

        runtime
            .block_on(bundle.access_control.create_org_unit(OrgUnitUpsertRequest {
                parent_id: Some("org-root".into()),
                code: "risk".into(),
                name: "Risk".into(),
                status: "active".into(),
            }))
            .expect("create risk org unit");
        runtime
            .block_on(bundle.access_control.upsert_user_org_assignment(
                UserOrgAssignmentUpsertRequest {
                    user_id: member.id.clone(),
                    org_unit_id: "org-risk".into(),
                    is_primary: true,
                    position_ids: Vec::new(),
                    user_group_ids: Vec::new(),
                },
            ))
            .expect("assign risk org unit");
        runtime
            .block_on(bundle.access_control.create_role_binding(RoleBindingUpsertRequest {
                role_id: "system.viewer".into(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            }))
            .expect("bind direct viewer");
        runtime
            .block_on(bundle.access_control.create_role_binding(RoleBindingUpsertRequest {
                role_id: custom_role.id.clone(),
                subject_type: "user".into(),
                subject_id: member.id.clone(),
                effect: "allow".into(),
            }))
            .expect("bind direct custom role");
        runtime
            .block_on(bundle.access_control.create_role_binding(RoleBindingUpsertRequest {
                role_id: "system.auditor".into(),
                subject_type: "org-unit".into(),
                subject_id: "org-risk".into(),
                effect: "allow".into(),
            }))
            .expect("bind inherited auditor role");

        let summary = runtime
            .block_on(bundle.access_control.assign_user_preset(
                &member.id,
                AccessUserPresetUpdateRequest {
                    preset_code: "admin".into(),
                },
            ))
            .expect("assign admin preset");

        assert_eq!(summary.user.id, member.id);
        assert_eq!(summary.primary_preset_code, Some("mixed".into()));
        assert_eq!(summary.primary_preset_name, "Mixed access");
        assert!(summary.effective_role_names.iter().any(|name| name == "Admin"));
        assert!(
            summary
                .effective_role_names
                .iter()
                .any(|name| name == "Member Helper")
        );
        assert!(
            summary
                .effective_role_names
                .iter()
                .any(|name| name == "Auditor")
        );

        let bindings = runtime
            .block_on(bundle.access_control.list_role_bindings())
            .expect("list role bindings");
        let direct_system_bindings = bindings
            .iter()
            .filter(|binding| binding.subject_type == "user" && binding.subject_id == member.id)
            .filter(|binding| binding.role_id.starts_with("system."))
            .collect::<Vec<_>>();
        assert_eq!(direct_system_bindings.len(), 1);
        assert_eq!(direct_system_bindings[0].role_id, "system.admin");
        assert!(
            bindings.iter().any(|binding| {
                binding.subject_type == "user"
                    && binding.subject_id == member.id
                    && binding.role_id == custom_role.id
            }),
            "custom role binding should be preserved",
        );
        assert!(
            bindings.iter().any(|binding| {
                binding.subject_type == "org-unit"
                    && binding.subject_id == "org-risk"
                    && binding.role_id == "system.auditor"
            }),
            "inherited org-unit role binding should be preserved",
        );
    }
}
