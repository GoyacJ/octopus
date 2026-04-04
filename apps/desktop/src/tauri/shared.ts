import type { HostBackendConnection, ShellPreferences } from '@octopus/schema'
import {
  createDefaultShellPreferences,
  normalizeShellPreferences,
} from '@octopus/schema'

const PREFERENCES_STORAGE_KEY = 'octopus-shell-preferences'

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export function loadStoredPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  if (typeof window === 'undefined' || !window.localStorage) {
    return createDefaultShellPreferences(defaultWorkspaceId, defaultProjectId)
  }

  const raw = window.localStorage.getItem(PREFERENCES_STORAGE_KEY)
  if (!raw) {
    return createDefaultShellPreferences(defaultWorkspaceId, defaultProjectId)
  }

  try {
    return normalizeShellPreferences(JSON.parse(raw) as Partial<ShellPreferences>, defaultWorkspaceId, defaultProjectId)
  } catch {
    return createDefaultShellPreferences(defaultWorkspaceId, defaultProjectId)
  }
}

export function saveStoredPreferences(preferences: ShellPreferences): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return
  }

  window.localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences))
}

export async function fetchBackend<T>(
  backend: HostBackendConnection | undefined,
  path: string,
  init?: RequestInit,
): Promise<T> {
  if (backend?.state !== 'ready' || !backend.baseUrl) {
    throw new Error('Desktop backend is unavailable')
  }

  const headers = new Headers(init?.headers)
  if (backend.authToken) {
    headers.set('Authorization', `Bearer ${backend.authToken}`)
  }
  if (!headers.has('Content-Type') && init?.body) {
    headers.set('Content-Type', 'application/json')
  }

  const response = await fetch(`${backend.baseUrl}${path}`, {
    ...init,
    headers,
  })

  if (!response.ok) {
    throw new Error(`Desktop backend request failed: ${response.status}`)
  }

  return await response.json() as T
}
