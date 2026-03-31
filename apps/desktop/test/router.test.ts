import { describe, expect, it } from 'vitest'

import { router } from '@/router'

describe('desktop router contract', () => {
  it('registers the GA workbench surfaces', () => {
    const routePaths = router.getRoutes().map((route) => route.path)

    expect(routePaths).toContain('/workspaces/:workspaceId/dashboard')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/conversations/:conversationId')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/knowledge')
    expect(routePaths).toContain('/workspaces/:workspaceId/projects/:projectId/trace')
    expect(routePaths).toContain('/workspaces/:workspaceId/agents')
    expect(routePaths).toContain('/workspaces/:workspaceId/teams')
    expect(routePaths).toContain('/workspaces/:workspaceId/settings')
    expect(routePaths).toContain('/workspaces/:workspaceId/automations')
    expect(routePaths).toContain('/connections')
  })
})
