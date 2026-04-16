import type {
  AccessAuditListResponse as OpenApiAccessAuditListResponse,
  AccessCapabilityBundle as OpenApiAccessCapabilityBundle,
  AccessExperienceCounts as OpenApiAccessExperienceCounts,
  AccessExperienceLevel as OpenApiAccessExperienceLevel,
  AccessExperienceResponse as OpenApiAccessExperienceResponse,
  AccessExperienceSummary as OpenApiAccessExperienceSummary,
  AccessMemberSummary as OpenApiAccessMemberSummary,
  AccessSessionRecord as OpenApiAccessSessionRecord,
  AccessSectionCode as OpenApiAccessSectionCode,
  AccessSectionGrant as OpenApiAccessSectionGrant,
  AccessRolePreset as OpenApiAccessRolePreset,
  AccessRoleRecord as OpenApiAccessRoleRecord,
  AccessRoleSource as OpenApiAccessRoleSource,
  AccessRoleTemplate as OpenApiAccessRoleTemplate,
  AccessUserRecord as OpenApiAccessUserRecord,
  AccessUserPresetUpdateRequest as OpenApiAccessUserPresetUpdateRequest,
  AccessUserUpsertRequest as OpenApiAccessUserUpsertRequest,
  AuthorizationSnapshot as OpenApiAuthorizationSnapshot,
  DataPolicyRecord as OpenApiDataPolicyRecord,
  DataPolicyUpsertRequest as OpenApiDataPolicyUpsertRequest,
  FeatureDefinition as OpenApiFeatureDefinition,
  MenuPolicyRecord as OpenApiMenuPolicyRecord,
  MenuPolicyUpsertRequest as OpenApiMenuPolicyUpsertRequest,
  MenuDefinition as OpenApiMenuDefinition,
  MenuGateResult as OpenApiMenuGateResult,
  CreateMenuPolicyRequest as OpenApiCreateMenuPolicyRequest,
  OrgUnitRecord as OpenApiOrgUnitRecord,
  OrgUnitUpsertRequest as OpenApiOrgUnitUpsertRequest,
  PermissionDefinition as OpenApiPermissionDefinition,
  PositionRecord as OpenApiPositionRecord,
  PositionUpsertRequest as OpenApiPositionUpsertRequest,
  ProtectedResourceDescriptor as OpenApiProtectedResourceDescriptor,
  ProtectedResourceMetadataUpsertRequest as OpenApiProtectedResourceMetadataUpsertRequest,
  ResourceActionGrant as OpenApiResourceActionGrant,
  ResourcePolicyRecord as OpenApiResourcePolicyRecord,
  ResourcePolicyUpsertRequest as OpenApiResourcePolicyUpsertRequest,
  RoleBindingRecord as OpenApiRoleBindingRecord,
  RoleBindingUpsertRequest as OpenApiRoleBindingUpsertRequest,
  RoleUpsertRequest as OpenApiRoleUpsertRequest,
  UserGroupRecord as OpenApiUserGroupRecord,
  UserGroupUpsertRequest as OpenApiUserGroupUpsertRequest,
  UserOrgAssignmentRecord as OpenApiUserOrgAssignmentRecord,
  UserOrgAssignmentUpsertRequest as OpenApiUserOrgAssignmentUpsertRequest,
} from './generated'

export interface AccessAuditQuery extends Record<string, string | number | boolean | null | undefined> {
  actorId?: string
  action?: string
  resourceType?: string
  outcome?: string
  from?: number
  to?: number
  cursor?: string
}

export type AccessAuditListResponse = OpenApiAccessAuditListResponse
export type AccessCapabilityBundle = OpenApiAccessCapabilityBundle
export type AccessExperienceCounts = OpenApiAccessExperienceCounts
export type AccessExperienceLevel = OpenApiAccessExperienceLevel
export type AccessExperienceResponse = OpenApiAccessExperienceResponse
export type AccessExperienceSummary = OpenApiAccessExperienceSummary
export type AccessMemberSummary = OpenApiAccessMemberSummary
export type AccessSessionRecord = OpenApiAccessSessionRecord
export type AccessSectionCode = OpenApiAccessSectionCode
export type AccessSectionGrant = OpenApiAccessSectionGrant
export type AccessRolePreset = OpenApiAccessRolePreset
export type AccessUserRecord = OpenApiAccessUserRecord
export type AccessUserPresetUpdateRequest = OpenApiAccessUserPresetUpdateRequest
export type AccessUserUpsertRequest = OpenApiAccessUserUpsertRequest
export type AuthorizationSnapshot = OpenApiAuthorizationSnapshot
export type OrgUnitRecord = OpenApiOrgUnitRecord
export type OrgUnitUpsertRequest = OpenApiOrgUnitUpsertRequest
export type PositionRecord = OpenApiPositionRecord
export type PositionUpsertRequest = OpenApiPositionUpsertRequest
export type UserGroupRecord = OpenApiUserGroupRecord
export type UserGroupUpsertRequest = OpenApiUserGroupUpsertRequest
export type UserOrgAssignmentRecord = OpenApiUserOrgAssignmentRecord
export type UserOrgAssignmentUpsertRequest = OpenApiUserOrgAssignmentUpsertRequest
export type PermissionDefinition = OpenApiPermissionDefinition
export type AccessRoleRecord = OpenApiAccessRoleRecord
export type AccessRoleSource = OpenApiAccessRoleSource
export type AccessRoleTemplate = OpenApiAccessRoleTemplate
export type RoleUpsertRequest = OpenApiRoleUpsertRequest
export type RoleBindingRecord = OpenApiRoleBindingRecord
export type RoleBindingUpsertRequest = OpenApiRoleBindingUpsertRequest
export type DataPolicyRecord = OpenApiDataPolicyRecord
export type DataPolicyUpsertRequest = OpenApiDataPolicyUpsertRequest
export type ResourcePolicyRecord = OpenApiResourcePolicyRecord
export type ResourcePolicyUpsertRequest = OpenApiResourcePolicyUpsertRequest
export type MenuDefinition = OpenApiMenuDefinition
export type MenuPolicyRecord = OpenApiMenuPolicyRecord
export type MenuPolicyUpsertRequest = OpenApiMenuPolicyUpsertRequest
export type CreateMenuPolicyRequest = OpenApiCreateMenuPolicyRequest
export type FeatureDefinition = OpenApiFeatureDefinition
export type MenuGateResult = OpenApiMenuGateResult
export type ProtectedResourceDescriptor = OpenApiProtectedResourceDescriptor
export type ProtectedResourceMetadataUpsertRequest = OpenApiProtectedResourceMetadataUpsertRequest
export type ResourceActionGrant = OpenApiResourceActionGrant
