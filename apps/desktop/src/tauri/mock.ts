import type {
  ConnectionProfile,
  HostBackendConnection,
  RuntimeBootstrap,
  RuntimeDecisionAction,
  RuntimeEventEnvelope,
  RuntimeMessage,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  RuntimeTraceItem,
  ShellBootstrap,
} from '@octopus/schema'

import {
  fallbackBackendConnection,
  fallbackHostState,
  loadStoredPreferences,
} from './shared'

function buildMockRuntimeBootstrap(): RuntimeBootstrap {
  return {
    provider: {
      provider: 'anthropic',
      defaultModel: 'claude-sonnet-4-5',
    },
    sessions: [],
  }
}

function createMockSessionSummary(
  conversationId: string,
  projectId: string,
  title: string,
  timestamp: number,
): RuntimeSessionSummary {
  return {
    id: `runtime-session-${conversationId}`,
    conversationId,
    projectId,
    title,
    status: 'idle',
    updatedAt: timestamp,
    lastMessagePreview: undefined,
  }
}

function createMockRuntimeSessionDetail(
  conversationId: string,
  projectId: string,
  title: string,
  timestamp: number,
): RuntimeSessionDetail {
  const summary = createMockSessionSummary(conversationId, projectId, title, timestamp)
  return {
    summary,
    run: {
      id: `runtime-run-${conversationId}`,
      sessionId: summary.id,
      conversationId,
      status: 'idle',
      currentStep: 'runtime.run.idle',
      startedAt: timestamp,
      updatedAt: timestamp,
      modelId: undefined,
      nextAction: 'runtime.run.awaitingInput',
    },
    messages: [],
    trace: [],
  }
}

function readMockRuntimeState(): Record<string, RuntimeSessionDetail> {
  if (typeof window === 'undefined') {
    return {}
  }

  return (window as typeof window & { __octopusRuntimeMock__?: Record<string, RuntimeSessionDetail> }).__octopusRuntimeMock__ ?? {}
}

function writeMockRuntimeState(state: Record<string, RuntimeSessionDetail>) {
  if (typeof window === 'undefined') {
    return
  }

  ;(window as typeof window & { __octopusRuntimeMock__?: Record<string, RuntimeSessionDetail> }).__octopusRuntimeMock__ = state
}

function upsertMockSession(detail: RuntimeSessionDetail): RuntimeSessionDetail {
  const state = readMockRuntimeState()
  state[detail.summary.id] = detail
  writeMockRuntimeState(state)
  return detail
}

function loadMockSession(sessionId: string): RuntimeSessionDetail {
  const state = readMockRuntimeState()
  const detail = state[sessionId]
  if (!detail) {
    throw new Error('Mock runtime session not found')
  }
  return detail
}

function listMockSessions(): RuntimeSessionSummary[] {
  return Object.values(readMockRuntimeState()).map((detail) => detail.summary)
}

export function shouldUseMockRuntime(): boolean {
  return true
}

export function resolveMockShellBootstrap(
  defaultWorkspaceId: string,
  defaultProjectId: string,
  mockConnections: ConnectionProfile[],
): ShellBootstrap {
  return {
    hostState: fallbackHostState(),
    preferences: loadStoredPreferences(defaultWorkspaceId, defaultProjectId),
    connections: mockConnections,
    backend: fallbackBackendConnection(),
  }
}

export function resolveMockRuntimeBackendConnection(): HostBackendConnection | undefined {
  return shouldUseMockRuntime() ? undefined : fallbackBackendConnection('unavailable')
}

export function bootstrapMockRuntime(): RuntimeBootstrap {
  return buildMockRuntimeBootstrap()
}

export function createMockRuntimeSession(
  conversationId: string,
  projectId: string,
  title: string,
): RuntimeSessionDetail {
  return upsertMockSession(createMockRuntimeSessionDetail(conversationId, projectId, title, Date.now()))
}

export function loadMockRuntimeSession(sessionId: string): RuntimeSessionDetail {
  return loadMockSession(sessionId)
}

export function pollMockRuntimeEvents(): RuntimeEventEnvelope[] {
  return []
}

export function submitMockRuntimeUserTurn(
  sessionId: string,
  content: string,
  modelId: string,
  permissionMode: string,
): RuntimeRunSnapshot {
  const detail = loadMockSession(sessionId)
  const timestamp = Date.now()
  const waitingApproval = /\b(pwd|rm|delete|terminal|bash|shell)\b/i.test(content)

  const userMessage: RuntimeMessage = {
    id: `runtime-message-user-${timestamp}`,
    sessionId,
    conversationId: detail.summary.conversationId,
    senderType: 'user',
    senderLabel: 'You',
    content,
    timestamp,
    modelId,
    status: 'completed',
  }
  const assistantMessage: RuntimeMessage = {
    id: `runtime-message-assistant-${timestamp}`,
    sessionId,
    conversationId: detail.summary.conversationId,
    senderType: 'assistant',
    senderLabel: 'Octopus Runtime',
    content: waitingApproval ? '运行前需要审批。' : '已记录你的运行请求，并生成了运行摘要。',
    timestamp: timestamp + 1,
    modelId,
    status: waitingApproval ? 'waiting_approval' : 'completed',
  }
  const traceItem: RuntimeTraceItem = {
    id: `runtime-trace-${timestamp}`,
    sessionId,
    runId: detail.run.id,
    conversationId: detail.summary.conversationId,
    kind: waitingApproval ? 'approval' : 'step',
    title: waitingApproval ? 'Requested approval for workspace terminal access' : 'Captured runtime execution step',
    detail: waitingApproval ? 'The runtime requested approval before executing a terminal command.' : `Processed a runtime turn with permission mode ${permissionMode}.`,
    tone: waitingApproval ? 'warning' : 'success',
    timestamp: timestamp + 2,
    actor: 'Octopus Runtime',
    relatedMessageId: assistantMessage.id,
    relatedToolName: waitingApproval ? 'terminal' : undefined,
  }

  const nextRun: RuntimeRunSnapshot = {
    ...detail.run,
    status: waitingApproval ? 'waiting_approval' : 'completed',
    currentStep: waitingApproval ? 'runtime.run.waitingApproval' : 'runtime.run.completed',
    updatedAt: timestamp + 2,
    startedAt: detail.run.startedAt || timestamp,
    modelId,
    nextAction: waitingApproval ? 'runtime.run.awaitingApproval' : 'runtime.run.idle',
  }

  const nextDetail: RuntimeSessionDetail = {
    ...detail,
    summary: {
      ...detail.summary,
      status: nextRun.status,
      updatedAt: timestamp + 2,
      lastMessagePreview: content,
    },
    run: nextRun,
    messages: [...detail.messages, userMessage, assistantMessage],
    trace: [...detail.trace, traceItem],
    pendingApproval: waitingApproval
      ? {
          id: `runtime-approval-${timestamp}`,
          sessionId,
          conversationId: detail.summary.conversationId,
          runId: detail.run.id,
          toolName: 'terminal',
          summary: 'Workspace terminal access requested',
          detail: content,
          riskLevel: 'medium',
          createdAt: timestamp + 1,
        }
      : undefined,
  }

  upsertMockSession(nextDetail)
  return nextRun
}

export function resolveMockRuntimeApproval(
  sessionId: string,
  approvalId: string,
  decision: RuntimeDecisionAction,
): void {
  const detail = loadMockSession(sessionId)
  const timestamp = Date.now()
  const nextDetail: RuntimeSessionDetail = {
    ...detail,
    summary: {
      ...detail.summary,
      status: decision === 'approve' ? 'completed' : 'blocked',
      updatedAt: timestamp,
    },
    run: {
      ...detail.run,
      status: decision === 'approve' ? 'completed' : 'blocked',
      currentStep: decision === 'approve' ? 'runtime.run.resuming' : 'runtime.run.blocked',
      updatedAt: timestamp,
      nextAction: decision === 'approve' ? 'runtime.run.idle' : 'runtime.run.manualRecovery',
    },
    trace: [
      ...detail.trace,
      {
        id: `runtime-trace-approval-${timestamp}`,
        sessionId,
        runId: detail.run.id,
        conversationId: detail.summary.conversationId,
        kind: 'approval',
        title: decision === 'approve' ? 'Approval resolved and run resumed' : 'Approval rejected and run blocked',
        detail: `Approval ${approvalId} was ${decision}.`,
        tone: decision === 'approve' ? 'success' : 'warning',
        timestamp,
        actor: 'Octopus Runtime',
        relatedToolName: 'terminal',
      },
    ],
    pendingApproval: undefined,
  }
  upsertMockSession(nextDetail)
}

export function listMockRuntimeSessions(): RuntimeSessionSummary[] {
  return listMockSessions()
}
