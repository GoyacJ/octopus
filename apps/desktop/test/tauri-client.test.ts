// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  ApiErrorEnvelope,
  RegisterWorkspaceOwnerRequest,
  RuntimeConfigPatch,
  ShellBootstrap,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

const invokeSpy = vi.fn()
const fetchSpy = vi.fn()

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

function createWorkspaceSession(
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

function firstRequest(): RequestInit {
  const calls = (fetchSpy as unknown as { ['mock']: { calls: unknown[][] } })['mock'].calls
  return calls[0]?.[1] as RequestInit
}

async function loadClientModule() {
  vi.resetModules()
  vi.doMock('@tauri-apps/api/core', () => ({
    invoke: invokeSpy,
  }))

  return await import('@/tauri/client')
}

describe('host client transport', () => {
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

  it('exposes host backend metadata without turning it into the business transport', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])

    expect(invokeSpy).toHaveBeenCalledWith('bootstrap_shell')
    expect(fetchSpy).not.toHaveBeenCalled()
    expect(payload.hostState.platform).toBe('tauri')
    expect(payload.backend?.transport).toBe('http')
    expect(payload.workspaceConnections?.[0]).toMatchObject({
      workspaceConnectionId: 'conn-local',
      workspaceId: 'ws-local',
      transportSecurity: 'loopback',
      authMode: 'session-token',
      status: 'connected',
    })
  })

  it('does not expose the removed legacy runtime helper exports', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()

    expect('bootstrapRuntime' in client).toBe(false)
    expect('createRuntimeSession' in client).toBe(false)
    expect('loadRuntimeSession' in client).toBe(false)
    expect('listRuntimeSessions' in client).toBe(false)
    expect('pollRuntimeEvents' in client).toBe(false)
    expect('resolveRuntimeApproval' in client).toBe(false)
    expect('submitRuntimeUserTurn' in client).toBe(false)
  })

  it('requires a workspace session token before workspace-plane calls can be made', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]

    expect(connection).toBeDefined()

    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    await expect(workspaceClient.workspace.get()).rejects.toThrow(/workspace session/i)
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('uses the workspace HTTP protocol and workspace session token for authenticated read calls', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'ws-local',
        name: 'Local Workspace',
        slug: 'local-workspace',
        deployment: 'local',
        bootstrapStatus: 'ready',
        host: '127.0.0.1',
        listenAddress: 'http://127.0.0.1:43127',
        defaultProjectId: 'proj-redesign',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session,
    })

    const workspace = await workspaceClient.workspace.get()

    expect(workspace.name).toBe('Local Workspace')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
    expect(headers.get('X-Request-Id')).toMatch(/^req-/)
  })

  it('submits first-owner registration through the public auth endpoint without an existing session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        session: {
          id: 'sess-owner',
          workspaceId: 'ws-local',
          userId: 'user-owner',
          clientAppId: 'octopus-desktop',
          token: 'token-owner',
          status: 'active',
          createdAt: 1,
          roleIds: ['owner'],
          scopeProjectIds: [],
        },
        workspace: {
          id: 'ws-local',
          name: 'Local Workspace',
          slug: 'local-workspace',
          deployment: 'local',
          bootstrapStatus: 'ready',
          ownerUserId: 'user-owner',
          host: '127.0.0.1',
          listenAddress: 'http://127.0.0.1:43127',
          defaultProjectId: 'proj-redesign',
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    const requestBody: RegisterWorkspaceOwnerRequest = {
      clientAppId: 'octopus-desktop',
      username: 'owner',
      displayName: 'Workspace Owner',
      password: 'owner-owner',
      confirmPassword: 'owner-owner',
      avatar: {
        fileName: 'owner-avatar.png',
        contentType: 'image/png',
        dataBase64: 'iVBORw0KGgo=',
        byteSize: 8,
      },
    }

    const response = await workspaceClient.auth.registerOwner(requestBody)

    expect(response.session.userId).toBe('user-owner')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/auth/register-owner',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })

  it('throws a typed API error for workspace auth failures', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: false,
      status: 401,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<ApiErrorEnvelope> => ({
        error: {
          code: 'SESSION_EXPIRED',
          message: 'session expired',
          details: null,
          requestId: 'req-auth-1',
          retryable: false,
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session,
    })

    await expect(workspaceClient.workspace.get()).rejects.toMatchObject({
      code: 'SESSION_EXPIRED',
      status: 401,
      requestId: 'req-auth-1',
      retryable: false,
    })
  })

  it('normalizes permission mode and forwards idempotency headers on runtime write requests', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'runtime-run-conv-1',
        sessionId: 'runtime-session-conv-1',
        conversationId: 'conv-1',
        status: 'completed',
        currentStep: 'runtime.run.completed',
        startedAt: 1,
        updatedAt: 2,
        modelId: 'claude-sonnet-4-5',
        nextAction: 'runtime.run.idle',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.runtime.submitUserTurn('runtime-session-conv-1', {
      content: 'hello',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
    }, 'idem-turn-1')

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(JSON.parse(String(request.body))).toMatchObject({
      content: 'hello',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'workspace-write',
    })
    expect(headers.get('Idempotency-Key')).toBe('idem-turn-1')
  })

  it('preserves danger-full-access for authenticated runtime requests', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'runtime-run-conv-2',
        sessionId: 'runtime-session-conv-2',
        conversationId: 'conv-2',
        status: 'completed',
        currentStep: 'runtime.run.completed',
        startedAt: 1,
        updatedAt: 2,
        modelId: 'claude-sonnet-4-5',
        nextAction: 'runtime.run.idle',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.runtime.submitUserTurn('runtime-session-conv-2', {
      content: 'hello',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'danger-full-access',
    })

    const request = firstRequest()
    expect(JSON.parse(String(request.body))).toMatchObject({
      permissionMode: 'danger-full-access',
    })
  })

  it('loads runtime config through the workspace API without requiring a workspace session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        effectiveConfig: {
          model: 'claude-sonnet-4-5',
        },
        effectiveConfigHash: 'cfg-hash-1',
        sources: [
          {
            scope: 'workspace',
            displayPath: 'config/runtime/workspace.json',
            sourceKey: 'workspace',
            exists: true,
            loaded: true,
            contentHash: 'src-hash-1',
            document: {
              model: 'claude-sonnet-4-5',
            },
          },
        ],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    const config = await workspaceClient.runtime.getConfig()

    expect(config.effectiveConfigHash).toBe('cfg-hash-1')
    expect(config.sources[0]).toMatchObject({
      scope: 'workspace',
      displayPath: 'config/runtime/workspace.json',
      sourceKey: 'workspace',
    })
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })

  it('posts runtime config validation requests to the workspace API without requiring a workspace session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        valid: true,
        errors: [],
        warnings: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    const patch: RuntimeConfigPatch = {
      scope: 'workspace',
      patch: {
        model: 'claude-sonnet-4-5',
      },
    }

    const result = await workspaceClient.runtime.validateConfig(patch)

    expect(result.valid).toBe(true)
    const request = firstRequest()
    expect(JSON.parse(String(request.body))).toMatchObject({
      scope: 'workspace',
      patch: {
        model: 'claude-sonnet-4-5',
      },
    })
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config/validate',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
  })

  it('patches runtime config scopes through the workspace API without requiring a workspace session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        effectiveConfig: {
          model: 'claude-sonnet-4-5',
          permissions: {
            defaultMode: 'plan',
          },
        },
        effectiveConfigHash: 'cfg-hash-2',
        sources: [
          {
            scope: 'workspace',
            displayPath: 'config/runtime/workspace.json',
            sourceKey: 'workspace',
            exists: true,
            loaded: true,
            contentHash: 'src-hash-2',
            document: {
              model: 'claude-sonnet-4-5',
            },
          },
        ],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    await workspaceClient.runtime.saveConfig({
      scope: 'workspace',
      patch: {
        permissions: {
          defaultMode: 'plan',
        },
      },
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config/scopes/workspace',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
  })

  it('uses authenticated project runtime config endpoints for project-scoped overrides', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        effectiveConfig: { model: 'claude-sonnet-4-5' },
        effectiveConfigHash: 'cfg-project',
        sources: [
          {
            scope: 'workspace',
            displayPath: 'config/runtime/workspace.json',
            sourceKey: 'workspace',
            exists: true,
            loaded: true,
          },
          {
            scope: 'project',
            ownerId: 'proj-redesign',
            displayPath: 'config/runtime/projects/proj-redesign.json',
            sourceKey: 'project:proj-redesign',
            exists: true,
            loaded: true,
          },
        ],
        validation: { valid: true, errors: [], warnings: [] },
        secretReferences: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.runtime.getProjectConfig('proj-redesign')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/runtime-config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('uses authenticated user runtime config endpoints for user-scoped overrides', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        effectiveConfig: { model: 'claude-sonnet-4-5' },
        effectiveConfigHash: 'cfg-user',
        sources: [
          {
            scope: 'workspace',
            displayPath: 'config/runtime/workspace.json',
            sourceKey: 'workspace',
            exists: true,
            loaded: true,
          },
          {
            scope: 'user',
            ownerId: 'user-owner',
            displayPath: 'config/runtime/users/user-owner.json',
            sourceKey: 'user:user-owner',
            exists: true,
            loaded: true,
          },
        ],
        validation: { valid: true, errors: [], warnings: [] },
        secretReferences: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.runtime.getUserConfig()

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/user-center/profile/runtime-config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('uses browser host HTTP endpoints when VITE_HOST_RUNTIME=browser', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => createHostBootstrap({
        hostState: {
          platform: 'web',
          mode: 'local',
          appVersion: '0.1.0-test',
          cargoWorkspace: true,
          shell: 'browser',
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign')

    expect(invokeSpy).not.toHaveBeenCalled()
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/bootstrap',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer browser-host-token')
    expect(payload.hostState.platform).toBe('web')
    expect(payload.workspaceConnections?.[0]?.workspaceConnectionId).toBe('conn-local')
  })

  it('persists browser host preferences through the host HTTP API instead of local storage fallback', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        theme: 'dark',
        locale: 'en-US',
        compactSidebar: true,
        leftSidebarCollapsed: true,
        rightSidebarCollapsed: false,
        fontSize: 15,
        fontFamily: 'Inter, sans-serif',
        fontStyle: 'sans',
        defaultWorkspaceId: 'ws-local',
        lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
      }),
    })

    const client = await loadClientModule()
    const preferences = await client.savePreferences({
      theme: 'dark',
      locale: 'en-US',
      compactSidebar: true,
      leftSidebarCollapsed: true,
      rightSidebarCollapsed: false,
      fontSize: 15,
      fontFamily: 'Inter, sans-serif',
      fontStyle: 'sans',
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
    })

    expect(preferences.theme).toBe('dark')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/preferences',
      expect.objectContaining({
        method: 'PUT',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer browser-host-token')
    expect(headers.get('Content-Type')).toBe('application/json')
  })
})
