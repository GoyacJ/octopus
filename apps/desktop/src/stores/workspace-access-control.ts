import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  AccessCapabilityBundle,
  AccessAuditQuery,
  AccessExperienceResponse,
  AccessMemberSummary,
  AccessRoleRecord,
  AccessRolePreset,
  AccessRoleTemplate,
  AccessSessionRecord,
  AccessSectionCode,
  AccessUserRecord,
  AccessUserPresetUpdateRequest,
  AccessUserUpsertRequest,
  AuditRecord,
  AuthorizationSnapshot,
  CreateMenuPolicyRequest,
  DataPolicyRecord,
  DataPolicyUpsertRequest,
  FeatureDefinition,
  MenuDefinition,
  MenuGateResult,
  MenuPolicyRecord,
  MenuPolicyUpsertRequest,
  OrgUnitRecord,
  OrgUnitUpsertRequest,
  PermissionDefinition,
  PositionRecord,
  PositionUpsertRequest,
  ProtectedResourceDescriptor,
  ProtectedResourceMetadataUpsertRequest,
  ResourceActionGrant,
  ResourcePolicyRecord,
  ResourcePolicyUpsertRequest,
  RoleBindingRecord,
  RoleBindingUpsertRequest,
  RoleUpsertRequest,
  UserGroupRecord,
  UserGroupUpsertRequest,
  UserOrgAssignmentRecord,
  UserOrgAssignmentUpsertRequest,
} from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'
import {
  createProjectAccessPolicyName,
  getAccessRoleName,
} from '@/views/workspace/access-control/display-i18n'

import {
  activeWorkspaceConnectionId,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

function sortMenus(left: MenuDefinition, right: MenuDefinition) {
  return left.order - right.order
}

function isAdvancedDataPolicy(policy: DataPolicyRecord) {
  return policy.resourceType !== 'project'
    || policy.scopeType !== 'selected-projects'
    || policy.effect !== 'allow'
}

const ACCESS_CONTROL_MEMBER_PERMISSION_CODES = [
  'access.users.read',
  'access.users.manage',
] as const

const ACCESS_CONTROL_GOVERNANCE_PERMISSION_CODES = [
  'access.org.read',
  'access.org.manage',
  'access.policies.read',
  'access.policies.manage',
  'access.menus.read',
  'access.menus.manage',
  'access.sessions.read',
  'access.sessions.manage',
  'audit.read',
] as const

interface AccessPresetCard extends AccessRolePreset {
  capabilityBundles: AccessCapabilityBundle[]
  templates: AccessRoleTemplate[]
}

export const useWorkspaceAccessControlStore = defineStore('workspace-access-control', () => {
  const authorizationsByConnection = ref<Record<string, AuthorizationSnapshot>>({})
  const experiencesByConnection = ref<Record<string, AccessExperienceResponse>>({})
  const membersByConnection = ref<Record<string, AccessMemberSummary[]>>({})
  const auditByConnection = ref<Record<string, AuditRecord[]>>({})
  const auditNextCursorByConnection = ref<Record<string, string | undefined>>({})
  const auditQueryByConnection = ref<Record<string, AccessAuditQuery>>({})
  const auditLoadingByConnection = ref<Record<string, boolean>>({})
  const auditErrorsByConnection = ref<Record<string, string>>({})
  const sessionsByConnection = ref<Record<string, AccessSessionRecord[]>>({})
  const usersByConnection = ref<Record<string, AccessUserRecord[]>>({})
  const orgUnitsByConnection = ref<Record<string, OrgUnitRecord[]>>({})
  const positionsByConnection = ref<Record<string, PositionRecord[]>>({})
  const userGroupsByConnection = ref<Record<string, UserGroupRecord[]>>({})
  const userOrgAssignmentsByConnection = ref<Record<string, UserOrgAssignmentRecord[]>>({})
  const rolesByConnection = ref<Record<string, AccessRoleRecord[]>>({})
  const permissionDefinitionsByConnection = ref<Record<string, PermissionDefinition[]>>({})
  const roleBindingsByConnection = ref<Record<string, RoleBindingRecord[]>>({})
  const dataPoliciesByConnection = ref<Record<string, DataPolicyRecord[]>>({})
  const resourcePoliciesByConnection = ref<Record<string, ResourcePolicyRecord[]>>({})
  const menuDefinitionsByConnection = ref<Record<string, MenuDefinition[]>>({})
  const featureDefinitionsByConnection = ref<Record<string, FeatureDefinition[]>>({})
  const menuGatesByConnection = ref<Record<string, MenuGateResult[]>>({})
  const menuPoliciesByConnection = ref<Record<string, MenuPolicyRecord[]>>({})
  const protectedResourcesByConnection = ref<Record<string, ProtectedResourceDescriptor[]>>({})
  const loadingByConnection = ref<Record<string, boolean>>({})
  const errorsByConnection = ref<Record<string, string>>({})
  const authorizationContextLoadedAtByConnection = ref<Record<string, number>>({})
  const experienceLoadedAtByConnection = ref<Record<string, number>>({})
  const membersDataLoadedAtByConnection = ref<Record<string, number>>({})
  const governanceDataLoadedAtByConnection = ref<Record<string, number>>({})
  const authorizationContextInflightByConnection: Record<string, Promise<void> | undefined> = {}
  const experienceInflightByConnection: Record<string, Promise<void> | undefined> = {}
  const membersDataInflightByConnection: Record<string, Promise<void> | undefined> = {}
  const governanceDataInflightByConnection: Record<string, Promise<void> | undefined> = {}

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const authorization = computed(() => authorizationsByConnection.value[activeConnectionId.value] ?? null)
  const experience = computed(() => experiencesByConnection.value[activeConnectionId.value] ?? null)
  const members = computed(() => membersByConnection.value[activeConnectionId.value] ?? [])
  const auditRecords = computed(() => auditByConnection.value[activeConnectionId.value] ?? [])
  const auditNextCursor = computed(() => auditNextCursorByConnection.value[activeConnectionId.value])
  const auditQuery = computed(() => auditQueryByConnection.value[activeConnectionId.value] ?? {})
  const auditLoading = computed(() => auditLoadingByConnection.value[activeConnectionId.value] ?? false)
  const auditError = computed(() => auditErrorsByConnection.value[activeConnectionId.value] ?? '')
  const sessions = computed(() => sessionsByConnection.value[activeConnectionId.value] ?? [])
  const users = computed(() => usersByConnection.value[activeConnectionId.value] ?? [])
  const orgUnits = computed(() => orgUnitsByConnection.value[activeConnectionId.value] ?? [])
  const positions = computed(() => positionsByConnection.value[activeConnectionId.value] ?? [])
  const userGroups = computed(() => userGroupsByConnection.value[activeConnectionId.value] ?? [])
  const userOrgAssignments = computed(() => userOrgAssignmentsByConnection.value[activeConnectionId.value] ?? [])
  const roles = computed(() => rolesByConnection.value[activeConnectionId.value] ?? [])
  const permissionDefinitions = computed(() => permissionDefinitionsByConnection.value[activeConnectionId.value] ?? [])
  const roleBindings = computed(() => roleBindingsByConnection.value[activeConnectionId.value] ?? [])
  const dataPolicies = computed(() => dataPoliciesByConnection.value[activeConnectionId.value] ?? [])
  const resourcePolicies = computed(() => resourcePoliciesByConnection.value[activeConnectionId.value] ?? [])
  const menuDefinitions = computed(() => menuDefinitionsByConnection.value[activeConnectionId.value] ?? [])
  const featureDefinitions = computed(() => featureDefinitionsByConnection.value[activeConnectionId.value] ?? [])
  const menuGates = computed(() => menuGatesByConnection.value[activeConnectionId.value] ?? [])
  const menuPolicies = computed(() => menuPoliciesByConnection.value[activeConnectionId.value] ?? [])
  const protectedResources = computed(() => protectedResourcesByConnection.value[activeConnectionId.value] ?? [])
  const loading = computed(() => loadingByConnection.value[activeConnectionId.value] ?? false)
  const error = computed(() => errorsByConnection.value[activeConnectionId.value] ?? '')

  const currentUser = computed<AccessUserRecord | null>(() => authorization.value?.principal ?? null)
  const currentOrgAssignments = computed(() => authorization.value?.orgAssignments ?? [])
  const currentRoleBindings = computed(() =>
    roleBindings.value.filter(binding => binding.subjectType === 'user' && binding.subjectId === currentUser.value?.id),
  )
  const currentRoleNames = computed(() =>
    authorization.value?.effectiveRoles.map(role => getAccessRoleName(role)) ?? [],
  )
  const currentEffectiveFeatureCodes = computed(() => authorization.value?.featureCodes ?? [])
  const currentEffectiveMenuIds = computed(() => authorization.value?.visibleMenuIds ?? [])
  const currentResourceActionGrants = computed<ResourceActionGrant[]>(() => authorization.value?.resourceActionGrants ?? [])
  const currentVisibleMenus = computed(() =>
    menuDefinitions.value
      .filter(menu => currentEffectiveMenuIds.value.includes(menu.id))
      .sort(sortMenus),
  )
  const experienceSummary = computed(() => experience.value?.summary ?? null)
  const roleTemplates = computed(() => experience.value?.roleTemplates ?? [])
  const rolePresets = computed(() => experience.value?.rolePresets ?? [])
  const capabilityBundles = computed(() => experience.value?.capabilityBundles ?? [])
  const accessSectionGrants = computed<Record<AccessSectionCode, boolean>>(() => {
    const grants: Record<AccessSectionCode, boolean> = {
      members: false,
      access: false,
      governance: false,
    }
    for (const grant of experience.value?.sectionGrants ?? []) {
      grants[grant.section] = grant.allowed
    }
    return grants
  })
  const sidebarAccessSectionGrants = computed<Record<AccessSectionCode, boolean>>(() => {
    if (experience.value?.sectionGrants.length) {
      return accessSectionGrants.value
    }

    const permissionCodes = new Set(authorization.value?.effectivePermissionCodes ?? [])
    const membersAllowed = ACCESS_CONTROL_MEMBER_PERMISSION_CODES
      .some(code => permissionCodes.has(code))
    const governanceAllowed = ACCESS_CONTROL_GOVERNANCE_PERMISSION_CODES
      .some(code => permissionCodes.has(code))

    return {
      members: membersAllowed,
      access: membersAllowed,
      governance: governanceAllowed,
    }
  })
  const canShowAccessControlNavigation = computed(() =>
    Object.values(sidebarAccessSectionGrants.value).some(Boolean),
  )
  const recommendedAccessSection = computed<AccessSectionCode | null>(() => {
    const recommended = experienceSummary.value?.recommendedLandingSection
    if (recommended && accessSectionGrants.value[recommended]) {
      return recommended
    }

    return (experience.value?.sectionGrants.find(grant => grant.allowed)?.section ?? null) as AccessSectionCode | null
  })
  const capabilityBundlesByCode = computed(() =>
    new Map(capabilityBundles.value.map(bundle => [bundle.code, bundle])),
  )
  const templateByCode = computed(() =>
    new Map(roleTemplates.value.map(template => [template.code, template])),
  )
  const presetCards = computed<AccessPresetCard[]>(() =>
    rolePresets.value.map((preset) => ({
      ...preset,
      capabilityBundles: preset.capabilityBundleCodes
        .map(code => capabilityBundlesByCode.value.get(code))
        .filter((bundle): bundle is AccessCapabilityBundle => Boolean(bundle)),
      templates: preset.templateCodes
        .map(code => templateByCode.value.get(code))
        .filter((template): template is AccessRoleTemplate => Boolean(template)),
    })),
  )
  const membersByPresetCode = computed(() => {
    const buckets = new Map<string, AccessMemberSummary[]>()
    for (const member of members.value) {
      if (!member.primaryPresetCode) {
        continue
      }
      const bucket = buckets.get(member.primaryPresetCode) ?? []
      bucket.push(member)
      buckets.set(member.primaryPresetCode, bucket)
    }
    return buckets
  })
  const rootOrgUnitId = computed(() =>
    orgUnits.value.find(unit => !unit.parentId)?.id ?? '',
  )
  const hasSummaryGovernanceSignals = computed(() =>
    Boolean(
      experienceSummary.value?.hasOrgStructure
      || experienceSummary.value?.hasCustomRoles
      || experienceSummary.value?.hasAdvancedPolicies
      || experienceSummary.value?.hasMenuGovernance
      || experienceSummary.value?.hasResourceGovernance,
    ),
  )
  const hasLoadedGovernanceSignals = computed(() => {
    const hasOrgStructure = orgUnits.value.some(unit => unit.id !== rootOrgUnitId.value)
      || positions.value.length > 0
      || userGroups.value.length > 0
      || userOrgAssignments.value.some(assignment =>
        assignment.orgUnitId !== rootOrgUnitId.value
        || assignment.positionIds.length > 0
        || assignment.userGroupIds.length > 0,
      )
    const hasCustomRoles = roles.value.some(role => role.source === 'custom')
    const hasAdvancedPolicies = dataPolicies.value.some(policy => isAdvancedDataPolicy(policy))
    const hasMenuGovernance = menuPolicies.value.length > 0
    const hasResourceGovernance = resourcePolicies.value.length > 0 || protectedResources.value.length > 0

    return hasOrgStructure
      || hasCustomRoles
      || hasAdvancedPolicies
      || hasMenuGovernance
      || hasResourceGovernance
  })
  const isGovernanceEmpty = computed(() => {
    if (hasSummaryGovernanceSignals.value) {
      return false
    }

    if (experienceSummary.value?.experienceLevel === 'personal') {
      return true
    }

    return !hasLoadedGovernanceSignals.value
  })
  const availableConsoleMenus = computed(() =>
    currentVisibleMenus.value.filter((menu) => getMenuDefinition(menu.id)?.section === 'console'),
  )
  const firstAccessibleConsoleRouteName = computed(() =>
    availableConsoleMenus.value.find(menu => Boolean(getMenuDefinition(menu.id)?.routeName))?.routeName ?? null,
  )
  const firstAccessibleAccessControlRouteName = computed(() =>
    recommendedAccessSection.value
      ? `workspace-access-control-${recommendedAccessSection.value}` as const
      : null,
  )

  function logDevTiming(label: string, startedAt: number, detail?: string) {
    if (!import.meta.env.DEV) {
      return
    }

    const suffix = detail ? ` ${detail}` : ''
    console.debug(`[access-control] ${label}${suffix} ${Math.round(performance.now() - startedAt)}ms`)
  }

  function hasAuthorizationContext(connectionId: string) {
    return Boolean(authorizationsByConnection.value[connectionId] && menuDefinitionsByConnection.value[connectionId])
  }

  function hasExperience(connectionId: string) {
    return Boolean(experiencesByConnection.value[connectionId])
  }

  function hasMembersData(connectionId: string) {
    return Boolean(membersDataLoadedAtByConnection.value[connectionId] && membersByConnection.value[connectionId])
  }

  async function ensureAuthorizationContext(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { client, connectionId } = resolvedClient
    if (!options.force && hasAuthorizationContext(connectionId)) {
      return
    }

    const inflight = authorizationContextInflightByConnection[connectionId]
    if (inflight && !options.force) {
      await inflight
      return
    }

    const startedAt = performance.now()
    const task = (async () => {
      const [
        nextAuthorization,
        nextMenuDefinitions,
      ] = await Promise.all([
        client.accessControl.getCurrentAuthorization(),
        client.accessControl.listMenuDefinitions(),
      ])

      authorizationsByConnection.value = {
        ...authorizationsByConnection.value,
        [connectionId]: nextAuthorization,
      }
      menuDefinitionsByConnection.value = {
        ...menuDefinitionsByConnection.value,
        [connectionId]: nextMenuDefinitions,
      }
      authorizationContextLoadedAtByConnection.value = {
        ...authorizationContextLoadedAtByConnection.value,
        [connectionId]: Date.now(),
      }
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: '',
      }
    })()
    authorizationContextInflightByConnection[connectionId] = task

    try {
      await task
    } catch (cause) {
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load authorization context',
      }
    } finally {
      if (authorizationContextInflightByConnection[connectionId] === task) {
        delete authorizationContextInflightByConnection[connectionId]
      }
      logDevTiming('ensureAuthorizationContext', startedAt, connectionId)
    }
  }

  async function loadExperience(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { client, connectionId } = resolvedClient
    if (!options.force && hasExperience(connectionId)) {
      return
    }

    const inflight = experienceInflightByConnection[connectionId]
    if (inflight && !options.force) {
      await inflight
      return
    }

    await ensureAuthorizationContext(connectionId, options)

    const startedAt = performance.now()

    const task = (async () => {
      const nextExperience = await client.accessControl.getAccessExperience()
      experiencesByConnection.value = {
        ...experiencesByConnection.value,
        [connectionId]: nextExperience,
      }
      experienceLoadedAtByConnection.value = {
        ...experienceLoadedAtByConnection.value,
        [connectionId]: Date.now(),
      }
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: '',
      }
    })()
    experienceInflightByConnection[connectionId] = task

    try {
      await task
    } catch (cause) {
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load access experience',
      }
    } finally {
      if (experienceInflightByConnection[connectionId] === task) {
        delete experienceInflightByConnection[connectionId]
      }
      logDevTiming('loadExperience', startedAt, connectionId)
    }
  }

  async function loadGovernanceData(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { client, connectionId } = resolvedClient
    if (!options.force && governanceDataLoadedAtByConnection.value[connectionId]) {
      return
    }

    const inflight = governanceDataInflightByConnection[connectionId]
    if (inflight && !options.force) {
      await inflight
      return
    }

    await loadMembersData(connectionId, options)

    const startedAt = performance.now()
    loadingByConnection.value = {
      ...loadingByConnection.value,
      [connectionId]: true,
    }

    const task = (async () => {
      const [
        nextAudit,
        nextSessions,
        nextUsers,
        nextOrgUnits,
        nextPositions,
        nextUserGroups,
        nextAssignments,
        nextRoles,
        nextPermissionDefinitions,
        nextRoleBindings,
        nextDataPolicies,
        nextResourcePolicies,
        nextFeatureDefinitions,
        nextMenuGates,
        nextMenuPolicies,
        nextProtectedResources,
      ] = await Promise.all([
        client.accessControl.listAudit(),
        client.accessControl.listSessions(),
        client.accessControl.listUsers(),
        client.accessControl.listOrgUnits(),
        client.accessControl.listPositions(),
        client.accessControl.listUserGroups(),
        client.accessControl.listUserOrgAssignments(),
        client.accessControl.listRoles(),
        client.accessControl.listPermissionDefinitions(),
        client.accessControl.listRoleBindings(),
        client.accessControl.listDataPolicies(),
        client.accessControl.listResourcePolicies(),
        client.accessControl.listFeatureDefinitions(),
        client.accessControl.listMenuGateResults(),
        client.accessControl.listMenuPolicies(),
        client.accessControl.listProtectedResources(),
      ])

      auditByConnection.value = {
        ...auditByConnection.value,
        [connectionId]: nextAudit.items,
      }
      auditNextCursorByConnection.value = {
        ...auditNextCursorByConnection.value,
        [connectionId]: nextAudit.nextCursor,
      }
      auditQueryByConnection.value = {
        ...auditQueryByConnection.value,
        [connectionId]: {},
      }
      auditErrorsByConnection.value = {
        ...auditErrorsByConnection.value,
        [connectionId]: '',
      }
      sessionsByConnection.value = {
        ...sessionsByConnection.value,
        [connectionId]: nextSessions,
      }
      usersByConnection.value = {
        ...usersByConnection.value,
        [connectionId]: nextUsers,
      }
      orgUnitsByConnection.value = {
        ...orgUnitsByConnection.value,
        [connectionId]: nextOrgUnits,
      }
      positionsByConnection.value = {
        ...positionsByConnection.value,
        [connectionId]: nextPositions,
      }
      userGroupsByConnection.value = {
        ...userGroupsByConnection.value,
        [connectionId]: nextUserGroups,
      }
      userOrgAssignmentsByConnection.value = {
        ...userOrgAssignmentsByConnection.value,
        [connectionId]: nextAssignments,
      }
      rolesByConnection.value = {
        ...rolesByConnection.value,
        [connectionId]: nextRoles,
      }
      permissionDefinitionsByConnection.value = {
        ...permissionDefinitionsByConnection.value,
        [connectionId]: nextPermissionDefinitions,
      }
      roleBindingsByConnection.value = {
        ...roleBindingsByConnection.value,
        [connectionId]: nextRoleBindings,
      }
      dataPoliciesByConnection.value = {
        ...dataPoliciesByConnection.value,
        [connectionId]: nextDataPolicies,
      }
      resourcePoliciesByConnection.value = {
        ...resourcePoliciesByConnection.value,
        [connectionId]: nextResourcePolicies,
      }
      featureDefinitionsByConnection.value = {
        ...featureDefinitionsByConnection.value,
        [connectionId]: nextFeatureDefinitions,
      }
      menuGatesByConnection.value = {
        ...menuGatesByConnection.value,
        [connectionId]: nextMenuGates,
      }
      menuPoliciesByConnection.value = {
        ...menuPoliciesByConnection.value,
        [connectionId]: nextMenuPolicies,
      }
      protectedResourcesByConnection.value = {
        ...protectedResourcesByConnection.value,
        [connectionId]: nextProtectedResources,
      }
      governanceDataLoadedAtByConnection.value = {
        ...governanceDataLoadedAtByConnection.value,
        [connectionId]: Date.now(),
      }
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: '',
      }
    })()
    governanceDataInflightByConnection[connectionId] = task

    try {
      await task
    } catch (cause) {
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load governance data',
      }
    } finally {
      if (governanceDataInflightByConnection[connectionId] === task) {
        delete governanceDataInflightByConnection[connectionId]
      }
      loadingByConnection.value = {
        ...loadingByConnection.value,
        [connectionId]: false,
      }
      logDevTiming('loadGovernanceData', startedAt, connectionId)
    }
  }

  async function loadMembersData(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { client, connectionId } = resolvedClient
    if (!options.force && hasMembersData(connectionId)) {
      return
    }

    const inflight = membersDataInflightByConnection[connectionId]
    if (inflight && !options.force) {
      await inflight
      return
    }

    await Promise.all([
      ensureAuthorizationContext(connectionId, options),
      loadExperience(connectionId, options),
    ])

    const startedAt = performance.now()
    loadingByConnection.value = {
      ...loadingByConnection.value,
      [connectionId]: true,
    }

    const task = (async () => {
      const nextMembers = await client.accessControl.listMembers()
      membersByConnection.value = {
        ...membersByConnection.value,
        [connectionId]: nextMembers,
      }
      membersDataLoadedAtByConnection.value = {
        ...membersDataLoadedAtByConnection.value,
        [connectionId]: Date.now(),
      }
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: '',
      }
    })()
    membersDataInflightByConnection[connectionId] = task

    try {
      await task
    } catch (cause) {
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load access members',
      }
    } finally {
      if (membersDataInflightByConnection[connectionId] === task) {
        delete membersDataInflightByConnection[connectionId]
      }
      loadingByConnection.value = {
        ...loadingByConnection.value,
        [connectionId]: false,
      }
      logDevTiming('loadMembersData', startedAt, connectionId)
    }
  }

  async function reloadSessions(workspaceConnectionId?: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const nextSessions = await client.accessControl.listSessions()
    sessionsByConnection.value = {
      ...sessionsByConnection.value,
      [connectionId]: nextSessions,
    }
    return nextSessions
  }

  async function loadAudit(
    query: AccessAuditQuery = {},
    workspaceConnectionId?: string,
    options: { append?: boolean } = {},
  ) {
    const { client, connectionId } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    auditLoadingByConnection.value = {
      ...auditLoadingByConnection.value,
      [connectionId]: true,
    }

    try {
      const mergedQuery = options.append
        ? {
            ...(auditQueryByConnection.value[connectionId] ?? {}),
            ...query,
          }
        : query
      const response = await client.accessControl.listAudit(mergedQuery)
      const nextItems = options.append
        ? [...(auditByConnection.value[connectionId] ?? []), ...response.items]
        : response.items
      auditByConnection.value = {
        ...auditByConnection.value,
        [connectionId]: nextItems,
      }
      auditNextCursorByConnection.value = {
        ...auditNextCursorByConnection.value,
        [connectionId]: response.nextCursor,
      }
      auditQueryByConnection.value = {
        ...auditQueryByConnection.value,
        [connectionId]: mergedQuery,
      }
      auditErrorsByConnection.value = {
        ...auditErrorsByConnection.value,
        [connectionId]: '',
      }
      return response
    } catch (cause) {
      auditErrorsByConnection.value = {
        ...auditErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load audit records',
      }
      throw cause
    } finally {
      auditLoadingByConnection.value = {
        ...auditLoadingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function loadMoreAudit(workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    const cursor = auditNextCursorByConnection.value[connectionId]
    if (!cursor) {
      return null
    }
    return await loadAudit({ cursor }, workspaceConnectionId, { append: true })
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const clearRecord = <T>(record: Record<string, T>) => {
      const next = { ...record }
      delete next[workspaceConnectionId]
      return next
    }

    authorizationsByConnection.value = clearRecord(authorizationsByConnection.value)
    experiencesByConnection.value = clearRecord(experiencesByConnection.value)
    membersByConnection.value = clearRecord(membersByConnection.value)
    auditByConnection.value = clearRecord(auditByConnection.value)
    auditNextCursorByConnection.value = clearRecord(auditNextCursorByConnection.value)
    auditQueryByConnection.value = clearRecord(auditQueryByConnection.value)
    auditLoadingByConnection.value = clearRecord(auditLoadingByConnection.value)
    auditErrorsByConnection.value = clearRecord(auditErrorsByConnection.value)
    sessionsByConnection.value = clearRecord(sessionsByConnection.value)
    usersByConnection.value = clearRecord(usersByConnection.value)
    orgUnitsByConnection.value = clearRecord(orgUnitsByConnection.value)
    positionsByConnection.value = clearRecord(positionsByConnection.value)
    userGroupsByConnection.value = clearRecord(userGroupsByConnection.value)
    userOrgAssignmentsByConnection.value = clearRecord(userOrgAssignmentsByConnection.value)
    rolesByConnection.value = clearRecord(rolesByConnection.value)
    permissionDefinitionsByConnection.value = clearRecord(permissionDefinitionsByConnection.value)
    roleBindingsByConnection.value = clearRecord(roleBindingsByConnection.value)
    dataPoliciesByConnection.value = clearRecord(dataPoliciesByConnection.value)
    resourcePoliciesByConnection.value = clearRecord(resourcePoliciesByConnection.value)
    menuDefinitionsByConnection.value = clearRecord(menuDefinitionsByConnection.value)
    featureDefinitionsByConnection.value = clearRecord(featureDefinitionsByConnection.value)
    menuGatesByConnection.value = clearRecord(menuGatesByConnection.value)
    menuPoliciesByConnection.value = clearRecord(menuPoliciesByConnection.value)
    protectedResourcesByConnection.value = clearRecord(protectedResourcesByConnection.value)
    loadingByConnection.value = clearRecord(loadingByConnection.value)
    errorsByConnection.value = clearRecord(errorsByConnection.value)
    authorizationContextLoadedAtByConnection.value = clearRecord(authorizationContextLoadedAtByConnection.value)
    experienceLoadedAtByConnection.value = clearRecord(experienceLoadedAtByConnection.value)
    membersDataLoadedAtByConnection.value = clearRecord(membersDataLoadedAtByConnection.value)
    governanceDataLoadedAtByConnection.value = clearRecord(governanceDataLoadedAtByConnection.value)
    delete authorizationContextInflightByConnection[workspaceConnectionId]
    delete experienceInflightByConnection[workspaceConnectionId]
    delete membersDataInflightByConnection[workspaceConnectionId]
    delete governanceDataInflightByConnection[workspaceConnectionId]
  }

  async function refreshMembersLayer(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { connectionId } = resolvedClient
    if (governanceDataLoadedAtByConnection.value[connectionId]) {
      await loadGovernanceData(connectionId, { force: true })
      return
    }

    await loadMembersData(connectionId, { force: true })
  }

  async function revokeSession(sessionId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.revokeSession(sessionId)
    await reloadAll(workspaceConnectionId)
  }

  async function revokeUserSessions(userId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.revokeUserSessions(userId)
    await reloadAll(workspaceConnectionId)
  }

  async function reloadAll(workspaceConnectionId?: string) {
    await loadGovernanceData(workspaceConnectionId, { force: true })
  }

  async function createUser(input: AccessUserUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createUser(input)
    await refreshMembersLayer(workspaceConnectionId)
    return record
  }

  async function updateUser(userId: string, input: AccessUserUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateUser(userId, input)
    await refreshMembersLayer(workspaceConnectionId)
    return record
  }

  async function deleteUser(userId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteUser(userId)
    await refreshMembersLayer(workspaceConnectionId)
  }

  async function assignUserPreset(
    userId: string,
    input: AccessUserPresetUpdateRequest,
    workspaceConnectionId?: string,
  ) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateUserPreset(userId, input)
    await refreshMembersLayer(workspaceConnectionId)
    return record
  }

  async function setProjectMembers(projectId: string, userIds: string[], workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const selectedUserIds = new Set(userIds)
    const directProjectPolicies = dataPolicies.value.filter(policy =>
      policy.subjectType === 'user'
      && policy.resourceType === 'project'
      && policy.scopeType === 'selected-projects'
      && policy.effect === 'allow',
    )
    const usersById = new Map(users.value.map(user => [user.id, user]))

    const existingPolicyByUserId = new Map(
      directProjectPolicies
        .filter(policy => policy.projectIds.includes(projectId))
        .map(policy => [policy.subjectId, policy]),
    )

    const createTargets = [...selectedUserIds]
      .filter(userId => !existingPolicyByUserId.has(userId))
      .map((userId) => {
        const user = usersById.get(userId)
        return {
          userId,
          payload: {
            name: createProjectAccessPolicyName(user?.displayName ?? userId),
            subjectType: 'user',
            subjectId: userId,
            resourceType: 'project',
            scopeType: 'selected-projects',
            projectIds: [projectId],
            tags: [],
            classifications: [],
            effect: 'allow',
          } satisfies DataPolicyUpsertRequest,
        }
      })

    const deleteTargets = [...existingPolicyByUserId.values()]
      .filter(policy => !selectedUserIds.has(policy.subjectId))

    await Promise.all([
      ...createTargets.map(({ payload }) => client.accessControl.createDataPolicy(payload)),
      ...deleteTargets.map(policy => client.accessControl.deleteDataPolicy(policy.id)),
    ])
    await reloadAll(workspaceConnectionId)
  }

  async function createOrgUnit(input: OrgUnitUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createOrgUnit(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateOrgUnit(orgUnitId: string, input: OrgUnitUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateOrgUnit(orgUnitId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteOrgUnit(orgUnitId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteOrgUnit(orgUnitId)
    await reloadAll(workspaceConnectionId)
  }

  async function createPosition(input: PositionUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createPosition(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updatePosition(positionId: string, input: PositionUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updatePosition(positionId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deletePosition(positionId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deletePosition(positionId)
    await reloadAll(workspaceConnectionId)
  }

  async function createUserGroup(input: UserGroupUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createUserGroup(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateUserGroup(groupId: string, input: UserGroupUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateUserGroup(groupId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteUserGroup(groupId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteUserGroup(groupId)
    await reloadAll(workspaceConnectionId)
  }

  async function upsertUserOrgAssignment(input: UserOrgAssignmentUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.upsertUserOrgAssignment(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteUserOrgAssignment(userId: string, orgUnitId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteUserOrgAssignment(userId, orgUnitId)
    await reloadAll(workspaceConnectionId)
  }

  async function createRole(input: RoleUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createRole(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateRole(roleId: string, input: RoleUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateRole(roleId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteRole(roleId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteRole(roleId)
    await reloadAll(workspaceConnectionId)
  }

  async function createRoleBinding(input: RoleBindingUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createRoleBinding(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateRoleBinding(bindingId: string, input: RoleBindingUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateRoleBinding(bindingId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteRoleBinding(bindingId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteRoleBinding(bindingId)
    await reloadAll(workspaceConnectionId)
  }

  async function createDataPolicy(input: DataPolicyUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createDataPolicy(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateDataPolicy(policyId: string, input: DataPolicyUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateDataPolicy(policyId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteDataPolicy(policyId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteDataPolicy(policyId)
    await reloadAll(workspaceConnectionId)
  }

  async function createResourcePolicy(input: ResourcePolicyUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createResourcePolicy(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateResourcePolicy(policyId: string, input: ResourcePolicyUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateResourcePolicy(policyId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteResourcePolicy(policyId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteResourcePolicy(policyId)
    await reloadAll(workspaceConnectionId)
  }

  async function createMenuPolicy(input: CreateMenuPolicyRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createMenuPolicy(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateMenuPolicy(menuId: string, input: MenuPolicyUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateMenuPolicy(menuId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteMenuPolicy(menuId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteMenuPolicy(menuId)
    await reloadAll(workspaceConnectionId)
  }

  async function upsertProtectedResource(
    resourceType: string,
    resourceId: string,
    input: ProtectedResourceMetadataUpsertRequest,
    workspaceConnectionId?: string,
  ) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.upsertProtectedResource(resourceType, resourceId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  return {
    authorization,
    auditRecords,
    auditNextCursor,
    auditQuery,
    auditLoading,
    auditError,
    members,
    membersByPresetCode,
    sessions,
    users,
    orgUnits,
    positions,
    userGroups,
    userOrgAssignments,
    roles,
    permissionDefinitions,
    roleBindings,
    dataPolicies,
    resourcePolicies,
    menuDefinitions,
    featureDefinitions,
    menuGates,
    menuPolicies,
    protectedResources,
    loading,
    error,
    currentUser,
    currentOrgAssignments,
    currentRoleBindings,
    currentRoleNames,
    currentEffectiveFeatureCodes,
    currentEffectiveMenuIds,
    currentResourceActionGrants,
    currentVisibleMenus,
    experience,
    experienceSummary,
    roleTemplates,
    rolePresets,
    capabilityBundles,
    capabilityBundlesByCode,
    accessSectionGrants,
    sidebarAccessSectionGrants,
    canShowAccessControlNavigation,
    recommendedAccessSection,
    presetCards,
    isGovernanceEmpty,
    availableConsoleMenus,
    firstAccessibleConsoleRouteName,
    firstAccessibleAccessControlRouteName,
    ensureAuthorizationContext,
    loadExperience,
    loadMembersData,
    loadGovernanceData,
    loadAudit,
    loadMoreAudit,
    reloadAll,
    reloadSessions,
    clearWorkspaceScope,
    createUser,
    updateUser,
    deleteUser,
    assignUserPreset,
    setProjectMembers,
    createOrgUnit,
    updateOrgUnit,
    deleteOrgUnit,
    createPosition,
    updatePosition,
    deletePosition,
    createUserGroup,
    updateUserGroup,
    deleteUserGroup,
    upsertUserOrgAssignment,
    deleteUserOrgAssignment,
    createRole,
    updateRole,
    deleteRole,
    createRoleBinding,
    updateRoleBinding,
    deleteRoleBinding,
    createDataPolicy,
    updateDataPolicy,
    deleteDataPolicy,
    createResourcePolicy,
    updateResourcePolicy,
    deleteResourcePolicy,
    createMenuPolicy,
    updateMenuPolicy,
    deleteMenuPolicy,
    upsertProtectedResource,
    revokeSession,
    revokeUserSessions,
  }
})
