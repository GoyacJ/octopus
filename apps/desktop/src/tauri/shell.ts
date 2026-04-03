import { invoke } from '@tauri-apps/api/core'

import type {
  ConnectionProfile,
  HealthcheckStatus,
  HostBackendConnection,
  HostState,
  ShellBootstrap,
  ShellPreferences,
} from '@octopus/schema'

import { resolveMockShellBootstrap, shouldUseMockRuntime } from './mock'
import {
  fallbackBackendConnection,
  fallbackHostState,
  extractProjectIdFromRoute,
  isTauriRuntime,
  loadStoredPreferences,
  normalizePreferences,
  saveStoredPreferences,
} from './shared'

async function resolveDesktopShellBootstrap(): Promise<ShellBootstrap | null> {
  if (!isTauriRuntime()) {
    return null
  }

  try {
    return await invoke<ShellBootstrap>('bootstrap_shell')
  } catch {
    return null
  }
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
  mockConnections: ConnectionProfile[],
): Promise<ShellBootstrap> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  const mockBootstrap = resolveMockShellBootstrap(defaultWorkspaceId, defaultProjectId, mockConnections)
  const desktopBootstrap = await resolveDesktopShellBootstrap()

  if (!desktopBootstrap) {
    return {
      ...mockBootstrap,
      preferences: fallbackPreferences,
    }
  }

  const preferences = desktopBootstrap.preferences
    ? normalizePreferences(desktopBootstrap.preferences, defaultWorkspaceId, defaultProjectId)
    : fallbackPreferences
  saveStoredPreferences(preferences)

  return {
    hostState: desktopBootstrap.hostState ?? fallbackHostState(),
    preferences,
    connections: shouldUseMockRuntime()
      ? mockConnections
      : desktopBootstrap.connections ?? mockConnections,
    backend: shouldUseMockRuntime()
      ? fallbackBackendConnection()
      : desktopBootstrap.backend ?? fallbackBackendConnection(),
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
      backend: {
        state: 'ready',
        transport: 'mock',
      },
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
      backend: {
        state: 'unavailable',
        transport: 'http',
      },
    }
  }
}

export async function restartDesktopBackend(): Promise<void> {
  if (!isTauriRuntime()) {
    return
  }

  try {
    await invoke('restart_desktop_backend')
  } catch {
    return
  }
}

export async function resolveRuntimeBackendConnection(): Promise<HostBackendConnection | undefined> {
  if (!isTauriRuntime()) {
    return undefined
  }

  const shellBootstrap = await resolveDesktopShellBootstrap()
  return shellBootstrap?.backend
}
