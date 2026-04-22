use super::*;
use std::collections::HashMap;

pub(crate) fn normalize_subject_type(value: &str) -> &str {
    match value {
        "org_unit" | "org-unit" => "org-unit",
        "user_group" | "user-group" => "user-group",
        other => other,
    }
}

pub(crate) fn org_unit_ancestor_ids(
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

pub(crate) fn assignments_for_user(
    assignments: &[UserOrgAssignmentRecord],
    user_id: &str,
) -> Vec<UserOrgAssignmentRecord> {
    assignments
        .iter()
        .filter(|assignment| assignment.user_id == user_id)
        .cloned()
        .collect()
}

pub(crate) fn resolve_subject_resource_policies(
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

pub(crate) fn resolve_subject_role_bindings(
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

pub(crate) fn resolve_subject_data_policies(
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

pub(crate) fn resolve_effective_role_ids(
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

pub(crate) fn resolve_effective_permission_codes(
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

fn project_delete_data_policy_matches(policy: &DataPolicyRecord, project_id: &str) -> bool {
    match policy.scope_type.as_str() {
        "all" | "all-projects" => true,
        "selected-projects" => policy
            .project_ids
            .iter()
            .any(|candidate| candidate == project_id),
        _ => false,
    }
}

fn resource_policy_action_matches(
    policy_action: &str,
    requested_action: &str,
    capability: &str,
) -> bool {
    policy_action == "*" || policy_action == requested_action || policy_action == capability
}

pub(crate) fn resolve_project_deletion_approver_user_ids(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<String>, AppError> {
    let users = load_users(connection)?;
    let all_resource_policies = load_resource_policies(connection)?;
    let mut approver_ids = BTreeSet::new();

    for user in users
        .into_iter()
        .filter(|user| user.record.status.trim() == "active")
    {
        let user_id = user.record.id;
        let (role_ids, _) = resolve_effective_role_ids(connection, &user_id)?;
        if role_ids
            .iter()
            .any(|role_id| role_id == SYSTEM_OWNER_ROLE_ID || role_id == SYSTEM_ADMIN_ROLE_ID)
        {
            approver_ids.insert(user_id);
            continue;
        }

        let (permission_codes, _) = resolve_effective_permission_codes(connection, &user_id)?;
        if !permission_codes
            .iter()
            .any(|permission| permission == "project.manage")
        {
            continue;
        }

        let data_policies = resolve_subject_data_policies(connection, &user_id)?;
        let relevant_data_policies = data_policies
            .iter()
            .filter(|policy| policy.resource_type == "project")
            .collect::<Vec<_>>();
        let matched_data_policies = relevant_data_policies
            .iter()
            .filter(|policy| project_delete_data_policy_matches(policy, project_id))
            .collect::<Vec<_>>();
        if matched_data_policies
            .iter()
            .any(|policy| policy.effect == "deny")
        {
            continue;
        }
        if !relevant_data_policies.is_empty()
            && !matched_data_policies
                .iter()
                .any(|policy| policy.effect == "allow")
        {
            continue;
        }

        let subject_resource_policies = resolve_subject_resource_policies(connection, &user_id)?;
        let relevant_resource_policies = all_resource_policies
            .iter()
            .filter(|policy| {
                policy.resource_type == "project"
                    && policy.resource_id == project_id
                    && resource_policy_action_matches(&policy.action, "manage", "project.manage")
            })
            .collect::<Vec<_>>();
        let matching_resource_policies = subject_resource_policies
            .iter()
            .filter(|policy| {
                policy.resource_type == "project"
                    && policy.resource_id == project_id
                    && resource_policy_action_matches(&policy.action, "manage", "project.manage")
            })
            .collect::<Vec<_>>();
        if matching_resource_policies
            .iter()
            .any(|policy| policy.effect == "deny")
        {
            continue;
        }
        if !relevant_resource_policies.is_empty()
            && !matching_resource_policies
                .iter()
                .any(|policy| policy.effect == "allow")
        {
            continue;
        }

        approver_ids.insert(user_id);
    }

    Ok(approver_ids.into_iter().collect())
}
