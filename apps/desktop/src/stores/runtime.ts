import { defineStore } from 'pinia'

import { enumLabel, resolveRunDisplayValue } from '@/i18n/copy'
import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'
import {
  createRuntimeConfigDrafts,
  createRuntimeConfigDraftsFromConfig,
  createRuntimeConfigValidationState,
  parseRuntimeConfigDraft,
  type RuntimeConfigDrafts,
  type RuntimeConfigValidationState,
} from '@/stores/runtime-config'

import {
  resolveRuntimePermissionMode,
  type ConversationAttachment,
  type ConversationActorKind,
  type CreateRuntimeSessionInput,
  type Message,
  type ProviderConfig,
  type ResolveRuntimeApprovalInput,
  type RuntimeApprovalRequest,
  type RuntimeConfigScope,
  type RuntimeConfigValidationResult,
  type RuntimeConfiguredModelProbeResult,
  type RuntimeDecisionAction,
  type RuntimeEventEnvelope,
  type RuntimeEffectiveConfig,
  type RuntimeMessage,
  type RuntimeRunSnapshot,
  type RuntimeSessionDetail,
  type RuntimeSessionSummary,
  type RuntimeTraceItem,
  type RunStatus,
  type SubmitRuntimeTurnInput,
  type ToolCatalogKind,
} from '@octopus/schema'


type EnsureRuntimeSessionInput = CreateRuntimeSessionInput

type RuntimeSubmitTurnInput = SubmitRuntimeTurnInput

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

function createQueueId(): string {
  return `queue-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function isBusyStatus(status?: string): boolean {
  return status === 'running' || status === 'waiting_input' || status === 'waiting_approval'
}

function toConversationAttachments(attachments?: string[]): ConversationAttachment[] {
  return (attachments ?? []).map((attachmentId) => ({
    id: attachmentId,
    name: attachmentId,
    kind: 'file',
  }))
}

function createOptimisticRuntimeMessage(
  sessionId: string,
  conversationId: string,
  input: RuntimeSubmitTurnInput,
  timestamp = Date.now(),
): RuntimeMessage {
  const requestedActorKind = input.actorKind
  const requestedActorId = input.actorId
  const status: RunStatus = resolveRuntimePermissionMode(input.permissionMode) === 'workspace-write'
    ? 'waiting_approval'
    : 'running'

  return {
    id: `optimistic-msg-${timestamp}`,
    sessionId,
    conversationId,
    senderType: 'user',
    senderLabel: 'You',
    content: input.content.trim(),
    timestamp,
    configuredModelId: input.configuredModelId,
    modelId: input.modelId,
    status,
    requestedActorKind,
    requestedActorId,
    resolvedActorKind: requestedActorKind,
    resolvedActorId: requestedActorId,
    resolvedActorLabel: 'You',
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
  }
}

function createOptimisticAssistantMessage(
  sessionId: string,
  conversationId: string,
  input: RuntimeSubmitTurnInput,
  timestamp = Date.now() + 1,
): RuntimeMessage {
  const requestedActorKind = input.actorKind
  const requestedActorId = input.actorId

  return {
    id: `optimistic-assistant-${timestamp}`,
    sessionId,
    conversationId,
    senderType: 'assistant',
    senderLabel: 'Assistant',
    content: 'Thinking…',
    timestamp,
    configuredModelId: input.configuredModelId,
    modelId: input.modelId,
    status: 'running',
    requestedActorKind,
    requestedActorId,
    resolvedActorKind: requestedActorKind,
    resolvedActorId: requestedActorId,
    resolvedActorLabel: 'Assistant',
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
    processEntries: [
      {
        id: `optimistic-process-${timestamp}`,
        type: 'thinking',
        title: 'Thinking',
        detail: 'Preparing the assistant response.',
        timestamp,
      },
    ],
  }
}

function createPendingApprovalAssistantMessage(
  sessionId: string,
  conversationId: string,
  approval: RuntimeApprovalRequest,
  run?: RuntimeRunSnapshot,
): RuntimeMessage {
  return {
    id: `approval-assistant-${approval.id}`,
    sessionId,
    conversationId,
    senderType: 'assistant',
    senderLabel: run?.resolvedActorLabel ?? 'Assistant',
    content: 'Awaiting approval…',
    timestamp: approval.createdAt,
    configuredModelId: run?.configuredModelId,
    modelId: run?.modelId,
    status: 'waiting_approval',
    requestedActorKind: run?.requestedActorKind,
    requestedActorId: run?.requestedActorId,
    resolvedActorKind: run?.resolvedActorKind,
    resolvedActorId: run?.resolvedActorId,
    resolvedActorLabel: run?.resolvedActorLabel ?? 'Assistant',
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
    processEntries: [
      {
        id: approval.id,
        type: 'result',
        title: approval.summary,
        detail: approval.detail,
        timestamp: approval.createdAt,
      },
    ],
  }
}

function appendProcessEntry(message: RuntimeMessage, entry: NonNullable<RuntimeMessage['processEntries']>[number]): RuntimeMessage {
  const entries = message.processEntries ?? []
  if (entries.some(item => item.id === entry.id)) {
    return message
  }

  return {
    ...message,
    processEntries: [...entries, entry],
  }
}

function appendToolCall(
  message: RuntimeMessage,
  toolCall: NonNullable<RuntimeMessage['toolCalls']>[number],
): RuntimeMessage {
  const toolCalls = message.toolCalls ?? []
  const existing = toolCalls.find(item => item.toolId === toolCall.toolId)
  if (!existing) {
    return {
      ...message,
      toolCalls: [...toolCalls, toolCall],
    }
  }

  return {
    ...message,
    toolCalls: toolCalls.map(item => item.toolId === toolCall.toolId
      ? {
          ...item,
          count: Math.max(item.count, toolCall.count),
          label: toolCall.label,
          kind: toolCall.kind,
        }
      : item),
  }
}

function updateOptimisticAssistantMessage(
  messages: RuntimeMessage[],
  updater: (message: RuntimeMessage) => RuntimeMessage,
): RuntimeMessage[] {
  const index = messages.findIndex(message => message.id.startsWith('optimistic-assistant-') && message.senderType === 'assistant')
  if (index === -1) {
    return messages
  }

  const nextMessages = [...messages]
  nextMessages[index] = updater(nextMessages[index]!)
  return nextMessages
}

function mergeAssistantMessageWithPlaceholder(
  incoming: RuntimeMessage,
  messages: RuntimeMessage[],
): RuntimeMessage {
  const placeholder = messages.find(message => message.id.startsWith('optimistic-assistant-') && message.senderType === 'assistant')
  if (!placeholder) {
    return incoming
  }

  const mergedProcessEntries = [
    ...(placeholder.processEntries ?? []),
    ...((incoming.processEntries ?? []).filter(entry => !(placeholder.processEntries ?? []).some(item => item.id === entry.id))),
  ]
  const mergedToolCalls = [
    ...(placeholder.toolCalls ?? []),
    ...((incoming.toolCalls ?? []).filter(entry => !(placeholder.toolCalls ?? []).some(item => item.toolId === entry.toolId && item.label === entry.label && item.count === entry.count))),
  ]

  return {
    ...incoming,
    processEntries: mergedProcessEntries.length ? mergedProcessEntries : incoming.processEntries,
    toolCalls: mergedToolCalls.length ? mergedToolCalls : incoming.toolCalls,
  }
}

function normalizeRuntimeSessionDetail(detail: RuntimeSessionDetail): RuntimeSessionDetail {
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

function toConversationMessage(message: RuntimeMessage, pendingApproval?: RuntimeApprovalRequest): Message {
  const isPendingApprovalMessage = message.senderType === 'assistant'
    && message.status === 'waiting_approval'
    && pendingApproval
    && message.conversationId === pendingApproval.conversationId

  return {
    id: message.id,
    conversationId: message.conversationId,
    senderId: message.resolvedActorId
      ?? (message.senderType === 'assistant' ? message.senderLabel : 'runtime-user'),
    senderType: message.senderType === 'assistant' ? 'agent' : 'user',
    content: message.content,
    modelId: message.configuredModelName ?? message.modelId,
    status: message.status,
    timestamp: message.timestamp,
    actorKind: message.resolvedActorKind,
    actorId: message.resolvedActorId,
    requestedActorKind: message.requestedActorKind,
    requestedActorId: message.requestedActorId,
    usedDefaultActor: message.usedDefaultActor,
    resourceIds: message.resourceIds ?? [],
    attachments: toConversationAttachments(message.attachments),
    artifacts: message.artifacts ?? [],
    usage: message.usage,
    toolCalls: message.toolCalls,
    processEntries: message.processEntries,
    approval: isPendingApprovalMessage
      ? {
          id: pendingApproval.id,
          toolName: pendingApproval.toolName,
          summary: pendingApproval.summary,
          detail: pendingApproval.detail,
          riskLevel: pendingApproval.riskLevel,
          status: pendingApproval.status,
        }
      : undefined,
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
    config: null as RuntimeEffectiveConfig | null,
    configDrafts: createRuntimeConfigDrafts(),
    configValidation: createRuntimeConfigValidationState(),
    configuredModelProbeResult: null as RuntimeConfiguredModelProbeResult | null,
    configuredModelProbing: false,
    configLoading: false,
    configSaving: false,
    configValidating: false,
    configError: '',
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
      return (this.activeSession?.messages ?? []).map((message) => toConversationMessage(message, this.activeSession?.pendingApproval))
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
        || (this.activeSession?.messages ?? []).some(message => (
          message.senderType === 'assistant' && message.id.startsWith('optimistic-assistant-')
        ))
    },
    activeWorkspaceConfig(): RuntimeEffectiveConfig | null {
      return this.config
    },
  },
  actions: {
    async loadWorkspaceConfig(force = false): Promise<RuntimeEffectiveConfig | null> {
      return this.loadConfig(force)
    },
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
    setConfigDraft(scope: RuntimeConfigScope, value: string) {
      this.configDrafts = {
        ...this.configDrafts,
        [scope]: value,
      }
      this.saveActiveWorkspaceSnapshot()
    },
    clearConfiguredModelProbeResult() {
      this.configuredModelProbeResult = null
      this.saveActiveWorkspaceSnapshot()
    },
    async loadConfig(force = false): Promise<RuntimeEffectiveConfig | null> {
      this.syncWorkspaceScopeFromShell()
      if (this.config && !force) {
        return this.config
      }

      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return null
      }
      const { connectionId, client } = resolvedClient

      this.configLoading = true
      this.configError = ''

      try {
        const config = await client.runtime.getConfig()
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return null
        }

        this.config = config
        this.configDrafts = createRuntimeConfigDraftsFromConfig(config)
        this.configValidation = createRuntimeConfigValidationState()
        this.saveActiveWorkspaceSnapshot()
        return config
      } catch (error) {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configError = error instanceof Error ? error.message : 'Failed to load runtime config'
        }
        return null
      } finally {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configLoading = false
        }
      }
    },
    async validateConfig(scope: RuntimeConfigScope): Promise<RuntimeConfigValidationResult> {
      if (scope !== 'workspace') {
        return {
          valid: false,
          errors: ['Settings only supports workspace runtime configuration'],
          warnings: [],
        }
      }

      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return {
          valid: false,
          errors: ['No active workspace connection selected'],
          warnings: [],
        }
      }
      const { connectionId, client } = resolvedClient

      this.configValidating = true
      this.configError = ''

      let patch
      try {
        patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
      } catch (error) {
        const result = {
          valid: false,
          errors: [error instanceof Error ? error.message : `Invalid ${scope} runtime config`],
          warnings: [],
        } satisfies RuntimeConfigValidationResult
        this.configValidation = {
          ...this.configValidation,
          [scope]: result,
        }
        this.configError = result.errors[0] ?? ''
        this.configValidating = false
        this.saveActiveWorkspaceSnapshot()
        return result
      }

      try {
        const result = await client.runtime.validateConfig(patch)
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return result
        }

        this.configValidation = {
          ...this.configValidation,
          [scope]: result,
        }
        this.saveActiveWorkspaceSnapshot()
        return result
      } catch (error) {
        const result = {
          valid: false,
          errors: [error instanceof Error ? error.message : 'Failed to validate runtime config'],
          warnings: [],
        } satisfies RuntimeConfigValidationResult
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configValidation = {
            ...this.configValidation,
            [scope]: result,
          }
          this.configError = result.errors[0] ?? ''
          this.saveActiveWorkspaceSnapshot()
        }
        return result
      } finally {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configValidating = false
        }
      }
    },
    async probeConfiguredModel(
      scope: RuntimeConfigScope,
      configuredModelId: string,
    ): Promise<RuntimeConfiguredModelProbeResult> {
      if (scope !== 'workspace') {
        return {
          valid: false,
          reachable: false,
          configuredModelId,
          errors: ['Settings only supports workspace runtime configuration'],
          warnings: [],
        }
      }

      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return {
          valid: false,
          reachable: false,
          configuredModelId,
          errors: ['No active workspace connection selected'],
          warnings: [],
        }
      }
      const { connectionId, client } = resolvedClient

      let patch
      try {
        patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
      } catch (error) {
        const result = {
          valid: false,
          reachable: false,
          configuredModelId,
          errors: [error instanceof Error ? error.message : `Invalid ${scope} runtime config`],
          warnings: [],
        } satisfies RuntimeConfiguredModelProbeResult
        this.configuredModelProbeResult = result
        this.configValidation = {
          ...this.configValidation,
          [scope]: {
            valid: false,
            errors: result.errors,
            warnings: result.warnings,
          },
        }
        this.configError = result.errors[0] ?? ''
        this.saveActiveWorkspaceSnapshot()
        return result
      }

      this.configuredModelProbing = true
      this.configError = ''
      try {
        const result = await client.runtime.validateConfiguredModel({
          scope,
          configuredModelId,
          patch: patch.patch,
        })
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return result
        }

        this.configuredModelProbeResult = result
        this.configValidation = {
          ...this.configValidation,
          [scope]: {
            valid: result.valid && result.reachable,
            errors: result.errors,
            warnings: result.warnings,
          },
        }
        if (result.errors.length > 0) {
          this.configError = result.errors[0] ?? ''
        }
        this.saveActiveWorkspaceSnapshot()
        return result
      } catch (error) {
        const result = {
          valid: false,
          reachable: false,
          configuredModelId,
          errors: [error instanceof Error ? error.message : 'Failed to validate configured model'],
          warnings: [],
        } satisfies RuntimeConfiguredModelProbeResult
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configuredModelProbeResult = result
          this.configValidation = {
            ...this.configValidation,
            [scope]: {
              valid: false,
              errors: result.errors,
              warnings: result.warnings,
            },
          }
          this.configError = result.errors[0] ?? ''
          this.saveActiveWorkspaceSnapshot()
        }
        return result
      } finally {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configuredModelProbing = false
        }
      }
    },
    async saveConfig(scope: RuntimeConfigScope): Promise<RuntimeEffectiveConfig | null> {
      if (scope !== 'workspace') {
        this.configError = 'Settings only supports workspace runtime configuration'
        return null
      }

      const validation = await this.validateConfig(scope)
      if (!validation.valid) {
        return null
      }

      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        this.configError = 'No active workspace connection selected'
        return null
      }
      const { connectionId, client } = resolvedClient

      this.configSaving = true
      this.configError = ''

      try {
        const patch = parseRuntimeConfigDraft(scope, this.configDrafts[scope])
        const config = await client.runtime.saveConfig(patch)
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return null
        }

        this.config = config
        this.configDrafts = createRuntimeConfigDraftsFromConfig(config)
        this.configValidation = {
          ...createRuntimeConfigValidationState(),
          [scope]: config.validation,
        }
        this.saveActiveWorkspaceSnapshot()
        return config
      } catch (error) {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configError = error instanceof Error ? error.message : 'Failed to save runtime config'
        }
        return null
      } finally {
        if (this.activeWorkspaceConnectionId === connectionId) {
          this.configSaving = false
        }
      }
    },
    setActiveSession(detail: RuntimeSessionDetail) {
      const normalizedDetail = normalizeRuntimeSessionDetail(detail)
      this.activeSessionId = normalizedDetail.summary.id
      this.activeConversationId = normalizedDetail.summary.conversationId
      this.error = ''
      this.cacheSessionDetail(normalizedDetail)
    },
    addOptimisticUserMessage(input: RuntimeSubmitTurnInput) {
      if (!this.activeSession) {
        return
      }

      const optimisticUserMessage = createOptimisticRuntimeMessage(
        this.activeSessionId,
        this.activeConversationId,
        input,
      )
      const optimisticAssistantMessage = createOptimisticAssistantMessage(
        this.activeSessionId,
        this.activeConversationId,
        input,
        optimisticUserMessage.timestamp + 1,
      )
      const detail: RuntimeSessionDetail = {
        ...this.activeSession,
        summary: {
          ...this.activeSession.summary,
          updatedAt: optimisticAssistantMessage.timestamp,
          lastMessagePreview: optimisticUserMessage.content,
        },
        messages: [
          ...this.activeSession.messages,
          optimisticUserMessage,
          optimisticAssistantMessage,
        ],
      }
      this.cacheSessionDetail(detail)
      this.saveActiveWorkspaceSnapshot()
    },
    replaceOptimisticMessages(content: string, sessionId?: string) {
      const targetSessionId = sessionId ?? this.activeSessionId
      if (!targetSessionId) {
        return
      }
      const detail = this.sessionDetails[targetSessionId]
      if (!detail) {
        return
      }

      const nextMessages = detail.messages.filter(message => !(
        (message.id.startsWith('optimistic-msg-')
          && message.senderType === 'user'
          && message.content === content)
        || (message.id.startsWith('optimistic-assistant-')
          && message.senderType === 'assistant')
      ))

      if (nextMessages.length === detail.messages.length) {
        return
      }

      this.cacheSessionDetail({
        ...detail,
        messages: nextMessages,
      })
      this.saveActiveWorkspaceSnapshot()
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

      const existingSession = this.sessions.find((session) => (
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
    async deleteSession(sessionId: string): Promise<void> {
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        return
      }
      const { client } = resolvedClient

      try {
        await client.runtime.deleteSession(sessionId)
        this.sessions = this.sessions.filter((session) => session.id !== sessionId)
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
    async submitTurn(input: RuntimeSubmitTurnInput): Promise<boolean> {
      if (!this.activeSessionId) {
        throw new Error('No active runtime session selected')
      }

      const trimmed = input.content.trim()
      if (!trimmed) {
        return false
      }

      if (this.isBusy) {
        this.enqueueTurn({
          ...input,
          content: trimmed,
        })
        return true
      }

      const normalizedInput = {
        ...input,
        content: trimmed,
      }

      this.error = ''
      this.addOptimisticUserMessage(normalizedInput)
      const resolvedClient = this.resolveWorkspaceClient(this.activeWorkspaceConnectionId)
      if (!resolvedClient) {
        this.replaceOptimisticMessages(trimmed)
        throw new Error('No active workspace connection selected')
      }
      const { connectionId, client } = resolvedClient

      try {
        const run = await client.runtime.submitUserTurn(this.activeSessionId, {
          content: trimmed,
          modelId: input.modelId,
          configuredModelId: input.configuredModelId,
          permissionMode: input.permissionMode,
          actorKind: input.actorKind,
          actorId: input.actorId,
        }, tauriClient.createIdempotencyKey(`runtime-turn-${connectionId}-${this.activeSessionId}`))
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return false
        }

        const detail = await client.runtime.loadSession(this.activeSessionId)
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return false
        }

        this.setActiveSession(detail)
        if (isBusyStatus(run.status)) {
          await this.startEventTransport(this.activeSessionId)
        } else {
          await this.finishTransportCycle(this.activeSessionId, connectionId)
        }
        this.saveActiveWorkspaceSnapshot()
        return true
      } catch (error) {
        this.replaceOptimisticMessages(trimmed)
        this.error = error instanceof Error ? error.message : 'Failed to submit runtime turn'
        return false
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

      if (event.message) {
        const runtimeMessage = event.message

        if (runtimeMessage.senderType === 'user') {
          nextDetail.messages = nextDetail.messages.filter(message => !(
            message.id.startsWith('optimistic-msg-')
            && message.senderType === 'user'
            && message.content === runtimeMessage.content
          ))
        }

        let nextRuntimeMessage = runtimeMessage
        if (runtimeMessage.senderType === 'assistant') {
          nextRuntimeMessage = mergeAssistantMessageWithPlaceholder(runtimeMessage, nextDetail.messages)
          nextDetail.messages = nextDetail.messages.filter(message => !(
            message.id.startsWith('optimistic-assistant-')
            && message.senderType === 'assistant'
          ))
        }

        if (!nextDetail.messages.some((message) => message.id === nextRuntimeMessage.id)) {
          nextDetail.messages.push(nextRuntimeMessage)
        }
      }

      if (event.trace && !nextDetail.trace.some((trace) => trace.id === event.trace?.id)) {
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
      if (eventType === 'runtime.approval.requested') {
        nextDetail.pendingApproval = event.approval
        if (event.approval) {
          const updatedMessages = updateOptimisticAssistantMessage(nextDetail.messages, message => ({
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
          nextDetail.messages = updatedMessages

          if (!updatedMessages.some(message => message.id.startsWith('optimistic-assistant-') && message.senderType === 'assistant')) {
            nextDetail.messages.push(createPendingApprovalAssistantMessage(
              nextDetail.summary.id,
              nextDetail.summary.conversationId,
              event.approval,
              event.run ?? nextDetail.run,
            ))
          }
        }
      }

      if (eventType === 'runtime.approval.resolved') {
        nextDetail.pendingApproval = undefined
      }

      if (event.error) {
        this.error = event.error
      }

      this.cacheSessionDetail(normalizeRuntimeSessionDetail(nextDetail))
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

        const detail = await client.runtime.loadSession(this.activeSessionId)
        if (this.activeWorkspaceConnectionId !== connectionId) {
          return
        }

        this.setActiveSession(detail)
        if (isBusyStatus(detail.run.status)) {
          await this.startEventTransport(this.activeSessionId)
        } else {
          await this.finishTransportCycle(this.activeSessionId, connectionId)
        }
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
