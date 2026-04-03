// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useRuntimeStore } from '@/stores/runtime'

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
  })

  it('creates a runtime session and streams messages plus trace through the web fallback', async () => {
    const runtime = useRuntimeStore()

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
    const runtime = useRuntimeStore()

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

  it('clears stale runtime errors after a session is successfully activated', async () => {
    const runtime = useRuntimeStore()

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
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-events',
      projectId: 'proj-redesign',
      title: 'Event Session',
    })

    runtime.applyRuntimeEvent({
      id: 'runtime-event-1',
      kind: 'run_updated',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-events',
      runId: 'runtime-run-1',
      emittedAt: 200,
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
})
