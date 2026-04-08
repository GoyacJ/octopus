import { createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { describe, expect, it } from 'vitest'

describe('workspace selector contract', () => {
  it('routes workspace dropdown changes to the workspace overview with the default project scope', () => {
    expect(createWorkspaceSwitchTarget([
      { id: 'ws-local', defaultProjectId: 'proj-redesign' },
      { id: 'ws-enterprise', defaultProjectId: 'proj-launch' },
    ], 'ws-enterprise')).toEqual({
      name: 'workspace-overview',
      params: { workspaceId: 'ws-enterprise' },
      query: { project: 'proj-launch' },
    })
  })
})
