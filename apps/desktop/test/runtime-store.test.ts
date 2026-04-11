// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { RuntimeSessionDetail } from '@octopus/schema'

import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for runtime condition')
    }

    await new Promise((resolve) => window.setTimeout(resolve, 20))
  }
}

describe('useRuntimeStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
  })

  async function prepareRuntimeStore() {
    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign', [])
    const runtime = useRuntimeStore()
    runtime.syncWorkspaceScopeFromShell()
    return { runtime, shell }
  }

  it('creates a runtime session and streams messages plus trace through the workspace API fixture', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-redesign',
      projectId: 'proj-redesign',
      title: 'Redesign Runtime Session',
    })

    expect(runtime.activeSession?.summary.conversationId).toBe('conv-redesign')
    expect(runtime.activeMessages).toHaveLength(0)

    await runtime.submitTurn({
      content: 'Summarize the desktop runtime integration progress.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    await waitFor(() =>
      runtime.activeRun?.status === 'completed'
      && runtime.activeMessages.length >= 2
      && runtime.activeTrace.length >= 1,
    )

    expect(runtime.activeMessages.map((message) => message.content)).toEqual(
      expect.arrayContaining([
        'Summarize the desktop runtime integration progress.',
      ]),
    )
    expect(runtime.activeMessages.some((message) => message.senderType === 'agent')).toBe(true)
    expect(runtime.activeMessages.some((message) => message.actorId === 'agent-architect')).toBe(true)
    expect(runtime.activeMessages.some((message) => (message.artifacts ?? []).length > 0)).toBe(true)
    expect(runtime.activeTrace[0]?.title.length).toBeGreaterThan(0)

    runtime.dispose()
  })

  it('shows the user message immediately before the submit request finishes', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-immediate-message',
      projectId: 'proj-redesign',
      title: 'Immediate Message Session',
    })

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        runtime: {
          ...client.runtime,
          async submitUserTurn(sessionId, input, idempotencyKey) {
            await new Promise(resolve => window.setTimeout(resolve, 120))
            return client.runtime.submitUserTurn(sessionId, input, idempotencyKey)
          },
        },
      }
    })

    const submitPromise = runtime.submitTurn({
      content: 'Show this message immediately.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'readonly',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    await waitFor(() => runtime.activeMessages.some(message => message.content === 'Show this message immediately.'))
    await waitFor(() => runtime.activeMessages.some(message => message.content === 'Thinking…'))
    expect(runtime.activeMessages.some(message => message.content.includes('Completed request'))).toBe(false)

    await submitPromise
    await waitFor(() => runtime.activeMessages.some(message => message.content.includes('Completed request')))

    runtime.dispose()
  })

  it('merges trace and approval updates into the optimistic assistant placeholder', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-placeholder-process',
      projectId: 'proj-redesign',
      title: 'Placeholder Process Session',
    })

    runtime.addOptimisticUserMessage({
      content: 'Run pwd in the workspace terminal.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    runtime.applyRuntimeEvent({
      id: 'evt-trace-placeholder',
      eventType: 'runtime.trace.emitted',
      kind: 'runtime.trace.emitted',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-placeholder-process',
      runId: runtime.activeRun?.id,
      emittedAt: 100,
      sequence: 1,
      trace: {
        id: 'trace-placeholder-step',
        sessionId: runtime.activeSessionId,
        runId: runtime.activeRun?.id ?? 'runtime-run-placeholder',
        conversationId: 'conv-placeholder-process',
        kind: 'step',
        title: 'Turn submitted',
        detail: 'Permission mode workspace-write requires explicit approval before execution.',
        tone: 'warning',
        timestamp: 100,
        actor: 'user',
        actorKind: 'agent',
        actorId: 'agent-architect',
      },
    })

    runtime.applyRuntimeEvent({
      id: 'evt-tool-placeholder',
      eventType: 'runtime.trace.emitted',
      kind: 'runtime.trace.emitted',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-placeholder-process',
      runId: runtime.activeRun?.id,
      emittedAt: 105,
      sequence: 2,
      trace: {
        id: 'trace-placeholder-tool',
        sessionId: runtime.activeSessionId,
        runId: runtime.activeRun?.id ?? 'runtime-run-placeholder',
        conversationId: 'conv-placeholder-process',
        kind: 'tool',
        title: 'Workspace API',
        detail: 'Calling workspace API before approval.',
        tone: 'info',
        timestamp: 105,
        actor: 'assistant',
        actorKind: 'agent',
        actorId: 'agent-architect',
        relatedToolName: 'workspace-api',
      },
    })

    runtime.applyRuntimeEvent({
      id: 'evt-approval-placeholder',
      eventType: 'runtime.approval.requested',
      kind: 'runtime.approval.requested',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-placeholder-process',
      runId: runtime.activeRun?.id,
      emittedAt: 110,
      sequence: 3,
      approval: {
        id: 'approval-placeholder',
        sessionId: runtime.activeSessionId,
        conversationId: 'conv-placeholder-process',
        runId: runtime.activeRun?.id ?? 'runtime-run-placeholder',
        toolName: 'runtime.turn',
        summary: 'Turn requires approval',
        detail: 'Permission mode workspace-write requires explicit approval.',
        riskLevel: 'medium',
        createdAt: 110,
        status: 'pending',
      },
    })

    const placeholder = runtime.activeSession?.messages.find(message => message.id.startsWith('optimistic-assistant-'))
    expect(placeholder?.content).toBe('Awaiting approval…')
    expect(placeholder?.status).toBe('waiting_approval')
    expect(placeholder?.processEntries?.some(entry => entry.title === 'Turn submitted')).toBe(true)
    expect(placeholder?.processEntries?.some(entry => entry.title === 'Turn requires approval')).toBe(true)
    expect(placeholder?.processEntries?.some(entry => entry.toolId === 'workspace-api')).toBe(true)
    expect(placeholder?.toolCalls).toEqual([
      {
        toolId: 'workspace-api',
        label: 'workspace-api',
        kind: 'builtin',
        count: 1,
      },
    ])
    expect(runtime.activeMessages.find(message => message.id === placeholder?.id)?.status).toBe('waiting_approval')

    runtime.dispose()
  })

  it('carries placeholder process entries into the final assistant message', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-placeholder-finalize',
      projectId: 'proj-redesign',
      title: 'Placeholder Finalize Session',
    })

    runtime.addOptimisticUserMessage({
      content: 'Explain the rollout plan.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'readonly',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    runtime.applyRuntimeEvent({
      id: 'evt-trace-finalize',
      eventType: 'runtime.trace.emitted',
      kind: 'runtime.trace.emitted',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-placeholder-finalize',
      runId: runtime.activeRun?.id,
      emittedAt: 100,
      sequence: 1,
      trace: {
        id: 'trace-finalize-step',
        sessionId: runtime.activeSessionId,
        runId: runtime.activeRun?.id ?? 'runtime-run-placeholder',
        conversationId: 'conv-placeholder-finalize',
        kind: 'step',
        title: 'Drafting response',
        detail: 'Collecting rollout details before replying.',
        tone: 'info',
        timestamp: 100,
        actor: 'assistant',
        actorKind: 'agent',
        actorId: 'agent-architect',
      },
    })

    runtime.applyRuntimeEvent({
      id: 'evt-message-finalize',
      eventType: 'runtime.message.created',
      kind: 'runtime.message.created',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-placeholder-finalize',
      runId: runtime.activeRun?.id,
      emittedAt: 120,
      sequence: 2,
      message: {
        id: 'msg-final-assistant',
        sessionId: runtime.activeSessionId,
        conversationId: 'conv-placeholder-finalize',
        senderType: 'assistant',
        senderLabel: 'Architect Agent · Agent',
        content: 'Here is the rollout plan.',
        timestamp: 120,
        configuredModelId: 'anthropic-primary',
        configuredModelName: 'Claude Sonnet 4.5',
        modelId: 'claude-sonnet-4-5',
        status: 'completed',
        requestedActorKind: 'agent',
        requestedActorId: 'agent-architect',
        resolvedActorKind: 'agent',
        resolvedActorId: 'agent-architect',
        resolvedActorLabel: 'Architect Agent · Agent',
        usedDefaultActor: false,
        resourceIds: [],
        attachments: [],
        artifacts: [],
      },
    })

    const finalAssistant = runtime.activeSession?.messages.find(message => message.id === 'msg-final-assistant')
    expect(finalAssistant?.content).toBe('Here is the rollout plan.')
    expect(finalAssistant?.processEntries?.some(entry => entry.title === 'Drafting response')).toBe(true)
    expect(runtime.activeSession?.messages.some(message => message.id.startsWith('optimistic-assistant-'))).toBe(false)

    runtime.dispose()
  })

  it('queues follow-up input behind a pending approval and drains it after approval', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-redesign',
      projectId: 'proj-redesign',
      title: 'Approval Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    await waitFor(() => runtime.pendingApproval !== null)

    expect(runtime.activeRun?.status).toBe('waiting_approval')

    await runtime.submitTurn({
      content: 'Then summarize the output.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    expect(runtime.activeQueue).toHaveLength(1)
    expect(runtime.activeQueue[0]?.content).toBe('Then summarize the output.')
    expect(runtime.activeQueue[0]?.actorId).toBe('agent-architect')

    await runtime.resolveApproval('approve')

    await waitFor(() =>
      runtime.pendingApproval === null
      && runtime.activeQueue.length === 0
      && runtime.activeRun?.status === 'completed'
      && runtime.activeMessages.some((message) => message.content === 'Then summarize the output.'),
    )

    runtime.dispose()
  })

  it('keeps readonly turns on the safe path even for dangerous wording', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-readonly-safe',
      projectId: 'proj-redesign',
      title: 'Readonly Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'readonly',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    await waitFor(() =>
      runtime.pendingApproval === null
      && runtime.activeRun?.status === 'completed'
      && runtime.activeTrace.length >= 1,
    )

    expect(runtime.activeTrace.some((trace) => trace.detail.includes('read-only'))).toBe(true)

    runtime.dispose()
  })

  it('preserves danger-full-access submissions instead of falling back to auto approval', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-danger-mode',
      projectId: 'proj-redesign',
      title: 'Danger Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'danger-full-access',
      actorKind: 'agent',
      actorId: 'agent-architect',
    })

    await waitFor(() =>
      runtime.pendingApproval === null
      && runtime.activeRun?.status === 'completed'
      && runtime.activeTrace.length >= 1,
    )

    expect(runtime.activeTrace.some((trace) => trace.detail.includes('danger-full-access'))).toBe(true)

    runtime.dispose()
  })

  it('clears stale runtime errors after a session is successfully activated', async () => {
    const { runtime } = await prepareRuntimeStore()

    runtime.error = 'Failed to create runtime session'

    await runtime.ensureSession({
      conversationId: 'conv-recovery',
      projectId: 'proj-redesign',
      title: 'Recovered Session',
    })

    expect(runtime.activeSession?.summary.conversationId).toBe('conv-recovery')
    expect(runtime.error).toBe('')

    runtime.dispose()
  })

  it('resolves team actor labels from runtime-backed turns', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-team-runtime',
      projectId: 'proj-redesign',
      title: 'Team Runtime Session',
    })

    await runtime.submitTurn({
      content: 'Coordinate the redesign rollout.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorKind: 'team',
      actorId: 'team-studio',
    })

    await waitFor(() =>
      runtime.activeRun?.status === 'completed'
      && runtime.activeMessages.some(message => message.actorId === 'team-studio'),
    )

    expect(runtime.activeRun?.resolvedActorLabel).toBe('Studio Direction Team · Team')

    runtime.dispose()
  })

  it('merges run_updated events into session summary timestamps and status', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-events',
      projectId: 'proj-redesign',
      title: 'Event Session',
    })

    runtime.applyRuntimeEvent({
      id: 'runtime-event-1',
      eventType: 'runtime.run.updated',
      kind: 'runtime.run.updated',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-events',
      runId: 'runtime-run-1',
      emittedAt: 200,
      sequence: 1,
      run: {
        id: 'runtime-run-1',
        sessionId: runtime.activeSessionId,
        conversationId: 'conv-events',
        status: 'completed',
        currentStep: 'runtime.run.completed',
        startedAt: 100,
        updatedAt: 200,
        modelId: 'claude-sonnet-4-5',
        nextAction: 'runtime.run.idle',
        requestedActorKind: 'agent',
        requestedActorId: 'agent-architect',
        resolvedActorKind: 'agent',
        resolvedActorId: 'agent-architect',
        resolvedActorLabel: '默认智能体',
      },
    })

    expect(runtime.activeSession?.summary.status).toBe('completed')
    expect(runtime.activeSession?.summary.updatedAt).toBe(200)
    expect(runtime.activeRun?.status).toBe('completed')

    runtime.dispose()
  })

  it('keeps runtime snapshots isolated per workspace connection when switching contexts', async () => {
    const { runtime, shell } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-local-scope',
      projectId: 'proj-redesign',
      title: 'Local Scope Session',
    })

    const localSessionId = runtime.activeSessionId
    expect(runtime.activeSession?.summary.conversationId).toBe('conv-local-scope')

    await shell.activateWorkspaceByWorkspaceId('ws-enterprise')
    runtime.syncWorkspaceScopeFromShell()

    expect(runtime.activeSessionId).toBe('')
    expect(runtime.sessions).toEqual([])

    await runtime.ensureSession({
      conversationId: 'conv-enterprise-scope',
      projectId: 'proj-launch',
      title: 'Enterprise Scope Session',
    })

    const enterpriseSessionId = runtime.activeSessionId
    expect(enterpriseSessionId).not.toBe(localSessionId)
    expect(runtime.activeSession?.summary.conversationId).toBe('conv-enterprise-scope')

    await shell.activateWorkspaceByWorkspaceId('ws-local')
    runtime.syncWorkspaceScopeFromShell()

    expect(runtime.activeSessionId).toBe(localSessionId)
    expect(runtime.activeSession?.summary.conversationId).toBe('conv-local-scope')
    expect(runtime.sessions.map(session => session.conversationId)).toContain('conv-local-scope')

    runtime.dispose()
  })

  it('loads and saves workspace runtime config through the shared workspace client', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.loadConfig()

    expect(runtime.config?.effectiveConfigHash).toContain('cfg-hash')
    expect(runtime.configDrafts.workspace).toContain('claude-sonnet-4-5')

    runtime.configDrafts.workspace = JSON.stringify({
      model: 'gpt-4o',
      permissions: {
        defaultMode: 'plan',
      },
    }, null, 2)

    await runtime.saveConfig('workspace')

    expect(runtime.config?.effectiveConfig).toMatchObject({
      model: 'gpt-4o',
      permissions: {
        defaultMode: 'plan',
      },
    })
    expect(runtime.config?.sources.find(source => source.scope === 'workspace')?.document).toMatchObject({
      model: 'gpt-4o',
    })

    runtime.dispose()
  })

  it('surfaces a runtime config error when the workspace session is missing', async () => {
    const { runtime, shell } = await prepareRuntimeStore()

    shell.clearWorkspaceSession('conn-local')

    await runtime.loadConfig(true)

    expect(runtime.config).toBeNull()
    expect(runtime.configError).toMatch(/workspace session/i)

    runtime.dispose()
  })

  it('ignores stale session responses from the previously active workspace connection', async () => {
    const { runtime, shell } = await prepareRuntimeStore()

    const localDetail = {
      summary: {
        id: 'rt-local',
        conversationId: 'conv-local-stale',
        projectId: 'proj-redesign',
        title: 'Local Stale Session',
        status: 'draft',
        updatedAt: 10,
      },
      run: {
        id: 'run-local',
        sessionId: 'rt-local',
        conversationId: 'conv-local-stale',
        status: 'draft',
        currentStep: 'runtime.run.idle',
        startedAt: 10,
        updatedAt: 10,
        modelId: 'claude-sonnet-4-5',
        nextAction: 'runtime.run.awaitingInput',
      },
      messages: [],
      trace: [],
      pendingApproval: undefined,
    } satisfies RuntimeSessionDetail
    const enterpriseDetail = {
      summary: {
        id: 'rt-enterprise',
        conversationId: 'conv-enterprise-fresh',
        projectId: 'proj-launch',
        title: 'Enterprise Fresh Session',
        status: 'draft',
        updatedAt: 20,
      },
      run: {
        id: 'run-enterprise',
        sessionId: 'rt-enterprise',
        conversationId: 'conv-enterprise-fresh',
        status: 'draft',
        currentStep: 'runtime.run.idle',
        startedAt: 20,
        updatedAt: 20,
        modelId: 'claude-sonnet-4-5',
        nextAction: 'runtime.run.awaitingInput',
      },
      messages: [],
      trace: [],
      pendingApproval: undefined,
    } satisfies RuntimeSessionDetail

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockImplementation(({ connection }) => ({
      runtime: {
        bootstrap: async () => ({
          provider: {
            provider: 'anthropic',
            defaultModel: 'claude-sonnet-4-5',
          },
          sessions: [],
        }),
        listSessions: async () => [],
        createSession: async () => connection.workspaceId === 'ws-local' ? localDetail : enterpriseDetail,
        loadSession: async () => {
          if (connection.workspaceId === 'ws-local') {
            await new Promise(resolve => window.setTimeout(resolve, 30))
            return localDetail
          }

          return enterpriseDetail
        },
        pollEvents: async () => [],
        subscribeEvents: async () => ({
          mode: 'sse',
          close: () => {},
        }),
        submitUserTurn: async () => connection.workspaceId === 'ws-local' ? localDetail.run : enterpriseDetail.run,
        resolveApproval: async () => {},
      },
    }) as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const staleLoad = runtime.loadSession('rt-local')

    await shell.activateWorkspaceByWorkspaceId('ws-enterprise')
    runtime.syncWorkspaceScopeFromShell()
    await runtime.loadSession('rt-enterprise')
    await staleLoad

    expect(runtime.activeWorkspaceConnectionId).toBe(shell.activeWorkspaceConnectionId)
    expect(runtime.activeSessionId).toBe('rt-enterprise')
    expect(runtime.activeSession?.summary.conversationId).toBe('conv-enterprise-fresh')
    expect(runtime.sessionDetails['rt-local']).toBeUndefined()

    runtime.dispose()
  })
})
