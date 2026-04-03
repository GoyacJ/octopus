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
      state: 'ready',
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
        state: 'unavailable',
        transport: 'http',
      },
    }))

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])

    expect(fetchMock).not.toHaveBeenCalled()
    expect(payload.hostState.platform).toBe('tauri')
    expect(payload.backend?.transport).toBe('mock')
  })

  it('falls back to mock runtime when desktop backend is unavailable', async () => {
    invokeMock.mockResolvedValue(createHostBootstrap({
      backend: {
        baseUrl: 'http://127.0.0.1:43127',
        authToken: 'desktop-test-token',
        state: 'unavailable',
        transport: 'http',
      },
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

  it('uses desktop backend HTTP for runtime calls when tauri backend is ready', async () => {
    invokeMock.mockResolvedValue(createHostBootstrap())
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        provider: {
          provider: 'anthropic',
          defaultModel: 'claude-sonnet-4-5',
        },
        sessions: [{
          id: 'runtime-session-conv-1',
          conversationId: 'conv-1',
          projectId: 'proj-1',
          title: 'Conversation',
          status: 'completed',
          updatedAt: 1,
        }],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapRuntime()

    expect(fetchMock).toHaveBeenCalledWith('http://127.0.0.1:43127/runtime/bootstrap', expect.objectContaining({
      method: 'GET',
      headers: expect.any(Headers),
    }))
    expect(payload.sessions).toHaveLength(1)
    expect(payload.sessions[0]?.id).toBe('runtime-session-conv-1')
  })

})
