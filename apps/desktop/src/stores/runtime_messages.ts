import {
  type ConversationAttachment,
  type Message,
  type PermissionMode,
  type RuntimeApprovalRequest,
  type RuntimeAuthChallengeSummary,
  type RuntimeMessage,
  type RuntimePermissionMode,
  type RuntimeRunSnapshot,
  type SubmitRuntimeTurnInput,
} from '@octopus/schema'

function parseActorRef(actorRef?: string): {
  actorKind?: RuntimeMessage['resolvedActorKind']
  actorId?: string
} {
  if (!actorRef) {
    return {}
  }

  const [actorKind, actorId] = actorRef.split(':', 2)
  if (!actorKind || !actorId) {
    return {}
  }

  if (actorKind !== 'agent' && actorKind !== 'team') {
    return {}
  }

  return {
    actorKind: actorKind as RuntimeMessage['resolvedActorKind'],
    actorId,
  }
}

function toConversationAttachments(attachments?: string[]): ConversationAttachment[] {
  return (attachments ?? []).map((attachmentId) => ({
    id: attachmentId,
    name: attachmentId,
    kind: 'file',
  }))
}

export function createOptimisticRuntimeMessage(
  sessionId: string,
  conversationId: string,
  input: Pick<SubmitRuntimeTurnInput, 'content'> & { permissionMode?: PermissionMode | RuntimePermissionMode },
  run?: RuntimeRunSnapshot,
  selectedActorRef?: string,
  timestamp = Date.now(),
): RuntimeMessage {
  const requestedActor = parseActorRef(selectedActorRef)
  const resolvedActorKind = run?.resolvedActorKind ?? requestedActor.actorKind
  const resolvedActorId = run?.resolvedActorId ?? requestedActor.actorId

  return {
    id: `optimistic-msg-${timestamp}`,
    sessionId,
    conversationId,
    senderType: 'user',
    senderLabel: 'You',
    content: input.content.trim(),
    timestamp,
    configuredModelId: run?.configuredModelId,
    configuredModelName: run?.configuredModelName,
    modelId: run?.modelId,
    status: 'running',
    requestedActorKind: requestedActor.actorKind,
    requestedActorId: requestedActor.actorId,
    resolvedActorKind,
    resolvedActorId,
    resolvedActorLabel: 'You',
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
    deliverableRefs: [],
  }
}

export function createOptimisticAssistantMessage(
  sessionId: string,
  conversationId: string,
  _input: Pick<SubmitRuntimeTurnInput, 'content'> & { permissionMode?: PermissionMode | RuntimePermissionMode },
  run?: RuntimeRunSnapshot,
  selectedActorRef?: string,
  timestamp = Date.now() + 1,
): RuntimeMessage {
  const requestedActor = parseActorRef(selectedActorRef)
  const resolvedActorKind = run?.resolvedActorKind ?? requestedActor.actorKind
  const resolvedActorId = run?.resolvedActorId ?? requestedActor.actorId
  const resolvedActorLabel = run?.resolvedActorLabel ?? 'Assistant'

  return {
    id: `optimistic-assistant-${timestamp}`,
    sessionId,
    conversationId,
    senderType: 'assistant',
    senderLabel: resolvedActorLabel,
    content: 'Thinking…',
    timestamp,
    configuredModelId: run?.configuredModelId,
    configuredModelName: run?.configuredModelName,
    modelId: run?.modelId,
    status: 'running',
    requestedActorKind: requestedActor.actorKind,
    requestedActorId: requestedActor.actorId,
    resolvedActorKind,
    resolvedActorId,
    resolvedActorLabel,
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
    deliverableRefs: [],
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

export function createPendingApprovalAssistantMessage(
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
    deliverableRefs: [],
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

export function createPendingAuthAssistantMessage(
  sessionId: string,
  conversationId: string,
  challenge: RuntimeAuthChallengeSummary,
  run?: RuntimeRunSnapshot,
): RuntimeMessage {
  return {
    id: `auth-assistant-${challenge.id}`,
    sessionId,
    conversationId,
    senderType: 'assistant',
    senderLabel: run?.resolvedActorLabel ?? 'Assistant',
    content: 'Awaiting authentication…',
    timestamp: challenge.createdAt,
    configuredModelId: run?.configuredModelId,
    modelId: run?.modelId,
    status: 'waiting_input',
    requestedActorKind: run?.requestedActorKind,
    requestedActorId: run?.requestedActorId,
    resolvedActorKind: run?.resolvedActorKind,
    resolvedActorId: run?.resolvedActorId,
    resolvedActorLabel: run?.resolvedActorLabel ?? 'Assistant',
    usedDefaultActor: false,
    resourceIds: [],
    attachments: [],
    artifacts: [],
    deliverableRefs: [],
    processEntries: [
      {
        id: challenge.id,
        type: 'result',
        title: challenge.summary,
        detail: challenge.detail,
        timestamp: challenge.createdAt,
      },
    ],
  }
}

export function appendProcessEntry(message: RuntimeMessage, entry: NonNullable<RuntimeMessage['processEntries']>[number]): RuntimeMessage {
  const entries = message.processEntries ?? []
  if (entries.some(item => item.id === entry.id)) {
    return message
  }

  return {
    ...message,
    processEntries: [...entries, entry],
  }
}

export function appendToolCall(
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

export function updateOptimisticAssistantMessage(
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

export function mergeAssistantMessageWithPlaceholder(
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

export function toConversationMessage(message: RuntimeMessage, pendingApproval?: RuntimeApprovalRequest): Message {
  const approvalEntryMatch = pendingApproval
    ? (message.processEntries ?? []).some(entry => entry.id === pendingApproval.id)
    : false
  const isPendingApprovalMessage = message.senderType === 'assistant'
    && message.status === 'waiting_approval'
    && pendingApproval
    && (
      message.id === `approval-assistant-${pendingApproval.id}`
      || approvalEntryMatch
    )

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
    deliverableRefs: message.deliverableRefs ?? [],
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
