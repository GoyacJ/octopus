import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { createMockWorkbenchSeed } from '@/mock/data'
import { useShellStore } from '@/stores/shell'

describe('useShellStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('bootstraps a web host fallback when Tauri is unavailable', async () => {
    const seed = createMockWorkbenchSeed()
    const store = useShellStore()

    await store.bootstrap(seed.currentWorkspaceId, seed.currentProjectId, seed.connections)

    expect(store.hostState?.platform).toBe('web')
    expect(store.preferences.defaultWorkspaceId).toBe(seed.currentWorkspaceId)
    expect(store.preferences.lastVisitedRoute).toBe(
      `/workspaces/${seed.currentWorkspaceId}/dashboard?project=${seed.currentProjectId}`,
    )
    expect(store.bootstrapConnections).toHaveLength(seed.connections.length)
  })

  it('syncs the context pane and selected artifact from route state', () => {
    const store = useShellStore()

    store.syncFromRoute({
      pane: 'artifacts',
      artifact: 'art-roadmap',
    })

    expect(store.contextPane).toBe('artifacts')
    expect(store.selectedArtifactId).toBe('art-roadmap')
  })
})
