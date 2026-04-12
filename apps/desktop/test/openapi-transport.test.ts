// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  AccessAuditListResponse,
  AuditRecord,
  ClientAppRecord,
  HealthcheckStatus,
  InboxItemRecord,
  KnowledgeEntryRecord,
  NotificationRecord,
  ProjectRecord,
  RuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  WorkspaceSkillFileDocument,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema/generated'

import {
  fetchHostOpenApi,
  fetchWorkspaceOpenApi,
  normalizeComparableApiPath,
  openWorkspaceOpenApiStream,
} from '@/tauri/shared'

const fetchSpy = vi.fn()

function createWorkspaceConnection(): WorkspaceConnectionRecord {
  return {
    workspaceConnectionId: 'conn-local',
    workspaceId: 'ws-local',
    label: 'Local Runtime',
    baseUrl: 'http://127.0.0.1:43127',
    transportSecurity: 'loopback',
    authMode: 'session-token',
    status: 'connected',
  }
}

function createWorkspaceSession(
  connection: WorkspaceConnectionRecord,
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
    },
  }
}

describe('OpenAPI transport helpers', () => {
  beforeEach(() => {
    fetchSpy.mockReset()
    vi.stubGlobal('fetch', fetchSpy)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('normalizes server, spec, and template paths into one comparable form', () => {
    expect(normalizeComparableApiPath('/api/v1/host/notifications/:notification_id/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/host/notifications/{notificationId}/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/host/notifications/${id}/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/workspace/catalog/skills/${skillId}/files/${relativePath}'))
      .toBe('/api/v1/workspace/catalog/skills/{param}/files/{param}')
  })

  it('uses generated OpenAPI paths for host requests', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<HealthcheckStatus> => ({
        status: 'ok',
        host: 'web',
        mode: 'local',
        cargoWorkspace: false,
        backend: {
          state: 'ready',
          transport: 'http',
        },
      }),
    })

    const payload = await fetchHostOpenApi(
      'http://127.0.0.1:43127',
      'desktop-test-token',
      '/api/v1/host/health',
      'get',
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/health',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.status).toBe('ok')
  })

  it('uses generated OpenAPI paths for workspace requests', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<RuntimeEffectiveConfig> => ({
        effectiveConfig: { locale: 'zh-CN' },
        effectiveConfigHash: 'hash-1',
        sources: [],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config',
      'get',
      { session },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
    expect(payload.effectiveConfigHash).toBe('hash-1')
  })

  it('resolves generated host path templates before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<NotificationRecord> => ({
        id: 'notif-1',
        scopeKind: 'app',
        level: 'info',
        title: 'Saved',
        body: 'Preferences updated.',
        source: 'settings',
        createdAt: 1,
      }),
    })

    const payload = await fetchHostOpenApi(
      'http://127.0.0.1:43127',
      'desktop-test-token',
      '/api/v1/host/notifications/{notificationId}/read',
      'post',
      {
        pathParams: {
          notificationId: 'notif-1',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/notifications/notif-1/read',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.id).toBe('notif-1')
  })

  it('resolves generated workspace path templates before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<ProjectRecord> => ({
        id: 'proj-redesign',
        workspaceId: 'ws-local',
        name: 'Redesign',
        status: 'active',
        description: 'Main redesign project',
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/projects/{projectId}',
      'patch',
      {
        session,
        body: JSON.stringify({
          name: 'Redesign',
          description: 'Main redesign project',
          status: 'active',
        }),
        pathParams: {
          projectId: 'proj-redesign',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(payload.id).toBe('proj-redesign')
  })

  it('builds project resource import requests with path params and JSON bodies', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'res-imported-folder',
        workspaceId: 'ws-local',
        projectId: 'proj-redesign',
        kind: 'folder',
        name: 'design-assets',
        origin: 'source',
        status: 'healthy',
        updatedAt: 1,
        tags: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    await (fetchWorkspaceOpenApi as any)(
      connection,
      '/api/v1/projects/{projectId}/resources/import',
      'post',
      {
        session,
        pathParams: {
          projectId: 'proj-redesign',
        },
        body: JSON.stringify({
          name: 'design-assets',
          rootDirName: 'design-assets',
          scope: 'project',
          visibility: 'public',
          files: [
            {
              fileName: 'brief.md',
              contentType: 'text/markdown',
              dataBase64: 'IyBCcmllZg==',
              byteSize: 7,
              relativePath: 'brief.md',
            },
          ],
        }),
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/resources/import',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
        body: JSON.stringify({
          name: 'design-assets',
          rootDirName: 'design-assets',
          scope: 'project',
          visibility: 'public',
          files: [
            {
              fileName: 'brief.md',
              contentType: 'text/markdown',
              dataBase64: 'IyBCcmllZg==',
              byteSize: 7,
              relativePath: 'brief.md',
            },
          ],
        }),
      }),
    )
  })

  it('builds workspace filesystem directory requests with query params', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        currentPath: '/remote/projects',
        parentPath: '/remote',
        entries: [
          {
            name: 'design',
            path: '/remote/projects/design',
          },
        ],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    await (fetchWorkspaceOpenApi as any)(
      connection,
      '/api/v1/workspace/filesystem/directories',
      'get',
      {
        session,
        queryParams: {
          path: '/remote/projects',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/filesystem/directories?path=%2Fremote%2Fprojects',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
  })

  it('encodes nested relativePath params for generated workspace skill file routes', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<WorkspaceSkillFileDocument> => ({
        skillId: 'skill-octopus',
        sourceKey: 'workspace:skill-octopus',
        path: 'docs/guide.md',
        displayPath: 'skills/skill-octopus/docs/guide.md',
        byteSize: 128,
        isText: true,
        content: '# Guide',
        contentType: 'text/markdown',
        language: 'markdown',
        readonly: false,
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
      'get',
      {
        session,
        pathParams: {
          skillId: 'skill-octopus',
          relativePath: 'docs/guide.md',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-octopus/files/docs%2Fguide.md',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.path).toBe('docs/guide.md')
  })

  it('resolves generated runtime config scope paths before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<RuntimeEffectiveConfig> => ({
        effectiveConfig: { locale: 'zh-CN' },
        effectiveConfigHash: 'hash-scope',
        sources: [],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/scopes/{scope}',
      'patch',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          patch: { locale: 'zh-CN' },
        }),
        pathParams: {
          scope: 'workspace',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config/scopes/workspace',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.effectiveConfigHash).toBe('hash-scope')
  })

  it('resolves generated runtime config validate and probe paths with session headers', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeEffectiveConfig['validation']> => ({
          valid: true,
          errors: [],
          warnings: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeConfiguredModelProbeResult> => ({
          valid: true,
          reachable: true,
          configuredModelId: 'anthropic-primary',
          configuredModelName: 'Claude Primary',
          requestId: 'probe-1',
          consumedTokens: 10,
          errors: [],
          warnings: [],
        }),
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/validate',
      'post',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          patch: { locale: 'zh-CN' },
        }),
      },
    )

    await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/configured-models/probe',
      'post',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          configuredModelId: 'anthropic-primary',
          patch: { configuredModels: {} },
        }),
      },
    )

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/runtime/config/validate',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/runtime/config/configured-models/probe',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    const firstHeaders = fetchSpy.mock.calls[0]?.[1]?.headers as Headers
    const secondHeaders = fetchSpy.mock.calls[1]?.[1]?.headers as Headers
    expect(firstHeaders.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(secondHeaders.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('resolves generated runtime session and approval paths with path params, query params, and idempotency headers', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeSessionSummary[]> => ([
          {
            id: 'rt-1',
            conversationId: 'conv-1',
            projectId: 'proj-redesign',
            title: 'Runtime Session',
            sessionKind: 'project',
            status: 'running',
            updatedAt: 2,
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeSessionDetail> => ({
          summary: {
            id: 'rt-1',
            conversationId: 'conv-1',
            projectId: 'proj-redesign',
            title: 'Runtime Session',
            sessionKind: 'project',
            status: 'running',
            updatedAt: 2,
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
          },
          run: {
            id: 'run-1',
            sessionId: 'rt-1',
            conversationId: 'conv-1',
            status: 'running',
            currentStep: 'runtime.turn.waiting',
            startedAt: 1,
            updatedAt: 2,
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
          },
          messages: [],
          trace: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeRunSnapshot> => ({
          id: 'run-1',
          sessionId: 'rt-1',
          conversationId: 'conv-1',
          status: 'running',
          currentStep: 'runtime.turn.waiting',
          startedAt: 1,
          updatedAt: 2,
          configSnapshotId: 'cfg-1',
          effectiveConfigHash: 'hash-1',
          startedFromScopeSet: ['workspace'],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 204,
        headers: new Headers(),
        text: async () => '',
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => [],
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}', 'get', {
      session,
      pathParams: { sessionId: 'rt-1' },
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/turns', 'post', {
      session,
      pathParams: { sessionId: 'rt-1' },
      idempotencyKey: 'idem-turn-1',
      body: JSON.stringify({
        content: 'hello',
        permissionMode: 'workspace-write',
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}', 'post', {
      session,
      pathParams: {
        sessionId: 'rt-1',
        approvalId: 'approval-1',
      },
      idempotencyKey: 'idem-approval-1',
      body: JSON.stringify({
        decision: 'approve',
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/events', 'get', {
      session,
      pathParams: { sessionId: 'rt-1' },
      queryParams: { after: 'evt-1' },
    })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/runtime/sessions',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/turns',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/approvals/approval-1',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/events?after=evt-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    const turnHeaders = fetchSpy.mock.calls[2]?.[1]?.headers as Headers
    expect(turnHeaders.get('Idempotency-Key')).toBe('idem-turn-1')
    const approvalHeaders = fetchSpy.mock.calls[3]?.[1]?.headers as Headers
    expect(approvalHeaders.get('Idempotency-Key')).toBe('idem-approval-1')
  })

  it('extends generated workspace transport paths to apps, audit, inbox, and knowledge routes', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<ClientAppRecord[]> => ([
          {
            id: 'octopus-web',
            name: 'Octopus Web',
            platform: 'web',
            status: 'active',
            firstParty: true,
            allowedOrigins: ['http://127.0.0.1'],
            allowedHosts: ['127.0.0.1'],
            sessionPolicy: 'session_token',
            defaultScopes: ['workspace'],
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<ClientAppRecord> => ({
          id: 'octopus-mobile',
          name: 'Octopus Mobile',
          platform: 'mobile',
          status: 'disabled',
          firstParty: true,
          allowedOrigins: [],
          allowedHosts: [],
          sessionPolicy: 'session_token',
          defaultScopes: ['workspace'],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<AccessAuditListResponse> => ({
          items: [
            {
              id: 'audit-1',
              workspaceId: 'ws-local',
              actorType: 'user',
              actorId: 'user-owner',
              action: 'runtime.session.create',
              resource: 'runtime-session',
              outcome: 'success',
              createdAt: 1,
            } satisfies AuditRecord,
          ],
          nextCursor: undefined,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<InboxItemRecord[]> => ([
          {
            id: 'inbox-1',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            itemType: 'approval',
            title: 'Need approval',
            description: 'Runtime needs approval.',
            status: 'pending',
            priority: 'high',
            actionable: true,
            routeTo: '/workspaces/ws-local/projects/proj-redesign/runtime',
            actionLabel: 'Review approval',
            createdAt: 1,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<KnowledgeEntryRecord[]> => ([
          {
            id: 'knowledge-1',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            title: 'Knowledge Entry',
            scope: 'project',
            status: 'active',
            sourceType: 'document',
            sourceRef: 'doc://knowledge-1',
            updatedAt: 1,
          },
        ]),
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(connection, '/api/v1/apps', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/apps', 'post', {
      session,
      body: JSON.stringify({
        id: 'octopus-mobile',
        name: 'Octopus Mobile',
        platform: 'mobile',
        status: 'disabled',
        firstParty: true,
        allowedOrigins: [],
        allowedHosts: [],
        sessionPolicy: 'session_token',
        defaultScopes: ['workspace'],
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/access/audit', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/inbox', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/knowledge', 'get', { session })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/apps',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/apps',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/access/audit',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/inbox',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/knowledge',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
  })

  it('uses generated runtime event paths for stream requests and resume headers', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'text/event-stream' }),
      body: new ReadableStream(),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await openWorkspaceOpenApiStream(
      connection,
      '/api/v1/runtime/sessions/{sessionId}/events',
      {
        session,
        pathParams: {
          sessionId: 'rt-1',
        },
        queryParams: {
          after: 'evt-1/next',
        },
        headers: {
          Accept: 'text/event-stream',
          'Last-Event-ID': 'evt-1/next',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/events?after=evt-1%2Fnext',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Accept')).toBe('text/event-stream')
    expect(headers.get('Last-Event-ID')).toBe('evt-1/next')
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })
})
