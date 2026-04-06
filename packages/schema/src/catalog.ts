import type {
  AutomationStatus,
  ConnectionMode,
  ConnectionState,
  DesktopSettingsTabId,
  InboxItemType,
  PermissionMode,
  RiskLevel,
  SettingsSectionId,
  ToolCatalogKind,
  ViewStatus,
} from './shared'

export interface InboxItem {
  id: string
  workspaceId: string
  projectId?: string
  type: InboxItemType
  title: string
  description: string
  relatedId?: string
  status: 'pending' | 'resolved' | 'dismissed'
  priority: RiskLevel
  createdAt: number
  impact: string
  riskNote: string
  recommendedAction: string
  conversationId?: string
  artifactId?: string
  traceId?: string
}

export interface InboxApproval extends InboxItem {
  approverLabel: string
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

export function createConnectionProfile(input: {
  id: string
  mode: ConnectionMode
  label: string
  workspaceId: string
  baseUrl?: string
  state?: ConnectionState
  lastSyncAt?: number
}): ConnectionProfile {
  return {
    id: input.id,
    mode: input.mode,
    label: input.label,
    workspaceId: input.workspaceId,
    baseUrl: input.baseUrl,
    state: input.state ?? (input.mode === 'local' ? 'local-ready' : 'connected'),
    lastSyncAt: input.lastSyncAt,
  }
}

export interface ModelCatalogItem {
  id: string
  label: string
  provider: 'Anthropic' | 'OpenAI' | 'xAI' | 'Custom' | string
  description: string
  recommendedFor: string
  availability: ViewStatus
  defaultPermission: PermissionMode

  // Model parameters and capabilities
  contextWindow?: number
  maxTokens?: number
  capabilities?: string[]

  // Custom model configuration
  isCustom?: boolean
  customBaseUrl?: string

  // Statistics and cost
  estimatedMonthlyCost?: number
  cacheHitRate?: number

  // Defaults
  defaultSystemPrompt?: string
}

export interface ProviderCredential {
  id: string
  provider: 'Anthropic' | 'OpenAI' | 'xAI'
  name: string
  apiKey: string
  baseUrl?: string
  status: 'healthy' | 'error' | 'unconfigured'
}

export type ModelCapabilityId =
  | 'streaming'
  | 'tool_calling'
  | 'structured_output'
  | 'reasoning'
  | 'vision_input'
  | 'image_generation'
  | 'audio_io'
  | 'realtime'
  | 'files'
  | 'batch'
  | 'context_cache'
  | 'mcp'
  | 'web_search'
  | 'computer_use'

export type ModelSurfaceId =
  | 'conversation'
  | 'responses'
  | 'files'
  | 'batch'
  | 'realtime'
  | 'media'
  | 'image'
  | 'audio'
  | 'video'
  | 'cache'
  | 'music'
  | 'embeddings'

export type ProtocolFamily =
  | 'anthropic_messages'
  | 'openai_chat'
  | 'openai_responses'
  | 'gemini_native'
  | 'vendor_native'

export type AuthStrategy = 'bearer' | 'x_api_key' | 'api_key' | 'none'
export type BaseUrlPolicy = 'fixed' | 'allow_override' | 'credential_only'

export interface CapabilityDescriptor {
  capabilityId: ModelCapabilityId | string
  label: string
}

export interface SurfaceDescriptor {
  surface: ModelSurfaceId | string
  protocolFamily: ProtocolFamily | string
  transport: string[]
  authStrategy: AuthStrategy | string
  baseUrl: string
  baseUrlPolicy: BaseUrlPolicy | string
  enabled: boolean
  capabilities: CapabilityDescriptor[]
}

export interface ProviderRegistryRecord {
  providerId: string
  label: string
  enabled: boolean
  surfaces: SurfaceDescriptor[]
  metadata: Record<string, unknown>
}

export interface ModelSurfaceBinding {
  surface: ModelSurfaceId | string
  protocolFamily: ProtocolFamily | string
  enabled: boolean
}

export interface ModelRegistryRecord {
  modelId: string
  providerId: string
  label: string
  description: string
  family: string
  track: string
  enabled: boolean
  recommendedFor: string
  availability: ViewStatus | string
  defaultPermission: PermissionMode
  surfaceBindings: ModelSurfaceBinding[]
  capabilities: CapabilityDescriptor[]
  contextWindow?: number
  maxOutputTokens?: number
  metadata: Record<string, unknown>
}

export interface CredentialBinding {
  credentialRef: string
  providerId: string
  label: string
  baseUrl?: string
  status: 'healthy' | 'error' | 'unconfigured' | 'configured'
  configured: boolean
  source: string
}

export interface DefaultSelection {
  providerId: string
  modelId: string
  surface: ModelSurfaceId | string
}

export interface ModelRegistryDiagnostics {
  warnings: string[]
  errors: string[]
}

export type WorkspaceToolKind = ToolCatalogKind
export type WorkspaceToolRequiredPermission = 'readonly' | 'workspace-write' | 'danger-full-access'
export type WorkspaceToolAvailability = ViewStatus

interface WorkspaceToolCatalogBase {
  id: string
  workspaceId: string
  name: string
  kind: WorkspaceToolKind
  description: string
  requiredPermission: WorkspaceToolRequiredPermission | null
  availability: WorkspaceToolAvailability
  sourceKey: string
  displayPath: string
}

export interface WorkspaceBuiltinToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'builtin'
  builtinKey: string
}

export interface WorkspaceSkillToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'skill'
  active: boolean
  shadowedBy?: string
  sourceOrigin: 'skills_dir' | 'legacy_commands_dir'
}

export interface WorkspaceMcpToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'mcp'
  serverName: string
  endpoint: string
  toolNames: string[]
  statusDetail?: string
  scope: 'workspace' | 'project' | 'user'
}

export type WorkspaceToolCatalogEntry =
  | WorkspaceBuiltinToolCatalogEntry
  | WorkspaceSkillToolCatalogEntry
  | WorkspaceMcpToolCatalogEntry

export interface WorkspaceToolCatalogSnapshot {
  entries: WorkspaceToolCatalogEntry[]
}

export interface AutomationSummary {
  id: string
  workspaceId: string
  projectId?: string
  title: string
  description: string
  cadence: string
  ownerType: 'agent' | 'team'
  ownerId: string
  status: AutomationStatus
  nextRunAt?: number
  lastRunAt?: number
  output: string
}

export interface DesktopSettingsTab {
  value: DesktopSettingsTabId
  label: string
}

export interface DesktopSettingsSection {
  id: string
  tab: DesktopSettingsTabId
  title: string
  description?: string
  items: string[]
}

export interface DesktopSettingsPage {
  tabs: DesktopSettingsTab[]
  sections: DesktopSettingsSection[]
}

export interface SettingsSection {
  id: SettingsSectionId
  title: string
  description: string
  status: ViewStatus
  items: string[]
}
