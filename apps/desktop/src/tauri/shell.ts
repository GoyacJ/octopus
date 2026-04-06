import { invoke } from '@tauri-apps/api/core'

import type {
  ConnectionProfile,
  HealthcheckStatus,
  HostBackendConnection,
  HostState,
  ShellBootstrap,
  ShellPreferences,
  TransportSecurityLevel,
  WorkspaceConnectionRecord,
} from '@octopus/schema'
import {
  extractProjectIdFromShellRoute,
  normalizeShellPreferences,
} from '@octopus/schema'

import {
  fetchHostApi,
  isTauriRuntime,
  resolveBrowserHostApiBaseUrl,
  resolveBrowserHostAuthToken,
  resolveHostRuntime,
} from './shared'

function assertTauriHostAvailable(): void {
  if (!isTauriRuntime()) {
    throw new Error('Tauri host runtime is unavailable')
  }
}

function resolveTransportSecurity(mode: ConnectionProfile['mode']): TransportSecurityLevel {
  switch (mode) {
    case 'local':
      return 'loopback'
    case 'shared':
      return 'trusted'
    case 'remote':
      return 'public'
    default:
      return 'trusted'
  }
}

function resolveWorkspaceConnectionStatus(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord['status'] {
  if (connection.mode === 'local') {
    if (backend?.state === 'ready') {
      return 'connected'
    }

    return backend?.state === 'unavailable' ? 'unreachable' : 'disconnected'
  }

  switch (connection.state) {
    case 'connected':
    case 'local-ready':
      return 'connected'
    case 'disconnected':
      return 'disconnected'
    default:
      return 'disconnected'
  }
}

function resolveWorkspaceConnectionBaseUrl(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): string {
  if (connection.baseUrl) {
    return connection.baseUrl
  }

  if (connection.mode === 'local' && backend?.baseUrl) {
    return backend.baseUrl
  }

  return 'http://127.0.0.1'
}

function toWorkspaceConnectionRecord(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord {
  return {
    workspaceConnectionId: connection.id,
    workspaceId: connection.workspaceId,
    label: connection.label,
    baseUrl: resolveWorkspaceConnectionBaseUrl(connection, backend),
    transportSecurity: resolveTransportSecurity(connection.mode),
    authMode: 'session-token',
    lastUsedAt: connection.lastSyncAt,
    status: resolveWorkspaceConnectionStatus(connection, backend),
  }
}

function resolveWorkspaceConnections(
  connections: ConnectionProfile[],
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord[] {
  return connections.map((connection) => toWorkspaceConnectionRecord(connection, backend))
}

function normalizeShellBootstrap(
  payload: ShellBootstrap,
  defaultWorkspaceId: string,
  defaultProjectId: string,
): ShellBootstrap {
  const preferences = normalizeShellPreferences(
    payload.preferences,
    defaultWorkspaceId,
    defaultProjectId,
  )

  return {
    ...payload,
    preferences,
    workspaceConnections: resolveWorkspaceConnections(
      payload.connections ?? [],
      payload.backend,
    ),
  }
}

async function resolveDesktopShellBootstrap(): Promise<ShellBootstrap> {
  assertTauriHostAvailable()
  return await invoke<ShellBootstrap>('bootstrap_shell')
}

async function resolveDesktopBackendConnection(): Promise<HostBackendConnection | undefined> {
  assertTauriHostAvailable()
  return await invoke<HostBackendConnection>('get_backend_connection')
}

async function bootstrapShellHostTauri(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return normalizeShellBootstrap(
    await resolveDesktopShellBootstrap(),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function loadPreferencesTauri(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  assertTauriHostAvailable()
  return normalizeShellPreferences(
    await invoke<ShellPreferences>('load_preferences'),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferencesTauri(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  assertTauriHostAvailable()
  return normalizeShellPreferences(
    await invoke<ShellPreferences>('save_preferences', { preferences }),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostStateTauri(): Promise<HostState> {
  assertTauriHostAvailable()
  return await invoke<HostState>('get_host_state')
}

async function healthcheckTauri(): Promise<HealthcheckStatus> {
  assertTauriHostAvailable()
  return await invoke<HealthcheckStatus>('healthcheck')
}

async function restartDesktopBackendTauri(): Promise<void> {
  assertTauriHostAvailable()
  await invoke('restart_desktop_backend')
}

function resolveBrowserHostConfig(): { baseUrl: string, authToken: string } {
  return {
    baseUrl: resolveBrowserHostApiBaseUrl(),
    authToken: resolveBrowserHostAuthToken(),
  }
}

async function bootstrapShellHostBrowser(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const payload = await fetchHostApi<ShellBootstrap>(
    baseUrl,
    authToken,
    '/api/v1/host/bootstrap',
    { method: 'GET' },
  )

  return normalizeShellBootstrap(payload, defaultWorkspaceId, defaultProjectId)
}

async function loadPreferencesBrowser(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostApi<ShellPreferences>(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      { method: 'GET' },
    ),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferencesBrowser(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostApi<ShellPreferences>(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      {
        method: 'PUT',
        body: JSON.stringify(preferences),
      },
    ),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostStateBrowser(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostState> {
  return (await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)).hostState
}

async function healthcheckBrowser(): Promise<HealthcheckStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<HealthcheckStatus>(
    baseUrl,
    authToken,
    '/api/v1/host/health',
    { method: 'GET' },
  )
}

async function restartDesktopBackendBrowser(): Promise<void> {
  throw new Error('Browser host runtime does not support desktop backend restart')
}

async function resolveDesktopBackendConnectionBrowser(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostBackendConnection | undefined> {
  return (await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)).backend
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return resolveHostRuntime() === 'browser'
    ? await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)
    : await bootstrapShellHostTauri(defaultWorkspaceId, defaultProjectId)
}

export async function loadPreferences(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  return resolveHostRuntime() === 'browser'
    ? await loadPreferencesBrowser(defaultWorkspaceId, defaultProjectId)
    : await loadPreferencesTauri(defaultWorkspaceId, defaultProjectId)
}

export async function savePreferences(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  return resolveHostRuntime() === 'browser'
    ? await savePreferencesBrowser(preferences)
    : await savePreferencesTauri(preferences)
}

export async function getHostState(): Promise<HostState> {
  return resolveHostRuntime() === 'browser'
    ? await getHostStateBrowser()
    : await getHostStateTauri()
}

export async function healthcheck(): Promise<HealthcheckStatus> {
  return resolveHostRuntime() === 'browser'
    ? await healthcheckBrowser()
    : await healthcheckTauri()
}

export async function restartDesktopBackend(): Promise<void> {
  if (resolveHostRuntime() === 'browser') {
    await restartDesktopBackendBrowser()
    return
  }

  await restartDesktopBackendTauri()
}

export async function resolveDesktopBackendConnectionForHost(): Promise<HostBackendConnection | undefined> {
  return resolveHostRuntime() === 'browser'
    ? await resolveDesktopBackendConnectionBrowser()
    : await resolveDesktopBackendConnection()
}

export const hostClient = {
  bootstrapShellHost,
  loadPreferences,
  savePreferences,
  getHostState,
  healthcheck,
  restartDesktopBackend,
  resolveDesktopBackendConnection: resolveDesktopBackendConnectionForHost,
}
