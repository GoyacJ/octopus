import type {
  CreateRuntimeSessionInput,
  ProviderConfig,
  RuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  SubmitRuntimeTurnInput,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'

import {
  createRuntimeConfigDrafts,
  createRuntimeConfigValidationState,
  type RuntimeConfigDrafts,
  type RuntimeConfigValidationState,
} from './runtime-config'
import { createPendingApprovalAssistantMessage } from './runtime_messages'

export interface RuntimeQueueItem extends SubmitRuntimeTurnInput {
  id: string
  sessionId: string
  createdAt: number
}

export type RuntimeTransportMode = 'idle' | 'sse' | 'polling'

export interface RuntimeWorkspaceSnapshot {
  provider: ProviderConfig | null
  bootstrapped: boolean
  loading: boolean
  sessions: RuntimeSessionSummary[]
  sessionDetails: Record<string, RuntimeSessionDetail>
  activeSessionId: string
  activeConversationId: string
  queuedTurns: Record<string, RuntimeQueueItem[]>
  lastEventIds: Record<string, string>
  config: RuntimeEffectiveConfig | null
  configDrafts: RuntimeConfigDrafts
  configValidation: RuntimeConfigValidationState
  configuredModelProbeResult: RuntimeConfiguredModelProbeResult | null
  configuredModelProbing: boolean
  configLoading: boolean
  configSaving: boolean
  configValidating: boolean
  configError: string
  error: string
}

export function createRuntimeWorkspaceSnapshot(): RuntimeWorkspaceSnapshot {
  return {
    provider: null,
    bootstrapped: false,
    loading: false,
    sessions: [],
    sessionDetails: {},
    activeSessionId: '',
    activeConversationId: '',
    queuedTurns: {},
    lastEventIds: {},
    config: null,
    configDrafts: createRuntimeConfigDrafts(),
    configValidation: createRuntimeConfigValidationState(),
    configuredModelProbeResult: null,
    configuredModelProbing: false,
    configLoading: false,
    configSaving: false,
    configValidating: false,
    configError: '',
    error: '',
  }
}

export function createQueueId(): string {
  return `queue-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export function isBusyStatus(status?: string): boolean {
  return status === 'running' || status === 'waiting_input' || status === 'waiting_approval'
}

export function normalizeRuntimeSessionDetail(detail: RuntimeSessionDetail): RuntimeSessionDetail {
  if (!detail.pendingApproval) {
    return detail
  }

  const hasApprovalMessage = detail.messages.some(message => message.senderType === 'assistant' && (
    message.id === `approval-assistant-${detail.pendingApproval!.id}`
    || message.status === 'waiting_approval'
  ))

  if (hasApprovalMessage) {
    return detail
  }

  return {
    ...detail,
    messages: [
      ...detail.messages,
      createPendingApprovalAssistantMessage(
        detail.summary.id,
        detail.summary.conversationId,
        detail.pendingApproval,
        detail.run,
      ),
    ],
  }
}

export function upsertSessionSummary(
  sessions: RuntimeSessionSummary[],
  summary: RuntimeSessionSummary,
): RuntimeSessionSummary[] {
  const next = sessions.filter((session) => session.id !== summary.id)
  next.push(summary)
  next.sort((left, right) => right.updatedAt - left.updatedAt)
  return next
}

export const runtimeSessionActions = {
  saveActiveWorkspaceSnapshot(this: any) {
    if (!this.activeWorkspaceConnectionId) {
      return
    }

    this.workspaceStateSnapshots = {
      ...this.workspaceStateSnapshots,
      [this.activeWorkspaceConnectionId]: {
        provider: this.provider,
        bootstrapped: this.bootstrapped,
        loading: this.loading,
        sessions: this.sessions,
        sessionDetails: this.sessionDetails,
        activeSessionId: this.activeSessionId,
        activeConversationId: this.activeConversationId,
        queuedTurns: this.queuedTurns,
        lastEventIds: this.lastEventIds,
        config: this.config,
        configDrafts: { ...this.configDrafts },
        configValidation: { ...this.configValidation },
        configuredModelProbeResult: this.configuredModelProbeResult,
        configuredModelProbing: this.configuredModelProbing,
        configLoading: this.configLoading,
        configSaving: this.configSaving,
        configValidating: this.configValidating,
        configError: this.configError,
        error: this.error,
      },
    }
  },
  restoreWorkspaceSnapshot(this: any, workspaceConnectionId: string) {
    const snapshot = this.workspaceStateSnapshots[workspaceConnectionId] ?? createRuntimeWorkspaceSnapshot()
    this.provider = snapshot.provider
    this.bootstrapped = snapshot.bootstrapped
    this.loading = snapshot.loading
    this.sessions = snapshot.sessions
    this.sessionDetails = snapshot.sessionDetails
    this.activeSessionId = snapshot.activeSessionId
    this.activeConversationId = snapshot.activeConversationId
    this.queuedTurns = snapshot.queuedTurns
    this.lastEventIds = snapshot.lastEventIds
    this.config = snapshot.config
    this.configDrafts = { ...snapshot.configDrafts }
    this.configValidation = { ...snapshot.configValidation }
    this.configuredModelProbeResult = snapshot.configuredModelProbeResult
    this.configuredModelProbing = snapshot.configuredModelProbing
    this.configLoading = snapshot.configLoading
    this.configSaving = snapshot.configSaving
    this.configValidating = snapshot.configValidating
    this.configError = snapshot.configError
    this.error = snapshot.error
  },
  clearWorkspaceScope(this: any, workspaceConnectionId: string) {
    const nextSnapshots = { ...this.workspaceStateSnapshots }
    delete nextSnapshots[workspaceConnectionId]
    this.workspaceStateSnapshots = nextSnapshots

    if (this.activeWorkspaceConnectionId === workspaceConnectionId) {
      this.stopRealtimeTransport()
      const snapshot = createRuntimeWorkspaceSnapshot()
      this.provider = snapshot.provider
      this.bootstrapped = snapshot.bootstrapped
      this.loading = snapshot.loading
      this.sessions = snapshot.sessions
      this.sessionDetails = snapshot.sessionDetails
      this.activeSessionId = snapshot.activeSessionId
      this.activeConversationId = snapshot.activeConversationId
      this.queuedTurns = snapshot.queuedTurns
      this.lastEventIds = snapshot.lastEventIds
      this.config = snapshot.config
      this.configDrafts = { ...snapshot.configDrafts }
      this.configValidation = { ...snapshot.configValidation }
      this.configuredModelProbeResult = snapshot.configuredModelProbeResult
      this.configuredModelProbing = snapshot.configuredModelProbing
      this.configLoading = snapshot.configLoading
      this.configSaving = snapshot.configSaving
      this.configValidating = snapshot.configValidating
      this.configError = snapshot.configError
      this.error = snapshot.error
    }
  },
  syncWorkspaceScopeFromShell(this: any) {
    const shell = useShellStore()
    const nextConnectionId = shell.activeWorkspaceConnection?.workspaceConnectionId ?? ''
    if (!nextConnectionId) {
      this.saveActiveWorkspaceSnapshot()
      this.stopRealtimeTransport()
      this.activeWorkspaceConnectionId = ''
      this.restoreWorkspaceSnapshot('')
      return
    }

    if (this.activeWorkspaceConnectionId === nextConnectionId) {
      return
    }

    this.saveActiveWorkspaceSnapshot()
    this.stopRealtimeTransport()
    this.activeWorkspaceConnectionId = nextConnectionId
    this.restoreWorkspaceSnapshot(nextConnectionId)
  },
  resolveWorkspaceClient(this: any, workspaceConnectionId?: string) {
    const shell = useShellStore()
    const targetConnectionId = workspaceConnectionId ?? shell.activeWorkspaceConnection?.workspaceConnectionId ?? ''
    if (!targetConnectionId) {
      return null
    }

    const connection = shell.workspaceConnections.find((item: { workspaceConnectionId: string }) => item.workspaceConnectionId === targetConnectionId)
    if (!connection) {
      return null
    }

    return {
      connectionId: connection.workspaceConnectionId,
      client: tauriClient.createWorkspaceClient({
        connection,
        session: shell.workspaceSessionsState[connection.workspaceConnectionId],
      }),
    }
  },
  cacheSessionDetail(this: any, detail: RuntimeSessionDetail) {
    this.sessionDetails = {
      ...this.sessionDetails,
      [detail.summary.id]: detail,
    }
    this.sessions = upsertSessionSummary(this.sessions, detail.summary)
  },
  setActiveSession(this: any, detail: RuntimeSessionDetail) {
    const normalizedDetail = normalizeRuntimeSessionDetail(detail)
    this.activeSessionId = normalizedDetail.summary.id
    this.activeConversationId = normalizedDetail.summary.conversationId
    this.error = ''
    this.cacheSessionDetail(normalizedDetail)
  },
  async ensureSession(this: any, input: CreateRuntimeSessionInput): Promise<RuntimeSessionDetail | null> {
    await this.bootstrap()
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const existingSession = this.sessions.find((session: RuntimeSessionSummary) => (
      session.conversationId === input.conversationId
      && session.sessionKind === (input.sessionKind ?? 'project')
    ))
    const { connectionId, client } = resolvedClient

    if (existingSession) {
      return await this.loadSession(existingSession.id)
    }

    try {
      const detail = await client.runtime.createSession(
        input,
        tauriClient.createIdempotencyKey(`runtime-session-${connectionId}-${input.conversationId}`),
      )
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return null
      }

      this.setActiveSession(detail)
      this.saveActiveWorkspaceSnapshot()
      return detail
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Failed to create runtime session'
      return null
    }
  },
  async deleteSession(this: any, sessionId: string): Promise<void> {
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client } = resolvedClient

    try {
      await client.runtime.deleteSession(sessionId)
      this.sessions = this.sessions.filter((session: RuntimeSessionSummary) => session.id !== sessionId)
      const details = { ...this.sessionDetails }
      delete details[sessionId]
      this.sessionDetails = details

      if (this.activeSessionId === sessionId) {
        this.activeSessionId = ''
        this.activeConversationId = ''
      }
      this.saveActiveWorkspaceSnapshot()
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Failed to delete runtime session'
    }
  },
  async loadSession(this: any, sessionId: string): Promise<RuntimeSessionDetail | null> {
    this.syncWorkspaceScopeFromShell()
    const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { connectionId, client } = resolvedClient

    try {
      const detail = await client.runtime.loadSession(sessionId)
      if (this.activeWorkspaceConnectionId !== connectionId) {
        return null
      }

      this.setActiveSession(detail)
      if (isBusyStatus(detail.run.status)) {
        await this.startEventTransport(detail.summary.id)
      } else if (this.pollingSessionId === detail.summary.id || this.streamSessionId === detail.summary.id) {
        this.stopRealtimeTransport()
      }
      this.saveActiveWorkspaceSnapshot()
      return detail
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Failed to load runtime session'
      return null
    }
  },
  enqueueTurn(this: any, input: SubmitRuntimeTurnInput) {
    if (!this.activeSessionId) {
      return
    }

    const nextQueueItem: RuntimeQueueItem = {
      ...input,
      id: createQueueId(),
      sessionId: this.activeSessionId,
      createdAt: Date.now(),
    }
    const queued = this.queuedTurns[this.activeSessionId] ?? []
    this.queuedTurns = {
      ...this.queuedTurns,
      [this.activeSessionId]: [...queued, nextQueueItem],
    }
    this.saveActiveWorkspaceSnapshot()
  },
  removeQueuedTurn(this: any, queueId: string) {
    if (!this.activeSessionId) {
      return
    }

    this.queuedTurns = {
      ...this.queuedTurns,
      [this.activeSessionId]: (this.queuedTurns[this.activeSessionId] ?? []).filter((item: RuntimeQueueItem) => item.id !== queueId),
    }
    this.saveActiveWorkspaceSnapshot()
  },
}
