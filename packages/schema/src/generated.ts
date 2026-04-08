/* eslint-disable */
// Generated from contracts/openapi/octopus.openapi.yaml by scripts/generate-schema.mjs.
// Source hash: cca8fdc8a235fca099f1e28fdbffdf22ffc03158c2b528e9cd3e330684a255ac

export const OCTOPUS_OPENAPI_VERSION = "3.1.0"
export const OCTOPUS_API_VERSION = "0.1.0"
export const OCTOPUS_OPENAPI_SOURCE_HASH = "cca8fdc8a235fca099f1e28fdbffdf22ffc03158c2b528e9cd3e330684a255ac"

export type ThemeMode = "light" | "dark" | "system"

export type Locale = "zh-CN" | "en-US"

export type HostPlatform = "tauri" | "web"

export type HostExecutionMode = "local" | "remote"

export type BackendTransport = "http" | "sse" | "ws"

export type BackendConnectionState = "ready" | "unavailable"

export type ConnectionMode = "local" | "shared" | "remote"

export type ConnectionState = "local-ready" | "connected" | "disconnected"

export type TransportSecurityLevel = "loopback" | "trusted" | "public"

export type WorkspaceAuthMode = "session-token"

export type WorkspaceConnectionStatus = "connected" | "disconnected" | "expired" | "unreachable"

export type ApiErrorCode = "UNAUTHENTICATED" | "SESSION_EXPIRED" | "FORBIDDEN" | "NOT_FOUND" | "CONFLICT" | "INVALID_INPUT" | "RATE_LIMITED" | "UNAVAILABLE" | "CAPABILITY_UNSUPPORTED" | "INTERNAL_ERROR"

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

export interface HealthcheckBackendStatus {
  state: BackendConnectionState
  transport: BackendTransport
}

export interface ConnectionProfile {
  id: string
  mode: ConnectionMode
  label: string
  workspaceId: string
  baseUrl?: string
  state: ConnectionState
  lastSyncAt?: number
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

export interface SessionRecord {
  id: string
  workspaceId: string
  userId: string
  clientAppId: string
  token: string
  status: string
  createdAt: number
  expiresAt?: number
  roleIds: string[]
  scopeProjectIds: string[]
}

export interface WorkspaceConnectionRecord {
  workspaceConnectionId: string
  workspaceId: string
  label: string
  baseUrl: string
  transportSecurity: TransportSecurityLevel
  authMode: WorkspaceAuthMode
  lastUsedAt?: number
  status: WorkspaceConnectionStatus
}

export interface WorkspaceSessionTokenEnvelope {
  workspaceConnectionId: string
  token: string
  session: SessionRecord
  issuedAt: number
  expiresAt?: number
}

export interface ShellBootstrap {
  hostState: HostState
  preferences: ShellPreferences
  connections: ConnectionProfile[]
  backend?: HostBackendConnection
  workspaceConnections?: WorkspaceConnectionRecord[]
  workspaceSessions?: WorkspaceSessionTokenEnvelope[]
}

export interface HealthcheckStatus {
  status: "ok"
  host: HostPlatform
  mode: HostExecutionMode
  cargoWorkspace: boolean
  backend: HealthcheckBackendStatus
}

export interface ClientAppRecord {
  id: string
  name: string
  platform: string
  status: string
  firstParty: boolean
  allowedOrigins: string[]
  allowedHosts: string[]
  sessionPolicy: string
  defaultScopes: string[]
}

export interface WorkspaceSummary {
  id: string
  name: string
  slug: string
  deployment: "local" | "remote"
  bootstrapStatus: "setup_required" | "ready"
  ownerUserId?: string
  host: string
  listenAddress: string
  defaultProjectId: string
}

export interface WorkspaceCapabilitySet {
  polling: boolean
  sse: boolean
  idempotency: boolean
  reconnect: boolean
  eventReplay: boolean
}

export interface SystemBootstrapStatus {
  workspace: WorkspaceSummary
  setupRequired: boolean
  ownerReady: boolean
  registeredApps: ClientAppRecord[]
  protocolVersion: string
  apiBasePath: string
  transportSecurity: TransportSecurityLevel
  authMode: WorkspaceAuthMode
  capabilities: WorkspaceCapabilitySet
}

export interface ProjectModelAssignments {
  configuredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectToolAssignments {
  sourceKeys: string[]
}

export interface ProjectAgentAssignments {
  agentIds: string[]
  teamIds: string[]
}

export interface ProjectWorkspaceAssignments {
  models?: ProjectModelAssignments
  tools?: ProjectToolAssignments
  agents?: ProjectAgentAssignments
}

export interface ProjectRecord {
  id: string
  workspaceId: string
  name: string
  status: "active" | "archived"
  description: string
  assignments?: ProjectWorkspaceAssignments
}

export interface WorkspaceMetricRecord {
  id: string
  label: string
  value: string
  helper?: string
  tone?: string
}

export interface ConversationRecord {
  id: string
  workspaceId: string
  projectId: string
  sessionId: string
  title: string
  status: string
  updatedAt: number
  lastMessagePreview?: string
}

export interface WorkspaceActivityRecord {
  id: string
  title: string
  description: string
  timestamp: number
}

export interface WorkspaceOverviewSnapshot {
  workspace: WorkspaceSummary
  metrics: WorkspaceMetricRecord[]
  projects: ProjectRecord[]
  recentConversations: ConversationRecord[]
  recentActivity: WorkspaceActivityRecord[]
}

export interface RuntimeConfigSource {
  scope: string
  ownerId?: string
  displayPath: string
  sourceKey: string
  exists: boolean
  loaded: boolean
  contentHash?: string
  document?: unknown
}

export interface RuntimeSecretReferenceStatus {
  scope: string
  path: string
  reference?: string
  status: string
}

export interface RuntimeConfigValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

export interface RuntimeEffectiveConfig {
  effectiveConfig: unknown
  effectiveConfigHash: string
  sources: RuntimeConfigSource[]
  validation: RuntimeConfigValidationResult
  secretReferences: RuntimeSecretReferenceStatus[]
}

export interface ApiErrorDetailEnvelope {
  code: ApiErrorCode
  message: string
  details?: unknown
  requestId: string
  retryable: boolean
}

export interface ApiErrorEnvelope {
  error: ApiErrorDetailEnvelope
}


export interface OctopusApiPaths {
  "/api/v1/host/bootstrap": {
    get: { operationId: "getHostBootstrap"; response: ShellBootstrap; error: ApiErrorEnvelope }
  }
  "/api/v1/host/health": {
    get: { operationId: "getHostHealthcheck"; response: HealthcheckStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/system/bootstrap": {
    get: { operationId: "getSystemBootstrap"; response: SystemBootstrapStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/overview": {
    get: { operationId: "getWorkspaceOverview"; response: WorkspaceOverviewSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config": {
    get: { operationId: "getRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
}

