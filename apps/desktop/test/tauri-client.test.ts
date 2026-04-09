// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  ApiErrorEnvelope,
  AvatarUploadPayload,
  BindPetConversationInput,
  NotificationRecord,
  RegisterWorkspaceOwnerRequest,
  RuntimeConfigPatch,
  SavePetPresenceInput,
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

function createNotificationRecord(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
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

function createHostUpdateStatus(overrides: Record<string, unknown> = {}) {
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

  it('creates notifications through the Tauri shell bridge', async () => {
    const notification = createNotificationRecord({
      id: 'notif-created',
      scopeKind: 'workspace',
    })
    invokeSpy.mockResolvedValue(notification)

    const client = await loadClientModule()
    const result = await client.createNotification({
      scopeKind: 'workspace',
      scopeOwnerId: 'ws-local',
      level: 'success',
      title: 'Workspace synced',
      body: 'The workspace status is up to date.',
      source: 'workspace-store',
      toastDurationMs: 30_000,
    })

    expect(invokeSpy).toHaveBeenCalledWith('create_notification', {
      input: {
        scopeKind: 'workspace',
        scopeOwnerId: 'ws-local',
        level: 'success',
        title: 'Workspace synced',
        body: 'The workspace status is up to date.',
        source: 'workspace-store',
        toastDurationMs: 30_000,
      },
    })
    expect(result.id).toBe('notif-created')
  })

  it('fans out notification events to local subscribers after successful creation', async () => {
    const notification = createNotificationRecord({
      id: 'notif-fanout',
      toastVisibleUntil: 30_000,
    })
    invokeSpy.mockResolvedValue(notification)

    const client = await loadClientModule()
    const received: NotificationRecord[] = []
    const unsubscribe = client.subscribeToNotifications((event) => {
      received.push(event)
    })

    await client.createNotification({
      scopeKind: 'app',
      level: 'info',
      title: 'Heads up',
      body: 'New notification.',
      source: 'test-suite',
    })

    expect(received).toEqual([notification])

    unsubscribe()

    await client.createNotification({
      scopeKind: 'app',
      level: 'info',
      title: 'Second',
      body: 'Should not be received.',
      source: 'test-suite',
    })

    expect(received).toEqual([notification])
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

  it('calls workspace pet endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          profile: {
            id: 'pet-octopus',
            displayName: '小章',
            species: 'octopus',
            ownerUserId: 'user-owner',
            avatarLabel: 'Octopus mascot',
            summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
            greeting: '嗨！我是小章，今天也要加油哦！',
            mood: 'happy',
            favoriteSnack: '新鲜小虾',
            promptHints: ['最近有什么好消息？'],
            fallbackAsset: 'octopus',
          },
          presence: {
            petId: 'pet-octopus',
            isVisible: true,
            chatOpen: false,
            motionState: 'idle',
            unreadCount: 0,
            lastInteractionAt: 0,
            position: { x: 0, y: 0 },
          },
          messages: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          petId: 'pet-octopus',
          isVisible: true,
          chatOpen: true,
          motionState: 'chat',
          unreadCount: 0,
          lastInteractionAt: 12,
          position: { x: 0, y: 0 },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          petId: 'pet-octopus',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          conversationId: 'conversation-1',
          sessionId: 'rt-conversation-1',
          updatedAt: 12,
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await workspaceClient.pet.getSnapshot('proj-redesign')
    await workspaceClient.pet.savePresence({
      petId: 'pet-octopus',
      chatOpen: true,
      motionState: 'chat',
    } satisfies SavePetPresenceInput, 'proj-redesign')
    await workspaceClient.pet.bindConversation({
      petId: 'pet-octopus',
      conversationId: 'conversation-1',
      sessionId: 'rt-conversation-1',
    } satisfies BindPetConversationInput, 'proj-redesign')

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet/presence',
      expect.objectContaining({ method: 'PATCH' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet/conversation',
      expect.objectContaining({ method: 'PUT' }),
    )
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
      actorKind: 'agent',
      actorId: 'agent-architect',
    }, 'idem-turn-1')

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(JSON.parse(String(request.body))).toMatchObject({
      content: 'hello',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'workspace-write',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })
    expect(headers.get('Idempotency-Key')).toBe('idem-turn-1')
  })

  it('lists workspace artifacts through the workspace API with the session token', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ([
        {
          id: 'artifact-1',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          title: 'Runtime Delivery Summary',
          status: 'review',
          latestVersion: 2,
          updatedAt: 10,
        },
      ]),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const artifacts = await workspaceClient.artifacts.listWorkspace()

    expect(artifacts[0]?.title).toBe('Runtime Delivery Summary')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/artifacts',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    const request = firstRequest()
    expect((request.headers as Headers).get('Authorization')).toBe('Bearer workspace-session-token')
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
      actorKind: 'team',
      actorId: 'team-studio',
    })

    const request = firstRequest()
    expect(JSON.parse(String(request.body))).toMatchObject({
      permissionMode: 'danger-full-access',
      actorKind: 'team',
      actorId: 'team-studio',
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

  it('posts configured model probe requests to the workspace API without requiring a workspace session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        valid: true,
        reachable: true,
        configuredModelId: 'anthropic-primary',
        configuredModelName: 'Claude Primary',
        requestId: 'probe-request-1',
        consumedTokens: 12,
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

    const result = await workspaceClient.runtime.validateConfiguredModel({
      scope: 'workspace',
      configuredModelId: 'anthropic-primary',
      patch: {
        configuredModels: {
          'anthropic-primary': {
            baseUrl: 'https://anthropic.example.test',
          },
        },
      },
    })

    expect(result.reachable).toBe(true)
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config/configured-models/probe',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
    expect(JSON.parse(String(request.body))).toMatchObject({
      scope: 'workspace',
      configuredModelId: 'anthropic-primary',
      patch: {
        configuredModels: {
          'anthropic-primary': {
            baseUrl: 'https://anthropic.example.test',
          },
        },
      },
    })
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
            scope: 'user',
            ownerId: 'user-owner',
            displayPath: 'config/runtime/users/user-owner.json',
            sourceKey: 'user:user-owner',
            exists: true,
            loaded: true,
          },
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

  it('uses authenticated project create endpoint for workspace project management', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'proj-studio',
        workspaceId: 'ws-local',
        name: 'Agent Studio',
        status: 'active',
        description: 'Project management workspace surface.',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const project = await (workspaceClient.projects as any).create({
      name: 'Agent Studio',
      description: 'Project management workspace surface.',
    })

    expect(project.name).toBe('Agent Studio')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('uses authenticated project update endpoint for archive/restore actions', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'proj-redesign',
        workspaceId: 'ws-local',
        name: 'Desktop Redesign',
        status: 'archived',
        description: 'Real workspace API migration for the desktop surface.',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const project = await (workspaceClient.projects as any).update('proj-redesign', {
      name: 'Desktop Redesign',
      description: 'Real workspace API migration for the desktop surface.',
      status: 'archived',
    })

    expect(project.status).toBe('archived')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign',
      expect.objectContaining({
        method: 'PATCH',
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
            scope: 'user',
            ownerId: 'user-owner',
            displayPath: 'config/runtime/users/user-owner.json',
            sourceKey: 'user:user-owner',
            exists: true,
            loaded: true,
          },
          {
            scope: 'workspace',
            displayPath: 'config/runtime/workspace.json',
            sourceKey: 'workspace',
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

  it('updates the current user profile through the workspace user center profile endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'user-owner',
        username: 'owner-updated',
        displayName: 'Owner Updated',
        status: 'active',
        passwordState: 'set',
        roleIds: ['role-owner'],
        scopeProjectIds: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const avatar: AvatarUploadPayload = {
      fileName: 'owner-avatar.png',
      contentType: 'image/png',
      dataBase64: 'iVBORw0KGgo=',
      byteSize: 8,
    }

    await workspaceClient.rbac.updateCurrentUserProfile({
      username: 'owner-updated',
      displayName: 'Owner Updated',
      avatar,
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/user-center/profile',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      username: 'owner-updated',
      displayName: 'Owner Updated',
      avatar,
    }))
  })

  it('changes the current user password through the workspace user center profile password endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        passwordState: 'set',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.rbac.changeCurrentUserPassword({
      currentPassword: 'owner-owner',
      newPassword: 'owner-owner-2',
      confirmPassword: 'owner-owner-2',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/user-center/profile/password',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      currentPassword: 'owner-owner',
      newPassword: 'owner-owner-2',
      confirmPassword: 'owner-owner-2',
    }))
  })

  it('creates workspace members through the RBAC users endpoint with avatar and password options', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'user-member-alpha',
        username: 'member-alpha',
        displayName: 'Member Alpha',
        avatar: undefined,
        status: 'active',
        passwordState: 'reset-required',
        roleIds: ['role-operator'],
        scopeProjectIds: ['proj-governance'],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const avatar: AvatarUploadPayload = {
      fileName: 'member-alpha.png',
      contentType: 'image/png',
      dataBase64: 'YWxwaGE=',
      byteSize: 5,
    }

    await workspaceClient.rbac.createUser({
      username: 'member-alpha',
      displayName: 'Member Alpha',
      status: 'active',
      roleIds: ['role-operator'],
      scopeProjectIds: ['proj-governance'],
      avatar,
      useDefaultPassword: true,
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/rbac/users',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      username: 'member-alpha',
      displayName: 'Member Alpha',
      status: 'active',
      roleIds: ['role-operator'],
      scopeProjectIds: ['proj-governance'],
      avatar,
      useDefaultPassword: true,
    }))
  })

  it('updates workspace members through the RBAC user detail endpoint with password reset options', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'user-member-beta',
        username: 'member-beta',
        displayName: 'Member Beta',
        avatar: 'data:image/png;base64,YmV0YQ==',
        status: 'active',
        passwordState: 'set',
        roleIds: ['role-owner'],
        scopeProjectIds: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const avatar: AvatarUploadPayload = {
      fileName: 'member-beta.png',
      contentType: 'image/png',
      dataBase64: 'YmV0YQ==',
      byteSize: 4,
    }

    await workspaceClient.rbac.updateUser('user-member-beta', {
      username: 'member-beta',
      displayName: 'Member Beta',
      status: 'active',
      roleIds: ['role-owner'],
      scopeProjectIds: [],
      avatar,
      password: 'member-beta-1',
      confirmPassword: 'member-beta-1',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/rbac/users/user-member-beta',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      username: 'member-beta',
      displayName: 'Member Beta',
      status: 'active',
      roleIds: ['role-owner'],
      scopeProjectIds: [],
      avatar,
      password: 'member-beta-1',
      confirmPassword: 'member-beta-1',
    }))
  })

  it('deletes workspace members through the RBAC user detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.rbac.deleteUser('user-member-beta')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/rbac/users/user-member-beta',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
  })

  it('deletes workspace roles through the RBAC role detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.rbac.deleteRole('role-operator')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/rbac/roles/role-operator',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
  })

  it('deletes workspace permissions through the RBAC permission detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.rbac.deletePermission('perm-manage-tools')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/rbac/permissions/perm-manage-tools',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
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

  it('bridges host update actions through Tauri commands', async () => {
    invokeSpy
      .mockResolvedValueOnce(createHostUpdateStatus())
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'up_to_date',
        lastCheckedAt: 1_710_000_000_000,
      }))
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'downloading',
        progress: {
          downloadedBytes: 512,
          totalBytes: 1024,
          percent: 50,
        },
      }))
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'installing',
      }))

    const client = await loadClientModule()
    const updateStatus = await (client as typeof client & {
      getHostUpdateStatus: () => Promise<unknown>
      checkHostUpdate: (channel: string) => Promise<unknown>
      downloadHostUpdate: () => Promise<unknown>
      installHostUpdate: () => Promise<unknown>
    }).getHostUpdateStatus()

    expect(updateStatus).toMatchObject({
      currentVersion: '0.2.0',
      currentChannel: 'formal',
      state: 'idle',
    })
    expect(invokeSpy).toHaveBeenCalledWith('get_host_update_status')

    await (client as typeof client & {
      checkHostUpdate: (channel: string) => Promise<unknown>
    }).checkHostUpdate('preview')
    expect(invokeSpy).toHaveBeenCalledWith('check_host_update', {
      channel: 'preview',
    })

    await (client as typeof client & {
      downloadHostUpdate: () => Promise<unknown>
    }).downloadHostUpdate()
    expect(invokeSpy).toHaveBeenCalledWith('download_host_update')

    await (client as typeof client & {
      installHostUpdate: () => Promise<unknown>
    }).installHostUpdate()
    expect(invokeSpy).toHaveBeenCalledWith('install_host_update')
  })

  it('uses browser host update HTTP endpoints when VITE_HOST_RUNTIME=browser', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus(),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          currentChannel: 'preview',
          state: 'update_available',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          state: 'downloaded',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          state: 'installing',
        }),
      })

    const client = await loadClientModule()
    await (client as typeof client & {
      getHostUpdateStatus: () => Promise<unknown>
      checkHostUpdate: (channel: string) => Promise<unknown>
      downloadHostUpdate: () => Promise<unknown>
      installHostUpdate: () => Promise<unknown>
    }).getHostUpdateStatus()
    await (client as typeof client & {
      checkHostUpdate: (channel: string) => Promise<unknown>
    }).checkHostUpdate('preview')
    await (client as typeof client & {
      downloadHostUpdate: () => Promise<unknown>
    }).downloadHostUpdate()
    await (client as typeof client & {
      installHostUpdate: () => Promise<unknown>
    }).installHostUpdate()

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/host/update-status',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/host/update-check',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/host/update-download',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/host/update-install',
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('creates, lists, and deletes host workspace connections through Tauri commands', async () => {
    invokeSpy.mockImplementation(async (command: string) => {
      if (command === 'list_workspace_connections') {
        return [{
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }]
      }

      if (command === 'create_workspace_connection') {
        return {
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }
      }

      if (command === 'delete_workspace_connection') {
        return null
      }

      return createHostBootstrap()
    })

    const client = await loadClientModule()

    const listed = await client.listWorkspaceConnections()
    expect(listed).toHaveLength(1)
    expect(invokeSpy).toHaveBeenCalledWith('list_workspace_connections')

    const created = await client.createWorkspaceConnection({
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
    })
    expect(created.workspaceConnectionId).toBe('conn-enterprise')
    expect(invokeSpy).toHaveBeenCalledWith('create_workspace_connection', {
      input: {
        workspaceId: 'ws-enterprise',
        label: 'Enterprise Workspace',
        baseUrl: 'https://enterprise.example.test',
        transportSecurity: 'trusted',
        authMode: 'session-token',
      },
    })

    await client.deleteWorkspaceConnection('conn-enterprise')
    expect(invokeSpy).toHaveBeenCalledWith('delete_workspace_connection', {
      workspaceConnectionId: 'conn-enterprise',
    })
  })

  it('creates, lists, and deletes host workspace connections through browser host HTTP endpoints', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([{
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers(),
        json: async () => null,
      })

    const client = await loadClientModule()

    const listed = await client.listWorkspaceConnections()
    expect(listed[0]?.workspaceConnectionId).toBe('conn-enterprise')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await client.createWorkspaceConnection({
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await client.deleteWorkspaceConnection('conn-enterprise')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections/conn-enterprise',
      expect.objectContaining({ method: 'DELETE', headers: expect.any(Headers) }),
    )
  })

  it('calls the workspace tool management routes through the catalog adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({ entries: [] }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({ entries: [] }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          name: 'ops-helper',
          description: 'Helpful local skill.',
          content: '---\\nname: ops-helper\\n---\\n',
          displayPath: 'data/skills/ops-helper/SKILL.md',
          rootPath: 'data/skills/ops-helper',
          tree: [],
          relativePath: 'data/skills/ops-helper/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          serverName: 'ops',
          sourceKey: 'mcp:ops',
          displayPath: 'config/runtime/workspace.json',
          scope: 'workspace',
          config: {
            type: 'http',
            url: 'https://ops.example.test/mcp',
          },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          displayPath: 'data/skills/ops-helper',
          rootPath: 'data/skills/ops-helper',
          tree: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          path: 'notes/overview.md',
          displayPath: 'data/skills/ops-helper/notes/overview.md',
          byteSize: 12,
          isText: true,
          content: '# Overview',
          contentType: 'text/markdown',
          language: 'markdown',
          readonly: false,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          path: 'notes/overview.md',
          displayPath: 'data/skills/ops-helper/notes/overview.md',
          byteSize: 14,
          isText: true,
          content: '# Updated',
          contentType: 'text/markdown',
          language: 'markdown',
          readonly: false,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-imported',
          sourceKey: 'skill:data/skills/imported/SKILL.md',
          name: 'imported',
          description: 'Imported skill.',
          content: '---\\nname: imported\\n---\\n',
          displayPath: 'data/skills/imported/SKILL.md',
          rootPath: 'data/skills/imported',
          tree: [],
          relativePath: 'data/skills/imported/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-foldered',
          sourceKey: 'skill:data/skills/foldered/SKILL.md',
          name: 'foldered',
          description: 'Folder import.',
          content: '---\\nname: foldered\\n---\\n',
          displayPath: 'data/skills/foldered/SKILL.md',
          rootPath: 'data/skills/foldered',
          tree: [],
          relativePath: 'data/skills/foldered/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-copied',
          sourceKey: 'skill:data/skills/copied/SKILL.md',
          name: 'copied',
          description: 'Copied skill.',
          content: '---\\nname: copied\\n---\\n',
          displayPath: 'data/skills/copied/SKILL.md',
          rootPath: 'data/skills/copied',
          tree: [],
          relativePath: 'data/skills/copied/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
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

    await workspaceClient.catalog.setToolDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/tool-catalog/disable',
      expect.objectContaining({ method: 'PATCH', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.createSkill({
      slug: 'ops-helper',
      content: '---\nname: ops-helper\n---\n',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkill('skill-workspace-ops-helper')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getMcpServer('ops')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/mcp-servers/ops',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkillTree('skill-workspace-ops-helper')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/tree',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkillFile('skill-workspace-ops-helper', 'notes/overview.md')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      6,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/files/notes%2Foverview.md',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.updateSkillFile('skill-workspace-ops-helper', 'notes/overview.md', {
      content: '# Updated',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      7,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/files/notes%2Foverview.md',
      expect.objectContaining({ method: 'PATCH', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.importSkillArchive({
      slug: 'imported',
      archive: {
        fileName: 'imported.zip',
        contentType: 'application/zip',
        dataBase64: 'UEsDBA==',
        byteSize: 8,
      },
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      8,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/import-archive',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.importSkillFolder({
      slug: 'foldered',
      files: [{
        relativePath: 'foldered/SKILL.md',
        fileName: 'SKILL.md',
        contentType: 'text/markdown',
        dataBase64: 'IyBza2lsbA==',
        byteSize: 8,
      }],
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      9,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/import-folder',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.copySkillToManaged('skill-external-help', {
      slug: 'copied',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      10,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-external-help/copy-to-managed',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
  })
})
