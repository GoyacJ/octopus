import type {
  ChangeCurrentUserPasswordRequest as OpenApiChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse as OpenApiChangeCurrentUserPasswordResponse,
  CreateWorkspaceResourceFolderInput as OpenApiCreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput as OpenApiCreateWorkspaceResourceInput,
  CreateWorkspaceUserRequest as OpenApiCreateWorkspaceUserRequest,
  ConversationRecord as OpenApiConversationRecord,
  KnowledgeRecord as OpenApiKnowledgeRecord,
  MenuRecord as OpenApiMenuRecord,
  PermissionRecord as OpenApiPermissionRecord,
  ProjectTeamLinkInput as OpenApiProjectTeamLinkInput,
  ProjectTeamLinkRecord as OpenApiProjectTeamLinkRecord,
  ProjectDashboardSnapshot as OpenApiProjectDashboardSnapshot,
  RoleRecord as OpenApiRoleRecord,
  TeamRecord as OpenApiTeamRecord,
  UpdateCurrentUserProfileRequest as OpenApiUpdateCurrentUserProfileRequest,
  UpdateWorkspaceUserRequest as OpenApiUpdateWorkspaceUserRequest,
  UpsertTeamInput as OpenApiUpsertTeamInput,
  UpdateWorkspaceResourceInput as OpenApiUpdateWorkspaceResourceInput,
  UserCenterAlertRecord as OpenApiUserCenterAlertRecord,
  UserCenterOverviewSnapshot as OpenApiUserCenterOverviewSnapshot,
  UserRecordSummary as OpenApiUserRecordSummary,
  WorkspaceResourceFolderUploadEntry as OpenApiWorkspaceResourceFolderUploadEntry,
  WorkspaceResourceRecord as OpenApiWorkspaceResourceRecord,
  WorkspaceActivityRecord as OpenApiWorkspaceActivityRecord,
  WorkspaceMetricRecord as OpenApiWorkspaceMetricRecord,
  WorkspaceOverviewSnapshot as OpenApiWorkspaceOverviewSnapshot,
} from './generated'
import type {
  AvatarUploadPayload,
} from './auth'
import type {
  AgentScope,
  AgentStatus,
  AutomationStatus,
  ViewStatus,
  WorkspaceToolPermissionMode,
  WorkspaceToolStatus,
} from './shared'
import type {
  ConfiguredModelRecord,
  CredentialBinding,
  DefaultSelection,
  ModelRegistryDiagnostics,
  ModelRegistryRecord,
  ProviderRegistryRecord,
} from './catalog'

export type WorkspaceMetricRecord = OpenApiWorkspaceMetricRecord
export type WorkspaceActivityRecord = OpenApiWorkspaceActivityRecord
export type ConversationRecord = OpenApiConversationRecord
export type WorkspaceOverviewSnapshot = OpenApiWorkspaceOverviewSnapshot
export type ProjectDashboardSnapshot = OpenApiProjectDashboardSnapshot
export type WorkspaceResourceRecord = OpenApiWorkspaceResourceRecord
export type CreateWorkspaceResourceInput = OpenApiCreateWorkspaceResourceInput
export type UpdateWorkspaceResourceInput = OpenApiUpdateWorkspaceResourceInput

export interface WorkspaceResourceUploadPayload {
  fileName: string
  contentType: string
  dataBase64: string
  byteSize: number
}

export type WorkspaceResourceFolderUploadEntry = OpenApiWorkspaceResourceFolderUploadEntry
export type CreateWorkspaceResourceFolderInput = OpenApiCreateWorkspaceResourceFolderInput
export type KnowledgeRecord = OpenApiKnowledgeRecord

export interface AgentRecord {
  id: string
  workspaceId: string
  projectId?: string
  scope: AgentScope
  name: string
  avatarPath?: string
  avatar?: string
  personality: string
  tags: string[]
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  integrationSource?: AgentIntegrationSource
  description: string
  status: AgentStatus
  updatedAt: number
}

export type TeamRecord = OpenApiTeamRecord

export interface WorkspaceLinkIntegrationSource {
  kind: 'workspace-link'
  sourceId: string
}

export type AgentIntegrationSource = WorkspaceLinkIntegrationSource
export type TeamIntegrationSource = WorkspaceLinkIntegrationSource

export interface UpsertAgentInput {
  workspaceId: string
  projectId?: string
  scope: AgentScope
  name: string
  avatar?: AvatarUploadPayload
  removeAvatar?: boolean
  personality: string
  tags: string[]
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  description: string
  status: AgentStatus
}

export type UpsertTeamInput = OpenApiUpsertTeamInput

export interface ProjectAgentLinkRecord {
  workspaceId: string
  projectId: string
  agentId: string
  linkedAt: number
}

export type ProjectTeamLinkRecord = OpenApiProjectTeamLinkRecord

export interface ProjectAgentLinkInput {
  projectId: string
  agentId: string
}

export type ProjectTeamLinkInput = OpenApiProjectTeamLinkInput

export interface ModelCatalogRecord {
  id: string
  workspaceId: string
  label: string
  provider: string
  description: string
  recommendedFor: string
  availability: ViewStatus
  defaultPermission: 'auto' | 'readonly' | 'danger-full-access'
}

export interface ProviderCredentialRecord {
  id: string
  workspaceId: string
  provider: string
  name: string
  baseUrl?: string
  status: 'healthy' | 'error' | 'unconfigured'
}

export interface ModelCatalogSnapshot {
  providers: ProviderRegistryRecord[]
  models: ModelRegistryRecord[]
  configuredModels: ConfiguredModelRecord[]
  credentialBindings: CredentialBinding[]
  defaultSelections: Record<string, DefaultSelection>
  diagnostics: ModelRegistryDiagnostics
}

export interface ToolRecord {
  id: string
  workspaceId: string
  kind: 'builtin' | 'skill' | 'mcp'
  name: string
  description: string
  status: WorkspaceToolStatus
  permissionMode: WorkspaceToolPermissionMode
  updatedAt: number
}

export interface AutomationRecord {
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

export type UserRecordSummary = OpenApiUserRecordSummary

export type CreateWorkspaceUserRequest = OpenApiCreateWorkspaceUserRequest

export type UpdateWorkspaceUserRequest = OpenApiUpdateWorkspaceUserRequest

export type UpdateCurrentUserProfileRequest = OpenApiUpdateCurrentUserProfileRequest

export type ChangeCurrentUserPasswordRequest = OpenApiChangeCurrentUserPasswordRequest

export type ChangeCurrentUserPasswordResponse = OpenApiChangeCurrentUserPasswordResponse

export type RoleRecord = OpenApiRoleRecord

export type PermissionRecord = OpenApiPermissionRecord

export type MenuRecord = OpenApiMenuRecord

export type UserCenterAlertRecord = OpenApiUserCenterAlertRecord

export type UserCenterOverviewSnapshot = OpenApiUserCenterOverviewSnapshot
