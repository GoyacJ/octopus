import type { ConnectionProfile } from './catalog'
import type { Locale, ThemeMode } from './shared'

export interface HostState {
  platform: 'tauri' | 'web'
  mode: 'local'
  appVersion: string
  cargoWorkspace: boolean
  shell: string
}

export interface HealthcheckStatus {
  status: 'ok'
  host: 'web' | 'tauri'
  mode: 'local'
  cargoWorkspace: boolean
  backendReady: boolean
  transport: string
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

export interface DesktopBackendConnection {
  baseUrl?: string
  authToken?: string
  ready: boolean
  transport: string
}

export interface ShellBootstrap {
  hostState: HostState
  preferences: ShellPreferences
  connections: ConnectionProfile[]
  backend?: DesktopBackendConnection
}
