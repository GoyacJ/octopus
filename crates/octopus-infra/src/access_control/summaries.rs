use super::*;
use octopus_core::{AccessExperienceCounts, AccessMemberRoleSummary};
use std::collections::HashMap;

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
        .filter(
            |binding| match normalize_subject_type(&binding.subject_type) {
                "user" => binding.subject_id == user_id,
                "org-unit" => org_unit_ids.contains(&binding.subject_id),
                "position" => position_ids.contains(&binding.subject_id),
                "user-group" => user_group_ids.contains(&binding.subject_id),
                _ => false,
            },
        )
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

pub(super) fn build_access_experience_snapshot(
    connection: &Connection,
) -> Result<AccessExperienceSnapshot, AppError> {
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
    let protected_resource_count =
        query_count(connection, "SELECT COUNT(*) FROM protected_resources")?;
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

    let has_org_structure = org_unit_count > 0
        || position_count > 0
        || user_group_count > 0
        || has_meaningful_org_assignments;
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
            normalize_subject_type(&binding.subject_type) == "user"
                && binding.subject_id == user.record.id
        })
        .cloned()
        .collect::<Vec<_>>();
    let effective_bindings =
        resolve_effective_bindings_for_user(org_units, assignments, bindings, &user.record.id);
    let effective_role_ids = effective_role_ids_from_bindings(&effective_bindings);
    let effective_roles = effective_role_ids
        .iter()
        .filter_map(|role_id| roles_by_id.get(role_id))
        .map(|role| AccessMemberRoleSummary {
            id: role.id.clone(),
            code: role.code.clone(),
            name: role.name.clone(),
            source: role.source.clone(),
        })
        .collect::<Vec<_>>();

    let effective_role_names = effective_roles
        .iter()
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
        let code: String = direct_preset_codes
            .iter()
            .next()
            .cloned()
            .unwrap_or_default();
        (Some(code.clone()), preset_display_name(&code).to_string())
    } else if !effective_role_ids.is_empty() {
        if direct_preset_codes.is_empty() {
            (
                Some(CUSTOM_ACCESS_CODE.into()),
                CUSTOM_ACCESS_NAME.to_string(),
            )
        } else {
            (
                Some(MIXED_ACCESS_CODE.into()),
                MIXED_ACCESS_NAME.to_string(),
            )
        }
    } else {
        (None, NO_PRESET_ASSIGNED_NAME.to_string())
    };

    Ok(AccessMemberSummary {
        user: access_user,
        primary_preset_code,
        primary_preset_name,
        effective_roles,
        effective_role_names,
        has_org_assignments: user_assignments.iter().any(has_meaningful_org_assignment),
    })
}

pub(super) fn build_access_member_summaries(
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
            build_access_member_summary(
                connection,
                user,
                &roles_by_id,
                &org_units,
                &assignments,
                &bindings,
            )
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

pub(super) fn map_access_user_record_from_parts(
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
