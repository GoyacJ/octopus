import { defineStore } from 'pinia'

import { enumLabel, resolveRunDisplayValue } from '@/i18n/copy'

import type {
  CreateRuntimeSessionInput,
  Message,
  PermissionMode,
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

import {
  bootstrapRuntime,
  createRuntimeSession,
  loadRuntimeSession,
  pollRuntimeEvents,
  resolveRuntimeApproval,
  submitRuntimeUserTurn,
} from '@/tauri/client'

type EnsureRuntimeSessionInput = CreateRuntimeSessionInput

type RuntimeSubmitTurnInput = SubmitRuntimeTurnInput & {
  actorLabel: string
}

export interface RuntimeQueueItem extends RuntimeSubmitTurnInput {
  id: string
  sessionId: string
  createdAt: number
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
      if (this.bootstrapped) {
        return
      }

      this.loading = true
      this.error = ''

      try {
        const payload = await bootstrapRuntime()
        this.provider = payload.provider
        this.sessions = payload.sessions
        this.bootstrapped = true
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
    },
    startPolling(sessionId: string) {
      if (this.pollingTimer && this.pollingSessionId === sessionId) {
        return
      }

      this.stopPolling()
      this.pollingSessionId = sessionId
      this.pollingTimer = setInterval(() => {
        void this.pollSessionEvents(sessionId)
      }, 60)
      void this.pollSessionEvents(sessionId)
    },
    async ensureSession(input: EnsureRuntimeSessionInput): Promise<RuntimeSessionDetail | null> {
      await this.bootstrap()
      const existingSession = this.sessions.find((session) => session.conversationId === input.conversationId)

      if (existingSession) {
        return await this.loadSession(existingSession.id)
      }

      try {
        const detail = await createRuntimeSession(input)
        this.setActiveSession(detail)
        return detail
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to create runtime session'
        return null
      }
    },
    async loadSession(sessionId: string): Promise<RuntimeSessionDetail | null> {
      try {
        const detail = await loadRuntimeSession(sessionId)
        this.setActiveSession(detail)
        if (isBusyStatus(detail.run.status)) {
          this.startPolling(detail.summary.id)
        } else if (this.pollingSessionId === detail.summary.id) {
          this.stopPolling()
        }
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
    },
    removeQueuedTurn(queueId: string) {
      if (!this.activeSessionId) {
        return
      }

      this.queuedTurns = {
        ...this.queuedTurns,
        [this.activeSessionId]: (this.queuedTurns[this.activeSessionId] ?? []).filter((item) => item.id !== queueId),
      }
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

      try {
        const run = await submitRuntimeUserTurn(this.activeSessionId, {
          content: trimmed,
          modelId: input.modelId,
          permissionMode: input.permissionMode,
        })

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
        this.startPolling(this.activeSessionId)
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to submit runtime turn'
      }
    },
    applyRuntimeEvent(event: RuntimeEventEnvelope) {
      const existing = this.sessionDetails[event.sessionId]
      if (!existing) {
        return
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

      if (event.kind === 'approval_requested') {
        nextDetail.pendingApproval = event.approval
      }

      if (event.kind === 'approval_resolved') {
        nextDetail.pendingApproval = undefined
      }

      if (event.error) {
        this.error = event.error
      }

      this.cacheSessionDetail(nextDetail)
    },
    async pollSessionEvents(sessionId?: string) {
      const targetSessionId = sessionId ?? this.activeSessionId
      if (!targetSessionId) {
        return
      }

      try {
        const events = await pollRuntimeEvents(targetSessionId)
        for (const event of events) {
          this.applyRuntimeEvent(event)
        }

        if (!events.length) {
          const detail = await loadRuntimeSession(targetSessionId)
          if (detail) {
            this.cacheSessionDetail(detail)
          }
        }

        if (targetSessionId !== this.activeSessionId) {
          return
        }

        const status = this.activeRun?.status
        if (status === 'waiting_approval' || status === 'blocked' || status === 'failed') {
          this.stopPolling()
          return
        }

        if (status === 'completed' || status === 'idle') {
          this.stopPolling()
          await this.flushQueuedTurn()
        }
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

      await this.submitTurn(nextQueuedTurn)
    },
    async resolveApproval(decision: RuntimeDecisionAction) {
      if (!this.activeSessionId || !this.pendingApproval) {
        return
      }

      this.error = ''

      try {
        const input: ResolveRuntimeApprovalInput = { decision }
        await resolveRuntimeApproval(this.activeSessionId, this.pendingApproval.id, input)
        const activeSession = this.activeSession
        if (activeSession) {
          this.cacheSessionDetail({
            ...activeSession,
            pendingApproval: undefined,
          })
        }
        this.startPolling(this.activeSessionId)
      } catch (error) {
        this.error = error instanceof Error ? error.message : 'Failed to resolve runtime approval'
      }
    },
    dispose() {
      this.stopPolling()
    },
  },
})
