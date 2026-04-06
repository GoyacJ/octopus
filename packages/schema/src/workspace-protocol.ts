import type { SessionRecord } from './auth'
import type { SystemBootstrapStatus } from './workspace'

export type TransportSecurityLevel = 'loopback' | 'trusted' | 'public'
export type WorkspaceAuthMode = 'session-token'
export type WorkspaceConnectionStatus = 'connected' | 'disconnected' | 'expired' | 'unreachable'

export type ApiErrorCode =
  | 'UNAUTHENTICATED'
  | 'SESSION_EXPIRED'
  | 'FORBIDDEN'
  | 'NOT_FOUND'
  | 'CONFLICT'
  | 'INVALID_INPUT'
  | 'RATE_LIMITED'
  | 'UNAVAILABLE'
  | 'CAPABILITY_UNSUPPORTED'
  | 'INTERNAL_ERROR'

export interface WorkspaceCapabilitySet {
  polling: boolean
  sse: boolean
  idempotency: boolean
  reconnect: boolean
  eventReplay: boolean
}

export interface WorkspaceConnectionRecord {
  workspaceConnectionId: string
  workspaceId: string
  label: string
  baseUrl: string
  transportSecurity: TransportSecurityLevel
  authMode: WorkspaceAuthMode
  lastUsedAt?: number
  status: WorkspaceConnectionStatus
}

export interface WorkspaceSessionTokenEnvelope {
  workspaceConnectionId: string
  token: string
  session: SessionRecord
  issuedAt: number
  expiresAt?: number
}

export interface ApiErrorDetailEnvelope {
  code: ApiErrorCode
  message: string
  details?: unknown
  requestId: string
  retryable: boolean
}

export interface ApiErrorEnvelope {
  error: ApiErrorDetailEnvelope
}

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
