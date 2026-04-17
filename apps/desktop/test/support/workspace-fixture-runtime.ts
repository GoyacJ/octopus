import type {
  PetConversationBinding,
  PetMessage,
  PetPresenceState,
  PetProfile,
  PetWorkspaceSnapshot,
  RuntimeCapabilityPlanSummary,
  RuntimeCapabilityProviderState,
  RuntimePendingMediationSummary,
  RuntimeApprovalRequest,
  RuntimeEffectiveConfig,
  RuntimeEventEnvelope,
  RuntimeMessage,
  RuntimeSessionDetail,
  RuntimeTraceItem,
} from '@octopus/schema'

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function createCapabilityPlanSummary(): RuntimeCapabilityPlanSummary {
  return {
    activatedTools: ['workspace-api'],
    approvedTools: [],
    authResolvedTools: [],
    availableResources: [],
    deferredTools: [],
    discoverableSkills: ['docs'],
    grantedTools: ['workspace-api'],
    hiddenCapabilities: [],
    pendingTools: [],
    providerFallbacks: [],
    visibleTools: ['workspace-api'],
  }
}

function createProviderStateSummary(): RuntimeCapabilityProviderState[] {
  return [
    {
      providerKey: 'workspace-api',
      state: 'ready',
      degraded: false,
    },
  ]
}

export function createPendingMediationSummary(): RuntimePendingMediationSummary {
  return {
    mediationKind: 'none',
  }
}

function createWorkflowSummary(
  workflowRunId: string,
  currentStepId: string,
  currentStepLabel: string,
  updatedAt: number,
) {
  return {
    workflowRunId,
    label: 'Runtime workflow',
    status: 'completed',
    totalSteps: 3,
    completedSteps: 3,
    currentStepId,
    currentStepLabel,
    backgroundCapable: true,
    updatedAt,
  }
}

function createMailboxSummary(mailboxRef: string, updatedAt: number) {
  return {
    mailboxRef,
    channel: 'leader-hub',
    status: 'completed',
    pendingCount: 0,
    totalMessages: 2,
    updatedAt,
  }
}

function createBackgroundRunSummary(runId: string, workflowRunId: string, updatedAt: number) {
  return {
    runId,
    workflowRunId,
    status: 'completed',
    backgroundCapable: true,
    updatedAt,
  }
}

export interface RuntimeSessionState {
  detail: RuntimeSessionDetail
  events: RuntimeEventEnvelope[]
  nextSequence: number
}

export function createRuntimeConfigSource(
  scope: 'workspace' | 'project' | 'user',
  workspaceId: string,
  ownerId?: string,
): RuntimeEffectiveConfig['sources'][number] {
  if (scope === 'workspace') {
    return {
      scope,
      displayPath: 'config/runtime/workspace.json',
      sourceKey: 'workspace',
      exists: true,
      loaded: true,
      contentHash: `${workspaceId}-workspace-runtime-source-hash`,
      document: {
        model: 'claude-sonnet-4-5',
        permissions: {
          defaultMode: 'plan',
        },
        toolCatalog: {
          disabledSourceKeys: [],
        },
        mcpServers: {
          ops: {
            type: 'http',
            url: 'https://ops.example.test/mcp',
          },
        },
      },
    }
  }

  if (scope === 'project') {
    return {
      scope,
      ownerId,
      displayPath: `config/runtime/projects/${ownerId}.json`,
      sourceKey: `project:${ownerId}`,
      exists: true,
      loaded: true,
      contentHash: `${workspaceId}-${ownerId}-project-runtime-source-hash`,
      document: {
        approvals: {
          defaultMode: 'manual',
        },
      },
    }
  }

  return {
    scope,
    ownerId,
    displayPath: `config/runtime/users/${ownerId}.json`,
    sourceKey: `user:${ownerId}`,
    exists: true,
    loaded: true,
    contentHash: `${workspaceId}-${ownerId}-user-runtime-source-hash`,
    document: {
      provider: {
        defaultModel: 'claude-sonnet-4-5',
      },
    },
  }
}

export function createPetProfile(): PetProfile {
  return {
    id: 'pet-octopus',
    species: 'octopus',
    displayName: '小章',
    ownerUserId: 'user-owner',
    avatarLabel: 'Octopus mascot',
    summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
    greeting: '嗨！我是小章，今天也要加油哦！',
    mood: 'happy',
    favoriteSnack: '新鲜小虾',
    promptHints: ['最近有什么好消息？', '给我讲个冷笑话', '我们要加油呀！'],
    fallbackAsset: 'octopus',
    riveAsset: undefined,
    stateMachine: undefined,
  }
}

export function createPetPresenceState(petId = 'pet-octopus'): PetPresenceState {
  return {
    petId,
    isVisible: true,
    chatOpen: false,
    motionState: 'idle',
    unreadCount: 0,
    lastInteractionAt: 0,
    position: { x: 0, y: 0 },
  }
}

function mapRuntimeMessageToPetMessage(message: RuntimeMessage, petId: string): PetMessage {
  return {
    id: message.id,
    petId,
    sender: message.senderType === 'assistant' ? 'pet' : 'user',
    content: message.content,
    timestamp: message.timestamp,
  }
}

export function createPetSnapshot(
  workspaceState: {
    workspace: { id: string }
    petProfile: PetProfile
    workspacePetPresence: PetPresenceState
    projectPetPresences: Record<string, PetPresenceState>
    workspacePetBinding?: PetConversationBinding
    projectPetBindings: Record<string, PetConversationBinding>
    runtimeSessions: Map<string, RuntimeSessionState>
  },
  projectId?: string,
): PetWorkspaceSnapshot {
  const binding = projectId
    ? workspaceState.projectPetBindings[projectId]
    : workspaceState.workspacePetBinding
  const presence = projectId
    ? workspaceState.projectPetPresences[projectId] ?? createPetPresenceState(workspaceState.petProfile.id)
    : workspaceState.workspacePetPresence
  const runtimeMessages = binding
    ? [...workspaceState.runtimeSessions.values()]
      .find(state => state.detail.summary.conversationId === binding.conversationId)
      ?.detail.messages
      .map(message => mapRuntimeMessageToPetMessage(message, workspaceState.petProfile.id)) ?? []
    : []
  return {
    workspaceId: workspaceState.workspace.id,
    ownerUserId: workspaceState.petProfile.ownerUserId,
    contextScope: projectId ? 'project' : 'home',
    projectId,
    profile: clone(workspaceState.petProfile),
    presence: clone(presence),
    binding: binding ? clone(binding) : undefined,
    messages: runtimeMessages,
  }
}

export function createSessionDetail(
  conversationId: string,
  projectId = '',
  title: string,
  sessionKind: 'project' | 'pet' = 'project',
  selectedActorRef?: string,
  selectedConfiguredModelId = 'anthropic-primary',
  executionPermissionMode = 'workspace-write',
): RuntimeSessionDetail {
  const sessionId = `rt-${conversationId}`
  const runId = `run-${conversationId}`
  const workflowRunId = `workflow-${runId}`
  const mailboxRef = `mailbox-${runId}`
  const memoryStateRef = `memstate-${runId}`
  const resolvedSelectedActorRef = selectedActorRef ?? (sessionKind === 'pet' ? 'team:pet-runtime' : 'agent:agent-architect')
  const isHomeSession = !projectId
  const selectedMemory = isHomeSession
    ? []
    : [
        {
          memoryId: `mem-${conversationId}-project-context`,
          title: 'Project context',
          summary: `Remember the active project scope for ${projectId}.`,
          kind: 'project',
          scope: 'project',
          ownerRef: `project:${projectId}`,
          sourceRunId: `source-${runId}`,
          freshnessState: 'fresh',
          lastValidatedAt: 1,
        },
      ]
  const memorySelectionSummary = {
    totalCandidateCount: selectedMemory.length,
    selectedCount: selectedMemory.length,
    ignoredCount: 0,
    recallMode: 'default',
    selectedMemoryIds: selectedMemory.map(item => item.memoryId),
  }
  const freshnessSummary = {
    freshnessRequired: true,
    staleCount: 0,
    freshCount: selectedMemory.length,
  }
  const pendingMemoryProposal = {
    proposalId: `memory-proposal-${conversationId}`,
    sessionId,
    sourceRunId: runId,
    memoryId: `mem-${conversationId}-feedback`,
    title: 'User feedback preference',
    summary: 'Capture the latest feedback so future turns can reuse it.',
    kind: 'feedback',
    scope: isHomeSession ? 'user' : 'project',
    proposalState: 'pending',
    proposalReason: 'user-feedback',
  }
  const sessionPolicy = {
    selectedActorRef: resolvedSelectedActorRef,
    selectedConfiguredModelId,
    executionPermissionMode,
    configSnapshotId: 'cfgsnap-fixture',
    manifestRevision: 'manifest-fixture-v2',
    capabilityPolicy: {},
    memoryPolicy: {},
    delegationPolicy: {},
    approvalPreference: {},
  }
  return {
    summary: {
      id: sessionId,
      conversationId,
      projectId,
      title,
      sessionKind,
      status: 'draft',
      updatedAt: 1,
      lastMessagePreview: undefined,
      configSnapshotId: 'cfgsnap-fixture',
      effectiveConfigHash: 'cfg-hash-fixture',
      startedFromScopeSet: isHomeSession ? ['user', 'workspace'] : ['project'],
      selectedActorRef: resolvedSelectedActorRef,
      manifestRevision: 'manifest-fixture-v2',
      sessionPolicy,
      activeRunId: runId,
      subrunCount: 2,
      workflow: createWorkflowSummary(workflowRunId, `${runId}-subrun-2`, 'agent project delivery', 1),
      pendingMailbox: createMailboxSummary(mailboxRef, 1),
      backgroundRun: createBackgroundRunSummary(runId, workflowRunId, 1),
      capabilityPlanSummary: createCapabilityPlanSummary(),
      capabilityStateRef: 'capstate-fixture',
      memorySummary: {
        summary: isHomeSession
          ? 'No durable memories selected for this home conversation yet.'
          : '1 durable memory selected with fresh validation.',
        durableMemoryCount: selectedMemory.length,
        selectedMemoryIds: selectedMemory.map(item => item.memoryId),
      },
      memorySelectionSummary,
      pendingMemoryProposalCount: 1,
      memoryStateRef,
      pendingMediation: createPendingMediationSummary(),
      providerStateSummary: createProviderStateSummary(),
      lastExecutionOutcome: {
        outcome: 'success',
      },
    },
    run: {
      id: runId,
      sessionId,
      conversationId,
      status: 'draft',
      currentStep: 'runtime.run.idle',
      startedAt: 1,
      updatedAt: 1,
      actorRef: resolvedSelectedActorRef,
      runKind: 'primary',
      workflowRun: workflowRunId,
      workflowRunDetail: {
        workflowRunId,
        status: 'completed',
        currentStepId: `${runId}-subrun-2`,
        currentStepLabel: 'agent project delivery',
        totalSteps: 3,
        completedSteps: 3,
        backgroundCapable: true,
      },
      mailboxRef,
      handoffRef: `handoff-${runId}-1`,
      backgroundState: 'completed',
      workerDispatch: {
        totalSubruns: 2,
        activeSubruns: 0,
        completedSubruns: 2,
        failedSubruns: 0,
      },
      configuredModelId: 'anthropic-primary',
      configuredModelName: 'Claude Sonnet 4.5',
      modelId: 'claude-sonnet-4-5',
      nextAction: 'runtime.run.awaitingInput',
      selectedMemory,
      freshnessSummary,
      pendingMemoryProposal,
      memoryStateRef,
      configSnapshotId: 'cfgsnap-fixture',
      effectiveConfigHash: 'cfg-hash-fixture',
      startedFromScopeSet: ['project'],
      capabilityPlanSummary: createCapabilityPlanSummary(),
      capabilityStateRef: 'capstate-fixture',
      approvalState: 'none',
      pendingMediation: createPendingMediationSummary(),
      providerStateSummary: createProviderStateSummary(),
      lastExecutionOutcome: {
        outcome: 'success',
      },
      usageSummary: {
        inputTokens: 0,
        outputTokens: 0,
        totalTokens: 0,
      },
      artifactRefs: [],
      deliverableRefs: [],
      traceContext: {
        sessionId,
        traceId: `trace-${conversationId}`,
        turnId: `turn-${conversationId}`,
      },
      checkpoint: {
        serializedSession: {
          sessionId,
          runId,
        },
        capabilityPlanSummary: createCapabilityPlanSummary(),
        capabilityStateRef: 'capstate-fixture',
        pendingMediation: createPendingMediationSummary(),
        currentIterationIndex: 0,
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
    subruns: [
      {
        runId: `${runId}-subrun-1`,
        parentRunId: runId,
        actorRef: 'agent:agent-architect',
        label: 'agent architect',
        status: 'completed',
        runKind: 'subrun',
        delegatedByToolCallId: 'team-dispatch-1',
        workflowRunId,
        mailboxRef,
        handoffRef: `handoff-${runId}-1`,
        startedAt: 1,
        updatedAt: 1,
      },
      {
        runId: `${runId}-subrun-2`,
        parentRunId: runId,
        actorRef: 'agent:agent-project-delivery',
        label: 'agent project delivery',
        status: 'completed',
        runKind: 'subrun',
        delegatedByToolCallId: 'team-dispatch-2',
        workflowRunId,
        mailboxRef,
        handoffRef: `handoff-${runId}-2`,
        startedAt: 1,
        updatedAt: 1,
      },
    ],
    handoffs: [
      {
        handoffRef: `handoff-${runId}-1`,
        mailboxRef,
        senderActorRef: resolvedSelectedActorRef,
        receiverActorRef: 'agent:agent-architect',
        state: 'delivered',
        artifactRefs: [`artifact-${runId}-subrun-1`],
        updatedAt: 1,
      },
      {
        handoffRef: `handoff-${runId}-2`,
        mailboxRef,
        senderActorRef: resolvedSelectedActorRef,
        receiverActorRef: 'agent:agent-project-delivery',
        state: 'delivered',
        artifactRefs: [`artifact-${runId}-subrun-2`],
        updatedAt: 1,
      },
    ],
    messages: [],
    trace: [],
    pendingApproval: undefined,
    activeRunId: runId,
    subrunCount: 2,
    workflow: createWorkflowSummary(workflowRunId, `${runId}-subrun-2`, 'agent project delivery', 1),
    pendingMailbox: createMailboxSummary(mailboxRef, 1),
    backgroundRun: createBackgroundRunSummary(runId, workflowRunId, 1),
    selectedActorRef: resolvedSelectedActorRef,
    manifestRevision: 'manifest-fixture-v2',
    sessionPolicy,
    capabilityPlanSummary: createCapabilityPlanSummary(),
    capabilityStateRef: 'capstate-fixture',
    memorySummary: {
      summary: '1 durable memory selected with fresh validation.',
      durableMemoryCount: selectedMemory.length,
      selectedMemoryIds: selectedMemory.map(item => item.memoryId),
    },
    memorySelectionSummary,
    pendingMemoryProposalCount: 1,
    memoryStateRef,
    pendingMediation: createPendingMediationSummary(),
    providerStateSummary: createProviderStateSummary(),
    lastExecutionOutcome: {
      outcome: 'success',
    },
  }
}

export function applyJsonMergePatch(
  target: Record<string, any>,
  patch: Record<string, any>,
): Record<string, any> {
  const next = structuredClone(target)
  for (const [key, value] of Object.entries(patch)) {
    if (value === null) {
      delete next[key]
      continue
    }
    if (Array.isArray(value)) {
      next[key] = structuredClone(value)
      continue
    }
    if (typeof value === 'object') {
      const current = typeof next[key] === 'object' && next[key] && !Array.isArray(next[key])
        ? next[key]
        : {}
      next[key] = applyJsonMergePatch(current, value as Record<string, any>)
      continue
    }
    next[key] = value
  }
  return next
}

export function createRuntimeMessage(
  state: RuntimeSessionState,
  senderType: RuntimeMessage['senderType'],
  senderLabel: string,
  content: string,
  modelId = 'claude-sonnet-4-5',
  configuredModelId = modelId,
  configuredModelName = modelId === 'claude-sonnet-4-5' ? 'Claude Sonnet 4.5' : 'GPT-4o',
  actorKind: RuntimeMessage['resolvedActorKind'] = 'agent',
  actorId = 'agent-architect',
  options: {
    artifacts?: RuntimeMessage['artifacts']
    deliverableRefs?: RuntimeMessage['deliverableRefs']
    resourceIds?: RuntimeMessage['resourceIds']
  } = {},
): RuntimeMessage {
  const timestamp = state.nextSequence * 10
  return {
    id: `msg-${state.detail.summary.id}-${state.nextSequence}`,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    senderType,
    senderLabel,
    content,
    timestamp,
    configuredModelId,
    configuredModelName,
    modelId,
    status: state.detail.run.status,
    requestedActorKind: actorKind,
    requestedActorId: actorId,
    resolvedActorKind: actorKind,
    resolvedActorId: actorId,
    resolvedActorLabel: senderType === 'assistant' ? senderLabel : 'You',
    usedDefaultActor: false,
    resourceIds: options.resourceIds ?? (senderType === 'assistant' ? [`${state.detail.summary.projectId}-res-2`] : []),
    attachments: [],
    artifacts: options.artifacts ?? [],
    deliverableRefs: options.deliverableRefs ?? [],
    usage: senderType === 'assistant'
      ? {
          inputTokens: 320,
          outputTokens: 180,
          totalTokens: 500,
        }
      : undefined,
    processEntries: senderType === 'assistant'
      ? [
          {
            id: `process-${state.detail.summary.id}-${state.nextSequence}`,
            type: 'execution',
            title: 'Runtime execution',
            detail: `Resolved ${actorKind}:${actorId} and produced a conversation response.`,
            timestamp,
          },
        ]
      : [],
    toolCalls: senderType === 'assistant'
      ? [
          {
            toolId: 'workspace-api',
            label: 'Workspace API',
            kind: 'builtin',
            count: 1,
          },
        ]
      : [],
  }
}

export function createTraceItem(
  state: RuntimeSessionState,
  detail: string,
  tone: RuntimeTraceItem['tone'] = 'info',
  actorKind: RuntimeTraceItem['actorKind'] = 'agent',
  actorId = 'agent-architect',
  actor = 'Octopus Runtime',
): RuntimeTraceItem {
  const timestamp = state.nextSequence * 10
  return {
    id: `trace-${state.detail.summary.id}-${state.nextSequence}`,
    sessionId: state.detail.summary.id,
    runId: state.detail.run.id,
    conversationId: state.detail.summary.conversationId,
    kind: 'reasoning',
    title: 'Runtime Trace',
    detail,
    tone,
    timestamp,
    actor,
    actorKind,
    actorId,
  }
}

export function createApproval(state: RuntimeSessionState): RuntimeApprovalRequest {
  return {
    id: `approval-${state.detail.summary.id}`,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    runId: state.detail.run.id,
    toolName: 'bash',
    summary: 'Approve workspace command execution',
    detail: 'Run pwd in the workspace terminal.',
    riskLevel: 'medium',
    createdAt: state.nextSequence * 10,
    status: 'pending',
    approvalLayer: 'execution',
    targetKind: 'tool',
    targetRef: 'bash',
    escalationReason: 'workspace-write execution requires approval',
  }
}

export function createEvent(
  state: RuntimeSessionState,
  workspaceId: string,
  eventType: RuntimeEventEnvelope['eventType'],
  patch: Partial<RuntimeEventEnvelope> = {},
): RuntimeEventEnvelope {
  const sequence = state.nextSequence++
  return {
    id: `event-${state.detail.summary.id}-${sequence}`,
    eventType,
    kind: eventType,
    workspaceId,
    projectId: state.detail.summary.projectId,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    runId: state.detail.run.id,
    emittedAt: sequence * 10,
    sequence,
    ...patch,
  }
}

export function eventsAfter(state: RuntimeSessionState, after?: string): RuntimeEventEnvelope[] {
  if (!after) {
    return state.events
  }

  const index = state.events.findIndex(event => event.id === after)
  return index === -1 ? state.events : state.events.slice(index + 1)
}
