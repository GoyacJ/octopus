import { defineStore } from 'pinia'

import { enumLabel, resolveRunDisplayValue } from '@/i18n/copy'
import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'

import type {
  CreateRuntimeSessionInput,
  Message,
  ProviderConfig,
  ResolveRuntimeApprovalInput,
  RuntimeApprovalRequest,
  RuntimeDecisionAction,
  RuntimeEventEnvelope,
  RuntimeMessage,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  RuntimeTraceItem,
  SubmitRuntimeTurnInput,
  ToolCatalogKind,
} from '@octopus/schema'

type EnsureRuntimeSessionInput = CreateRuntimeSessionInput

type RuntimeSubmitTurnInput = SubmitRuntimeTurnInput & {
  actorLabel: string
}

export interface RuntimeQueueItem extends RuntimeSubmitTurnInput {
  id: string
  sessionId: string
  createdAt: number
}

type RuntimeTransportMode = 'idle' | 'sse' | 'polling'

interface RuntimeWorkspaceSnapshot {
  provider: ProviderConfig | null
  bootstrapped: boolean
  loading: boolean
  sessions: RuntimeSessionSummary[]
  sessionDetails: Record<string, RuntimeSessionDetail>
  activeSessionId: string
  activeConversationId: string
  queuedTurns: Record<string, RuntimeQueueItem[]>
  lastEventIds: Record<string, string>
  error: string
}

function createRuntimeWorkspaceSnapshot(): RuntimeWorkspaceSnapshot {
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
    error: '',
  }
}

function createQueueId(): string {
  return `queue-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function isBusyStatus(status?: string): boolean {
  return status === 'running' || status === 'waiting_input' || status === 'waiting_approval'
}

function toConversationMessage(message: RuntimeMessage): Message {
  return {
    id: message.id,
    conversationId: message.conversationId,
    senderId: message.senderType === 'assistant' ? message.senderLabel : 'runtime-user',
    senderType: message.senderType === 'assistant' ? 'agent' : 'user',
    content: message.content,
    modelId: message.modelId,
    timestamp: message.timestamp,
  }
}

function upsertSessionSummary(
  sessions: RuntimeSessionSummary[],
  summary: RuntimeSessionSummary,
): RuntimeSessionSummary[] {
  const next = sessions.filter((session) => session.id !== summary.id)
  next.push(summary)
  next.sort((left, right) => right.updatedAt - left.updatedAt)
  return next
}

function buildToolStats(trace: RuntimeTraceItem[]): Array<{
  toolId: string
  label: string
  kind: ToolCatalogKind
  count: number
}> {
  const counts = new Map<string, { toolId: string, label: string, kind: ToolCatalogKind, count: number }>()

  for (const item of trace) {
    if (item.kind !== 'tool') {
      continue
    }

    const toolId = item.relatedToolName ?? item.title
    const current = counts.get(toolId)
    if (current) {
      current.count += 1
      continue
    }

    counts.set(toolId, {
      toolId,
      label: item.relatedToolName ?? item.title,
      kind: 'builtin',
      count: 1,
    })
  }

  return [...counts.values()].sort((left, right) => right.count - left.count)
}

function resolveRuntimeEventType(event: RuntimeEventEnvelope): string {
  return event.eventType ?? event.kind ?? 'runtime.error'
}

export const useRuntimeStore = defineStore('runtime', {
  state: () => ({
    provider: null as ProviderConfig | null,
    bootstrapped: false,
    loading: false,
    sessions: [] as RuntimeSessionSummary[],
    sessionDetails: {} as Record<string, RuntimeSessionDetail>,
    activeSessionId: '',
    activeConversationId: '',
    queuedTurns: {} as Record<string, RuntimeQueueItem[]>,
    lastEventIds: {} as Record<string, string>,
    activeWorkspaceConnectionId: '',
    workspaceStateSnapshots: {} as Record<string, RuntimeWorkspaceSnapshot>,
    transportMode: 'idle' as RuntimeTransportMode,
    streamSessionId: '',
    streamSubscription: null as { close: () => void } | null,
    pollingSessionId: '',
    pollingTimer: null as ReturnType<typeof setInterval> | null,
    error: '',
  }),
  getters: {
    activeSession(state): RuntimeSessionDetail | null {
      return state.activeSessionId ? state.sessionDetails[state.activeSessionId] ?? null : null
    },
    activeRun(): RuntimeRunSnapshot | null {
      return this.activeSession?.run ?? null
    },
    activeTrace(): RuntimeTraceItem[] {
      return this.activeSession?.trace ?? []
    },
    activeMessages(): Message[] {
      return (this.activeSession?.messages ?? []).map((message) => toConversationMessage(message))
    },
    pendingApproval(): RuntimeApprovalRequest | null {
      return this.activeSession?.pendingApproval ?? null
    },
    activeRunStatusLabel(): string {
      const status = this.activeRun?.status
      if (!status) {
        return 'N/A'
      }

      try {
        return enumLabel('runStatus', status)
      } catch {
        return status
      }
    },
    activeRunCurrentStepLabel(): string {
      return resolveRunDisplayValue(this.activeRun?.currentStep)
    },
    activeRunNextActionLabel(): string {
      return resolveRunDisplayValue(this.activeRun?.nextAction)
    },
    activeQueue(state): RuntimeQueueItem[] {
      return state.activeSessionId ? state.queuedTurns[state.activeSessionId] ?? [] : []
    },
    activeToolStats(): Array<{ toolId: string, label: string, kind: ToolCatalogKind, count: number }> {
      return buildToolStats(this.activeTrace)
    },
    isBusy(): boolean {
      return isBusyStatus(this.activeRun?.status)
    },
  },
  actions: {
    saveActiveWorkspaceSnapshot() {
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
          error: this.error,
        },
      }
    },
    restoreWorkspaceSnapshot(workspaceConnectionId: string) {
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
      this.error = snapshot.error
    },
    syncWorkspaceScopeFromShell() {
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
    resolveWorkspaceClient(workspaceConnectionId?: string) {
      const shell = useShellStore()
      const targetConnectionId = workspaceConnectionId ?? shell.activeWorkspaceConnection?.workspaceConnectionId ?? ''
      if (!targetConnectionId) {
        return null
      }

      const connection = shell.workspaceConnections.find(item => item.workspaceConnectionId === targetConnectionId)
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
    cacheSessionDetail(detail: RuntimeSessionDetail) {
      this.sessionDetails = {
        ...this.sessionDetails,
        [detail.summary.id]: detail,
      }
      this.sessions = upsertSessionSummary(this.sessions, detail.summary)
    },
    setActiveSession(detail: RuntimeSessionDetail) {
      this.activeSessionId = detail.summary.id
      this.activeConversationId = detail.summary.conversationId
      this.error = ''
      this.cacheSessionDetail(detail)
    },
    async bootstrap() {
      this.syncWorkspaceScopeFromShell()
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return
      }
      const { connectionId, client } = resolvedClient

      if (this.bootstrapped && this.activeWorkspaceConnectionId === connectionId) {
        return
      }

      this.loading = true
      this.error = ''

      try {
        const payload = await client.runtime.bootstrap()
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return
        }

        this.provider = payload.provider
        this.sessions = payload.sessions
        this.bootstrapped = true
        this.saveActiveWorkspaceSnapshot()
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to bootstrap runtime'
      } finally {
        this.loading = false
      }
    },
    stopPolling() {
      if (this.pollingTimer) {
        clearInterval(this.pollingTimer)
        this.pollingTimer = null
      }

      this.pollingSessionId = ''
      if (this.transportMode === 'polling') {
        this.transportMode = 'idle'
      }
    },
    stopRealtimeTransport() {
      if (this.streamSubscription) {
        this.streamSubscription.close()
        this.streamSubscription = null
      }

      this.streamSessionId = ''
      this.stopPolling()
      this.transportMode = 'idle'
    },
    startPolling(sessionId: string, workspaceConnectionId?: string) {
      const targetWorkspaceConnectionId = workspaceConnectionId ?? this.activeWorkspaceConnectionId
      if (this.pollingTimer && this.pollingSessionId === sessionId) {
        return
      }

      this.stopPolling()
      this.transportMode = 'polling'
      this.pollingSessionId = sessionId
      this.pollingTimer = setInterval(() => {
        void this.pollSessionEvents(sessionId, targetWorkspaceConnectionId)
      }, 250)
      void this.pollSessionEvents(sessionId, targetWorkspaceConnectionId)
    },
    async startEventTransport(sessionId: string) {
      const workspaceConnectionId = this.activeWorkspaceConnectionId
      const resolvedClient = this.resolveWorkspaceClient(workspaceConnectionId)
      if (!resolvedClient) {
        return
      }
      const { client } = resolvedClient

      if (this.streamSubscription && this.streamSessionId === sessionId) {
        return
      }

      this.stopRealtimeTransport()

      try {
        const subscription = await client.runtime.subscribeEvents(sessionId, {
          lastEventId: this.lastEventIds[sessionId],
          onEvent: (event) => {
            if (workspaceConnectionId !== this.activeWorkspaceConnectionId) {
              return
            }

            this.applyRuntimeEvent(event)
            void this.finishTransportCycle(sessionId, workspaceConnectionId)
          },
          onError: (error) => {
            if (workspaceConnectionId !== this.activeWorkspaceConnectionId) {
              return
            }

            this.error = error.message
            this.startPolling(sessionId, workspaceConnectionId)
          },
        })

        if (workspaceConnectionId !== this.activeWorkspaceConnectionId) {
          subscription.close()
          return
        }

        this.streamSubscription = subscription
        this.streamSessionId = sessionId
        this.transportMode = 'sse'
      } catch {
        if (workspaceConnectionId === this.activeWorkspaceConnectionId) {
          this.startPolling(sessionId, workspaceConnectionId)
        }
      }
    },
    async finishTransportCycle(sessionId: string, workspaceConnectionId?: string) {
      const targetWorkspaceConnectionId = workspaceConnectionId ?? this.activeWorkspaceConnectionId
      if (targetWorkspaceConnectionId !== this.activeWorkspaceConnectionId || sessionId !== this.activeSessionId) {
        return
      }

      const status = this.activeRun?.status
      if ((status === 'waiting_approval' && this.pendingApproval) || status === 'blocked' || status === 'failed') {
        this.stopRealtimeTransport()
        return
      }

      if (status === 'completed' || status === 'idle') {
        this.stopRealtimeTransport()
        await this.flushQueuedTurn()
      }
    },
    async ensureSession(input: EnsureRuntimeSessionInput): Promise<RuntimeSessionDetail | null> {
      await this.bootstrap()
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return null
      }

      const existingSession = this.sessions.find((session) => session.conversationId === input.conversationId)
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
    async loadSession(sessionId: string): Promise<RuntimeSessionDetail | null> {
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
    enqueueTurn(input: RuntimeSubmitTurnInput) {
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
    removeQueuedTurn(queueId: string) {
      if (!this.activeSessionId) {
        return
      }

      this.queuedTurns = {
        ...this.queuedTurns,
        [this.activeSessionId]: (this.queuedTurns[this.activeSessionId] ?? []).filter((item) => item.id !== queueId),
      }
      this.saveActiveWorkspaceSnapshot()
    },
    async submitTurn(input: RuntimeSubmitTurnInput) {
      if (!this.activeSessionId) {
        throw new Error('No active runtime session selected')
      }

      const trimmed = input.content.trim()
      if (!trimmed) {
        return
      }

      if (this.isBusy) {
        this.enqueueTurn({
          ...input,
          content: trimmed,
        })
        return
      }

      this.error = ''
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        throw new Error('No active workspace connection selected')
      }
      const { connectionId, client } = resolvedClient

      try {
        const run = await client.runtime.submitUserTurn(this.activeSessionId, {
          content: trimmed,
          modelId: input.modelId,
          permissionMode: input.permissionMode,
        }, tauriClient.createIdempotencyKey(`runtime-turn-${connectionId}-${this.activeSessionId}`))
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return
        }

        const activeSession = this.activeSession
        if (activeSession) {
          this.cacheSessionDetail({
            ...activeSession,
            run,
            summary: {
              ...activeSession.summary,
              status: run.status,
              updatedAt: run.updatedAt,
            },
          })
        }
        await this.startEventTransport(this.activeSessionId)
        this.saveActiveWorkspaceSnapshot()
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to submit runtime turn'
      }
    },
    applyRuntimeEvent(event: RuntimeEventEnvelope) {
      const existing = this.sessionDetails[event.sessionId]
      if (!existing) {
        return
      }

      this.lastEventIds = {
        ...this.lastEventIds,
        [event.sessionId]: event.id,
      }

      const nextSummary = event.summary
        ? {
            ...existing.summary,
            ...event.summary,
          }
        : event.run
          ? {
              ...existing.summary,
              status: event.run.status,
              updatedAt: event.run.updatedAt,
            }
          : existing.summary

      const nextDetail: RuntimeSessionDetail = {
        ...existing,
        summary: nextSummary,
        run: event.run ?? existing.run,
        messages: [...existing.messages],
        trace: [...existing.trace],
        pendingApproval: existing.pendingApproval,
      }

      if (event.message && !nextDetail.messages.some((message) => message.id === event.message?.id)) {
        nextDetail.messages.push(event.message)
      }

      if (event.trace && !nextDetail.trace.some((trace) => trace.id === event.trace?.id)) {
        nextDetail.trace.push(event.trace)
      }

      const eventType = resolveRuntimeEventType(event)
      if (eventType === 'runtime.approval.requested') {
        nextDetail.pendingApproval = event.approval
      }

      if (eventType === 'runtime.approval.resolved') {
        nextDetail.pendingApproval = undefined
      }

      if (event.error) {
        this.error = event.error
      }

      this.cacheSessionDetail(nextDetail)
      this.saveActiveWorkspaceSnapshot()
    },
    async pollSessionEvents(sessionId?: string, workspaceConnectionId?: string) {
      const targetWorkspaceConnectionId = workspaceConnectionId ?? this.activeWorkspaceConnectionId
      const targetSessionId = sessionId ?? this.activeSessionId
      if (!targetSessionId) {
        return
      }

      const resolvedClient = this.resolveWorkspaceClient(targetWorkspaceConnectionId)
      if (!resolvedClient) {
        return
      }
      const { client } = resolvedClient

      try {
        const events = await client.runtime.pollEvents(targetSessionId, {
          after: this.lastEventIds[targetSessionId],
        })
        if (targetWorkspaceConnectionId !== this.activeWorkspaceConnectionId) {
          return
        }

        for (const event of events) {
          this.applyRuntimeEvent(event)
        }

        if (!events.length) {
          const detail = await client.runtime.loadSession(targetSessionId)
          if (targetWorkspaceConnectionId !== this.activeWorkspaceConnectionId) {
            return
          }
          this.cacheSessionDetail(detail)
        }

        await this.finishTransportCycle(targetSessionId, targetWorkspaceConnectionId)
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to poll runtime events'
        this.stopPolling()
      }
    },
    async flushQueuedTurn() {
      if (!this.activeSessionId || this.pendingApproval || this.isBusy) {
        return
      }

      const [nextQueuedTurn, ...rest] = this.activeQueue
      if (!nextQueuedTurn) {
        return
      }

      if (this.activeRun?.status === 'blocked' || this.activeRun?.status === 'failed') {
        return
      }

      this.queuedTurns = {
        ...this.queuedTurns,
        [this.activeSessionId]: rest,
      }
      this.saveActiveWorkspaceSnapshot()

      await this.submitTurn(nextQueuedTurn)
    },
    async resolveApproval(decision: RuntimeDecisionAction) {
      if (!this.activeSessionId || !this.pendingApproval) {
        return
      }

      this.error = ''
      const pendingApprovalId = this.pendingApproval.id
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return
      }
      const { connectionId, client } = resolvedClient

      try {
        const input: ResolveRuntimeApprovalInput = { decision }
        await client.runtime.resolveApproval(
          this.activeSessionId,
          pendingApprovalId,
          input,
          tauriClient.createIdempotencyKey(`runtime-approval-${connectionId}-${pendingApprovalId}`),
        )
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return
        }

        const activeSession = this.activeSession
        if (activeSession) {
          this.cacheSessionDetail({
            ...activeSession,
            pendingApproval: undefined,
          })
        }
        await this.startEventTransport(this.activeSessionId)
        this.saveActiveWorkspaceSnapshot()
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to resolve runtime approval'
      }
    },
    dispose() {
      this.saveActiveWorkspaceSnapshot()
      this.stopRealtimeTransport()
    },
  },
})
