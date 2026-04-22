use super::*;
use std::collections::{BTreeMap, BTreeSet};

mod authorization;
mod definitions;
mod permissions;
mod protected_resources;
mod routes_management;
mod routes_session;

use authorization::{
    build_access_experience_response, build_access_session_payloads,
    build_current_authorization_snapshot,
};
#[cfg(test)]
pub(crate) use definitions::system_menu_definitions;
use definitions::{
    build_access_capability_bundles, build_access_feature_definitions,
    build_access_menu_definitions, build_access_role_presets, build_access_role_templates,
    build_access_section_grants, recommended_access_section_for_snapshot,
};
use permissions::default_permission_definitions;
use protected_resources::build_access_protected_resource_descriptors;
pub(crate) use routes_management::{
    create_access_data_policy, create_access_menu_policy, create_access_org_unit,
    create_access_position, create_access_resource_policy, create_access_role,
    create_access_role_binding, create_access_user, create_access_user_group,
    delete_access_data_policy, delete_access_menu_policy, delete_access_org_unit,
    delete_access_position, delete_access_resource_policy, delete_access_role,
    delete_access_role_binding, delete_access_user, delete_access_user_group,
    delete_access_user_org_assignment, list_access_data_policies, list_access_feature_definitions,
    list_access_members, list_access_menu_definitions, list_access_menu_gate_results,
    list_access_menu_policies, list_access_org_units, list_access_permission_definitions,
    list_access_positions, list_access_protected_resources, list_access_resource_policies,
    list_access_role_bindings, list_access_roles, list_access_user_groups,
    list_access_user_org_assignments, list_access_users, update_access_data_policy,
    update_access_menu_policy, update_access_org_unit, update_access_position,
    update_access_resource_policy, update_access_role, update_access_role_binding,
    update_access_user, update_access_user_group, update_access_user_preset,
    upsert_access_protected_resource, upsert_access_user_org_assignment,
};
pub(crate) use routes_session::{
    current_authorization, get_access_experience, list_access_audit, list_access_sessions,
    revoke_access_session, revoke_access_user_sessions, revoke_current_access_session,
};
