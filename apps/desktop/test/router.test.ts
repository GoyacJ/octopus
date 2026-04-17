// @vitest-environment jsdom

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { router } from '@/router'
import { getRouteMenuId } from '@/navigation/menuRegistry'
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
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/deliverables')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/resources')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/knowledge')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/trace')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/settings')
    expect(routePaths).toContain('/workspaces/:workspaceId/teams')
    expect(routePaths).toContain('/settings')
    expect(routePaths).toContain('/connections')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center/profile')
    expect(routePaths).toContain('/workspaces/:workspaceId/personal-center/pet')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/members')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/access')
    expect(routePaths).toContain('/workspaces/:workspaceId/access-control/governance')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/users')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/org')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/roles')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/policies')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/menus')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/resources')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/access-control/sessions')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/users')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/roles')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/permissions')
    expect(routePaths).not.toContain('/workspaces/:workspaceId/permission-center/menus')
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
    await workspaceAccessControlStore.loadGovernanceData()

    await router.push('/workspaces/ws-local/console')

    expect(router.currentRoute.value.name).toBe('workspace-console-projects')
  })

  it('redirects unauthorized console routes back to workspace overview', async () => {
    const shell = useShellStore()
    const workspaceAccessControlStore = useWorkspaceAccessControlStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    await workspaceAccessControlStore.loadGovernanceData()

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

  it('keeps project deliverables on the dedicated project route', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/deliverables')

    expect(router.currentRoute.value.name).toBe('project-deliverables')
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

  it('redirects removed project runtime deep links through the existing fallback route', async () => {
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

    expect(router.currentRoute.value.name).toBe('workspace-overview')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
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
    await workspaceAccessControlStore.loadExperience()

    await router.push('/workspaces/ws-local/access-control')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-members')
  })

  it('redirects denied governance routes back to the recommended access section', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/access-control/governance')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-members')
  })

  it('redirects personal workspaces to the access section instead of governance or legacy members scaffolding', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-owner'
        state.users = state.users.filter(user => user.id === 'user-owner')
        state.userOrgAssignments = state.userOrgAssignments.filter(assignment => assignment.userId === 'user-owner')
        state.roleBindings = state.roleBindings.filter(binding => binding.subjectId === 'user-owner')
        state.dataPolicies = []
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/access-control')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-access')
  })

  it('redirects denied access routes to governance instead of stalling on the members preload', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        state.roles = state.roles.map((role) => {
          if (role.id !== 'role-operator') {
            return role
          }

          return {
            ...role,
            permissionCodes: ['access.org.read', 'audit.read'],
          }
        })
        state.dataPolicies = [{
          id: 'policy-owner-confidential',
          name: 'Owner confidential access',
          subjectType: 'user',
          subjectId: 'user-owner',
          resourceType: 'resource',
          scopeType: 'tag-match',
          projectIds: [],
          tags: ['confidential'],
          classifications: [],
          effect: 'allow',
        }]
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/access-control/access')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-governance')
  })

  it('allows access-center routes even when the access menu is hidden by menu policy', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.menuPolicies = [{
          menuId: 'menu-workspace-access-control',
          enabled: true,
          order: 100,
          group: 'Security',
          visibility: 'hidden',
        }]
      },
    })

    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')

    await router.push('/workspaces/ws-local/access-control/members')

    expect(router.currentRoute.value.name).toBe('workspace-access-control-members')
  })

  it('maps the progressive access routes to the root access-control menu only', () => {
    expect(getRouteMenuId('workspace-access-control')).toBe('menu-workspace-access-control')
    expect(getRouteMenuId('workspace-access-control-members')).toBe('menu-workspace-access-control')
    expect(getRouteMenuId('workspace-access-control-access')).toBe('menu-workspace-access-control')
    expect(getRouteMenuId('workspace-access-control-governance')).toBe('menu-workspace-access-control')
    expect(getRouteMenuId('workspace-access-control-users')).toBeUndefined()
    expect(getRouteMenuId('workspace-access-control-org')).toBeUndefined()
    expect(getRouteMenuId('workspace-access-control-roles')).toBeUndefined()
  })

  it('removes legacy access child menu permission mappings from the desktop fixture layer', () => {
    const fixtureClient = readFileSync(
      resolve(import.meta.dirname, './support/workspace-fixture-client.ts'),
      'utf8',
    )

    expect(fixtureClient).not.toContain('menu-workspace-access-control-users')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-org')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-roles')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-policies')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-menus')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-resources')
    expect(fixtureClient).not.toContain('menu-workspace-access-control-sessions')
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
