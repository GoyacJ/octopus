import type { ConnectionProfile } from './catalog'
import type {
  BackendConnectionState,
  BackendTransport,
  HostExecutionMode,
  HostPlatform,
  Locale,
  ThemeMode,
} from './shared'

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
}
