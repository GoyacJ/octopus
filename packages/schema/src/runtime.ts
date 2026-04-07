import type {
  ConversationActorKind,
  DecisionAction,
  PermissionMode,
  RiskLevel,
  RunStatus,
  TraceKind,
  TraceTone,
  WorkspaceToolPermissionMode,
} from './shared'
import type { MessageProcessEntry, MessageToolCall, MessageUsage } from './workbench'
import type { RuntimePermissionMode } from './permissions'
import type { SseEventEnvelope } from './workspace-protocol'
import type { CapabilityDescriptor } from './catalog'

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JsonValue }
  | JsonValue[]

export interface ProviderConfig {
  providerId: string
  credentialRef?: string
  baseUrl?: string
  defaultModel?: string
  defaultSurface?: string
  protocolFamily?: string
}

export type RuntimeConfigScope = 'workspace' | 'project' | 'user'

export interface RuntimeConfigSource {
  scope: RuntimeConfigScope
  ownerId?: string
  displayPath: string
  sourceKey: string
  exists: boolean
  loaded: boolean
  contentHash?: string
  document?: Record<string, JsonValue>
}

export interface RuntimeSecretReferenceStatus {
  scope: RuntimeConfigScope
  path: string
  reference?: string
  status: 'reference-present' | 'reference-missing' | 'inline-redacted'
}

export interface RuntimeConfigPatch {
  scope: RuntimeConfigScope
  patch: Record<string, JsonValue>
}

export interface ProjectModelSettings {
  allowedConfiguredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectToolPermissionOverride {
  permissionMode: WorkspaceToolPermissionMode
}

export interface ProjectToolSettings {
  enabledSourceKeys: string[]
  overrides: Record<string, ProjectToolPermissionOverride>
}

export interface ProjectAgentSettings {
  enabledAgentIds: string[]
  enabledTeamIds: string[]
}

export interface ProjectSettingsConfig {
  models?: ProjectModelSettings
  tools?: ProjectToolSettings
  agents?: ProjectAgentSettings
}

export interface RuntimeConfigValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

export interface RuntimeConfiguredModelProbeInput {
  scope: RuntimeConfigScope
  configuredModelId: string
  patch: Record<string, JsonValue>
}

export interface RuntimeConfiguredModelProbeResult {
  valid: boolean
  reachable: boolean
  configuredModelId: string
  configuredModelName?: string
  requestId?: string
  consumedTokens?: number
  errors: string[]
  warnings: string[]
}

export interface RuntimeEffectiveConfig {
  effectiveConfig: Record<string, JsonValue>
  effectiveConfigHash: string
  sources: RuntimeConfigSource[]
  validation: RuntimeConfigValidationResult
  secretReferences: RuntimeSecretReferenceStatus[]
}

export interface RuntimeConfigSnapshotSummary {
  id: string
  effectiveConfigHash: string
  startedFromScopeSet: RuntimeConfigScope[]
  sourceRefs: string[]
  createdAt: number
  effectiveConfig?: Record<string, JsonValue>
}

export interface ResolvedExecutionTarget {
  configuredModelId: string
  configuredModelName: string
  providerId: string
  registryModelId: string
  modelId: string
  surface: string
  protocolFamily: string
  credentialRef?: string
  baseUrl?: string
  capabilities: CapabilityDescriptor[]
}

export type RuntimeActorType = 'user' | 'assistant' | 'system'
export type RuntimeEventKind =
  | 'runtime.run.updated'
  | 'runtime.message.created'
  | 'runtime.trace.emitted'
  | 'runtime.approval.requested'
  | 'runtime.approval.resolved'
  | 'runtime.session.updated'
  | 'runtime.error'

export type RuntimeSessionKind = 'project' | 'pet'

export interface RuntimeSessionSummary {
  id: string
  conversationId: string
  projectId: string
  title: string
  sessionKind: RuntimeSessionKind
  status: RunStatus
  updatedAt: number
  lastMessagePreview?: string
  configSnapshotId: string
  effectiveConfigHash: string
  startedFromScopeSet: RuntimeConfigScope[]
}

export interface RuntimeRunSnapshot {
  id: string
  sessionId: string
  conversationId: string
  status: RunStatus
  currentStep: string
  startedAt: number
  updatedAt: number
  configuredModelId?: string
  configuredModelName?: string
  modelId?: string
  consumedTokens?: number
  nextAction?: string
  configSnapshotId: string
  effectiveConfigHash: string
  startedFromScopeSet: RuntimeConfigScope[]
  resolvedTarget?: ResolvedExecutionTarget
  requestedActorKind?: ConversationActorKind
  requestedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorLabel?: string
}

export interface RuntimeMessage {
  id: string
  sessionId: string
  conversationId: string
  senderType: RuntimeActorType
  senderLabel: string
  content: string
  timestamp: number
  configuredModelId?: string
  configuredModelName?: string
  modelId?: string
  status: RunStatus
  requestedActorKind?: ConversationActorKind
  requestedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorLabel?: string
  usedDefaultActor?: boolean
  resourceIds?: string[]
  attachments?: string[]
  artifacts?: string[]
  usage?: MessageUsage
  toolCalls?: MessageToolCall[]
  processEntries?: MessageProcessEntry[]
}

export interface RuntimeTraceItem {
  id: string
  sessionId: string
  runId: string
  conversationId: string
  kind: TraceKind
  title: string
  detail: string
  tone: TraceTone | 'default'
  timestamp: number
  actor: string
  actorKind?: ConversationActorKind
  actorId?: string
  relatedMessageId?: string
  relatedToolName?: string
}

export interface RuntimeApprovalRequest {
  id: string
  sessionId: string
  conversationId: string
  runId: string
  toolName: string
  summary: string
  detail: string
  riskLevel: RiskLevel
  createdAt: number
  status?: 'pending' | 'approved' | 'rejected'
}

export type RuntimeDecisionAction = DecisionAction

export interface RuntimeEventEnvelope extends SseEventEnvelope {
  eventType: RuntimeEventKind
  kind?: RuntimeEventKind
  run?: RuntimeRunSnapshot
  message?: RuntimeMessage
  trace?: RuntimeTraceItem
  approval?: RuntimeApprovalRequest
  decision?: RuntimeDecisionAction
  summary?: RuntimeSessionSummary
  error?: string
}

export interface RuntimeSessionDetail {
  summary: RuntimeSessionSummary
  run: RuntimeRunSnapshot
  messages: RuntimeMessage[]
  trace: RuntimeTraceItem[]
  pendingApproval?: RuntimeApprovalRequest
}

export interface RuntimeBootstrap {
  provider: ProviderConfig
  sessions: RuntimeSessionSummary[]
}

export interface CreateRuntimeSessionInput {
  conversationId: string
  projectId: string
  title: string
  sessionKind?: RuntimeSessionKind
}

export interface SubmitRuntimeTurnInput {
  content: string
  modelId?: string
  configuredModelId?: string
  permissionMode: PermissionMode | RuntimePermissionMode
  actorKind?: ConversationActorKind
  actorId?: string
}

export interface ResolveRuntimeApprovalInput {
  decision: RuntimeDecisionAction
}
