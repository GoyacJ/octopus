import type { RuntimeEventEnvelope, RuntimeMessage, RuntimeTraceItem, ToolCatalogKind } from '@octopus/schema'

import {
  appendProcessEntry,
  appendToolCall,
  mergeAssistantMessageWithPlaceholder,
  updateOptimisticAssistantMessage,
} from './runtime_messages'

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

export function resolveRuntimeEventType(event: RuntimeEventEnvelope): string {
  return event.eventType ?? event.kind ?? 'runtime.error'
}

export { buildToolStats }

export const runtimeEventActions = {
  stopPolling(this: any) {
    if (this.pollingTimer) {
      clearInterval(this.pollingTimer)
      this.pollingTimer = null
    }

    this.pollingSessionId = ''
    if (this.transportMode === 'polling') {
      this.transportMode = 'idle'
    }
  },
  stopRealtimeTransport(this: any) {
    if (this.streamSubscription) {
      this.streamSubscription.close()
      this.streamSubscription = null
    }

    this.streamSessionId = ''
    this.stopPolling()
    this.transportMode = 'idle'
  },
  startPolling(this: any, sessionId: string, workspaceConnectionId?: string) {
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
  async startEventTransport(this: any, sessionId: string) {
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
        onEvent: (event: RuntimeEventEnvelope) => {
          if (workspaceConnectionId !== this.activeWorkspaceConnectionId) {
            return
          }

          this.applyRuntimeEvent(event)
          void this.finishTransportCycle(sessionId, workspaceConnectionId)
        },
        onError: (error: Error) => {
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
  async finishTransportCycle(this: any, sessionId: string, workspaceConnectionId?: string) {
    const targetWorkspaceConnectionId = workspaceConnectionId ?? this.activeWorkspaceConnectionId
    if (targetWorkspaceConnectionId !== this.activeWorkspaceConnectionId || sessionId !== this.activeSessionId) {
      return
    }

    const status = this.activeRun?.status
    if (
      (status === 'waiting_approval' && this.pendingApproval)
      || (status === 'waiting_input' && (this.pendingMediation || this.authTarget || this.pendingMemoryProposal))
      || status === 'blocked'
      || status === 'failed'
    ) {
      this.stopRealtimeTransport()
      return
    }

    if (status === 'completed' || status === 'idle') {
      this.stopRealtimeTransport()
      await this.flushQueuedTurn()
    }
  },
  applyRuntimeEvent(this: any, event: RuntimeEventEnvelope) {
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

    const nextDetail = {
      ...existing,
      summary: nextSummary,
      run: event.run ?? existing.run,
      messages: [...existing.messages],
      trace: [...existing.trace],
      pendingApproval: existing.pendingApproval,
      pendingMediation: event.run?.pendingMediation ?? nextSummary.pendingMediation ?? existing.pendingMediation,
      lastMediationOutcome: event.run?.lastMediationOutcome ?? existing.lastMediationOutcome,
      authStateSummary: nextSummary.authStateSummary ?? existing.authStateSummary,
      policyDecisionSummary: nextSummary.policyDecisionSummary ?? existing.policyDecisionSummary,
    }

    if (event.message) {
      const runtimeMessage = event.message

      if (runtimeMessage.senderType === 'user') {
        nextDetail.messages = nextDetail.messages.filter((message: RuntimeMessage) => !(
          message.id.startsWith('optimistic-msg-')
          && message.senderType === 'user'
          && message.content === runtimeMessage.content
        ))
      }

      let nextRuntimeMessage = runtimeMessage
      if (runtimeMessage.senderType === 'assistant') {
        nextRuntimeMessage = mergeAssistantMessageWithPlaceholder(runtimeMessage, nextDetail.messages)
        nextDetail.messages = nextDetail.messages.filter((message: RuntimeMessage) => !(
          message.id.startsWith('optimistic-assistant-')
          && message.senderType === 'assistant'
        ))
      }

      if (!nextDetail.messages.some((message: RuntimeMessage) => message.id === nextRuntimeMessage.id)) {
        nextDetail.messages.push(nextRuntimeMessage)
      }
    }

    if (event.trace && !nextDetail.trace.some((trace: RuntimeTraceItem) => trace.id === event.trace?.id)) {
      nextDetail.trace.push(event.trace)
      nextDetail.messages = updateOptimisticAssistantMessage(nextDetail.messages, (message) => {
        const updatedMessage = {
          ...appendProcessEntry(message, {
            id: event.trace!.id,
            type: event.trace!.kind === 'tool' ? 'tool' : 'execution',
            title: event.trace!.title,
            detail: event.trace!.detail,
            timestamp: event.trace!.timestamp,
            toolId: event.trace!.relatedToolName,
          }),
          content: event.trace!.title,
        }

        if (event.trace!.kind !== 'tool') {
          return updatedMessage
        }

        const toolId = event.trace!.relatedToolName ?? event.trace!.title
        const currentCount = (updatedMessage.toolCalls ?? []).find(item => item.toolId === toolId)?.count ?? 0
        return appendToolCall(updatedMessage, {
          toolId,
          label: event.trace!.relatedToolName ?? event.trace!.title,
          kind: 'builtin',
          count: currentCount + 1,
        })
      })
    }

    const eventType = resolveRuntimeEventType(event)
    if (eventType === 'approval.requested' || eventType === 'runtime.approval.requested') {
      nextDetail.pendingApproval = event.approval
      nextDetail.pendingMediation = event.run?.pendingMediation ?? nextDetail.pendingMediation
      if (event.approval) {
        nextDetail.messages = updateOptimisticAssistantMessage(nextDetail.messages, (message) => ({
          ...appendProcessEntry(message, {
            id: event.approval!.id,
            type: 'result',
            title: event.approval!.summary,
            detail: event.approval!.detail,
            timestamp: event.approval!.createdAt,
          }),
          content: 'Awaiting approval…',
          status: 'waiting_approval',
        }))
      }
    }

    if (eventType === 'approval.resolved' || eventType === 'runtime.approval.resolved') {
      nextDetail.pendingApproval = undefined
      nextDetail.pendingMediation = event.run?.pendingMediation ?? undefined
    }

    if (eventType === 'auth.challenge_requested' && event.authChallenge) {
      nextDetail.pendingApproval = undefined
      nextDetail.pendingMediation = event.run?.pendingMediation ?? nextDetail.pendingMediation
      nextDetail.messages = updateOptimisticAssistantMessage(nextDetail.messages, (message) => ({
        ...appendProcessEntry(message, {
          id: event.authChallenge!.id,
          type: 'result',
          title: event.authChallenge!.summary,
          detail: event.authChallenge!.detail,
          timestamp: event.authChallenge!.createdAt,
        }),
        content: 'Awaiting authentication…',
        status: 'waiting_input',
      }))
    }

    if (eventType === 'auth.resolved' || eventType === 'auth.failed') {
      nextDetail.pendingMediation = event.run?.pendingMediation ?? undefined
    }

    if (event.error) {
      this.error = event.error
    }

    this.cacheSessionDetail(nextDetail)
    this.saveActiveWorkspaceSnapshot()
  },
  async pollSessionEvents(this: any, sessionId?: string, workspaceConnectionId?: string) {
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
}
