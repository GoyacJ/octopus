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
import type { JsonValue } from './runtime'

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
  configuredModelId?: string
  providerId: string
  modelId: string
  surface: ModelSurfaceId | string
}

export interface ConfiguredModelTokenQuota {
  totalTokens?: number
}

export interface ConfiguredModelTokenUsage {
  usedTokens: number
  remainingTokens?: number
  exhausted: boolean
}

export interface ConfiguredModelRecord {
  configuredModelId: string
  name: string
  providerId: string
  modelId: string
  credentialRef?: string
  baseUrl?: string
  tokenQuota?: ConfiguredModelTokenQuota
  tokenUsage: ConfiguredModelTokenUsage
  enabled: boolean
  source: string
  status: CredentialBinding['status'] | 'missing'
  configured: boolean
}

export interface ModelRegistryDiagnostics {
  warnings: string[]
  errors: string[]
}

export type WorkspaceToolKind = ToolCatalogKind
export type WorkspaceToolRequiredPermission = 'readonly' | 'workspace-write' | 'danger-full-access'
export type WorkspaceToolAvailability = ViewStatus

export interface WorkspaceToolManagementCapabilities {
  canDisable: boolean
  canEdit: boolean
  canDelete: boolean
}

export type WorkspaceSkillSourceOrigin = 'skills_dir' | 'legacy_commands_dir' | 'builtin_bundle'

export interface WorkspaceToolConsumerSummary {
  kind: 'agent' | 'team' | string
  id: string
  name: string
  scope: 'workspace' | 'project' | string
  ownerId?: string
  ownerLabel?: string
}

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
  disabled: boolean
  management: WorkspaceToolManagementCapabilities
  ownerScope?: 'builtin' | 'workspace' | 'project' | string
  ownerId?: string
  ownerLabel?: string
  consumers?: WorkspaceToolConsumerSummary[]
}

export interface WorkspaceBuiltinToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'builtin'
  builtinKey: string
}

export interface WorkspaceSkillToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'skill'
  active: boolean
  shadowedBy?: string
  sourceOrigin: WorkspaceSkillSourceOrigin
  workspaceOwned: boolean
  relativePath?: string
}

export interface WorkspaceMcpToolCatalogEntry extends WorkspaceToolCatalogBase {
  kind: 'mcp'
  serverName: string
  endpoint: string
  toolNames: string[]
  statusDetail?: string
  scope: 'workspace' | 'project' | 'user' | 'builtin'
}

export type WorkspaceToolCatalogEntry =
  | WorkspaceBuiltinToolCatalogEntry
  | WorkspaceSkillToolCatalogEntry
  | WorkspaceMcpToolCatalogEntry

export type CapabilityAssetState =
  | 'builtin'
  | 'workspace'
  | 'project'
  | 'user'
  | 'external'
  | 'managed'
  | 'shadowed'
  | 'disabled'

export type CapabilityAssetImportStatus = 'managed' | 'copy-required' | 'not-importable'
export type CapabilityAssetExportStatus = 'exportable' | 'readonly' | 'not-exportable'

export interface CapabilityAssetManifest {
  assetId: string
  workspaceId: string
  sourceKey: string
  kind: WorkspaceToolKind
  name: string
  description: string
  displayPath: string
  ownerScope?: WorkspaceToolCatalogEntry['ownerScope']
  ownerId?: string
  ownerLabel?: string
  requiredPermission: WorkspaceToolRequiredPermission | null
  management: WorkspaceToolManagementCapabilities
  installed: boolean
  enabled: boolean
  health: WorkspaceToolAvailability
  state: CapabilityAssetState
  importStatus: CapabilityAssetImportStatus
  exportStatus: CapabilityAssetExportStatus
}

export interface SkillPackageManifest extends CapabilityAssetManifest {
  kind: 'skill'
  packageKind: 'workspace' | 'project' | 'builtin' | 'external'
  active: boolean
  shadowedBy?: string
  sourceOrigin: WorkspaceSkillSourceOrigin
  workspaceOwned: boolean
  relativePath?: string
}

export interface McpServerPackageManifest extends CapabilityAssetManifest {
  kind: 'mcp'
  packageKind: WorkspaceMcpToolCatalogEntry['scope']
  serverName: string
  endpoint: string
  toolNames: string[]
  scope: WorkspaceMcpToolCatalogEntry['scope']
  statusDetail?: string
}

export type CapabilityManagementEntry =
  | (WorkspaceBuiltinToolCatalogEntry & CapabilityAssetManifest)
  | (WorkspaceSkillToolCatalogEntry & CapabilityAssetManifest)
  | (WorkspaceMcpToolCatalogEntry & CapabilityAssetManifest)

export interface CapabilityManagementProjection {
  entries: CapabilityManagementEntry[]
  assets: CapabilityAssetManifest[]
  skillPackages: SkillPackageManifest[]
  mcpServerPackages: McpServerPackageManifest[]
}

export interface CapabilityAssetDisablePatch {
  sourceKey: string
  disabled: boolean
}

export interface CreateWorkspaceSkillInput {
  slug: string
  content: string
}

export interface UpdateWorkspaceSkillInput {
  content: string
}

export interface WorkspaceSkillTreeNode {
  path: string
  name: string
  kind: 'directory' | 'file'
  children?: WorkspaceSkillTreeNode[]
  byteSize?: number
  isText?: boolean
}

export interface WorkspaceSkillDocument {
  id: string
  sourceKey: string
  name: string
  description: string
  content: string
  displayPath: string
  rootPath: string
  tree: WorkspaceSkillTreeNode[]
  sourceOrigin: WorkspaceSkillSourceOrigin
  workspaceOwned: boolean
  relativePath?: string
}

export interface WorkspaceSkillTreeDocument {
  skillId: string
  sourceKey: string
  displayPath: string
  rootPath: string
  tree: WorkspaceSkillTreeNode[]
}

export interface WorkspaceSkillFileDocument {
  skillId: string
  sourceKey: string
  path: string
  displayPath: string
  byteSize: number
  isText: boolean
  content: string | null
  contentType?: string
  language?: string
  readonly: boolean
}

export interface UpdateWorkspaceSkillFileInput {
  content: string
}

export interface WorkspaceFileUploadPayload {
  fileName: string
  contentType: string
  dataBase64: string
  byteSize: number
}

export interface WorkspaceDirectoryUploadEntry extends WorkspaceFileUploadPayload {
  relativePath: string
}

export interface ImportWorkspaceSkillArchiveInput {
  slug: string
  archive: WorkspaceFileUploadPayload
}

export interface ImportWorkspaceSkillFolderInput {
  slug: string
  files: WorkspaceDirectoryUploadEntry[]
}

export interface CopyWorkspaceSkillToManagedInput {
  slug: string
}

export interface UpsertWorkspaceMcpServerInput {
  serverName: string
  config: Record<string, JsonValue>
}

export interface WorkspaceMcpServerDocument {
  serverName: string
  sourceKey: string
  displayPath: string
  scope: 'workspace' | 'project' | 'user' | 'builtin'
  config: Record<string, JsonValue>
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
