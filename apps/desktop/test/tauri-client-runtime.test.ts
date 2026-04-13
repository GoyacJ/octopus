// @vitest-environment jsdom

import { describe, expect, it } from 'vitest'

import type { RuntimeConfigPatch } from '@octopus/schema'

import {
  createHostBootstrap,
  createWorkspaceSession,
  fetchSpy,
  firstRequest,
  installTauriClientTestHooks,
  invokeSpy,
  loadClientModule,
} from './tauri-client-test-helpers'

describe('runtime client transport', () => {
  installTauriClientTestHooks()

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
        runKind: 'primary',
        workflowRun: 'wf-run-1',
        mailboxRef: 'mailbox-1',
        backgroundState: 'completed',
        workerDispatch: {
          totalSubruns: 2,
          activeSubruns: 0,
          completedSubruns: 2,
          failedSubruns: 0,
        },
        usageSummary: {
          inputTokens: 0,
          outputTokens: 0,
          totalTokens: 0,
        },
        artifactRefs: [],
        traceContext: {
          sessionId: 'runtime-session-conv-1',
          traceId: 'trace-1',
          turnId: 'turn-1',
        },
        checkpoint: {
          serializedSession: {},
          currentIterationIndex: 0,
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
          capabilityPlanSummary: {
            activatedTools: [],
            approvedTools: [],
            authResolvedTools: [],
            availableResources: [],
            deferredTools: [],
            discoverableSkills: [],
            grantedTools: [],
            hiddenCapabilities: [],
            pendingTools: [],
            providerFallbacks: [],
            visibleTools: [],
          },
        },
        capabilityPlanSummary: {
          activatedTools: [],
          approvedTools: [],
          authResolvedTools: [],
          availableResources: [],
          deferredTools: [],
          discoverableSkills: [],
          grantedTools: [],
          hiddenCapabilities: [],
          pendingTools: [],
          providerFallbacks: [],
          visibleTools: [],
        },
        providerStateSummary: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const run = await workspaceClient.runtime.submitUserTurn('runtime-session-conv-1', {
      content: 'hello',
      permissionMode: 'auto',
      recallMode: 'skip',
      ignoredMemoryIds: ['mem-1', 'mem-2'],
      memoryIntent: 'feedback',
    } as any, 'idem-turn-1')

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(JSON.parse(String(request.body))).toMatchObject({
      content: 'hello',
      permissionMode: 'workspace-write',
      recallMode: 'skip',
      ignoredMemoryIds: ['mem-1', 'mem-2'],
      memoryIntent: 'feedback',
    })
    expect(headers.get('Idempotency-Key')).toBe('idem-turn-1')
    expect(run.workflowRun).toBe('wf-run-1')
    expect(run.mailboxRef).toBe('mailbox-1')
    expect(run.workerDispatch?.totalSubruns).toBe(2)
  })

  it('posts memory proposal review decisions to the runtime memory proposal endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'runtime-run-conv-memory-1',
        sessionId: 'runtime-session-conv-memory-1',
        conversationId: 'conv-memory-1',
        status: 'completed',
        currentStep: 'runtime.run.completed',
        startedAt: 1,
        updatedAt: 2,
        runKind: 'primary',
        actorRef: 'agent:agent-architect',
        usageSummary: {
          inputTokens: 0,
          outputTokens: 0,
          totalTokens: 0,
        },
        artifactRefs: [],
        traceContext: {
          sessionId: 'runtime-session-conv-memory-1',
          traceId: 'trace-memory-1',
          turnId: 'turn-memory-1',
        },
        checkpoint: {
          serializedSession: {},
          currentIterationIndex: 0,
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
          capabilityPlanSummary: {
            activatedTools: [],
            approvedTools: [],
            authResolvedTools: [],
            availableResources: [],
            deferredTools: [],
            discoverableSkills: [],
            grantedTools: [],
            hiddenCapabilities: [],
            pendingTools: [],
            providerFallbacks: [],
            visibleTools: [],
          },
        },
        capabilityPlanSummary: {
          activatedTools: [],
          approvedTools: [],
          authResolvedTools: [],
          availableResources: [],
          deferredTools: [],
          discoverableSkills: [],
          grantedTools: [],
          hiddenCapabilities: [],
          pendingTools: [],
          providerFallbacks: [],
          visibleTools: [],
        },
        providerStateSummary: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await (workspaceClient.runtime as any).resolveMemoryProposal(
      'runtime-session-conv-memory-1',
      'memory-proposal-1',
      {
        decision: 'approve',
        note: 'Approved for durable reuse.',
      },
      'idem-memory-proposal-1',
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/sessions/runtime-session-conv-memory-1/memory-proposals/memory-proposal-1',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    expect(JSON.parse(String(request.body))).toMatchObject({
      decision: 'approve',
      note: 'Approved for durable reuse.',
    })
    expect(headers.get('Idempotency-Key')).toBe('idem-memory-proposal-1')
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
      permissionMode: 'danger-full-access',
    })

    const request = firstRequest()
    expect(JSON.parse(String(request.body))).toMatchObject({
      permissionMode: 'danger-full-access',
    })
  })

  it('loads runtime config through the workspace API with the workspace session token', async () => {
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
      session: createWorkspaceSession(connection!),
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
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })

  it('posts runtime config validation requests to the workspace API with the workspace session token', async () => {
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
      session: createWorkspaceSession(connection!),
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
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('posts configured model probe requests to the workspace API with the workspace session token', async () => {
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
      session: createWorkspaceSession(connection!),
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
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
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

  it('patches runtime config scopes through the workspace API with the workspace session token', async () => {
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
      session: createWorkspaceSession(connection!),
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
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('uses authenticated project runtime config endpoints for project-scoped overrides', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        effectiveConfig: { model: 'claude-sonnet-4-5' },
        effectiveConfigHash: 'project-cfg-hash-1',
        sources: [
          {
            scope: 'project',
            ownerId: 'proj-redesign',
            displayPath: 'config/runtime/projects/proj-redesign.json',
            sourceKey: 'project:proj-redesign',
            exists: true,
            loaded: true,
            contentHash: 'project-src-hash-1',
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
        effectiveConfigHash: 'user-cfg-hash-1',
        sources: [
          {
            scope: 'user',
            ownerId: 'user-owner',
            displayPath: 'config/runtime/users/user-owner.json',
            sourceKey: 'user:user-owner',
            exists: true,
            loaded: true,
            contentHash: 'user-src-hash-1',
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
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.runtime.getUserConfig()

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/personal-center/profile/runtime-config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
  })
})
