import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

describe('workspace selector contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('routes workspace dropdown changes to the target dashboard and keeps scoped data aligned', () => {
    const store = useWorkbenchStore()

    expect(createWorkspaceSwitchTarget(store.workspaces, 'ws-enterprise')).toEqual({
      name: 'dashboard',
      params: { workspaceId: 'ws-enterprise' },
      query: { project: 'proj-launch' },
    })

    store.selectWorkspace('ws-enterprise')

    expect(store.workspaceProjects.map((project) => project.id)).toEqual(['proj-launch'])
    expect(store.workspaceInbox.every((item) => item.workspaceId === 'ws-enterprise')).toBe(true)
    expect(store.projectKnowledge.every((entry) => entry.projectId === 'proj-launch')).toBe(true)
    expect(store.workspaceAgents.some((agent) => agent.id === 'agent-gov')).toBe(true)
    expect(store.workspaceTeams.every((team) => team.workspaceId === 'ws-enterprise')).toBe(true)
  })
})
