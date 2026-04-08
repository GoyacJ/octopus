import type {
  AvatarUploadPayload,
} from './auth'
import type {
  AgentScope,
  AgentStatus,
  AutomationStatus,
  KnowledgeKind,
  KnowledgeSourceType,
  KnowledgeStatus,
  MenuSource,
  MenuStatus,
  PasswordState,
  ProjectResourceKind,
  ProjectResourceOrigin,
  RbacPermissionKind,
  RbacPermissionStatus,
  RbacPermissionTargetType,
  RbacRoleStatus,
  RiskLevel,
  TeamScope,
  TeamStatus,
  UserStatus,
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
import type { ProjectRecord, WorkspaceSummary } from './workspace'

export interface WorkspaceMetricRecord {
  id: string
  label: string
  value: string
  helper?: string
  tone?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'accent'
}

export interface WorkspaceActivityRecord {
  id: string
  title: string
  description: string
  timestamp: number
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

export interface WorkspaceOverviewSnapshot {
  workspace: WorkspaceSummary
  metrics: WorkspaceMetricRecord[]
  projects: ProjectRecord[]
  recentConversations: ConversationRecord[]
  recentActivity: WorkspaceActivityRecord[]
}

export interface ProjectDashboardSnapshot {
  project: ProjectRecord
  metrics: WorkspaceMetricRecord[]
  recentConversations: ConversationRecord[]
  recentActivity: WorkspaceActivityRecord[]
}

export interface WorkspaceResourceRecord {
  id: string
  workspaceId: string
  projectId?: string
  kind: ProjectResourceKind
  name: string
  location?: string
  origin: ProjectResourceOrigin
  status: ViewStatus
  updatedAt: number
  tags: string[]
  sourceArtifactId?: string
}

export interface CreateWorkspaceResourceInput {
  projectId?: string
  kind: ProjectResourceKind
  name: string
  location?: string
  tags?: string[]
  sourceArtifactId?: string
}

export interface UpdateWorkspaceResourceInput {
  name?: string
  location?: string
  status?: ViewStatus
  tags?: string[]
}

export interface WorkspaceResourceUploadPayload {
  fileName: string
  contentType: string
  dataBase64: string
  byteSize: number
}

export interface WorkspaceResourceFolderUploadEntry extends WorkspaceResourceUploadPayload {
  relativePath: string
}

export interface CreateWorkspaceResourceFolderInput {
  projectId?: string
  files: WorkspaceResourceFolderUploadEntry[]
}

export interface KnowledgeRecord {
  id: string
  workspaceId: string
  projectId?: string
  title: string
  summary: string
  kind: KnowledgeKind
  status: KnowledgeStatus
  sourceType: KnowledgeSourceType
  sourceRef: string
  updatedAt: number
}

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

export interface TeamRecord {
  id: string
  workspaceId: string
  projectId?: string
  scope: TeamScope
  name: string
  avatarPath?: string
  avatar?: string
  personality: string
  tags: string[]
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  leaderAgentId?: string
  memberAgentIds: string[]
  integrationSource?: TeamIntegrationSource
  description: string
  status: TeamStatus
  updatedAt: number
}

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

export interface UpsertTeamInput {
  workspaceId: string
  projectId?: string
  scope: TeamScope
  name: string
  avatar?: AvatarUploadPayload
  removeAvatar?: boolean
  personality: string
  tags: string[]
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  leaderAgentId?: string
  memberAgentIds: string[]
  description: string
  status: TeamStatus
}

export interface ProjectAgentLinkRecord {
  workspaceId: string
  projectId: string
  agentId: string
  linkedAt: number
}

export interface ProjectTeamLinkRecord {
  workspaceId: string
  projectId: string
  teamId: string
  linkedAt: number
}

export interface ProjectAgentLinkInput {
  projectId: string
  agentId: string
}

export interface ProjectTeamLinkInput {
  projectId: string
  teamId: string
}

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

export interface UserRecordSummary {
  id: string
  username: string
  displayName: string
  avatar?: string
  status: UserStatus
  passwordState: PasswordState
  roleIds: string[]
  scopeProjectIds: string[]
}

export interface CreateWorkspaceUserRequest {
  username: string
  displayName: string
  status: UserStatus
  roleIds: string[]
  scopeProjectIds: string[]
  avatar?: AvatarUploadPayload
  useDefaultAvatar?: boolean
  password?: string
  confirmPassword?: string
  useDefaultPassword?: boolean
}

export interface UpdateWorkspaceUserRequest {
  username: string
  displayName: string
  status: UserStatus
  roleIds: string[]
  scopeProjectIds: string[]
  avatar?: AvatarUploadPayload
  removeAvatar?: boolean
  password?: string
  confirmPassword?: string
  resetPasswordToDefault?: boolean
}

export interface UpdateCurrentUserProfileRequest {
  username: string
  displayName: string
  avatar?: AvatarUploadPayload
  removeAvatar?: boolean
}

export interface ChangeCurrentUserPasswordRequest {
  currentPassword: string
  newPassword: string
  confirmPassword: string
}

export interface ChangeCurrentUserPasswordResponse {
  passwordState: PasswordState
}

export interface RoleRecord {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacRoleStatus
  permissionIds: string[]
  menuIds: string[]
}

export interface PermissionRecord {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacPermissionStatus
  kind: RbacPermissionKind
  targetType?: RbacPermissionTargetType
  targetIds: string[]
  action?: string
  memberPermissionIds: string[]
}

export interface MenuRecord {
  id: string
  workspaceId: string
  parentId?: string
  source: MenuSource
  label: string
  routeName?: string
  status: MenuStatus
  order: number
}

export interface UserCenterAlertRecord {
  id: string
  title: string
  description: string
  severity: RiskLevel
}

export interface UserCenterOverviewSnapshot {
  workspaceId: string
  currentUser: UserRecordSummary
  roleNames: string[]
  metrics: WorkspaceMetricRecord[]
  alerts: UserCenterAlertRecord[]
  quickLinks: MenuRecord[]
}
