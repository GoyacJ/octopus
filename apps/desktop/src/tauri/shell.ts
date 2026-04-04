import { invoke } from '@tauri-apps/api/core'

import type {
  ConnectionProfile,
  HealthcheckStatus,
  HostBackendConnection,
  HostState,
  ShellBootstrap,
  ShellPreferences,
} from '@octopus/schema'
import {
  createFallbackBackendConnection,
  createFallbackHostState,
  extractProjectIdFromShellRoute,
  normalizeShellPreferences,
} from '@octopus/schema'

import { resolveMockShellBootstrap } from './mock'
import {
  isTauriRuntime,
  loadStoredPreferences,
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

async function resolveDesktopBackendConnection(): Promise<HostBackendConnection | undefined> {
  if (!isTauriRuntime()) {
    return undefined
  }

  try {
    return await invoke<HostBackendConnection>('get_backend_connection')
  } catch {
    return undefined
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
    ? normalizeShellPreferences(desktopBootstrap.preferences, defaultWorkspaceId, defaultProjectId)
    : fallbackPreferences
  saveStoredPreferences(preferences)

  return {
    hostState: desktopBootstrap.hostState ?? createFallbackHostState(),
    preferences,
    connections: desktopBootstrap.connections ?? mockConnections,
    backend: desktopBootstrap.backend ?? createFallbackBackendConnection('unavailable', 'http'),
  }
}

export async function loadPreferences(defaultWorkspaceId: string, defaultProjectId: string): Promise<ShellPreferences> {
  const fallbackPreferences = loadStoredPreferences(defaultWorkspaceId, defaultProjectId)
  if (!isTauriRuntime()) {
    return fallbackPreferences
  }

  try {
    const preferences = normalizeShellPreferences(await invoke<ShellPreferences>('load_preferences'), defaultWorkspaceId, defaultProjectId)
    saveStoredPreferences(preferences)
    return preferences
  } catch {
    return fallbackPreferences
  }
}

export async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  const normalizedPreferences = normalizeShellPreferences(
    {
      ...preferences,
      compactSidebar: preferences.leftSidebarCollapsed,
    },
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
  saveStoredPreferences(normalizedPreferences)
  if (!isTauriRuntime()) {
    return normalizedPreferences
  }

  try {
    const savedPreferences = normalizeShellPreferences(
      await invoke<ShellPreferences>('save_preferences', { preferences: normalizedPreferences }),
      normalizedPreferences.defaultWorkspaceId,
      extractProjectIdFromShellRoute(normalizedPreferences.lastVisitedRoute),
    )
    saveStoredPreferences(savedPreferences)
    return savedPreferences
  } catch {
    return normalizedPreferences
  }
}

export async function getHostState(): Promise<HostState> {
  if (!isTauriRuntime()) {
    return createFallbackHostState()
  }

  try {
    return await invoke<HostState>('get_host_state')
  } catch {
    return createFallbackHostState()
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
  return await resolveDesktopBackendConnection()
}
