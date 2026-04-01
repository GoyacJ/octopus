import { invoke } from '@tauri-apps/api/core'

import type { ConnectionProfile, HostState, ShellBootstrap, ShellPreferences } from '@octopus/schema'

const PREFERENCES_STORAGE_KEY = 'octopus-shell-preferences'

export interface HealthcheckStatus {
  status: 'ok'
  host: 'web' | 'tauri'
  mode: 'local'
  cargoWorkspace: boolean
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function createDefaultPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
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

function extractProjectIdFromRoute(lastVisitedRoute: string): string {
  if (lastVisitedRoute.includes('?project=')) {
    return lastVisitedRoute.split('?project=')[1]?.split('&')[0] ?? 'proj-redesign'
  }

  const projectMatch = lastVisitedRoute.match(/\/projects\/([^/]+)/)
  return projectMatch?.[1] ?? 'proj-redesign'
}

function normalizePreferences(
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

function fallbackHostState(): HostState {
  return {
    platform: 'web',
    mode: 'local',
    appVersion: '0.1.0',
    cargoWorkspace: false,
    shell: 'browser',
  }
}

function loadStoredPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
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

function saveStoredPreferences(preferences: ShellPreferences): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    return
  }

  window.localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences))
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
  mockConnections: ConnectionProfile[],
): Promise<ShellBootstrap> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  if (!isTauriRuntime()) {
    return {
      hostState: fallbackHostState(),
      preferences: fallbackPreferences,
      connections: mockConnections,
    }
  }

  try {
    const bootstrap = await invoke<ShellBootstrap>('bootstrap_shell')
    const preferences = bootstrap.preferences
      ? normalizePreferences(bootstrap.preferences, defaultWorkspaceId, defaultProjectId)
      : fallbackPreferences
    saveStoredPreferences(preferences)

    return {
      hostState: bootstrap.hostState ?? fallbackHostState(),
      preferences,
      connections: mockConnections,
    }
  } catch {
    return {
      hostState: fallbackHostState(),
      preferences: fallbackPreferences,
      connections: mockConnections,
    }
  }
}

export async function loadPreferences(defaultWorkspaceId: string, defaultProjectId: string): Promise<ShellPreferences> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  if (!isTauriRuntime()) {
    return fallbackPreferences
  }

  try {
    const preferences = normalizePreferences(await invoke<ShellPreferences>('load_preferences'), defaultWorkspaceId, defaultProjectId)
    saveStoredPreferences(preferences)
    return preferences
  } catch {
    return fallbackPreferences
  }
}

export async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  const normalizedPreferences = normalizePreferences(
    {
      ...preferences,
      compactSidebar: preferences.leftSidebarCollapsed,
    },
    preferences.defaultWorkspaceId,
    extractProjectIdFromRoute(preferences.lastVisitedRoute),
  )
  saveStoredPreferences(normalizedPreferences)
  if (!isTauriRuntime()) {
    return normalizedPreferences
  }

  try {
    const savedPreferences = normalizePreferences(
      await invoke<ShellPreferences>('save_preferences', { preferences: normalizedPreferences }),
      normalizedPreferences.defaultWorkspaceId,
      extractProjectIdFromRoute(normalizedPreferences.lastVisitedRoute),
    )
    saveStoredPreferences(savedPreferences)
    return savedPreferences
  } catch {
    return normalizedPreferences
  }
}

export async function getHostState(): Promise<HostState> {
  if (!isTauriRuntime()) {
    return fallbackHostState()
  }

  try {
    return await invoke<HostState>('get_host_state')
  } catch {
    return fallbackHostState()
  }
}

export async function listConnectionsStub(): Promise<ConnectionProfile[]> {
  if (!isTauriRuntime()) {
    return []
  }

  try {
    return await invoke<ConnectionProfile[]>('list_connections_stub')
  } catch {
    return []
  }
}

export async function healthcheck(): Promise<HealthcheckStatus> {
  if (!isTauriRuntime()) {
    return {
      status: 'ok',
      host: 'web',
      mode: 'local',
      cargoWorkspace: false,
    }
  }

  try {
    return await invoke<HealthcheckStatus>('healthcheck')
  } catch {
    return {
      status: 'ok',
      host: 'tauri',
      mode: 'local',
      cargoWorkspace: false,
    }
  }
}
