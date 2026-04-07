import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CreateWorkspaceUserRequest,
  MenuRecord,
  PermissionRecord,
  RoleRecord,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  UpdateCurrentUserProfileRequest,
  UpdateWorkspaceUserRequest,
  UserCenterOverviewSnapshot,
  UserRecordSummary,
} from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import {
  createRuntimeConfigDraftsFromConfig,
  parseRuntimeConfigDraft,
} from './runtime-config'

function permissionMatches(
  permission: PermissionRecord,
  code: string,
  action?: string,
  targetType?: string,
  targetId?: string,
) {
  if (permission.code !== code) {
    return false
  }
  if (action && permission.action && permission.action !== action) {
    return false
  }
  if (targetType && permission.targetType && permission.targetType !== targetType) {
    return false
  }
  if (targetId && permission.targetIds.length > 0 && !permission.targetIds.includes(targetId)) {
    return false
  }
  return permission.status === 'active'
}

export const useUserCenterStore = defineStore('user-center', () => {
  const overviews = ref<Record<string, UserCenterOverviewSnapshot>>({})
  const usersByConnection = ref<Record<string, UserRecordSummary[]>>({})
  const rolesByConnection = ref<Record<string, RoleRecord[]>>({})
  const permissionsByConnection = ref<Record<string, PermissionRecord[]>>({})
  const menusByConnection = ref<Record<string, MenuRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})
  const runtimeConfigsByConnection = ref<Record<string, RuntimeEffectiveConfig>>({})
  const runtimeDraftsByConnection = ref<Record<string, string>>({})
  const runtimeValidationByConnection = ref<Record<string, RuntimeConfigValidationResult | null>>({})
  const runtimeLoadingByConnection = ref<Record<string, boolean>>({})
  const runtimeSavingByConnection = ref<Record<string, boolean>>({})
  const runtimeValidatingByConnection = ref<Record<string, boolean>>({})
  const runtimeErrorsByConnection = ref<Record<string, string>>({})
  const profileSavingByConnection = ref<Record<string, boolean>>({})
  const profileErrorsByConnection = ref<Record<string, string>>({})
  const passwordSavingByConnection = ref<Record<string, boolean>>({})
  const passwordErrorsByConnection = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const overview = computed(() => overviews.value[activeConnectionId.value] ?? null)
  const users = computed(() => usersByConnection.value[activeConnectionId.value] ?? [])
  const roles = computed(() => rolesByConnection.value[activeConnectionId.value] ?? [])
  const permissions = computed(() => permissionsByConnection.value[activeConnectionId.value] ?? [])
  const menus = computed(() => menusByConnection.value[activeConnectionId.value] ?? [])
  const currentUser = computed(() => overview.value?.currentUser ?? users.value[0] ?? null)
  const currentRoleRecords = computed(() =>
    roles.value.filter(role => currentUser.value?.roleIds.includes(role.id)),
  )
  const currentRoleNames = computed(() => overview.value?.roleNames ?? roles.value.filter(role => currentUser.value?.roleIds.includes(role.id)).map(role => role.name))
  const currentPermissionRecords = computed(() => {
    const permissionIds = new Set<string>()
    currentRoleRecords.value.forEach((role) => {
      role.permissionIds.forEach(permissionId => permissionIds.add(permissionId))
    })
    return permissions.value.filter(permission => permissionIds.has(permission.id))
  })
  const currentEffectiveMenuIds = computed(() => {
    const menuIds = new Set<string>()
    currentRoleRecords.value.forEach((role) => {
      role.menuIds.forEach(menuId => menuIds.add(menuId))
    })
    if (!menuIds.size) {
      menus.value.forEach((menu) => {
        if (menu.status === 'active') {
          menuIds.add(menu.id)
        }
      })
    }
    return [...menuIds]
  })
  const currentEffectiveMenus = computed(() =>
    menus.value
      .filter(menu => currentEffectiveMenuIds.value.includes(menu.id))
      .sort((left, right) => left.order - right.order),
  )
  const availableUserCenterMenus = computed(() =>
    currentEffectiveMenus.value.filter(menu => menu.source === 'user-center'),
  )
  const firstAccessibleUserCenterRouteName = computed(() => {
    const firstMenu = availableUserCenterMenus.value.find(menu => menu.routeName && getMenuDefinition(menu.id)?.routeName)
    return firstMenu?.routeName ?? null
  })
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')
  const runtimeConfig = computed(() => runtimeConfigsByConnection.value[activeConnectionId.value] ?? null)
  const runtimeDraft = computed(() => runtimeDraftsByConnection.value[activeConnectionId.value] ?? '{}')
  const runtimeValidation = computed(() => runtimeValidationByConnection.value[activeConnectionId.value] ?? null)
  const runtimeLoading = computed(() => runtimeLoadingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeSaving = computed(() => runtimeSavingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeValidating = computed(() => runtimeValidatingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeError = computed(() => runtimeErrorsByConnection.value[activeConnectionId.value] ?? '')
  const profileSaving = computed(() => profileSavingByConnection.value[activeConnectionId.value] ?? false)
  const profileError = computed(() => profileErrorsByConnection.value[activeConnectionId.value] ?? '')
  const passwordSaving = computed(() => passwordSavingByConnection.value[activeConnectionId.value] ?? false)
  const passwordError = computed(() => passwordErrorsByConnection.value[activeConnectionId.value] ?? '')

  function updateCurrentUserState(connectionId: string, updatedUser: UserRecordSummary) {
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: (usersByConnection.value[connectionId] ?? []).map(item => item.id === updatedUser.id ? updatedUser : item),
    }

    const currentOverview = overviews.value[connectionId]
    if (currentOverview?.currentUser.id === updatedUser.id) {
      overviews.value = {
        ...overviews.value,
        [connectionId]: {
          ...currentOverview,
          currentUser: updatedUser,
        },
      }
    }
  }

  function removeUserState(connectionId: string, userId: string) {
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: (usersByConnection.value[connectionId] ?? []).filter(item => item.id !== userId),
    }
  }

  function removeRoleState(connectionId: string, roleId: string) {
    rolesByConnection.value = {
      ...rolesByConnection.value,
      [connectionId]: (rolesByConnection.value[connectionId] ?? []).filter(item => item.id !== roleId),
    }
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: (usersByConnection.value[connectionId] ?? []).map(user => ({
        ...user,
        roleIds: user.roleIds.filter(id => id !== roleId),
      })),
    }

    const currentOverview = overviews.value[connectionId]
    if (currentOverview) {
      overviews.value = {
        ...overviews.value,
        [connectionId]: {
          ...currentOverview,
          currentUser: {
            ...currentOverview.currentUser,
            roleIds: currentOverview.currentUser.roleIds.filter(id => id !== roleId),
          },
          roleNames: currentOverview.roleNames.filter(name =>
            (rolesByConnection.value[connectionId] ?? []).some(role => role.name === name),
          ),
        },
      }
    }
  }

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const [nextOverview, nextUsers, nextRoles, nextPermissions, nextMenus] = await Promise.all([
        client.rbac.getUserCenterOverview(),
        client.rbac.listUsers(),
        client.rbac.listRoles(),
        client.rbac.listPermissions(),
        client.rbac.listMenus(),
      ])
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      overviews.value = {
        ...overviews.value,
        [connectionId]: nextOverview,
      }
      usersByConnection.value = {
        ...usersByConnection.value,
        [connectionId]: nextUsers,
      }
      rolesByConnection.value = {
        ...rolesByConnection.value,
        [connectionId]: nextRoles,
      }
      permissionsByConnection.value = {
        ...permissionsByConnection.value,
        [connectionId]: nextPermissions,
      }
      menusByConnection.value = {
        ...menusByConnection.value,
        [connectionId]: nextMenus,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load user center',
        }
      }
    }
  }

  async function createUser(record: CreateWorkspaceUserRequest) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.rbac.createUser(record)
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: [...(usersByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateUser(userId: string, record: UpdateWorkspaceUserRequest) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.rbac.updateUser(userId, record)
    updateCurrentUserState(connectionId, updated)
    return updated
  }

  async function setProjectMembers(projectId: string, memberUserIds: string[]) {
    const selectedUserIds = new Set(memberUserIds)
    const currentUsers = [...users.value]
    const pendingUpdates = currentUsers.filter((user) => {
      const currentlyAssigned = user.scopeProjectIds.includes(projectId)
      const nextAssigned = selectedUserIds.has(user.id)
      return currentlyAssigned !== nextAssigned
    })

    for (const user of pendingUpdates) {
      const nextScopeProjectIds = selectedUserIds.has(user.id)
        ? Array.from(new Set([...user.scopeProjectIds, projectId]))
        : user.scopeProjectIds.filter(id => id !== projectId)
      await updateUser(user.id, {
        username: user.username,
        displayName: user.displayName,
        status: user.status,
        roleIds: [...user.roleIds],
        scopeProjectIds: nextScopeProjectIds,
      })
    }

    return users.value.filter(user => selectedUserIds.has(user.id))
  }

  async function deleteUser(userId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.rbac.deleteUser(userId)
    removeUserState(connectionId, userId)
  }

  async function updateCurrentUserProfile(input: UpdateCurrentUserProfileRequest) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    profileSavingByConnection.value = {
      ...profileSavingByConnection.value,
      [connectionId]: true,
    }
    profileErrorsByConnection.value = {
      ...profileErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const updated = await client.rbac.updateCurrentUserProfile(input)
      updateCurrentUserState(connectionId, updated)
      return updated
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Failed to update profile'
      profileErrorsByConnection.value = {
        ...profileErrorsByConnection.value,
        [connectionId]: message,
      }
      throw cause
    } finally {
      profileSavingByConnection.value = {
        ...profileSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function changeCurrentUserPassword(input: ChangeCurrentUserPasswordRequest) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    passwordSavingByConnection.value = {
      ...passwordSavingByConnection.value,
      [connectionId]: true,
    }
    passwordErrorsByConnection.value = {
      ...passwordErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const response = await client.rbac.changeCurrentUserPassword(input)
      const activeUser = overviews.value[connectionId]?.currentUser
      if (activeUser) {
        updateCurrentUserState(connectionId, {
          ...activeUser,
          passwordState: response.passwordState as ChangeCurrentUserPasswordResponse['passwordState'],
        })
      }
      return response
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Failed to update password'
      passwordErrorsByConnection.value = {
        ...passwordErrorsByConnection.value,
        [connectionId]: message,
      }
      throw cause
    } finally {
      passwordSavingByConnection.value = {
        ...passwordSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function createRole(record: RoleRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.rbac.createRole(record)
    rolesByConnection.value = {
      ...rolesByConnection.value,
      [connectionId]: [...(rolesByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateRole(roleId: string, record: RoleRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.rbac.updateRole(roleId, record)
    rolesByConnection.value = {
      ...rolesByConnection.value,
      [connectionId]: (rolesByConnection.value[connectionId] ?? []).map(item => item.id === roleId ? updated : item),
    }
    return updated
  }

  async function deleteRole(roleId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.rbac.deleteRole(roleId)
    removeRoleState(connectionId, roleId)
  }

  async function createPermission(record: PermissionRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.rbac.createPermission(record)
    permissionsByConnection.value = {
      ...permissionsByConnection.value,
      [connectionId]: [...(permissionsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updatePermission(permissionId: string, record: PermissionRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.rbac.updatePermission(permissionId, record)
    permissionsByConnection.value = {
      ...permissionsByConnection.value,
      [connectionId]: (permissionsByConnection.value[connectionId] ?? []).map(item => item.id === permissionId ? updated : item),
    }
    return updated
  }

  async function deletePermission(permissionId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.rbac.deletePermission(permissionId)
    permissionsByConnection.value = {
      ...permissionsByConnection.value,
      [connectionId]: (permissionsByConnection.value[connectionId] ?? [])
        .filter(item => item.id !== permissionId)
        .map(permission => ({
          ...permission,
          memberPermissionIds: permission.memberPermissionIds.filter(id => id !== permissionId),
        })),
    }
    rolesByConnection.value = {
      ...rolesByConnection.value,
      [connectionId]: (rolesByConnection.value[connectionId] ?? []).map(role => ({
        ...role,
        permissionIds: role.permissionIds.filter(id => id !== permissionId),
      })),
    }
  }

  async function createMenu(record: MenuRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.rbac.createMenu(record)
    menusByConnection.value = {
      ...menusByConnection.value,
      [connectionId]: [...(menusByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateMenu(menuId: string, record: MenuRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.rbac.updateMenu(menuId, record)
    menusByConnection.value = {
      ...menusByConnection.value,
      [connectionId]: (menusByConnection.value[connectionId] ?? []).map(item => item.id === menuId ? updated : item),
    }
    return updated
  }

  function hasPermission(code: string, action?: string, targetType?: string, targetId?: string) {
    const effectivePermissionIds = new Set<string>()
    for (const role of roles.value) {
      if (currentUser.value?.roleIds.includes(role.id)) {
        role.permissionIds.forEach(permissionId => effectivePermissionIds.add(permissionId))
      }
    }
    return permissions.value.some(permission =>
      effectivePermissionIds.has(permission.id) && permissionMatches(permission, code, action, targetType, targetId))
  }

  function setCurrentUserRuntimeDraft(value: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    if (!connectionId) {
      return
    }
    runtimeDraftsByConnection.value = {
      ...runtimeDraftsByConnection.value,
      [connectionId]: value,
    }
  }

  async function loadCurrentUserRuntimeConfig(force = false, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    if (runtimeConfigsByConnection.value[connectionId] && !force) {
      return runtimeConfigsByConnection.value[connectionId]
    }

    runtimeLoadingByConnection.value = {
      ...runtimeLoadingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const config = await client.runtime.getUserConfig()
      runtimeConfigsByConnection.value = {
        ...runtimeConfigsByConnection.value,
        [connectionId]: config,
      }
      runtimeDraftsByConnection.value = {
        ...runtimeDraftsByConnection.value,
        [connectionId]: createRuntimeConfigDraftsFromConfig(config).user,
      }
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: null,
      }
      return config
    } catch (cause) {
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load user runtime config',
      }
      return null
    } finally {
      runtimeLoadingByConnection.value = {
        ...runtimeLoadingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function validateCurrentUserRuntimeConfig(workspaceConnectionId?: string): Promise<RuntimeConfigValidationResult> {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return {
        valid: false,
        errors: ['Active workspace connection is unavailable'],
        warnings: [],
      }
    }
    const { client, connectionId } = resolvedClient
    runtimeValidatingByConnection.value = {
      ...runtimeValidatingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const patch = parseRuntimeConfigDraft('user', runtimeDraftsByConnection.value[connectionId] ?? '{}')
      const result = await client.runtime.validateUserConfig(patch)
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: result,
      }
      return result
    } catch (cause) {
      const result = {
        valid: false,
        errors: [cause instanceof Error ? cause.message : 'Failed to validate user runtime config'],
        warnings: [],
      } satisfies RuntimeConfigValidationResult
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: result,
      }
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: result.errors[0] ?? '',
      }
      return result
    } finally {
      runtimeValidatingByConnection.value = {
        ...runtimeValidatingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function saveCurrentUserRuntimeConfig(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    const validation = await validateCurrentUserRuntimeConfig(connectionId)
    if (!validation.valid) {
      return null
    }

    runtimeSavingByConnection.value = {
      ...runtimeSavingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const patch = parseRuntimeConfigDraft('user', runtimeDraftsByConnection.value[connectionId] ?? '{}')
      const config = await client.runtime.saveUserConfig(patch)
      runtimeConfigsByConnection.value = {
        ...runtimeConfigsByConnection.value,
        [connectionId]: config,
      }
      runtimeDraftsByConnection.value = {
        ...runtimeDraftsByConnection.value,
        [connectionId]: createRuntimeConfigDraftsFromConfig(config).user,
      }
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: config.validation,
      }
      return config
    } catch (cause) {
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to save user runtime config',
      }
      return null
    } finally {
      runtimeSavingByConnection.value = {
        ...runtimeSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  return {
    overview,
    users,
    roles,
    permissions,
    menus,
    currentUser,
    currentRoleRecords,
    currentRoleNames,
    currentPermissionRecords,
    currentEffectiveMenuIds,
    currentEffectiveMenus,
    availableUserCenterMenus,
    firstAccessibleUserCenterRouteName,
    error,
    runtimeConfig,
    runtimeDraft,
    runtimeValidation,
    runtimeLoading,
    runtimeSaving,
    runtimeValidating,
    runtimeError,
    profileSaving,
    profileError,
    passwordSaving,
    passwordError,
    load,
    createUser,
    updateUser,
    setProjectMembers,
    deleteUser,
    updateCurrentUserProfile,
    changeCurrentUserPassword,
    createRole,
    updateRole,
    deleteRole,
    createPermission,
    updatePermission,
    deletePermission,
    createMenu,
    updateMenu,
    hasPermission,
    setCurrentUserRuntimeDraft,
    loadCurrentUserRuntimeConfig,
    validateCurrentUserRuntimeConfig,
    saveCurrentUserRuntimeConfig,
  }
})
