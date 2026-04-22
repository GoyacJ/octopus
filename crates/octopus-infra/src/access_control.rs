mod defaults;
mod loaders;
mod resolve;
mod service_governance;
mod service_members;
mod summaries;
mod system_roles;
#[cfg(test)]
mod tests;

use super::*;
use octopus_core::{
    AccessExperienceSnapshot, AccessMemberSummary, AccessRoleRecord, AccessUserPresetUpdateRequest,
    AccessUserRecord, AccessUserUpsertRequest, DataPolicyRecord, DataPolicyUpsertRequest,
    MenuPolicyRecord, MenuPolicyUpsertRequest, OrgUnitRecord, OrgUnitUpsertRequest, PositionRecord,
    PositionUpsertRequest, ProtectedResourceDescriptor, ProtectedResourceMetadataUpsertRequest,
    ResourcePolicyRecord, ResourcePolicyUpsertRequest, RoleBindingRecord, RoleBindingUpsertRequest,
    RoleUpsertRequest, UserGroupRecord, UserGroupUpsertRequest, UserOrgAssignmentRecord,
    UserOrgAssignmentUpsertRequest,
};
use std::collections::BTreeSet;

use defaults::*;
use loaders::*;
use resolve::*;
use summaries::*;
use system_roles::*;

pub(super) use loaders::{
    load_data_policies, load_org_units, load_resource_policies, load_user_org_assignments,
};
pub(super) use resolve::{
    assignments_for_user, org_unit_ancestor_ids, resolve_effective_permission_codes,
    resolve_effective_role_ids, resolve_project_deletion_approver_user_ids,
    resolve_subject_data_policies, resolve_subject_resource_policies,
};
pub(super) use system_roles::ensure_default_owner_role_permissions;

pub(crate) const SYSTEM_ROLE_NAMESPACE_PREFIX: &str = "system.";
pub(crate) const SYSTEM_OWNER_ROLE_ID: &str = "system.owner";
pub(crate) const SYSTEM_ADMIN_ROLE_ID: &str = "system.admin";
pub(crate) const SYSTEM_MEMBER_ROLE_ID: &str = "system.member";
pub(crate) const SYSTEM_VIEWER_ROLE_ID: &str = "system.viewer";
pub(crate) const SYSTEM_AUDITOR_ROLE_ID: &str = "system.auditor";
pub(super) const LEGACY_OWNER_ROLE_ID: &str = "owner";
pub(super) const ROOT_ORG_UNIT_ID: &str = "org-root";
pub(super) const CUSTOM_ACCESS_CODE: &str = "custom";
pub(super) const CUSTOM_ACCESS_NAME: &str = "Custom access";
pub(super) const MIXED_ACCESS_CODE: &str = "mixed";
pub(super) const MIXED_ACCESS_NAME: &str = "Mixed access";
pub(super) const NO_PRESET_ASSIGNED_NAME: &str = "No preset assigned";

pub(super) fn read_json_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

pub(super) fn bool_to_sql(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(super) fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(super) fn merge_permission_codes(
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

#[async_trait]
impl AccessControlService for InfraAccessControlService {
    async fn list_users(&self) -> Result<Vec<AccessUserRecord>, AppError> {
        self.list_users_impl().await
    }

    async fn get_experience_snapshot(&self) -> Result<AccessExperienceSnapshot, AppError> {
        self.get_experience_snapshot_impl().await
    }

    async fn list_member_summaries(&self) -> Result<Vec<AccessMemberSummary>, AppError> {
        self.list_member_summaries_impl().await
    }

    async fn assign_user_preset(
        &self,
        user_id: &str,
        request: AccessUserPresetUpdateRequest,
    ) -> Result<AccessMemberSummary, AppError> {
        self.assign_user_preset_impl(user_id, request).await
    }

    async fn create_user(
        &self,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        self.create_user_impl(request).await
    }

    async fn update_user(
        &self,
        user_id: &str,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        self.update_user_impl(user_id, request).await
    }

    async fn delete_user(&self, user_id: &str) -> Result<(), AppError> {
        self.delete_user_impl(user_id).await
    }

    async fn list_org_units(&self) -> Result<Vec<OrgUnitRecord>, AppError> {
        self.list_org_units_impl().await
    }

    async fn create_org_unit(
        &self,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        self.create_org_unit_impl(request).await
    }

    async fn update_org_unit(
        &self,
        org_unit_id: &str,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        self.update_org_unit_impl(org_unit_id, request).await
    }

    async fn delete_org_unit(&self, org_unit_id: &str) -> Result<(), AppError> {
        self.delete_org_unit_impl(org_unit_id).await
    }

    async fn list_positions(&self) -> Result<Vec<PositionRecord>, AppError> {
        self.list_positions_impl().await
    }

    async fn create_position(
        &self,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        self.create_position_impl(request).await
    }

    async fn update_position(
        &self,
        position_id: &str,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        self.update_position_impl(position_id, request).await
    }

    async fn delete_position(&self, position_id: &str) -> Result<(), AppError> {
        self.delete_position_impl(position_id).await
    }

    async fn list_user_groups(&self) -> Result<Vec<UserGroupRecord>, AppError> {
        self.list_user_groups_impl().await
    }

    async fn create_user_group(
        &self,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        self.create_user_group_impl(request).await
    }

    async fn update_user_group(
        &self,
        group_id: &str,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        self.update_user_group_impl(group_id, request).await
    }

    async fn delete_user_group(&self, group_id: &str) -> Result<(), AppError> {
        self.delete_user_group_impl(group_id).await
    }

    async fn list_user_org_assignments(&self) -> Result<Vec<UserOrgAssignmentRecord>, AppError> {
        self.list_user_org_assignments_impl().await
    }

    async fn upsert_user_org_assignment(
        &self,
        request: UserOrgAssignmentUpsertRequest,
    ) -> Result<UserOrgAssignmentRecord, AppError> {
        self.upsert_user_org_assignment_impl(request).await
    }

    async fn delete_user_org_assignment(
        &self,
        user_id: &str,
        org_unit_id: &str,
    ) -> Result<(), AppError> {
        self.delete_user_org_assignment_impl(user_id, org_unit_id)
            .await
    }

    async fn list_roles(&self) -> Result<Vec<AccessRoleRecord>, AppError> {
        self.list_roles_impl().await
    }

    async fn create_role(&self, request: RoleUpsertRequest) -> Result<AccessRoleRecord, AppError> {
        self.create_role_impl(request).await
    }

    async fn update_role(
        &self,
        role_id: &str,
        request: RoleUpsertRequest,
    ) -> Result<AccessRoleRecord, AppError> {
        self.update_role_impl(role_id, request).await
    }

    async fn delete_role(&self, role_id: &str) -> Result<(), AppError> {
        self.delete_role_impl(role_id).await
    }

    async fn list_role_bindings(&self) -> Result<Vec<RoleBindingRecord>, AppError> {
        self.list_role_bindings_impl().await
    }

    async fn create_role_binding(
        &self,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        self.create_role_binding_impl(request).await
    }

    async fn update_role_binding(
        &self,
        binding_id: &str,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        self.update_role_binding_impl(binding_id, request).await
    }

    async fn delete_role_binding(&self, binding_id: &str) -> Result<(), AppError> {
        self.delete_role_binding_impl(binding_id).await
    }

    async fn list_data_policies(&self) -> Result<Vec<DataPolicyRecord>, AppError> {
        self.list_data_policies_impl().await
    }

    async fn create_data_policy(
        &self,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        self.create_data_policy_impl(request).await
    }

    async fn update_data_policy(
        &self,
        policy_id: &str,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        self.update_data_policy_impl(policy_id, request).await
    }

    async fn delete_data_policy(&self, policy_id: &str) -> Result<(), AppError> {
        self.delete_data_policy_impl(policy_id).await
    }

    async fn list_resource_policies(&self) -> Result<Vec<ResourcePolicyRecord>, AppError> {
        self.list_resource_policies_impl().await
    }

    async fn create_resource_policy(
        &self,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        self.create_resource_policy_impl(request).await
    }

    async fn update_resource_policy(
        &self,
        policy_id: &str,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        self.update_resource_policy_impl(policy_id, request).await
    }

    async fn delete_resource_policy(&self, policy_id: &str) -> Result<(), AppError> {
        self.delete_resource_policy_impl(policy_id).await
    }

    async fn list_menu_policies(&self) -> Result<Vec<MenuPolicyRecord>, AppError> {
        self.list_menu_policies_impl().await
    }

    async fn upsert_menu_policy(
        &self,
        menu_id: &str,
        request: MenuPolicyUpsertRequest,
    ) -> Result<MenuPolicyRecord, AppError> {
        self.upsert_menu_policy_impl(menu_id, request).await
    }

    async fn delete_menu_policy(&self, menu_id: &str) -> Result<(), AppError> {
        self.delete_menu_policy_impl(menu_id).await
    }

    async fn list_protected_resources(&self) -> Result<Vec<ProtectedResourceDescriptor>, AppError> {
        self.list_protected_resources_impl().await
    }

    async fn upsert_protected_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        request: ProtectedResourceMetadataUpsertRequest,
    ) -> Result<ProtectedResourceDescriptor, AppError> {
        self.upsert_protected_resource_impl(resource_type, resource_id, request)
            .await
    }
}
