import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  MenuRecord,
  PermissionRecord,
  RoleRecord,
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

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const overview = computed(() => overviews.value[activeConnectionId.value] ?? null)
  const users = computed(() => usersByConnection.value[activeConnectionId.value] ?? [])
  const roles = computed(() => rolesByConnection.value[activeConnectionId.value] ?? [])
  const permissions = computed(() => permissionsByConnection.value[activeConnectionId.value] ?? [])
  const menus = computed(() => menusByConnection.value[activeConnectionId.value] ?? [])
  const currentUser = computed(() => overview.value?.currentUser ?? users.value[0] ?? null)
  const currentRoleNames = computed(() => overview.value?.roleNames ?? roles.value.filter(role => currentUser.value?.roleIds.includes(role.id)).map(role => role.name))
  const currentEffectiveMenuIds = computed(() => {
    const menuIds = new Set<string>()
    for (const role of roles.value) {
      if (currentUser.value?.roleIds.includes(role.id)) {
        role.menuIds.forEach(menuId => menuIds.add(menuId))
      }
    }
    if (!menuIds.size) {
      menus.value.forEach((menu) => {
        if (menu.status === 'active') {
          menuIds.add(menu.id)
        }
      })
    }
    return [...menuIds]
  })
  const availableUserCenterMenus = computed(() =>
    menus.value
      .filter(menu => menu.source === 'user-center' && currentEffectiveMenuIds.value.includes(menu.id))
      .sort((left, right) => left.order - right.order),
  )
  const firstAccessibleUserCenterRouteName = computed(() => {
    const firstMenu = availableUserCenterMenus.value.find(menu => menu.routeName && getMenuDefinition(menu.id)?.routeName)
    return firstMenu?.routeName ?? null
  })
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

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

  async function createUser(record: UserRecordSummary) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.rbac.createUser(record)
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: [...(usersByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateUser(userId: string, record: UserRecordSummary) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.rbac.updateUser(userId, record)
    usersByConnection.value = {
      ...usersByConnection.value,
      [connectionId]: (usersByConnection.value[connectionId] ?? []).map(item => item.id === userId ? updated : item),
    }
    return updated
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

  return {
    overview,
    users,
    roles,
    permissions,
    menus,
    currentUser,
    currentRoleNames,
    currentEffectiveMenuIds,
    availableUserCenterMenus,
    firstAccessibleUserCenterRouteName,
    error,
    load,
    createUser,
    updateUser,
    createRole,
    updateRole,
    createPermission,
    updatePermission,
    createMenu,
    updateMenu,
    hasPermission,
  }
})
