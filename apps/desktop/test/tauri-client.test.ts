// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { ShellBootstrap } from '@octopus/schema'

const invokeMock = vi.fn()
const fetchMock = vi.fn()

function createHostBootstrap(overrides: Partial<ShellBootstrap> = {}): ShellBootstrap {
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
      ready: true,
      transport: 'http',
    },
    ...overrides,
  }
}

async function loadClientModule() {
  vi.resetModules()
  vi.doMock('@tauri-apps/api/core', () => ({
    invoke: invokeMock,
  }))

  return await import('@/tauri/client')
}

describe('desktop tauri client transport', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  })

  it('keeps shell bootstrap mock-first even inside tauri', async () => {
    invokeMock.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])

    expect(invokeMock).toHaveBeenCalledWith('bootstrap_shell')
    expect(fetchMock).not.toHaveBeenCalled()
    expect(payload.hostState.platform).toBe('tauri')
    expect(payload.backend?.transport).toBe('mock')
  })

  it('keeps shell bootstrap mock-first when desktop backend is unavailable', async () => {
    invokeMock.mockResolvedValue(createHostBootstrap({
      backend: {
        baseUrl: 'http://127.0.0.1:43127',
        authToken: 'desktop-test-token',
        ready: false,
        transport: 'http',
      },
    }))

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])

    expect(fetchMock).not.toHaveBeenCalled()
    expect(payload.hostState.platform).toBe('tauri')
    expect(payload.backend?.transport).toBe('mock')
  })

  it('keeps runtime bootstrap mock-first and does not call backend HTTP by default', async () => {
    invokeMock.mockResolvedValue(createHostBootstrap({
      preferences: {
        theme: 'light',
        locale: 'en-US',
        compactSidebar: false,
        leftSidebarCollapsed: false,
        rightSidebarCollapsed: false,
        defaultWorkspaceId: 'ws-local',
        lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
      },
    }))

    const client = await loadClientModule()
    const payload = await client.bootstrapRuntime()

    expect(payload.provider.provider).toBe('anthropic')
    expect(payload.sessions).toEqual([])
    expect(fetchMock).not.toHaveBeenCalled()
  })
})
