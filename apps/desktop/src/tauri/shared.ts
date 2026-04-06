import type {
  ApiErrorCode,
  ApiErrorEnvelope,
  HostBackendConnection,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

const WORKSPACE_SESSION_STORAGE_KEY = 'octopus-workspace-sessions'
export const WORKSPACE_AUTH_FAILURE_EVENT = 'octopus:workspace-auth-failure'

export type HostRuntimeMode = 'tauri' | 'browser'
export type WorkspaceAuthFailureCode = Extract<ApiErrorCode, 'UNAUTHENTICATED' | 'SESSION_EXPIRED'>

export interface WorkspaceAuthFailureDetail {
  workspaceConnectionId: string
  code: WorkspaceAuthFailureCode
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

export function isWorkspaceApiError(error: unknown): error is WorkspaceApiError {
  return error instanceof WorkspaceApiError
}

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export function resolveHostRuntime(): HostRuntimeMode {
  const runtime = import.meta.env.VITE_HOST_RUNTIME
  if (runtime === 'tauri' || runtime === 'browser') {
    return runtime
  }

  throw new Error('VITE_HOST_RUNTIME must be set to "tauri" or "browser"')
}

export function resolveBrowserHostApiBaseUrl(): string {
  const baseUrl = import.meta.env.VITE_HOST_API_BASE_URL
  if (!baseUrl) {
    throw new Error('VITE_HOST_API_BASE_URL is required when VITE_HOST_RUNTIME=browser')
  }

  return baseUrl
}

export function resolveBrowserHostAuthToken(): string {
  const authToken = import.meta.env.VITE_HOST_AUTH_TOKEN
  if (!authToken) {
    throw new Error('VITE_HOST_AUTH_TOKEN is required when VITE_HOST_RUNTIME=browser')
  }

  return authToken
}

export function loadStoredWorkspaceSessions(): Record<string, WorkspaceSessionTokenEnvelope> {
  if (typeof window === 'undefined' || !window.localStorage) {
    return {}
  }

  const raw = window.localStorage.getItem(WORKSPACE_SESSION_STORAGE_KEY)
  if (!raw) {
    return {}
  }

  try {
    return JSON.parse(raw) as Record<string, WorkspaceSessionTokenEnvelope>
  } catch {
    return {}
  }
}

function saveStoredWorkspaceSessions(sessions: Record<string, WorkspaceSessionTokenEnvelope>): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return
  }

  window.localStorage.setItem(WORKSPACE_SESSION_STORAGE_KEY, JSON.stringify(sessions))
}

export function saveStoredWorkspaceSession(session: WorkspaceSessionTokenEnvelope): void {
  const sessions = loadStoredWorkspaceSessions()
  sessions[session.workspaceConnectionId] = session
  saveStoredWorkspaceSessions(sessions)
}

export function clearStoredWorkspaceSession(workspaceConnectionId: string): void {
  const sessions = loadStoredWorkspaceSessions()
  delete sessions[workspaceConnectionId]
  saveStoredWorkspaceSessions(sessions)
}

export function createRequestId(): string {
  return `req-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`
}

export function joinBaseUrl(baseUrl: string, path: string): string {
  const normalizedBaseUrl = baseUrl.replace(/\/+$/, '')
  const normalizedPath = path.startsWith('/') ? path : `/${path}`
  return `${normalizedBaseUrl}${normalizedPath}`
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
  }
  return error
}

async function fetchJson<T>(
  input: string,
  init: RequestInit,
  requestId: string,
  workspaceConnectionId?: string,
): Promise<T> {
  const response = await fetch(input, init)

  if (!response.ok) {
    throw await decodeApiError(response, requestId, workspaceConnectionId)
  }

  return await response.json() as T
}

export function createHostHeaders(authToken: string, init?: RequestInit): Headers {
  const headers = new Headers(init?.headers)
  headers.set('Authorization', `Bearer ${authToken}`)
  headers.set('Accept', 'application/json')
  headers.set('X-Request-Id', createRequestId())
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }

  return headers
}

export async function fetchHostApi<T>(
  baseUrl: string,
  authToken: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  const headers = createHostHeaders(authToken, init)
  const requestId = headers.get('X-Request-Id') ?? createRequestId()

  return await fetchJson<T>(joinBaseUrl(baseUrl, path), {
    ...init,
    headers,
  }, requestId)
}

export async function fetchBackend<T>(
  backend: HostBackendConnection | undefined,
  path: string,
  init?: RequestInit,
): Promise<T> {
  if (backend?.state !== 'ready' || !backend.baseUrl) {
    throw new Error('Desktop backend is unavailable')
  }

  const requestId = createRequestId()
  const headers = new Headers(init?.headers)
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }
  headers.set('X-Request-Id', requestId)

  return await fetchJson<T>(joinBaseUrl(backend.baseUrl, path), {
    ...init,
    headers,
  }, requestId)
}

export interface WorkspaceRequestOptions extends Omit<RequestInit, 'headers'> {
  headers?: HeadersInit
  idempotencyKey?: string
  session?: WorkspaceSessionTokenEnvelope
  workspace?: WorkspaceConnectionRecord
}

export function createWorkspaceHeaders(options: WorkspaceRequestOptions = {}): Headers {
  const headers = new Headers(options.headers)

  if (options.session?.token) {
    headers.set('Authorization', `Bearer ${options.session.token}`)
  }
  if (options.workspace?.workspaceId) {
    headers.set('X-Workspace-Id', options.workspace.workspaceId)
  }
  if (!headers.has('Content-Type') && options.body) {
    headers.set('Content-Type', 'application/json')
  }
  if (!headers.has('Accept')) {
    headers.set('Accept', 'application/json')
  }

  headers.set('X-Request-Id', createRequestId())

  if (options.idempotencyKey) {
    headers.set('Idempotency-Key', options.idempotencyKey)
  }

  return headers
}

export async function fetchWorkspaceApi<T>(
  workspace: WorkspaceConnectionRecord,
  path: string,
  options: WorkspaceRequestOptions = {},
): Promise<T> {
  const headers = createWorkspaceHeaders({
    ...options,
    workspace,
  })
  const requestId = headers.get('X-Request-Id') ?? createRequestId()

  return await fetchJson<T>(joinBaseUrl(workspace.baseUrl, path), {
    ...options,
    headers,
  }, requestId, workspace.workspaceConnectionId)
}
