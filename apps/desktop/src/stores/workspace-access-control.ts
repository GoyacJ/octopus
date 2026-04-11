import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  AccessAuditQuery,
  AccessRoleRecord,
  AccessSessionRecord,
  AccessUserRecord,
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
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

function sortMenus(left: MenuDefinition, right: MenuDefinition) {
  return left.order - right.order
}

export const useWorkspaceAccessControlStore = defineStore('workspace-access-control', () => {
  const authorizationsByConnection = ref<Record<string, AuthorizationSnapshot>>({})
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
  const requestTokens = ref<Record<string, number>>({})
  const loadingByConnection = ref<Record<string, boolean>>({})
  const errorsByConnection = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const authorization = computed(() => authorizationsByConnection.value[activeConnectionId.value] ?? null)
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
  const currentRoleNames = computed(() => authorization.value?.effectiveRoles.map(role => role.name) ?? [])
  const currentEffectiveFeatureCodes = computed(() => authorization.value?.featureCodes ?? [])
  const currentEffectiveMenuIds = computed(() => authorization.value?.visibleMenuIds ?? [])
  const currentResourceActionGrants = computed<ResourceActionGrant[]>(() => authorization.value?.resourceActionGrants ?? [])
  const currentVisibleMenus = computed(() =>
    menuDefinitions.value
      .filter(menu => currentEffectiveMenuIds.value.includes(menu.id))
      .sort(sortMenus),
  )
  const availableConsoleMenus = computed(() =>
    currentVisibleMenus.value.filter((menu) => getMenuDefinition(menu.id)?.section === 'console'),
  )
  const availableAccessControlMenus = computed(() =>
    currentVisibleMenus.value.filter((menu) => getMenuDefinition(menu.id)?.section === 'access-control'),
  )
  const firstAccessibleConsoleRouteName = computed(() =>
    availableConsoleMenus.value.find(menu => Boolean(getMenuDefinition(menu.id)?.routeName))?.routeName ?? null,
  )
  const firstAccessibleAccessControlRouteName = computed(() =>
    availableAccessControlMenus.value.find(menu => Boolean(getMenuDefinition(menu.id)?.routeName))?.routeName ?? null,
  )

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    loadingByConnection.value = {
      ...loadingByConnection.value,
      [connectionId]: true,
    }

    try {
      const [
        nextAuthorization,
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
        nextMenuDefinitions,
        nextFeatureDefinitions,
        nextMenuGates,
        nextMenuPolicies,
        nextProtectedResources,
      ] = await Promise.all([
        client.accessControl.getCurrentAuthorization(),
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
        client.accessControl.listMenuDefinitions(),
        client.accessControl.listFeatureDefinitions(),
        client.accessControl.listMenuGateResults(),
        client.accessControl.listMenuPolicies(),
        client.accessControl.listProtectedResources(),
      ])

      if (requestTokens.value[connectionId] !== token) {
        return
      }

      authorizationsByConnection.value = {
        ...authorizationsByConnection.value,
        [connectionId]: nextAuthorization,
      }
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
      menuDefinitionsByConnection.value = {
        ...menuDefinitionsByConnection.value,
        [connectionId]: nextMenuDefinitions,
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
      errorsByConnection.value = {
        ...errorsByConnection.value,
        [connectionId]: '',
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errorsByConnection.value = {
          ...errorsByConnection.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load access-control data',
        }
      }
    } finally {
      if (requestTokens.value[connectionId] === token) {
        loadingByConnection.value = {
          ...loadingByConnection.value,
          [connectionId]: false,
        }
      }
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
    requestTokens.value = clearRecord(requestTokens.value)
    loadingByConnection.value = clearRecord(loadingByConnection.value)
    errorsByConnection.value = clearRecord(errorsByConnection.value)
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
    await load(workspaceConnectionId)
  }

  async function createUser(input: AccessUserUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.createUser(input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function updateUser(userId: string, input: AccessUserUpsertRequest, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    const record = await client.accessControl.updateUser(userId, input)
    await reloadAll(workspaceConnectionId)
    return record
  }

  async function deleteUser(userId: string, workspaceConnectionId?: string) {
    const { client } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    await client.accessControl.deleteUser(userId)
    await reloadAll(workspaceConnectionId)
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
            name: `${user?.displayName ?? userId} project access`,
            subjectType: 'user',
            subjectId: userId,
            resourceType: 'project',
            scopeType: 'selected-projects',
            projectIds: [projectId],
            tags: [],
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
    availableConsoleMenus,
    availableAccessControlMenus,
    firstAccessibleConsoleRouteName,
    firstAccessibleAccessControlRouteName,
    load,
    loadAudit,
    loadMoreAudit,
    reloadAll,
    reloadSessions,
    clearWorkspaceScope,
    createUser,
    updateUser,
    deleteUser,
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
