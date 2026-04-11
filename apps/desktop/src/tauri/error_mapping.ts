import type { ApiErrorCode, ApiErrorEnvelope } from '@octopus/schema'

export const WORKSPACE_AUTH_FAILURE_EVENT = 'octopus:workspace-auth-failure'
export const WORKSPACE_AUTHORIZATION_FAILURE_EVENT = 'octopus:workspace-authorization-failure'

export type WorkspaceAuthFailureCode = Extract<ApiErrorCode, 'UNAUTHENTICATED' | 'SESSION_EXPIRED'>
export type WorkspaceAuthorizationFailureCode = Extract<ApiErrorCode, 'PERMISSION_DENIED' | 'AUTHORIZATION_STALE'>

export interface WorkspaceAuthFailureDetail {
  workspaceConnectionId: string
  code: WorkspaceAuthFailureCode
  status: number
  requestId: string
}

export interface WorkspaceAuthorizationFailureDetail {
  workspaceConnectionId: string
  code: WorkspaceAuthorizationFailureCode
  status: number
  requestId: string
}

export class WorkspaceApiError extends Error {
  readonly code?: ApiErrorCode
  readonly status: number
  readonly requestId: string
  readonly retryable: boolean
  readonly details?: unknown

  constructor(options: {
    message: string
    status: number
    requestId: string
    retryable?: boolean
    code?: ApiErrorCode
    details?: unknown
  }) {
    super(options.message)
    this.name = 'WorkspaceApiError'
    this.code = options.code
    this.status = options.status
    this.requestId = options.requestId
    this.retryable = options.retryable ?? false
    this.details = options.details
  }
}

function isWorkspaceAuthFailureCode(code?: ApiErrorCode): code is WorkspaceAuthFailureCode {
  return code === 'UNAUTHENTICATED' || code === 'SESSION_EXPIRED'
}

function isWorkspaceAuthorizationFailureCode(code?: ApiErrorCode): code is WorkspaceAuthorizationFailureCode {
  return code === 'PERMISSION_DENIED' || code === 'AUTHORIZATION_STALE'
}

function dispatchWorkspaceAuthFailure(
  workspaceConnectionId: string,
  error: WorkspaceApiError,
): void {
  if (typeof window === 'undefined' || !isWorkspaceAuthFailureCode(error.code)) {
    return
  }

  window.dispatchEvent(new CustomEvent<WorkspaceAuthFailureDetail>(WORKSPACE_AUTH_FAILURE_EVENT, {
    detail: {
      workspaceConnectionId,
      code: error.code,
      status: error.status,
      requestId: error.requestId,
    },
  }))
}

function dispatchWorkspaceAuthorizationFailure(
  workspaceConnectionId: string,
  error: WorkspaceApiError,
): void {
  if (typeof window === 'undefined' || !isWorkspaceAuthorizationFailureCode(error.code)) {
    return
  }

  window.dispatchEvent(new CustomEvent<WorkspaceAuthorizationFailureDetail>(WORKSPACE_AUTHORIZATION_FAILURE_EVENT, {
    detail: {
      workspaceConnectionId,
      code: error.code,
      status: error.status,
      requestId: error.requestId,
    },
  }))
}

export function isWorkspaceApiError(error: unknown): error is WorkspaceApiError {
  return error instanceof WorkspaceApiError
}

export async function decodeApiError(
  response: Response,
  fallbackRequestId: string,
  workspaceConnectionId?: string,
): Promise<WorkspaceApiError> {
  try {
    const payload = await response.json() as ApiErrorEnvelope
    if (payload?.error?.message) {
      const error = new WorkspaceApiError({
        message: payload.error.message,
        status: response.status,
        requestId: payload.error.requestId ?? fallbackRequestId,
        retryable: payload.error.retryable,
        code: payload.error.code,
        details: payload.error.details,
      })
      if (workspaceConnectionId) {
        dispatchWorkspaceAuthFailure(workspaceConnectionId, error)
        dispatchWorkspaceAuthorizationFailure(workspaceConnectionId, error)
      }
      return error
    }
  } catch {
    // Fall through to the status-based fallback below.
  }

  const error = new WorkspaceApiError({
    message: `Workspace request failed: ${response.status} (${fallbackRequestId})`,
    status: response.status,
    requestId: fallbackRequestId,
  })
  if (workspaceConnectionId) {
    dispatchWorkspaceAuthFailure(workspaceConnectionId, error)
    dispatchWorkspaceAuthorizationFailure(workspaceConnectionId, error)
  }
  return error
}
