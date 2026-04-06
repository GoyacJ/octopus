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
  WorkspaceToolPermissionMode,
  WorkspaceToolStatus,
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

interface WorkspaceToolBase {
  id: string
  workspaceId: string
  name: string
  kind: ToolCatalogKind
  description: string
  availability: ViewStatus
  status: WorkspaceToolStatus
  permissionMode: WorkspaceToolPermissionMode
}

export interface BuiltinToolDefinition extends WorkspaceToolBase {
  kind: 'builtin'
  builtinKey: string
}

export interface SkillToolDefinition extends WorkspaceToolBase {
  kind: 'skill'
  content: string
}

export interface McpToolDefinition extends WorkspaceToolBase {
  kind: 'mcp'
  serverName: string
  endpoint: string
  toolNames: string[]
  notes: string
}

export type WorkspaceToolDefinition = BuiltinToolDefinition | SkillToolDefinition | McpToolDefinition

export interface ToolCatalogItem {
  id: string
  workspaceId: string
  name: string
  kind: ToolCatalogKind
  description: string
  availability: ViewStatus
  status: WorkspaceToolStatus
  permissionMode: WorkspaceToolPermissionMode
  content?: string
  serverName?: string
  endpoint?: string
  toolNames?: string[]
  notes?: string
}

export interface ToolCatalogGroup {
  id: ToolCatalogKind
  title: string
  items: ToolCatalogItem[]
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
