// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { RuntimeSessionDetail } from '@octopus/schema'

import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'
import { createSessionDetail } from './support/workspace-fixture-runtime'
import { createPendingMediationSummary } from './support/workspace-fixture-runtime'

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
      permissionMode: 'auto',
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
    expect(runtime.activeSession?.workflow?.status).toBe('completed')
    expect(runtime.activeSession?.pendingMailbox?.channel).toBe('leader-hub')
    expect(runtime.activeSession?.backgroundRun?.status).toBe('completed')
    expect(runtime.activeSession?.subruns.length).toBeGreaterThan(0)
    expect(runtime.activeSession?.handoffs.length).toBeGreaterThan(0)
    expect(runtime.activeRun?.workflowRun).toBeTruthy()
    expect(runtime.activeRun?.workerDispatch?.totalSubruns).toBeGreaterThan(0)

    runtime.dispose()
  })

  it('creates a pet home runtime session without requiring a project id', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-pet-home',
      title: 'Pet Home Session',
      sessionKind: 'pet',
      selectedActorRef: 'agent:agent-architect',
      executionPermissionMode: 'readonly',
    })

    expect(runtime.activeSession?.summary.conversationId).toBe('conv-pet-home')
    expect(runtime.activeSession?.summary.projectId).toBe('')
    expect(runtime.activeSession?.summary.sessionKind).toBe('pet')

    await runtime.submitTurn({
      content: 'Keep this as a home conversation.',
      permissionMode: 'readonly',
    })

    await waitFor(() =>
      runtime.activeRun?.status === 'completed'
      && runtime.activeMessages.some(message => message.content === 'Keep this as a home conversation.'),
    )

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
      permissionMode: 'readonly',
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
      permissionMode: 'auto',
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
        toolName: 'agent-architect',
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

  it('merges auth challenge updates into the optimistic assistant placeholder', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-auth-placeholder',
      projectId: 'proj-redesign',
      title: 'Auth Placeholder Session',
    })

    runtime.addOptimisticUserMessage({
      content: 'Connect the workspace provider before executing.',
      permissionMode: 'auto',
    })

    const baseRun = runtime.activeRun!
    const authChallenge = {
      id: 'auth-placeholder',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-auth-placeholder',
      runId: baseRun.id,
      summary: 'Provider sign-in required',
      detail: 'Resolve the provider auth challenge before execution can continue.',
      status: 'pending',
      createdAt: 110,
      approvalLayer: 'provider-auth',
      escalationReason: 'provider authentication required',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    } as const

    runtime.applyRuntimeEvent({
      id: 'evt-auth-placeholder',
      eventType: 'auth.challenge_requested',
      kind: 'auth.challenge_requested',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-auth-placeholder',
      runId: baseRun.id,
      emittedAt: 110,
      sequence: 1,
      authChallenge,
      run: {
        ...baseRun,
        status: 'waiting_input',
        currentStep: 'awaiting_auth',
        updatedAt: 110,
        nextAction: 'auth',
        authTarget: authChallenge,
        pendingMediation: {
          mediationKind: 'auth',
          state: 'pending',
          summary: authChallenge.summary,
          detail: authChallenge.detail,
          targetKind: authChallenge.targetKind,
          targetRef: authChallenge.targetRef,
          providerKey: authChallenge.providerKey,
          authChallengeId: authChallenge.id,
          requiresAuth: true,
          requiresApproval: false,
        },
      },
    })

    const placeholder = runtime.activeSession?.messages.find(message => message.id.startsWith('optimistic-assistant-'))
    expect(placeholder?.content).toBe('Awaiting authentication…')
    expect(placeholder?.status).toBe('waiting_input')
    expect(placeholder?.processEntries?.some(entry => entry.title === 'Provider sign-in required')).toBe(true)
    expect(runtime.pendingMediation?.mediationKind).toBe('auth')
    expect(runtime.authTarget?.id).toBe('auth-placeholder')

    runtime.applyRuntimeEvent({
      id: 'evt-auth-resolved',
      eventType: 'auth.resolved',
      kind: 'auth.resolved',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: 'conv-auth-placeholder',
      runId: baseRun.id,
      emittedAt: 120,
      sequence: 2,
      authChallenge: {
        ...authChallenge,
        status: 'resolved',
        resolution: 'resolved',
      },
      run: {
        ...baseRun,
        status: 'completed',
        currentStep: 'completed',
        updatedAt: 120,
        nextAction: 'idle',
        authTarget: undefined,
        pendingMediation: undefined,
      },
    })

    expect(runtime.pendingMediation).toBeNull()
    expect(runtime.authTarget).toBeNull()
    expect(runtime.activeRun?.status).toBe('completed')

    runtime.dispose()
  })

  it('preserves the optimistic assistant placeholder across session reload until approval events attach typed mediation state', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-approval-placeholder',
      projectId: 'proj-redesign',
      title: 'Approval Placeholder Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.pendingApproval !== null)
    await waitFor(() => runtime.activeMessages.some(message => message.id.startsWith('optimistic-assistant-')))
    await waitFor(() => runtime.activeMessages.some(message => !!message.approval))

    const approvalMessage = runtime.activeMessages.find(message => !!message.approval)
    expect(approvalMessage?.content).toBe('Awaiting approval…')
    expect(approvalMessage?.status).toBe('waiting_approval')
    expect(approvalMessage?.approval?.summary).toBe('Approve workspace command execution')

    runtime.dispose()
  })

  it('keeps loaded approval state typed without synthesizing an assistant message', async () => {
    const { runtime } = await prepareRuntimeStore()

    const detail = createSessionDetail('conv-loaded-approval', 'proj-redesign')
    detail.run.status = 'waiting_approval'
    detail.run.currentStep = 'runtime.run.waitingApproval'
    detail.run.approvalTarget = {
      id: 'approval-loaded',
      sessionId: detail.summary.id,
      conversationId: detail.summary.conversationId,
      runId: detail.run.id,
      toolName: 'runtime.turn',
      summary: 'Loaded approval',
      detail: 'Server returned a pending approval target.',
      riskLevel: 'medium',
      createdAt: 10,
      status: 'pending',
    }
    detail.pendingApproval = undefined
    detail.messages = []

    runtime.setActiveSession(detail)

    expect(runtime.pendingApproval?.id).toBe('approval-loaded')
    expect(runtime.activeSession?.messages).toEqual([])

    runtime.dispose()
  })

  it('does not synthesize approval assistant messages when an approval event arrives without a placeholder', async () => {
    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-approval-event-without-placeholder',
      projectId: 'proj-redesign',
      title: 'Approval Event Without Placeholder',
    })

    runtime.applyRuntimeEvent({
      id: 'evt-approval-no-placeholder',
      eventType: 'approval.requested',
      kind: 'approval.requested',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      sessionId: runtime.activeSessionId,
      conversationId: runtime.activeConversationId,
      runId: runtime.activeRun?.id,
      emittedAt: 110,
      sequence: 1,
      approval: {
        id: 'approval-no-placeholder',
        sessionId: runtime.activeSessionId,
        conversationId: runtime.activeConversationId,
        runId: runtime.activeRun?.id ?? 'runtime-run-approval-no-placeholder',
        toolName: 'runtime.turn',
        summary: 'Turn requires approval',
        detail: 'The server surfaced a pending approval target.',
        riskLevel: 'medium',
        createdAt: 110,
        status: 'pending',
      },
      run: {
        ...runtime.activeRun!,
        status: 'waiting_approval',
        currentStep: 'runtime.run.waitingApproval',
        updatedAt: 110,
        nextAction: 'runtime.run.awaitingApproval',
        pendingMediation: {
          ...createPendingMediationSummary(),
          mediationKind: 'approval',
          state: 'pending',
        },
      },
    })

    expect(runtime.pendingApproval?.id).toBe('approval-no-placeholder')
    expect(runtime.activeSession?.messages).toEqual([])

    runtime.dispose()
  })

  it('keeps loaded auth state typed without synthesizing an assistant message', async () => {
    const { runtime } = await prepareRuntimeStore()

    const detail = createSessionDetail('conv-loaded-auth', 'proj-redesign')
    detail.run.status = 'waiting_input'
    detail.run.currentStep = 'runtime.run.awaitingAuth'
    detail.run.authTarget = {
      id: 'auth-loaded',
      sessionId: detail.summary.id,
      conversationId: detail.summary.conversationId,
      runId: detail.run.id,
      summary: 'Loaded auth challenge',
      detail: 'Server returned a pending auth challenge.',
      status: 'pending',
      createdAt: 12,
      approvalLayer: 'provider-auth',
      escalationReason: 'provider authentication required',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    }
    detail.run.pendingMediation = {
      ...createPendingMediationSummary(),
      mediationKind: 'auth',
      state: 'pending',
      summary: 'Loaded auth challenge',
      detail: 'Server returned a pending auth challenge.',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      authChallengeId: 'auth-loaded',
      requiresAuth: true,
      requiresApproval: false,
    }
    detail.messages = []

    runtime.setActiveSession(detail)

    expect(runtime.authTarget?.id).toBe('auth-loaded')
    expect(runtime.activeSession?.messages).toEqual([])

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
      permissionMode: 'readonly',
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
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.pendingApproval !== null)

    expect(runtime.activeRun?.status).toBe('waiting_approval')

    await runtime.submitTurn({
      content: 'Then summarize the output.',
      permissionMode: 'auto',
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

  it('ignores stale approval ids instead of resolving the current pending approval', async () => {
    let resolveCalls = 0

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        runtime: {
          ...client.runtime,
          async resolveApproval(sessionId, approvalId, input, idempotencyKey) {
            resolveCalls += 1
            return await client.runtime.resolveApproval(sessionId, approvalId, input, idempotencyKey)
          },
        },
      }
    })

    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-stale-approval',
      projectId: 'proj-redesign',
      title: 'Stale Approval Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.pendingApproval !== null)
    const activeApprovalId = runtime.pendingApproval!.id

    await (runtime as any).resolveApproval('approve', 'approval-stale')

    expect(resolveCalls).toBe(0)
    expect(runtime.pendingApproval?.id).toBe(activeApprovalId)

    runtime.dispose()
  })

  it('locks approval resolution while the current approval request is in flight', async () => {
    let resolveCalls = 0
    let releaseResolution: (() => void) | null = null
    const resolutionGate = new Promise<void>((resolve) => {
      releaseResolution = resolve
    })

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        runtime: {
          ...client.runtime,
          async resolveApproval(sessionId, approvalId, input, idempotencyKey) {
            resolveCalls += 1
            await resolutionGate
            return await client.runtime.resolveApproval(sessionId, approvalId, input, idempotencyKey)
          },
        },
      }
    })

    const { runtime } = await prepareRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-approval-lock',
      projectId: 'proj-redesign',
      title: 'Approval Lock Session',
    })

    await runtime.submitTurn({
      content: 'Run pwd in the workspace terminal.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.pendingApproval !== null)
    const approvalId = runtime.pendingApproval!.id

    const firstResolution = (runtime as any).resolveApproval('approve', approvalId)
    const secondResolution = (runtime as any).resolveApproval('approve', approvalId)

    await waitFor(() => resolveCalls > 0)

    expect(resolveCalls).toBe(1)
    expect((runtime as any).resolvingApprovalIds?.[approvalId]).toBe(true)

    releaseResolution?.()
    await Promise.all([firstResolution, secondResolution])
    await waitFor(() => runtime.pendingApproval === null)
    expect((runtime as any).resolvingApprovalIds?.[approvalId]).toBeFalsy()

    runtime.dispose()
  })

  it('treats typed mediation and auth targets as the canonical paused runtime state', async () => {
    const { runtime } = await prepareRuntimeStore()

    const detail = createSessionDetail(
      'conv-auth-mediation',
      'proj-redesign',
      'Auth Mediation Session',
    )
    detail.summary.pendingMediation = {
      mediationKind: 'auth',
      state: 'pending',
      summary: 'Provider sign-in required',
      detail: 'Resolve the provider auth challenge before execution can continue.',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      authChallengeId: 'auth-workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    }
    detail.pendingMediation = detail.summary.pendingMediation
    detail.run.status = 'blocked'
    detail.run.pendingMediation = detail.summary.pendingMediation
    detail.run.authTarget = {
      id: 'auth-workspace-api',
      sessionId: detail.summary.id,
      conversationId: detail.summary.conversationId,
      runId: detail.run.id,
      summary: 'Provider sign-in required',
      detail: 'Resolve the provider auth challenge before execution can continue.',
      status: 'pending',
      createdAt: detail.run.updatedAt,
      approvalLayer: 'provider-auth',
      escalationReason: 'provider authentication required',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    }

    runtime.setActiveSession(detail)

    expect(runtime.pendingMediation?.mediationKind).toBe('auth')
    expect(runtime.authTarget?.id).toBe('auth-workspace-api')
    expect(runtime.pendingApproval).toBeNull()

    runtime.dispose()
  })

  it('resolves pending memory proposals through the runtime client and refreshes the active session', async () => {
    const { runtime } = await prepareRuntimeStore()

    const detail = createSessionDetail(
      'conv-memory-proposal',
      'proj-redesign',
      'Memory Proposal Session',
    ) as RuntimeSessionDetail & {
      summary: RuntimeSessionDetail['summary'] & {
        memorySelectionSummary: {
          totalCandidateCount: number
          selectedCount: number
          ignoredCount: number
          recallMode: 'default'
          selectedMemoryIds: string[]
        }
        pendingMemoryProposalCount: number
        memoryStateRef: string
      }
      run: RuntimeSessionDetail['run'] & {
        selectedMemory: Array<{
          memoryId: string
          title: string
          summary: string
          kind: string
          scope: string
          freshnessState: string
        }>
        freshnessSummary: {
          freshnessRequired: boolean
          staleCount: number
          freshCount: number
        }
        pendingMemoryProposal: {
          proposalId: string
          status: string
          reason: string
          targetScope: string
          review: {
            status: string
            requiresApproval: boolean
          }
        }
        memoryStateRef: string
      }
    }

    detail.summary.memorySelectionSummary = {
      totalCandidateCount: 1,
      selectedCount: 1,
      ignoredCount: 0,
      recallMode: 'default',
      selectedMemoryIds: ['mem-1'],
    }
    detail.summary.pendingMemoryProposalCount = 1
    detail.summary.memoryStateRef = 'memory-state-conv-memory-proposal'
    ;(detail.run as any).selectedMemory = [
      {
        memoryId: 'mem-1',
        title: 'Preferred delivery style',
        summary: 'User prefers concise implementation summaries.',
        kind: 'user',
        scope: 'user-private',
        freshnessState: 'fresh',
      },
    ]
    ;(detail.run as any).freshnessSummary = {
      freshnessRequired: true,
      staleCount: 0,
      freshCount: 1,
    }
    ;(detail.run as any).pendingMemoryProposal = {
      proposalId: 'memory-proposal-1',
      title: 'User memory proposal',
      summary: 'User prefers concise implementation summaries.',
      proposalState: 'pending',
      proposalReason: 'Validated explicit user preference from latest turn.',
      kind: 'user',
      scope: 'user-private',
    }
    ;(detail.run as any).memoryStateRef = 'memory-state-conv-memory-proposal'

    const resolvedDetail = structuredClone(detail) as typeof detail
    resolvedDetail.summary.pendingMemoryProposalCount = 0
    ;(resolvedDetail.run as any).pendingMemoryProposal = undefined

    const resolveMemoryProposalSpy = vi.fn(async () => resolvedDetail.run)
    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockImplementation((context) => {
      const baseClient = baseImplementation!(context)
      return {
        ...baseClient,
        runtime: {
          ...baseClient.runtime,
          loadSession: async () => structuredClone(resolvedDetail),
          resolveMemoryProposal: resolveMemoryProposalSpy,
        },
      } as ReturnType<typeof tauriClient.createWorkspaceClient>
    })

    runtime.setActiveSession(detail)

    await (runtime as any).resolveMemoryProposal('approve')

    expect(resolveMemoryProposalSpy).toHaveBeenCalledWith(
      detail.summary.id,
      'memory-proposal-1',
      {
        decision: 'approve',
      },
      expect.any(String),
    )
    expect((runtime.activeSession?.run as any).pendingMemoryProposal).toBeUndefined()
    expect((runtime.activeSession?.summary as any).pendingMemoryProposalCount).toBe(0)

    runtime.dispose()
  })

  it('does not expose resolved memory proposals as pending store state', async () => {
    const { runtime } = await prepareRuntimeStore()
    const detail = createSessionDetail(
      'conv-memory-resolved-state',
      'proj-redesign',
      'Resolved Memory Proposal Session',
    )

    ;(detail.run as any).pendingMemoryProposal = {
      proposalId: 'memory-proposal-resolved',
      title: 'Resolved proposal',
      summary: 'This proposal was already reviewed.',
      proposalState: 'approved',
      proposalReason: 'validated',
      kind: 'feedback',
      scope: 'user-private',
    }
    ;(detail.summary as any).pendingMemoryProposalCount = 0
    runtime.setActiveSession(detail)

    expect((runtime as any).pendingMemoryProposal).toBeNull()

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
      permissionMode: 'readonly',
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
      permissionMode: 'danger-full-access',
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
      selectedActorRef: 'team:team-studio',
      selectedConfiguredModelId: 'anthropic-primary',
      executionPermissionMode: 'workspace-write',
    })

    await runtime.submitTurn({
      content: 'Coordinate the redesign rollout.',
      permissionMode: 'auto',
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
        capabilityStateRef: 'capstate-events',
        nextAction: 'runtime.run.idle',
        requestedActorKind: 'agent',
        requestedActorId: 'agent-architect',
        resolvedActorKind: 'agent',
        resolvedActorId: 'agent-architect',
        resolvedActorLabel: '默认智能体',
        pendingMediation: {
          mediationKind: 'none',
        },
        providerStateSummary: [],
        lastExecutionOutcome: {
          outcome: 'success',
        },
        usageSummary: {
          inputTokens: 0,
          outputTokens: 0,
          totalTokens: 0,
        },
        artifactRefs: [],
        traceContext: {
          sessionId: runtime.activeSessionId,
          traceId: 'trace-events',
          turnId: 'turn-events',
        },
        checkpoint: {
          serializedSession: {
            sessionId: runtime.activeSessionId,
            runId: 'runtime-run-1',
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
          capabilityStateRef: 'capstate-events',
          currentIterationIndex: 0,
          pendingMediation: {
            mediationKind: 'none',
          },
          lastExecutionOutcome: {
            outcome: 'success',
          },
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
        },
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
        selectedActorRef: 'agent:agent-architect',
        manifestRevision: 'manifest-local-v2',
        sessionPolicy: {
          selectedActorRef: 'agent:agent-architect',
          selectedConfiguredModelId: 'anthropic-primary',
          executionPermissionMode: 'workspace-write',
          configSnapshotId: 'cfgsnap-local',
          manifestRevision: 'manifest-local-v2',
          capabilityPolicy: {},
          memoryPolicy: {},
          delegationPolicy: {},
          approvalPreference: {},
        },
        activeRunId: 'run-local',
        subrunCount: 0,
        memorySummary: {
          summary: 'No durable memories selected.',
          durableMemoryCount: 0,
          selectedMemoryIds: [],
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
        capabilityStateRef: 'capstate-local',
        pendingMediation: {
          mediationKind: 'none',
        },
        providerStateSummary: [],
        lastExecutionOutcome: {
          outcome: 'success',
        },
      },
      run: {
        id: 'run-local',
        sessionId: 'rt-local',
        conversationId: 'conv-local-stale',
        status: 'draft',
        currentStep: 'runtime.run.idle',
        startedAt: 10,
        updatedAt: 10,
        actorRef: 'agent:agent-architect',
        runKind: 'primary',
        modelId: 'claude-sonnet-4-5',
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
        capabilityStateRef: 'capstate-local',
        configuredModelId: 'anthropic-primary',
        configuredModelName: 'Claude Sonnet 4.5',
        nextAction: 'runtime.run.awaitingInput',
        configSnapshotId: 'cfgsnap-local',
        effectiveConfigHash: 'cfg-hash-local',
        startedFromScopeSet: ['project'],
        approvalState: 'none',
        pendingMediation: {
          mediationKind: 'none',
        },
        providerStateSummary: [],
        lastExecutionOutcome: {
          outcome: 'success',
        },
        usageSummary: {
          inputTokens: 0,
          outputTokens: 0,
          totalTokens: 0,
        },
        artifactRefs: [],
        traceContext: {
          sessionId: 'rt-local',
          traceId: 'trace-local',
          turnId: 'turn-local',
        },
        checkpoint: {
          serializedSession: {
            sessionId: 'rt-local',
            runId: 'run-local',
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
          capabilityStateRef: 'capstate-local',
          currentIterationIndex: 0,
          pendingMediation: {
            mediationKind: 'none',
          },
          lastExecutionOutcome: {
            outcome: 'success',
          },
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
        },
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
        selectedActorRef: 'agent:agent-architect',
        manifestRevision: 'manifest-enterprise-v2',
        sessionPolicy: {
          selectedActorRef: 'agent:agent-architect',
          selectedConfiguredModelId: 'anthropic-primary',
          executionPermissionMode: 'workspace-write',
          configSnapshotId: 'cfgsnap-enterprise',
          manifestRevision: 'manifest-enterprise-v2',
          capabilityPolicy: {},
          memoryPolicy: {},
          delegationPolicy: {},
          approvalPreference: {},
        },
        activeRunId: 'run-enterprise',
        subrunCount: 0,
        memorySummary: {
          summary: 'No durable memories selected.',
          durableMemoryCount: 0,
          selectedMemoryIds: [],
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
        capabilityStateRef: 'capstate-enterprise',
        pendingMediation: {
          mediationKind: 'none',
        },
        providerStateSummary: [],
        lastExecutionOutcome: {
          outcome: 'success',
        },
      },
      run: {
        id: 'run-enterprise',
        sessionId: 'rt-enterprise',
        conversationId: 'conv-enterprise-fresh',
        status: 'draft',
        currentStep: 'runtime.run.idle',
        startedAt: 20,
        updatedAt: 20,
        actorRef: 'agent:agent-architect',
        runKind: 'primary',
        modelId: 'claude-sonnet-4-5',
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
        capabilityStateRef: 'capstate-enterprise',
        configuredModelId: 'anthropic-primary',
        configuredModelName: 'Claude Sonnet 4.5',
        nextAction: 'runtime.run.awaitingInput',
        configSnapshotId: 'cfgsnap-enterprise',
        effectiveConfigHash: 'cfg-hash-enterprise',
        startedFromScopeSet: ['project'],
        approvalState: 'none',
        pendingMediation: {
          mediationKind: 'none',
        },
        providerStateSummary: [],
        lastExecutionOutcome: {
          outcome: 'success',
        },
        usageSummary: {
          inputTokens: 0,
          outputTokens: 0,
          totalTokens: 0,
        },
        artifactRefs: [],
        traceContext: {
          sessionId: 'rt-enterprise',
          traceId: 'trace-enterprise',
          turnId: 'turn-enterprise',
        },
        checkpoint: {
          serializedSession: {
            sessionId: 'rt-enterprise',
            runId: 'run-enterprise',
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
          capabilityStateRef: 'capstate-enterprise',
          currentIterationIndex: 0,
          pendingMediation: {
            mediationKind: 'none',
          },
          lastExecutionOutcome: {
            outcome: 'success',
          },
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
        },
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
        resolveMemoryProposal: async () => {},
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
