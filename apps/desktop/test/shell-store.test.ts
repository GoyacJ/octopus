// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { createMockWorkbenchSeed } from '@/mock/data'
import { useShellStore } from '@/stores/shell'

describe('useShellStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
  })

  it('bootstraps a web host fallback when Tauri is unavailable', async () => {
    const seed = createMockWorkbenchSeed()
    const store = useShellStore()

    await store.bootstrap(seed.currentWorkspaceId, seed.currentProjectId, seed.connections)

    expect(store.hostState?.platform).toBe('web')
    expect(store.preferences.defaultWorkspaceId).toBe(seed.currentWorkspaceId)
    expect(store.preferences.lastVisitedRoute).toBe(
      `/workspaces/${seed.currentWorkspaceId}/overview?project=${seed.currentProjectId}`,
    )
    expect(store.preferences.leftSidebarCollapsed).toBe(false)
    expect(store.preferences.rightSidebarCollapsed).toBe(false)
    expect(store.bootstrapConnections).toHaveLength(seed.connections.length)
  })

  it('syncs the detail focus and selected artifact from route state', () => {
    const store = useShellStore()

    store.syncFromRoute({
      detail: 'resources',
      artifact: 'art-roadmap',
    })

    expect(store.detailFocus).toBe('resources')
    expect(store.selectedArtifactId).toBe('art-roadmap')
  })

  it('migrates legacy compact sidebar preferences into the new left collapse flag', async () => {
    const seed = createMockWorkbenchSeed()
    const store = useShellStore()

    window.localStorage.setItem('octopus-shell-preferences', JSON.stringify({
      compactSidebar: true,
      locale: 'en-US',
    }))

    await store.bootstrap(seed.currentWorkspaceId, seed.currentProjectId, seed.connections)

    expect(store.preferences.compactSidebar).toBe(true)
    expect(store.preferences.leftSidebarCollapsed).toBe(true)
    expect(store.preferences.rightSidebarCollapsed).toBe(false)
    expect(store.preferences.locale).toBe('en-US')
  })

  it('toggles the shell chrome state for both rails and the search overlay', () => {
    const store = useShellStore()

    expect(store.searchOpen).toBe(false)
    expect(store.leftSidebarCollapsed).toBe(false)
    expect(store.rightSidebarCollapsed).toBe(false)

    store.toggleLeftSidebar()
    store.toggleRightSidebar()
    store.openSearch()

    expect(store.leftSidebarCollapsed).toBe(true)
    expect(store.rightSidebarCollapsed).toBe(true)
    expect(store.searchOpen).toBe(true)

    store.closeSearch()
    store.toggleLeftSidebar()
    store.toggleRightSidebar()

    expect(store.leftSidebarCollapsed).toBe(false)
    expect(store.rightSidebarCollapsed).toBe(false)
    expect(store.searchOpen).toBe(false)
  })
})
