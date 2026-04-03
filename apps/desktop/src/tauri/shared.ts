import type {
  HostBackendConnection,
  HostState,
  ShellPreferences,
} from '@octopus/schema'

const PREFERENCES_STORAGE_KEY = 'octopus-shell-preferences'
const DEFAULT_BACKEND_BASE_URL = 'http://127.0.0.1:43127'

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export function createDefaultPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  return {
    theme: 'system',
    locale: 'zh-CN',
    compactSidebar: false,
    leftSidebarCollapsed: false,
    rightSidebarCollapsed: false,
    defaultWorkspaceId,
    lastVisitedRoute: `/workspaces/${defaultWorkspaceId}/overview?project=${defaultProjectId}`,
  }
}

export function extractProjectIdFromRoute(lastVisitedRoute: string): string {
  if (lastVisitedRoute.includes('?project=')) {
    return lastVisitedRoute.split('?project=')[1]?.split('&')[0] ?? 'proj-redesign'
  }

  const projectMatch = lastVisitedRoute.match(/\/projects\/([^/]+)/)
  return projectMatch?.[1] ?? 'proj-redesign'
}

export function normalizePreferences(
  value: Partial<ShellPreferences>,
  defaultWorkspaceId: string,
  defaultProjectId: string,
): ShellPreferences {
  const defaults = createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  const leftSidebarCollapsed = typeof value.leftSidebarCollapsed === 'boolean'
    ? value.leftSidebarCollapsed
    : Boolean(value.compactSidebar)

  return {
    ...defaults,
    ...value,
    compactSidebar: typeof value.compactSidebar === 'boolean' ? value.compactSidebar : leftSidebarCollapsed,
    leftSidebarCollapsed,
    rightSidebarCollapsed: typeof value.rightSidebarCollapsed === 'boolean' ? value.rightSidebarCollapsed : defaults.rightSidebarCollapsed,
  }
}

export function fallbackHostState(): HostState {
  return {
    platform: 'web',
    mode: 'local',
    appVersion: '0.1.0',
    cargoWorkspace: false,
    shell: 'browser',
  }
}

export function fallbackBackendConnection(
  state: HostBackendConnection['state'] = 'ready',
  transport: HostBackendConnection['transport'] = 'mock',
): HostBackendConnection {
  const isReady = state === 'ready'

  return {
    baseUrl: isReady ? DEFAULT_BACKEND_BASE_URL : undefined,
    authToken: isReady ? 'desktop-mock-token' : undefined,
    state,
    transport,
  }
}

export function loadStoredPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  if (typeof window === 'undefined' || !window.localStorage) {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  }

  const raw = window.localStorage.getItem(PREFERENCES_STORAGE_KEY)
  if (!raw) {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
  }

  try {
    return normalizePreferences(JSON.parse(raw) as Partial<ShellPreferences>, defaultWorkspaceId, defaultProjectId)
  } catch {
    return createDefaultPreferences(defaultWorkspaceId, defaultProjectId)
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
