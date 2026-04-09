import type {
  ConnectionProfile,
  HostBackendConnection,
  ShellBootstrap,
  TransportSecurityLevel,
  WorkspaceConnectionRecord,
} from '@octopus/schema'
import { normalizeShellPreferences } from '@octopus/schema'

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
  return connections.map(connection => toWorkspaceConnectionRecord(connection, backend))
}

export function normalizeShellBootstrap(
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
    workspaceConnections: resolveWorkspaceConnections(payload.connections ?? [], payload.backend),
  }
}
