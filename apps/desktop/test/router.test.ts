import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { router } from '@/router'
import { useWorkbenchStore } from '@/stores/workbench'

describe('desktop router contract', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
  })

  it('registers the GA workbench surfaces', () => {
    const routePaths = router.getRoutes().map((route) => route.path)

    expect(routePaths).toContain('/workspaces/:workspaceId/overview')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/dashboard')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/conversations')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/conversations/:conversationId')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/resources')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/knowledge')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/trace')
    expect(routePaths).toContain('/workspaces/:workspaceId/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/models')
    expect(routePaths).toContain('/workspaces/:workspaceId/tools')
    expect(routePaths).toContain('/workspaces/:workspaceId/teams')
    expect(routePaths).toContain('/workspaces/:workspaceId/settings')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center/profile')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center/users')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center/roles')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center/permissions')
    expect(routePaths).toContain('/workspaces/:workspaceId/user-center/menus')
    expect(routePaths).toContain('/workspaces/:workspaceId/automations')
    expect(routePaths).toContain('/connections')
  })

  it('redirects the legacy teams route into the merged agents surface', async () => {
    await router.push('/workspaces/ws-local/teams')

    expect(router.currentRoute.value.name).toBe('agents')
    expect(router.currentRoute.value.query.kind).toBe('team')
  })

  it('redirects the user center root to the first authorized child route', async () => {
    await router.push('/workspaces/ws-local/user-center')

    expect(router.currentRoute.value.name).toBe('user-center-profile')
  })

  it('blocks unauthorized user center child routes and falls back to profile', async () => {
    const workbench = useWorkbenchStore()

    workbench.switchCurrentUser('user-operator')

    await router.push('/workspaces/ws-local/user-center/roles')

    expect(router.currentRoute.value.name).toBe('user-center-profile')
  })
})
