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
      actorLabel: '默认智能体',
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
    expect(runtime.activeTrace[0]?.title.length).toBeGreaterThan(0)

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
      actorLabel: '默认智能体',
    })

    await waitFor(() => runtime.pendingApproval !== null)

    expect(runtime.activeRun?.status).toBe('waiting_approval')

    await runtime.submitTurn({
      content: 'Then summarize the output.',
      modelId: 'claude-sonnet-4-5',
      permissionMode: 'auto',
      actorLabel: '默认智能体',
    })

    expect(runtime.activeQueue).toHaveLength(1)
    expect(runtime.activeQueue[0]?.content).toBe('Then summarize the output.')

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
      actorLabel: '默认智能体',
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
      actorLabel: '默认智能体',
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

  it('loads and saves runtime config through the shared workspace client', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.loadConfig()

    expect(runtime.config?.effectiveConfigHash).toContain('cfg-hash')
    expect(runtime.configDrafts.project).toContain('claude-sonnet-4-5')

    runtime.configDrafts.project = JSON.stringify({
      model: 'gpt-4o',
      permissions: {
        defaultMode: 'plan',
      },
    }, null, 2)

    await runtime.saveConfig('project')

    expect(runtime.config?.effectiveConfig).toMatchObject({
      model: 'gpt-4o',
      permissions: {
        defaultMode: 'plan',
      },
    })
    expect(runtime.config?.sources.find(source => source.scope === 'project')?.document).toMatchObject({
      model: 'gpt-4o',
    })

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
