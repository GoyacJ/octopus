// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { router } from '@/router'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
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
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/users')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/org')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/roles')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/policies')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/menus')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/resources')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/sessions')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/users')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/roles')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/permissions')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/menus')
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
    const workspaceAccessControlStore = useWorkspaceAccessControlStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessControlStore.load()

    await router.push('/workspaces/ws-local/console')

    expect(router.currentRoute.value.name).toBe('workspace-console-projects')
  })

  it('redirects unauthorized console routes back to workspace overview', async () => {
    const shell = useShellStore()
    const workspaceAccessControlStore = useWorkspaceAccessControlStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessControlStore.load()

    if (!workspaceAccessControlStore.authorization) {
      throw new Error('Expected access-control authorization in fixture')
    }
    workspaceAccessControlStore.authorization.visibleMenuIds = workspaceAccessControlStore.authorization.visibleMenuIds
      .filter(menuId => !menuId.startsWith('menu-workspace-console'))
    workspaceAccessControlStore.authorization.menuGates = workspaceAccessControlStore.authorization.menuGates
      .map(gate => gate.menuId.startsWith('menu-workspace-console')
        ? { ...gate, allowed: false, reason: 'removed in test' }
        : gate)

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

  it('redirects non-members away from project routes', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner']
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/projects/proj-redesign/settings')

    expect(router.currentRoute.value.name).toBe('workspace-overview')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
  })

  it('redirects non-owners away from project governance routes', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner', 'user-operator']
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/projects/proj-redesign/runtime')

    expect(router.currentRoute.value.name).toBe('project-dashboard')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')
  })

  it('redirects denied project modules back to the project dashboard', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner']
        ;(project as any).permissionOverrides = {
          agents: 'inherit',
          resources: 'deny',
          tools: 'inherit',
          knowledge: 'inherit',
        }
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/projects/proj-redesign/resources')

    expect(router.currentRoute.value.name).toBe('project-dashboard')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')
  })

  it('redirects the access control root to the first authorized child route', async () => {
    const shell = useShellStore()
    const workspaceAccessControlStore = useWorkspaceAccessControlStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessControlStore.load()

    await router.push('/workspaces/ws-local/access-control')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-users')
  })

  it('rejects legacy permission center deep links because the routes are removed', async () => {
    const resolved = router.resolve('/workspaces/ws-local/permission-center/roles')

    expect(resolved.matched).toHaveLength(1)
    expect(resolved.matched[0]?.path).toBe('/:pathMatch(.*)*')
    await router.push('/workspaces/ws-local/permission-center/roles')
    expect(router.currentRoute.value.matched).toHaveLength(1)
    expect(router.currentRoute.value.matched[0]?.path).toBe('/workspaces/:workspaceId/overview')
    expect(router.currentRoute.value.name).toBe('workspace-overview')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
  })

  it('redirects the personal center root to the profile route', async () => {
    await router.push('/workspaces/ws-local/personal-center')

    expect(router.currentRoute.value.name).toBe('workspace-personal-center-profile')
  })
})
