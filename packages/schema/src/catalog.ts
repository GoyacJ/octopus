import type {
  AutomationStatus,
  ConnectionMode,
  ConnectionState,
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

export interface ModelCatalogItem {
  id: string
  label: string
  provider: string
  description: string
  recommendedFor: string
  availability: ViewStatus
  defaultPermission: PermissionMode
}

export interface ToolCatalogItem {
  id: string
  name: string
  kind: ToolCatalogKind
  description: string
  availability: ViewStatus
  permissions: string[]
}

export interface ToolCatalogGroup {
  id: ToolCatalogKind
  title: string
  description: string
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

export interface SettingsSection {
  id: SettingsSectionId
  title: string
  description: string
  status: ViewStatus
  items: string[]
}
