import type { WorkspaceSessionTokenEnvelope } from '@octopus/schema'

const WORKSPACE_SESSION_STORAGE_KEY = 'octopus-workspace-sessions'

export type HostRuntimeMode = 'tauri' | 'browser'

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
