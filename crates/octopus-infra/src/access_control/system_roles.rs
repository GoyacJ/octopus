use super::*;

#[derive(Debug, Clone)]
struct SystemRoleDefinition {
    code: &'static str,
    name: &'static str,
    description: &'static str,
    permission_codes: Vec<String>,
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
            description:
                "Manage members, presets, governance configuration, and workspace operations.",
            permission_codes: default_admin_permission_codes(),
        },
        SystemRoleDefinition {
            code: SYSTEM_MEMBER_ROLE_ID,
            name: "Member",
            description:
                "Participate in day-to-day workspace projects, resources, and runtime work.",
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
            description:
                "Inspect members, policies, sessions, and audit records without editing them.",
            permission_codes: default_auditor_permission_codes(),
        },
    ]
}

pub(crate) fn preset_code_to_system_role_code(preset_code: &str) -> Option<&'static str> {
    match preset_code.trim() {
        "owner" => Some(SYSTEM_OWNER_ROLE_ID),
        "admin" => Some(SYSTEM_ADMIN_ROLE_ID),
        "member" => Some(SYSTEM_MEMBER_ROLE_ID),
        "viewer" => Some(SYSTEM_VIEWER_ROLE_ID),
        "auditor" => Some(SYSTEM_AUDITOR_ROLE_ID),
        _ => None,
    }
}

pub(crate) fn system_role_code_to_preset_code(role_code: &str) -> Option<&'static str> {
    match role_code {
        SYSTEM_OWNER_ROLE_ID => Some("owner"),
        SYSTEM_ADMIN_ROLE_ID => Some("admin"),
        SYSTEM_MEMBER_ROLE_ID => Some("member"),
        SYSTEM_VIEWER_ROLE_ID => Some("viewer"),
        SYSTEM_AUDITOR_ROLE_ID => Some("auditor"),
        _ => None,
    }
}

pub(crate) fn preset_display_name(code: &str) -> &'static str {
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

    if canonical_id == definition.code {
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
    } else {
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
                    .execute(
                        "DELETE FROM access_roles WHERE id = ?1",
                        params![legacy_role_id],
                    )
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

pub(crate) fn reject_reserved_system_role_code(code: &str) -> Result<(), AppError> {
    if is_system_role_code(code) {
        return Err(AppError::invalid_input(
            "system.* role codes are reserved for platform-managed roles",
        ));
    }
    Ok(())
}

pub(crate) fn ensure_default_owner_role_permissions(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_system_roles(connection)
}
