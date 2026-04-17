use async_trait::async_trait;
use octopus_core::{
    AccessExperienceSnapshot, AccessMemberSummary, AccessRoleRecord, AccessUserPresetUpdateRequest,
    AccessUserRecord, AccessUserUpsertRequest, AppError, DataPolicyRecord, DataPolicyUpsertRequest,
    MenuPolicyRecord, MenuPolicyUpsertRequest, OrgUnitRecord, OrgUnitUpsertRequest, PositionRecord,
    PositionUpsertRequest, ProtectedResourceDescriptor, ProtectedResourceMetadataUpsertRequest,
    ResourcePolicyRecord, ResourcePolicyUpsertRequest, RoleBindingRecord, RoleBindingUpsertRequest,
    RoleUpsertRequest, UserGroupRecord, UserGroupUpsertRequest, UserOrgAssignmentRecord,
    UserOrgAssignmentUpsertRequest,
};

#[async_trait]
pub trait AccessControlService: Send + Sync {
    async fn list_users(&self) -> Result<Vec<AccessUserRecord>, AppError>;
    async fn get_experience_snapshot(&self) -> Result<AccessExperienceSnapshot, AppError>;
    async fn list_member_summaries(&self) -> Result<Vec<AccessMemberSummary>, AppError>;
    async fn assign_user_preset(
        &self,
        user_id: &str,
        request: AccessUserPresetUpdateRequest,
    ) -> Result<AccessMemberSummary, AppError>;
    async fn create_user(
        &self,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError>;
    async fn update_user(
        &self,
        user_id: &str,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError>;
    async fn delete_user(&self, user_id: &str) -> Result<(), AppError>;

    async fn list_org_units(&self) -> Result<Vec<OrgUnitRecord>, AppError>;
    async fn create_org_unit(
        &self,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError>;
    async fn update_org_unit(
        &self,
        org_unit_id: &str,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError>;
    async fn delete_org_unit(&self, org_unit_id: &str) -> Result<(), AppError>;

    async fn list_positions(&self) -> Result<Vec<PositionRecord>, AppError>;
    async fn create_position(
        &self,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError>;
    async fn update_position(
        &self,
        position_id: &str,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError>;
    async fn delete_position(&self, position_id: &str) -> Result<(), AppError>;

    async fn list_user_groups(&self) -> Result<Vec<UserGroupRecord>, AppError>;
    async fn create_user_group(
        &self,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError>;
    async fn update_user_group(
        &self,
        group_id: &str,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError>;
    async fn delete_user_group(&self, group_id: &str) -> Result<(), AppError>;

    async fn list_user_org_assignments(&self) -> Result<Vec<UserOrgAssignmentRecord>, AppError>;
    async fn upsert_user_org_assignment(
        &self,
        request: UserOrgAssignmentUpsertRequest,
    ) -> Result<UserOrgAssignmentRecord, AppError>;
    async fn delete_user_org_assignment(
        &self,
        user_id: &str,
        org_unit_id: &str,
    ) -> Result<(), AppError>;

    async fn list_roles(&self) -> Result<Vec<AccessRoleRecord>, AppError>;
    async fn create_role(&self, request: RoleUpsertRequest) -> Result<AccessRoleRecord, AppError>;
    async fn update_role(
        &self,
        role_id: &str,
        request: RoleUpsertRequest,
    ) -> Result<AccessRoleRecord, AppError>;
    async fn delete_role(&self, role_id: &str) -> Result<(), AppError>;

    async fn list_role_bindings(&self) -> Result<Vec<RoleBindingRecord>, AppError>;
    async fn create_role_binding(
        &self,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError>;
    async fn update_role_binding(
        &self,
        binding_id: &str,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError>;
    async fn delete_role_binding(&self, binding_id: &str) -> Result<(), AppError>;

    async fn list_data_policies(&self) -> Result<Vec<DataPolicyRecord>, AppError>;
    async fn create_data_policy(
        &self,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError>;
    async fn update_data_policy(
        &self,
        policy_id: &str,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError>;
    async fn delete_data_policy(&self, policy_id: &str) -> Result<(), AppError>;

    async fn list_resource_policies(&self) -> Result<Vec<ResourcePolicyRecord>, AppError>;
    async fn create_resource_policy(
        &self,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError>;
    async fn update_resource_policy(
        &self,
        policy_id: &str,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError>;
    async fn delete_resource_policy(&self, policy_id: &str) -> Result<(), AppError>;

    async fn list_menu_policies(&self) -> Result<Vec<MenuPolicyRecord>, AppError>;
    async fn upsert_menu_policy(
        &self,
        menu_id: &str,
        request: MenuPolicyUpsertRequest,
    ) -> Result<MenuPolicyRecord, AppError>;
    async fn delete_menu_policy(&self, menu_id: &str) -> Result<(), AppError>;

    async fn list_protected_resources(&self) -> Result<Vec<ProtectedResourceDescriptor>, AppError>;
    async fn upsert_protected_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        request: ProtectedResourceMetadataUpsertRequest,
    ) -> Result<ProtectedResourceDescriptor, AppError>;
}
