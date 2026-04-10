import { afterEach, beforeEach, vi } from 'vitest'

import type {
  NotificationRecord,
  ShellBootstrap,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

export const invokeSpy = vi.fn()
export const fetchSpy = vi.fn()

export function createHostBootstrap(overrides: Partial<ShellBootstrap> = {}): ShellBootstrap {
  return {
    hostState: {
      platform: 'tauri',
      mode: 'local',
      appVersion: '0.1.0-test',
      cargoWorkspace: true,
      shell: 'tauri2',
    },
    preferences: {
      theme: 'system',
      locale: 'zh-CN',
      compactSidebar: false,
      leftSidebarCollapsed: false,
      rightSidebarCollapsed: false,
      updateChannel: 'formal',
      fontSize: 16,
      fontFamily: 'Inter, sans-serif',
      fontStyle: 'sans',
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
    },
    connections: [
      {
        id: 'conn-local',
        mode: 'local',
        label: 'Local Runtime',
        workspaceId: 'ws-local',
        state: 'local-ready',
      },
    ],
    backend: {
      baseUrl: 'http://127.0.0.1:43127',
      authToken: 'desktop-test-token',
      state: 'ready',
      transport: 'http',
    },
    ...overrides,
  }
}

export function createWorkspaceSession(
  connection: WorkspaceConnectionRecord,
  overrides: Partial<WorkspaceSessionTokenEnvelope> = {},
): WorkspaceSessionTokenEnvelope {
  return {
    workspaceConnectionId: connection.workspaceConnectionId,
    token: 'workspace-session-token',
    issuedAt: 1,
    session: {
      id: 'sess-1',
      workspaceId: connection.workspaceId,
      userId: 'user-owner',
      clientAppId: 'octopus-desktop',
      token: 'workspace-session-token',
      status: 'active',
      createdAt: 1,
      expiresAt: undefined,
      roleIds: ['owner'],
      scopeProjectIds: [],
    },
    ...overrides,
  }
}

export function firstRequest(): RequestInit {
  const calls = (fetchSpy as unknown as { ['mock']: { calls: unknown[][] } })['mock'].calls
  return calls[0]?.[1] as RequestInit
}

export function createNotificationRecord(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
  return {
    id: 'notif-1',
    scopeKind: 'app',
    level: 'info',
    title: 'Saved',
    body: 'Preferences updated.',
    source: 'settings',
    createdAt: 1,
    readAt: undefined,
    toastVisibleUntil: undefined,
    scopeOwnerId: undefined,
    routeTo: undefined,
    actionLabel: undefined,
    ...overrides,
  }
}

export function createHostUpdateStatus(overrides: Record<string, unknown> = {}) {
  return {
    currentVersion: '0.2.0',
    currentChannel: 'formal',
    state: 'idle',
    latestRelease: null,
    lastCheckedAt: null,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
    ...overrides,
  }
}

export async function loadClientModule() {
  vi.resetModules()
  vi.doMock('@tauri-apps/api/core', () => ({
    invoke: invokeSpy,
  }))

  return await import('@/tauri/client')
}

export function installTauriClientTestHooks() {
  beforeEach(() => {
    invokeSpy.mockReset()
    fetchSpy.mockReset()
    vi.stubGlobal('fetch', fetchSpy)
    vi.unstubAllEnvs()
    vi.stubEnv('VITE_HOST_RUNTIME', 'tauri')
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.unstubAllEnvs()
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  })
}
