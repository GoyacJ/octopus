import type { DecisionAction, PermissionMode, RiskLevel, RunStatus, TraceKind, TraceTone } from './shared'

export interface ProviderConfig {
  provider: string
  apiKey?: string
  baseUrl?: string
  defaultModel?: string
}

export type RuntimeActorType = 'user' | 'assistant' | 'system'
export type RuntimeEventKind =
  | 'run_updated'
  | 'message_created'
  | 'trace_emitted'
  | 'approval_requested'
  | 'approval_resolved'
  | 'session_updated'
  | 'error'

export interface RuntimeSessionSummary {
  id: string
  conversationId: string
  projectId: string
  title: string
  status: RunStatus
  updatedAt: number
  lastMessagePreview?: string
}

export interface RuntimeRunSnapshot {
  id: string
  sessionId: string
  conversationId: string
  status: RunStatus
  currentStep: string
  startedAt: number
  updatedAt: number
  modelId?: string
  nextAction?: string
}

export interface RuntimeMessage {
  id: string
  sessionId: string
  conversationId: string
  senderType: RuntimeActorType
  senderLabel: string
  content: string
  timestamp: number
  modelId?: string
  status: RunStatus
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
}

export type RuntimeDecisionAction = DecisionAction

export interface RuntimeEventEnvelope {
  id: string
  kind: RuntimeEventKind
  sessionId: string
  conversationId: string
  runId?: string
  emittedAt: number
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
}

export interface SubmitRuntimeTurnInput {
  content: string
  modelId: string
  permissionMode: PermissionMode
}

export interface ResolveRuntimeApprovalInput {
  decision: RuntimeDecisionAction
}
