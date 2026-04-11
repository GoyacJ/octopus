// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useShellStore } from '@/stores/shell'

vi.mock('@/tauri/client', async () => {
  const actual = await vi.importActual<typeof import('@/tauri/client')>('@/tauri/client')
  return {
    ...actual,
    bootstrapShellHost: vi.fn(),
    createWorkspaceConnection: vi.fn(),
    deleteWorkspaceConnection: vi.fn(),
  }
})

import { bootstrapShellHost } from '@/tauri/client'
import { createWorkspaceConnection, deleteWorkspaceConnection } from '@/tauri/client'

const bootstrapShellHostMock = vi.mocked(bootstrapShellHost)
const createWorkspaceConnectionMock = vi.mocked(createWorkspaceConnection)
const deleteWorkspaceConnectionMock = vi.mocked(deleteWorkspaceConnection)

const testConnections = [
  {
    id: 'conn-local',
    mode: 'local' as const,
    label: 'Local Workspace',
    workspaceId: 'ws-local',
    state: 'local-ready' as const,
  },
]

describe('useShellStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    bootstrapShellHostMock.mockReset()
    createWorkspaceConnectionMock.mockReset()
    deleteWorkspaceConnectionMock.mockReset()
  })

  it('enters an explicit host failure state when shell bootstrap fails', async () => {
    const store = useShellStore()
    bootstrapShellHostMock.mockRejectedValue(new Error('Host runtime unavailable'))

    await store.bootstrap('ws-local', 'proj-redesign', testConnections)

    expect(store.error).toContain('Host runtime unavailable')
    expect(store.hostState.platform).toBe('tauri')
    expect(store.preferences.defaultWorkspaceId).toBe('ws-local')
    expect(store.preferences.lastVisitedRoute).toBe(
      '/workspaces/ws-local/overview?project=proj-redesign',
    )
    expect(store.bootstrapConnections).toEqual([])
    expect(store.workspaceConnections).toEqual([])
    expect(store.activeWorkspaceConnectionId).toBe('')
  })

  it('does not read local storage preferences as a bootstrap fallback when host bootstrap fails', async () => {
    const store = useShellStore()
    bootstrapShellHostMock.mockRejectedValue(new Error('Host runtime unavailable'))

    window.localStorage.setItem('octopus-shell-preferences', JSON.stringify({
      compactSidebar: true,
      locale: 'en-US',
      defaultWorkspaceId: 'ws-shadow',
      lastVisitedRoute: '/workspaces/ws-shadow/overview?project=proj-shadow',
    }))

    await store.bootstrap('ws-local', 'proj-redesign', testConnections)

    expect(store.preferences.compactSidebar).toBe(false)
    expect(store.preferences.leftSidebarCollapsed).toBe(false)
    expect(store.preferences.locale).toBe('zh-CN')
    expect(store.preferences.defaultWorkspaceId).toBe('ws-local')
    expect(store.preferences.lastVisitedRoute).toBe(
      '/workspaces/ws-local/overview?project=proj-redesign',
    )
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

  it('does not synthesize workspace connections that were not provided by the host plane', async () => {
    const store = useShellStore()

    await store.activateWorkspaceByWorkspaceId('ws-enterprise')

    expect(store.activeWorkspaceConnectionId).toBe('')
    expect(store.activeWorkspaceConnection).toBeNull()
  })

  it('persists workspace sessions per connection without leaking across connections', () => {
    const store = useShellStore()

    store.setWorkspaceSession({
      workspaceConnectionId: 'conn-local',
      token: 'token-local',
      issuedAt: 1,
      session: {
        id: 'sess-local',
        workspaceId: 'ws-local',
        userId: 'user-owner',
        clientAppId: 'octopus-desktop',
        token: 'token-local',
        status: 'active',
        createdAt: 1,
        expiresAt: undefined,
      },
    })
    store.setWorkspaceSession({
      workspaceConnectionId: 'conn-enterprise',
      token: 'token-enterprise',
      issuedAt: 2,
      session: {
        id: 'sess-enterprise',
        workspaceId: 'ws-enterprise',
        userId: 'user-owner',
        clientAppId: 'octopus-desktop',
        token: 'token-enterprise',
        status: 'active',
        createdAt: 2,
        expiresAt: undefined,
      },
    })

    expect(store.workspaceSessionsState['conn-local']?.token).toBe('token-local')
    expect(store.workspaceSessionsState['conn-enterprise']?.token).toBe('token-enterprise')

    store.clearWorkspaceSession('conn-local')

    expect(store.workspaceSessionsState['conn-local']).toBeUndefined()
    expect(store.workspaceSessionsState['conn-enterprise']?.token).toBe('token-enterprise')
  })

  it('adds a persisted remote workspace connection without switching the active connection before login state is stored', async () => {
    const store = useShellStore()
    store.workspaceConnectionsState = [{
      workspaceConnectionId: 'conn-local',
      workspaceId: 'ws-local',
      label: 'Local Workspace',
      baseUrl: 'http://127.0.0.1:43127',
      transportSecurity: 'loopback',
      authMode: 'session-token',
      status: 'connected',
    }]
    store.activeWorkspaceConnectionId = 'conn-local'
    createWorkspaceConnectionMock.mockResolvedValue({
      workspaceConnectionId: 'conn-enterprise',
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
      status: 'connected',
    })

    const created = await store.createWorkspaceConnection({
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
    })

    expect(created.workspaceConnectionId).toBe('conn-enterprise')
    expect(store.workspaceConnections.map(item => item.workspaceConnectionId)).toEqual([
      'conn-local',
      'conn-enterprise',
    ])
    expect(store.activeWorkspaceConnectionId).toBe('conn-local')
  })

  it('deletes a remote workspace connection, clears its session, and falls back to local when it was active', async () => {
    const store = useShellStore()
    store.workspaceConnectionsState = [
      {
        workspaceConnectionId: 'conn-local',
        workspaceId: 'ws-local',
        label: 'Local Workspace',
        baseUrl: 'http://127.0.0.1:43127',
        transportSecurity: 'loopback',
        authMode: 'session-token',
        status: 'connected',
      },
      {
        workspaceConnectionId: 'conn-enterprise',
        workspaceId: 'ws-enterprise',
        label: 'Enterprise Workspace',
        baseUrl: 'https://enterprise.example.test',
        transportSecurity: 'trusted',
        authMode: 'session-token',
        status: 'connected',
      },
    ]
    store.activeWorkspaceConnectionId = 'conn-enterprise'
    store.workspaceSessionsState = {
      'conn-enterprise': {
        workspaceConnectionId: 'conn-enterprise',
        token: 'token-enterprise',
        issuedAt: 2,
        session: {
          id: 'sess-enterprise',
          workspaceId: 'ws-enterprise',
          userId: 'user-owner',
          clientAppId: 'octopus-desktop',
          token: 'token-enterprise',
          status: 'active',
          createdAt: 2,
          expiresAt: undefined,
        },
      },
    }
    deleteWorkspaceConnectionMock.mockResolvedValue()

    await store.deleteWorkspaceConnection('conn-enterprise')

    expect(store.workspaceConnections.map(item => item.workspaceConnectionId)).toEqual(['conn-local'])
    expect(store.workspaceSessionsState['conn-enterprise']).toBeUndefined()
    expect(store.activeWorkspaceConnectionId).toBe('conn-local')
  })
})
