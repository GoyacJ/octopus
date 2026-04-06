import type { ConnectionProfile } from './catalog'
import type {
  BackendConnectionState,
  BackendTransport,
  HostExecutionMode,
  HostPlatform,
  Locale,
  ThemeMode,
} from './shared'
import type {
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from './workspace-protocol'

export interface HostState {
  platform: HostPlatform
  mode: HostExecutionMode
  appVersion: string
  cargoWorkspace: boolean
  shell: string
}

export interface HostBackendConnection {
  baseUrl?: string
  authToken?: string
  state: BackendConnectionState
  transport: BackendTransport
}

export interface HealthcheckStatus {
  status: 'ok'
  host: HostPlatform
  mode: HostExecutionMode
  cargoWorkspace: boolean
  backend: Pick<HostBackendConnection, 'state' | 'transport'>
}

export interface ShellPreferences {
  theme: ThemeMode
  locale: Locale
  fontSize: number
  fontFamily: string
  fontStyle: string
  compactSidebar: boolean
  leftSidebarCollapsed: boolean
  rightSidebarCollapsed: boolean
  defaultWorkspaceId: string
  lastVisitedRoute: string
}

export interface ShellBootstrap {
  hostState: HostState
  preferences: ShellPreferences
  connections: ConnectionProfile[]
  backend?: HostBackendConnection
  workspaceConnections?: WorkspaceConnectionRecord[]
  workspaceSessions?: WorkspaceSessionTokenEnvelope[]
}

export type ConversationDetailFocus = 'summary' | 'memories' | 'artifacts' | 'knowledge' | 'resources' | 'tools' | 'timeline'

export interface ShellRouteState {
  detail?: string | null
  pane?: string | null
  artifact?: string | null
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
    defaultWorkspaceId: nextWorkspaceId,
    lastVisitedRoute: nextRoute,
  }
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

export function normalizeConversationDetailFocus(
  detail?: string | null,
  pane?: string | null,
): ConversationDetailFocus {
  const value = detail ?? pane
  switch (value) {
    case 'summary':
    case 'memories':
    case 'artifacts':
    case 'knowledge':
    case 'resources':
    case 'tools':
    case 'timeline':
      return value
    default:
      return 'summary'
  }
}
