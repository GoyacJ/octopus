use serde::{Deserialize, Serialize};

use crate::{AuditRecord, AvatarUploadPayload};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRecordSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub status: String,
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentUserProfileRequest {
    pub username: String,
    pub display_name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentUserPasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentUserPasswordResponse {
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserRecord {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub password_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserUpsertRequest {
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub password: Option<String>,
    pub confirm_password: Option<String>,
    pub reset_password: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnitRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnitUpsertRequest {
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionUpsertRequest {
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupUpsertRequest {
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgAssignmentRecord {
    pub user_id: String,
    pub org_unit_id: String,
    pub is_primary: bool,
    pub position_ids: Vec<String>,
    pub user_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgAssignmentUpsertRequest {
    pub user_id: String,
    pub org_unit_id: String,
    pub is_primary: bool,
    pub position_ids: Vec<String>,
    pub user_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDefinition {
    pub code: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub resource_type: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRoleRecord {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_codes: Vec<String>,
    pub source: String,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleUpsertRequest {
    pub code: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleBindingRecord {
    pub id: String,
    pub role_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleBindingUpsertRequest {
    pub role_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceSummary {
    pub experience_level: String,
    pub member_count: u32,
    pub has_org_structure: bool,
    pub has_custom_roles: bool,
    pub has_advanced_policies: bool,
    pub has_menu_governance: bool,
    pub has_resource_governance: bool,
    pub recommended_landing_section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceSnapshot {
    pub experience_level: String,
    pub member_count: u32,
    pub has_org_structure: bool,
    pub has_custom_roles: bool,
    pub has_advanced_policies: bool,
    pub has_menu_governance: bool,
    pub has_resource_governance: bool,
    pub counts: AccessExperienceCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessSectionGrant {
    pub section: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRoleTemplate {
    pub code: String,
    pub name: String,
    pub description: String,
    pub managed_role_codes: Vec<String>,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRolePreset {
    pub code: String,
    pub name: String,
    pub description: String,
    pub recommended_for: String,
    pub template_codes: Vec<String>,
    pub capability_bundle_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessCapabilityBundle {
    pub code: String,
    pub name: String,
    pub description: String,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceCounts {
    pub custom_role_count: u32,
    pub org_unit_count: u32,
    pub data_policy_count: u32,
    pub resource_policy_count: u32,
    pub menu_policy_count: u32,
    pub protected_resource_count: u32,
    pub session_count: u32,
    pub audit_event_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessExperienceResponse {
    pub summary: AccessExperienceSummary,
    pub section_grants: Vec<AccessSectionGrant>,
    pub role_templates: Vec<AccessRoleTemplate>,
    pub role_presets: Vec<AccessRolePreset>,
    pub capability_bundles: Vec<AccessCapabilityBundle>,
    pub counts: AccessExperienceCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessMemberRoleSummary {
    pub id: String,
    pub code: String,
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessMemberSummary {
    pub user: AccessUserRecord,
    pub primary_preset_code: Option<String>,
    pub primary_preset_name: String,
    pub effective_roles: Vec<AccessMemberRoleSummary>,
    pub effective_role_names: Vec<String>,
    pub has_org_assignments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessUserPresetUpdateRequest {
    pub preset_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicyRecord {
    pub id: String,
    pub name: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub scope_type: String,
    pub project_ids: Vec<String>,
    pub tags: Vec<String>,
    pub classifications: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicyUpsertRequest {
    pub name: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub scope_type: String,
    pub project_ids: Vec<String>,
    pub tags: Vec<String>,
    pub classifications: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicyRecord {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicyUpsertRequest {
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuDefinition {
    pub id: String,
    pub parent_id: Option<String>,
    pub label: String,
    pub route_name: Option<String>,
    pub source: String,
    pub status: String,
    pub order: i64,
    pub feature_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuPolicyRecord {
    pub menu_id: String,
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuPolicyUpsertRequest {
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateMenuPolicyRequest {
    pub menu_id: String,
    pub enabled: bool,
    pub order: i64,
    pub group: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub id: String,
    pub code: String,
    pub label: String,
    pub required_permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuGateResult {
    pub menu_id: String,
    pub feature_code: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceActionGrant {
    pub resource_type: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessSessionRecord {
    pub session_id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub client_app_id: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedResourceDescriptor {
    pub id: String,
    pub resource_type: String,
    pub resource_subtype: Option<String>,
    pub name: String,
    pub project_id: Option<String>,
    pub tags: Vec<String>,
    pub classification: String,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedResourceMetadataUpsertRequest {
    pub resource_subtype: Option<String>,
    pub project_id: Option<String>,
    pub tags: Vec<String>,
    pub classification: String,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationSnapshot {
    pub principal: AccessUserRecord,
    pub effective_role_ids: Vec<String>,
    pub effective_roles: Vec<AccessRoleRecord>,
    pub effective_permission_codes: Vec<String>,
    pub org_assignments: Vec<UserOrgAssignmentRecord>,
    pub feature_codes: Vec<String>,
    pub visible_menu_ids: Vec<String>,
    pub menu_gates: Vec<MenuGateResult>,
    pub resource_action_grants: Vec<ResourceActionGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub subject_id: String,
    pub capability: String,
    pub project_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_subtype: Option<String>,
    pub tags: Vec<String>,
    pub classification: Option<String>,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditQuery {
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub outcome: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditListResponse {
    pub items: Vec<AuditRecord>,
    pub next_cursor: Option<String>,
}
