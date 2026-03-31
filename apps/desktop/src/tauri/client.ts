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
    defaultWorkspaceId,
    lastVisitedRoute: `/workspaces/${defaultWorkspaceId}/dashboard?project=${defaultProjectId}`,
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
    return {
      ...createDefaultPreferences(defaultWorkspaceId, defaultProjectId),
      ...(JSON.parse(raw) as Partial<ShellPreferences>),
    }
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
    const preferences = bootstrap.preferences ?? fallbackPreferences
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
    const preferences = await invoke<ShellPreferences>('load_preferences')
    saveStoredPreferences(preferences)
    return preferences
  } catch {
    return fallbackPreferences
  }
}

export async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  saveStoredPreferences(preferences)
  if (!isTauriRuntime()) {
    return preferences
  }

  try {
    const savedPreferences = await invoke<ShellPreferences>('save_preferences', { preferences })
    saveStoredPreferences(savedPreferences)
    return savedPreferences
  } catch {
    return preferences
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
