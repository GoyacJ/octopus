import type { ConnectionProfile } from './catalog'
import type {
  HealthcheckStatus as OpenApiHealthcheckStatus,
  HostBackendConnection as OpenApiHostBackendConnection,
  HostState as OpenApiHostState,
  ShellBootstrap as OpenApiShellBootstrap,
  ShellPreferences as OpenApiShellPreferences,
} from './generated'
import type {
  BackendConnectionState,
  BackendTransport,
  HostExecutionMode,
  HostPlatform,
  Locale,
  ThemeMode,
} from './shared'
import type { HostUpdateChannel } from './updates'
import type {
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from './workspace-protocol'

export type HostState = OpenApiHostState
export type HostBackendConnection = OpenApiHostBackendConnection
export type HealthcheckStatus = OpenApiHealthcheckStatus
export type ShellPreferences = OpenApiShellPreferences
export type ShellBootstrap = OpenApiShellBootstrap

export type ConversationWorkbenchMode = 'deliverable' | 'context' | 'ops'

export interface ShellRouteState {
  conversationId?: string | null
  mode?: string | null
  deliverable?: string | null
  version?: string | number | null
}

export function createDefaultShellPreferences(defaultWorkspaceId: string, defaultProjectId: string): ShellPreferences {
  return {
    theme: 'system',
    locale: 'zh-CN',
    fontSize: 14,
    fontFamily: 'Inter, sans-serif',
    fontStyle: 'sans',
    compactSidebar: false,
    leftSidebarCollapsed: false,
    rightSidebarCollapsed: false,
    updateChannel: 'formal',
    defaultWorkspaceId,
    lastVisitedRoute: `/workspaces/${defaultWorkspaceId}/overview?project=${defaultProjectId}`,
  }
}

export function normalizeShellPreferences(
  preferences: Partial<ShellPreferences> | undefined,
  defaultWorkspaceId: string,
  defaultProjectId: string,
): ShellPreferences {
  const fallback = createDefaultShellPreferences(defaultWorkspaceId, defaultProjectId)
  const nextWorkspaceId = preferences?.defaultWorkspaceId || fallback.defaultWorkspaceId
  const nextProjectId = extractProjectIdFromShellRoute(preferences?.lastVisitedRoute) || defaultProjectId
  const nextRoute = preferences?.lastVisitedRoute || `/workspaces/${nextWorkspaceId}/overview?project=${nextProjectId}`
  const leftSidebarCollapsed = typeof preferences?.leftSidebarCollapsed === 'boolean'
    ? preferences.leftSidebarCollapsed
    : typeof preferences?.compactSidebar === 'boolean'
      ? preferences.compactSidebar
      : fallback.leftSidebarCollapsed

  return {
    theme: preferences?.theme ?? fallback.theme,
    locale: preferences?.locale ?? fallback.locale,
    fontSize: preferences?.fontSize ?? fallback.fontSize,
    fontFamily: preferences?.fontFamily ?? fallback.fontFamily,
    fontStyle: preferences?.fontStyle ?? fallback.fontStyle,
    compactSidebar: typeof preferences?.compactSidebar === 'boolean'
      ? preferences.compactSidebar
      : leftSidebarCollapsed,
    leftSidebarCollapsed,
    rightSidebarCollapsed: preferences?.rightSidebarCollapsed ?? fallback.rightSidebarCollapsed,
    updateChannel: normalizeUpdateChannel(preferences?.updateChannel),
    defaultWorkspaceId: nextWorkspaceId,
    lastVisitedRoute: nextRoute,
  }
}

function normalizeUpdateChannel(channel?: HostUpdateChannel | null): HostUpdateChannel {
  return channel === 'preview' ? 'preview' : 'formal'
}

export function createFallbackHostState(platform: HostPlatform = 'web'): HostState {
  return {
    platform,
    mode: 'local',
    appVersion: '0.1.0',
    cargoWorkspace: false,
    shell: platform === 'tauri' ? 'tauri2' : 'web',
  }
}

export function createFallbackBackendConnection(
  state: BackendConnectionState = 'ready',
  transport: BackendTransport = 'http',
): HostBackendConnection {
  return {
    state,
    transport,
  }
}

export function extractProjectIdFromShellRoute(route?: string | null): string {
  if (!route) {
    return 'proj-redesign'
  }

  const projectMatch = route.match(/\/projects\/([^/?#]+)/)
  if (projectMatch?.[1]) {
    return decodeURIComponent(projectMatch[1])
  }

  const queryMatch = route.match(/[?&]project=([^&#]+)/)
  if (queryMatch?.[1]) {
    return decodeURIComponent(queryMatch[1])
  }

  return 'proj-redesign'
}

export function normalizeConversationWorkbenchMode(
  mode?: string | null,
): ConversationWorkbenchMode {
  switch (mode) {
    case 'deliverable':
    case 'context':
    case 'ops':
      return mode
    default:
      return 'context'
  }
}

export function normalizeWorkbenchVersion(
  version?: string | number | null,
): number | null {
  if (typeof version === 'number' && Number.isFinite(version) && version > 0) {
    return version
  }

  if (typeof version === 'string' && version.trim()) {
    const parsed = Number.parseInt(version, 10)
    if (Number.isFinite(parsed) && parsed > 0) {
      return parsed
    }
  }

  return null
}
