// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { router } from '@/router'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

describe('desktop router contract', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
  })

  it('registers the GA workbench surfaces', () => {
    const routePaths = router.getRoutes().map((route) => route.path)

    expect(routePaths).toContain('/workspaces/:workspaceId/overview')
    expect(routePaths).toContain('/workspaces/:workspaceId/console')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/projects')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/knowledge')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/resources')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/models')
    expect(routePaths).toContain('/workspaces/:workspaceId/console/tools')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/dashboard')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/conversations')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/conversations/:conversationId')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/resources')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/knowledge')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/trace')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/settings')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/runtime')
    expect(routePaths).toContain('/workspaces/:workspaceId/teams')
    expect(routePaths).toContain('/settings')
    expect(routePaths).toContain('/connections')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center/profile')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center/pet')
    expect(routePaths).toContain('/workspaces/:workspaceId/permission-center')
    expect(routePaths).toContain('/workspaces/:workspaceId/permission-center/users')
    expect(routePaths).toContain('/workspaces/:workspaceId/permission-center/roles')
    expect(routePaths).toContain('/workspaces/:workspaceId/permission-center/permissions')
    expect(routePaths).toContain('/workspaces/:workspaceId/permission-center/menus')
    expect(routePaths).toContain('/workspaces/:workspaceId/automations')
    expect(routePaths).toContain('/connections')
  })

  it('redirects workspace teams to the team tab in the agent center', async () => {
    await router.push('/workspaces/ws-local/teams')

    expect(router.currentRoute.value.name).toBe('workspace-console-agents')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.query.tab).toBe('team')
  })

  it('keeps project management on the console route', async () => {
    await router.push('/workspaces/ws-local/console/projects')

    expect(router.currentRoute.value.name).toBe('workspace-console-projects')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
  })

  it('redirects the console root to the first authorized child route', async () => {
    const shell = useShellStore()
    const workspaceAccessStore = useWorkspaceAccessStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessStore.load()

    await router.push('/workspaces/ws-local/console')

    expect(router.currentRoute.value.name).toBe('workspace-console-projects')
  })

  it('redirects unauthorized console routes back to workspace overview', async () => {
    const shell = useShellStore()
    const workspaceAccessStore = useWorkspaceAccessStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessStore.load()

    const ownerRole = workspaceAccessStore.roles.find(role => role.id === 'role-owner')
    if (!ownerRole) {
      throw new Error('Expected owner role in fixture')
    }
    ownerRole.menuIds = ownerRole.menuIds.filter(menuId => !menuId.startsWith('menu-workspace-console'))

    await router.push('/workspaces/ws-local/console/projects')

    expect(router.currentRoute.value.name).toBe('workspace-overview')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
  })

  it('keeps project settings on the dedicated project route', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/settings')

    expect(router.currentRoute.value.name).toBe('project-settings')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')
  })

  it('redirects the permission center root to the first authorized child route', async () => {
    const shell = useShellStore()
    const workspaceAccessStore = useWorkspaceAccessStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessStore.load()

    await router.push('/workspaces/ws-local/permission-center')

    expect(router.currentRoute.value.name).toBe('workspace-permission-center-users')
  })

  it('redirects the personal center root to the profile route', async () => {
    await router.push('/workspaces/ws-local/personal-center')

    expect(router.currentRoute.value.name).toBe('workspace-personal-center-profile')
  })
})
