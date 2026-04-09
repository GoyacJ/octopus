import type {
  ApiErrorCode as OpenApiApiErrorCode,
  ApiErrorDetailEnvelope as OpenApiApiErrorDetailEnvelope,
  ApiErrorEnvelope as OpenApiApiErrorEnvelope,
  CreateHostWorkspaceConnectionInput as OpenApiCreateHostWorkspaceConnectionInput,
  TransportSecurityLevel as OpenApiTransportSecurityLevel,
  WorkspaceAuthMode as OpenApiWorkspaceAuthMode,
  WorkspaceCapabilitySet as OpenApiWorkspaceCapabilitySet,
  WorkspaceConnectionRecord as OpenApiWorkspaceConnectionRecord,
  WorkspaceConnectionStatus as OpenApiWorkspaceConnectionStatus,
  WorkspaceSessionTokenEnvelope as OpenApiWorkspaceSessionTokenEnvelope,
} from './generated'
import type { SystemBootstrapStatus } from './workspace'

export type TransportSecurityLevel = OpenApiTransportSecurityLevel
export type WorkspaceAuthMode = OpenApiWorkspaceAuthMode
export type WorkspaceConnectionStatus = OpenApiWorkspaceConnectionStatus
export type ApiErrorCode = OpenApiApiErrorCode
export type WorkspaceCapabilitySet = OpenApiWorkspaceCapabilitySet
export type WorkspaceConnectionRecord = OpenApiWorkspaceConnectionRecord

export interface HostWorkspaceConnectionRecord extends WorkspaceConnectionRecord {}
export type CreateHostWorkspaceConnectionInput = OpenApiCreateHostWorkspaceConnectionInput

export type WorkspaceSessionTokenEnvelope = OpenApiWorkspaceSessionTokenEnvelope
export type ApiErrorDetailEnvelope = OpenApiApiErrorDetailEnvelope
export type ApiErrorEnvelope = OpenApiApiErrorEnvelope

export interface ApiPagination {
  cursor?: string
  nextCursor?: string
  total?: number
}

export interface SseEventEnvelope<TPayload = unknown> {
  id: string
  eventType: string
  workspaceId: string
  projectId?: string
  sessionId: string
  conversationId: string
  runId?: string
  emittedAt: number
  sequence: number
  payload?: TPayload
}

export interface IdempotentCommandResult<TResult = unknown> {
  idempotencyKey: string
  replayed: boolean
  result: TResult
}

export type WorkspaceBootstrap = SystemBootstrapStatus
