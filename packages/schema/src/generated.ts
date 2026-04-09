/* eslint-disable */
// Generated from contracts/openapi/octopus.openapi.yaml by scripts/generate-schema.mjs.
// Source hash: d6e8f193e6097fa5776b7e651c467bf6e637e95112aa2a5d5381f742ec19ca27

export const OCTOPUS_OPENAPI_VERSION = "3.1.0"
export const OCTOPUS_API_VERSION = "0.2.4"
export const OCTOPUS_OPENAPI_SOURCE_HASH = "d6e8f193e6097fa5776b7e651c467bf6e637e95112aa2a5d5381f742ec19ca27"

export interface AgentRecord {
  avatar?: string
  avatarPath?: string
  builtinToolKeys: string[]
  description: string
  id: string
  integrationSource?: {
  kind: "workspace-link"
  sourceId: string
}
  mcpServerNames: string[]
  name: string
  personality: string
  projectId?: string
  prompt: string
  scope: AgentScope
  skillIds: string[]
  status: AgentStatus
  tags: string[]
  updatedAt: number
  workspaceId: string
}

export type AgentScope = "personal" | "workspace" | "project"

export type AgentStatus = "active" | "archived"

export type ApiErrorCode = "UNAUTHENTICATED" | "SESSION_EXPIRED" | "FORBIDDEN" | "NOT_FOUND" | "CONFLICT" | "INVALID_INPUT" | "RATE_LIMITED" | "UNAVAILABLE" | "CAPABILITY_UNSUPPORTED" | "INTERNAL_ERROR"

export interface ApiErrorDetailEnvelope {
  code: ApiErrorCode
  details?: unknown
  message: string
  requestId: string
  retryable: boolean
}

export interface ApiErrorEnvelope {
  error: ApiErrorDetailEnvelope
}

export interface ArtifactRecord {
  byteSize?: number
  contentHash?: string
  contentType?: string
  id: string
  latestVersion: number
  projectId?: string
  status: ArtifactStatus
  storagePath?: string
  title: string
  updatedAt: number
  workspaceId: string
}

export type ArtifactStatus = "draft" | "review" | "approved" | "published"

export interface AuditRecord {
  action: string
  actorId: string
  actorType: string
  createdAt: number
  id: string
  outcome: string
  projectId?: string
  resource: string
  workspaceId: string
}

export interface AutomationRecord {
  cadence: string
  description: string
  id: string
  lastRunAt?: number
  nextRunAt?: number
  output: string
  ownerId: string
  ownerType: "agent" | "team"
  projectId?: string
  status: AutomationStatus
  title: string
  workspaceId: string
}

export type AutomationStatus = "active" | "paused" | "error"

export interface AvatarUploadPayload {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
}

export type BackendConnectionState = "ready" | "unavailable"

export type BackendTransport = "http" | "sse" | "ws"

export interface BindPetConversationInput {
  conversationId: string
  petId: string
  sessionId?: string
}

export interface CapabilityDescriptor {
  capabilityId: string
  label: string
}

export interface ChangeCurrentUserPasswordRequest {
  confirmPassword: string
  currentPassword: string
  newPassword: string
}

export interface ChangeCurrentUserPasswordResponse {
  passwordState: PasswordState
}

export interface ClientAppRecord {
  allowedHosts: string[]
  allowedOrigins: string[]
  defaultScopes: string[]
  firstParty: boolean
  id: string
  name: string
  platform: string
  sessionPolicy: string
  status: string
}

export type ConnectionMode = "local" | "shared" | "remote"

export interface ConnectionProfile {
  baseUrl?: string
  id: string
  label: string
  lastSyncAt?: number
  mode: ConnectionMode
  state: ConnectionState
  workspaceId: string
}

export type ConnectionState = "local-ready" | "connected" | "disconnected"

export type ConversationActorKind = "agent" | "team"

export interface ConversationRecord {
  id: string
  lastMessagePreview?: string
  projectId: string
  sessionId: string
  status: string
  title: string
  updatedAt: number
  workspaceId: string
}

export interface CopyWorkspaceSkillToManagedInput {
  slug: string
}

export interface CreateHostWorkspaceConnectionInput {
  authMode: WorkspaceAuthMode
  baseUrl: string
  label: string
  transportSecurity: TransportSecurityLevel
  workspaceId: string
}

export interface CreateNotificationInput {
  actionLabel?: string
  body?: string
  level?: NotificationLevel
  routeTo?: string
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  source?: string
  title?: string
  toastDurationMs?: number
}

export interface CreateProjectRequest {
  assignments?: ProjectWorkspaceAssignments
  description: string
  name: string
}

export interface CreateRuntimeSessionInput {
  conversationId: string
  projectId: string
  sessionKind?: RuntimeSessionKind
  title: string
}

export interface CreateWorkspaceResourceFolderInput {
  files: WorkspaceResourceFolderUploadEntry[]
  projectId?: string
}

export interface CreateWorkspaceResourceInput {
  kind: ProjectResourceKind
  location?: string
  name: string
  projectId?: string
  sourceArtifactId?: string
  tags?: string[]
}

export interface CreateWorkspaceSkillInput {
  content: string
  slug: string
}

export interface CreateWorkspaceUserRequest {
  avatar?: AvatarUploadPayload
  confirmPassword?: string
  displayName: string
  password?: string
  roleIds: string[]
  scopeProjectIds: string[]
  status: UserStatus
  useDefaultAvatar?: boolean
  useDefaultPassword?: boolean
  username: string
}

export interface CredentialBinding {
  baseUrl?: string
  configured: boolean
  credentialRef: string
  label: string
  providerId: string
  source: string
  status: "healthy" | "error" | "unconfigured" | "configured"
}

export type DecisionAction = "approve" | "reject"

export interface HealthcheckBackendStatus {
  state: BackendConnectionState
  transport: BackendTransport
}

export interface HealthcheckStatus {
  backend: HealthcheckBackendStatus
  cargoWorkspace: boolean
  host: HostPlatform
  mode: HostExecutionMode
  status: "ok"
}

export interface HostBackendConnection {
  authToken?: string
  baseUrl?: string
  state: BackendConnectionState
  transport: BackendTransport
}

export type HostExecutionMode = "local" | "remote"

export type HostPlatform = "tauri" | "web"

export interface HostReleaseSummary {
  channel: HostUpdateChannel
  notes?: string
  notesUrl?: string
  publishedAt: string
  version: string
}

export interface HostState {
  appVersion: string
  cargoWorkspace: boolean
  mode: HostExecutionMode
  platform: HostPlatform
  shell: string
}

export interface HostUpdateCapabilities {
  canCheck: boolean
  canDownload: boolean
  canInstall: boolean
  supportsChannels: boolean
}

export type HostUpdateChannel = "formal" | "preview"

export interface HostUpdateCheckRequest {
  channel?: HostUpdateChannel
}

export interface HostUpdateProgress {
  downloadedBytes: number
  percent: number
  totalBytes: number
}

export type HostUpdateState = "idle" | "checking" | "up_to_date" | "update_available" | "downloading" | "downloaded" | "installing" | "error"

export interface HostUpdateStatus {
  capabilities: HostUpdateCapabilities
  currentChannel: HostUpdateChannel
  currentVersion: string
  errorCode?: string
  errorMessage?: string
  lastCheckedAt?: number
  latestRelease?: HostReleaseSummary
  progress?: HostUpdateProgress
  state: HostUpdateState
}

export interface ImportedAgentPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  agentId?: string
  department: string
  name: string
  skillSlugs: string[]
  sourceId: string
}

export interface ImportedSkillPreviewItem {
  action: "create" | "update" | "skip" | "failed"
  agentNames: string[]
  contentHash: string
  departments: string[]
  fileCount: number
  name: string
  skillId: string
  slug: string
  sourceIds: string[]
}

export interface ImportIssue {
  message: string
  scope: "bundle" | "agent" | "skill"
  severity: "warning" | "error"
  sourceId?: string
}

export interface ImportWorkspaceAgentBundleInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportWorkspaceAgentBundlePreview {
  agents: ImportedAgentPreviewItem[]
  createCount: number
  departmentCount: number
  departments: string[]
  detectedAgentCount: number
  failureCount: number
  filteredFileCount: number
  importableAgentCount: number
  issues: ImportIssue[]
  skills: ImportedSkillPreviewItem[]
  skipCount: number
  uniqueSkillCount: number
  updateCount: number
}

export interface ImportWorkspaceAgentBundlePreviewInput {
  files: WorkspaceDirectoryUploadEntry[]
}

export interface ImportWorkspaceAgentBundleResult {
  agents: ImportedAgentPreviewItem[]
  createCount: number
  departmentCount: number
  departments: string[]
  detectedAgentCount: number
  failureCount: number
  filteredFileCount: number
  importableAgentCount: number
  issues: ImportIssue[]
  skills: ImportedSkillPreviewItem[]
  skipCount: number
  uniqueSkillCount: number
  updateCount: number
}

export interface ImportWorkspaceSkillArchiveInput {
  archive: WorkspaceFileUploadPayload
  slug: string
}

export interface ImportWorkspaceSkillFolderInput {
  files: WorkspaceDirectoryUploadEntry[]
  slug: string
}

export interface InboxItemRecord {
  createdAt: number
  description: string
  id: string
  itemType: string
  priority: string
  projectId?: string
  status: string
  title: string
  workspaceId: string
}

export interface KnowledgeEntryRecord {
  id: string
  projectId?: string
  scope: string
  sourceRef: string
  sourceType: string
  status: string
  title: string
  updatedAt: number
  workspaceId: string
}

export type KnowledgeKind = "private" | "shared" | "candidate"

export interface KnowledgeRecord {
  id: string
  kind: KnowledgeKind
  projectId?: string
  sourceRef: string
  sourceType: KnowledgeSourceType
  status: KnowledgeStatus
  summary: string
  title: string
  updatedAt: number
  workspaceId: string
}

export type KnowledgeSourceType = "conversation" | "artifact" | "run"

export type KnowledgeStatus = "candidate" | "reviewed" | "shared" | "archived"

export type Locale = "zh-CN" | "en-US"

export interface LoginRequest {
  clientAppId: string
  password: string
  username: string
  workspaceId?: string
}

export interface LoginResponse {
  session: SessionRecord
  workspace: WorkspaceSummary
}

export interface MenuRecord {
  id: string
  label: string
  order: number
  parentId?: string
  routeName?: string
  source: MenuSource
  status: MenuStatus
  workspaceId: string
}

export type MenuSource = "main-sidebar" | "user-center"

export type MenuStatus = "active" | "disabled"

export interface MessageProcessEntry {
  detail: string
  id: string
  timestamp: number
  title: string
  toolId?: string
  type: "thinking" | "execution" | "tool" | "result"
}

export interface MessageToolCall {
  count: number
  kind: ToolCatalogKind
  label: string
  toolId: string
}

export interface MessageUsage {
  inputTokens: number
  outputTokens: number
  totalTokens: number
}

export interface ModelCatalogSnapshot {
  configuredModels?: Record<string, unknown>[]
  credentialBindings?: Record<string, unknown>[]
  defaultSelections?: Record<string, Record<string, unknown>>
  diagnostics?: Record<string, unknown>
  models?: Record<string, unknown>[]
  providers?: Record<string, unknown>[]
}

export interface NotificationFilter {
  scope?: NotificationFilterScope
}

export type NotificationFilterScope = "all" | "app" | "workspace" | "user"

export type NotificationLevel = "info" | "success" | "warning" | "error"

export interface NotificationListResponse {
  notifications: NotificationRecord[]
  unread: NotificationUnreadSummary
}

export interface NotificationRecord {
  actionLabel?: string
  body: string
  createdAt: number
  id: string
  level: NotificationLevel
  readAt?: number
  routeTo?: string
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  source: string
  title: string
  toastVisibleUntil?: number
}

export type NotificationScopeKind = "app" | "workspace" | "user"

export interface NotificationUnreadScopeSummary {
  app: number
  user: number
  workspace: number
}

export interface NotificationUnreadSummary {
  byScope: NotificationUnreadScopeSummary
  total: number
}

export type PasswordState = "set" | "reset-required" | "temporary"

export type PermissionMode = "auto" | "readonly" | "danger-full-access"

export interface PermissionRecord {
  action?: string
  code: string
  description: string
  id: string
  kind: RbacPermissionKind
  memberPermissionIds: string[]
  name: string
  status: RbacPermissionStatus
  targetIds: string[]
  targetType?: RbacPermissionTargetType
  workspaceId: string
}

export type PetChatSender = "user" | "pet"

export interface PetConversationBinding {
  conversationId: string
  petId: string
  projectId: string
  sessionId?: string
  updatedAt: number
  workspaceId: string
}

export interface PetMessage {
  content: string
  id: string
  petId: string
  sender: PetChatSender
  timestamp: number
}

export type PetMood = "curious" | "happy" | "sleepy" | "playful" | "focused"

export type PetMotionState = "idle" | "hover" | "walk" | "chat" | "sleep"

export interface PetPosition {
  x: number
  y: number
}

export interface PetPresenceState {
  chatOpen: boolean
  isVisible: boolean
  lastInteractionAt: number
  motionState: PetMotionState
  petId: string
  position: PetPosition
  unreadCount: number
}

export interface PetProfile {
  avatarLabel: string
  displayName: string
  fallbackAsset: string
  favoriteSnack: string
  greeting: string
  id: string
  mood: PetMood
  ownerUserId: string
  promptHints: string[]
  riveAsset?: string
  species: PetSpecies
  stateMachine?: string
  summary: string
}

export type PetSpecies = "duck" | "goose" | "blob" | "cat" | "dragon" | "octopus" | "owl" | "penguin" | "turtle" | "snail" | "ghost" | "axolotl" | "capybara" | "cactus" | "robot" | "rabbit" | "mushroom" | "chonk"

export interface PetWorkspaceSnapshot {
  binding?: PetConversationBinding
  messages: PetMessage[]
  presence: PetPresenceState
  profile: PetProfile
}

export interface ProjectAgentAssignments {
  agentIds: string[]
  teamIds: string[]
}

export interface ProjectAgentLinkInput {
  agentId: string
  projectId: string
}

export interface ProjectAgentLinkRecord {
  agentId: string
  linkedAt: number
  projectId: string
  workspaceId: string
}

export interface ProjectDashboardSnapshot {
  metrics: WorkspaceMetricRecord[]
  project: ProjectRecord
  recentActivity: WorkspaceActivityRecord[]
  recentConversations: ConversationRecord[]
}

export interface ProjectModelAssignments {
  configuredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectRecord {
  assignments?: ProjectWorkspaceAssignments
  description: string
  id: string
  name: string
  status: "active" | "archived"
  workspaceId: string
}

export type ProjectResourceKind = "file" | "folder" | "artifact" | "url"

export type ProjectResourceOrigin = "source" | "generated"

export interface ProjectTeamLinkInput {
  projectId: string
  teamId: string
}

export interface ProjectTeamLinkRecord {
  linkedAt: number
  projectId: string
  teamId: string
  workspaceId: string
}

export interface ProjectToolAssignments {
  sourceKeys: string[]
}

export interface ProjectWorkspaceAssignments {
  agents?: ProjectAgentAssignments
  models?: ProjectModelAssignments
  tools?: ProjectToolAssignments
}

export interface ProviderConfig {
  baseUrl?: string
  credentialRef?: string
  defaultModel?: string
  defaultSurface?: string
  protocolFamily?: string
  providerId: string
}

export type RbacPermissionKind = "atomic" | "bundle"

export type RbacPermissionStatus = "active" | "disabled"

export type RbacPermissionTargetType = "workspace" | "project" | "user" | "role" | "permission" | "menu" | "resource" | "agent" | "knowledge" | "tool" | "skill" | "mcp"

export type RbacRoleStatus = "active" | "disabled"

export interface RegisterWorkspaceOwnerRequest {
  avatar: AvatarUploadPayload
  clientAppId: string
  confirmPassword: string
  displayName: string
  password: string
  username: string
  workspaceId?: string
}

export interface RegisterWorkspaceOwnerResponse {
  session: SessionRecord
  workspace: WorkspaceSummary
}

export interface ResolvedExecutionTarget {
  baseUrl?: string
  capabilities: CapabilityDescriptor[]
  configuredModelId: string
  configuredModelName: string
  credentialRef?: string
  modelId: string
  protocolFamily: string
  providerId: string
  registryModelId: string
  surface: string
}

export interface ResolveRuntimeApprovalInput {
  decision: DecisionAction
}

export type RiskLevel = "low" | "medium" | "high"

export interface RoleRecord {
  code: string
  description: string
  id: string
  menuIds: string[]
  name: string
  permissionIds: string[]
  status: RbacRoleStatus
  workspaceId: string
}

export type RunStatus = "idle" | "draft" | "planned" | "running" | "waiting_input" | "waiting_approval" | "blocked" | "paused" | "completed" | "failed" | "terminated"

export type RuntimeActorType = "user" | "assistant" | "system"

export interface RuntimeApprovalRequest {
  conversationId: string
  createdAt: number
  detail: string
  id: string
  riskLevel: RiskLevel
  runId: string
  sessionId: string
  status: "pending" | "approved" | "rejected"
  summary: string
  toolName: string
}

export interface RuntimeBootstrap {
  provider: ProviderConfig
  sessions: RuntimeSessionSummary[]
}

export interface RuntimeConfigPatch {
  patch: Record<string, unknown>
  scope: RuntimeConfigScope
}

export type RuntimeConfigScope = "workspace" | "project" | "user"

export interface RuntimeConfigSource {
  contentHash?: string
  displayPath: string
  document?: Record<string, unknown>
  exists: boolean
  loaded: boolean
  ownerId?: string
  scope: RuntimeConfigScope
  sourceKey: string
}

export interface RuntimeConfiguredModelProbeInput {
  configuredModelId: string
  patch: Record<string, unknown>
  scope: RuntimeConfigScope
}

export interface RuntimeConfiguredModelProbeResult {
  configuredModelId: string
  configuredModelName?: string
  consumedTokens?: number
  errors: string[]
  reachable: boolean
  requestId?: string
  valid: boolean
  warnings: string[]
}

export interface RuntimeConfigValidationResult {
  errors: string[]
  valid: boolean
  warnings: string[]
}

export interface RuntimeEffectiveConfig {
  effectiveConfig: Record<string, unknown>
  effectiveConfigHash: string
  secretReferences: RuntimeSecretReferenceStatus[]
  sources: RuntimeConfigSource[]
  validation: RuntimeConfigValidationResult
}

export interface RuntimeEventEnvelope {
  approval?: RuntimeApprovalRequest
  conversationId: string
  decision?: DecisionAction
  emittedAt: number
  error?: string
  eventType: RuntimeEventKind
  id: string
  kind?: RuntimeEventKind
  message?: RuntimeMessage
  payload?: Record<string, unknown>
  projectId?: string
  run?: RuntimeRunSnapshot
  runId?: string
  sequence: number
  sessionId: string
  summary?: RuntimeSessionSummary
  trace?: RuntimeTraceItem
  workspaceId: string
}

export type RuntimeEventKind = "runtime.run.updated" | "runtime.message.created" | "runtime.trace.emitted" | "runtime.approval.requested" | "runtime.approval.resolved" | "runtime.session.updated" | "runtime.error"

export interface RuntimeMessage {
  artifacts?: string[]
  attachments?: string[]
  configuredModelId?: string
  configuredModelName?: string
  content: string
  conversationId: string
  id: string
  modelId?: string
  processEntries?: MessageProcessEntry[]
  requestedActorId?: string
  requestedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorLabel?: string
  resourceIds?: string[]
  senderLabel: string
  senderType: RuntimeActorType
  sessionId: string
  status: RunStatus
  timestamp: number
  toolCalls?: MessageToolCall[]
  usage?: MessageUsage
  usedDefaultActor?: boolean
}

export type RuntimePermissionMode = "read-only" | "workspace-write" | "danger-full-access"

export interface RuntimeRunSnapshot {
  configSnapshotId: string
  configuredModelId?: string
  configuredModelName?: string
  consumedTokens?: number
  conversationId: string
  currentStep: string
  effectiveConfigHash: string
  id: string
  modelId?: string
  nextAction?: string
  requestedActorId?: string
  requestedActorKind?: ConversationActorKind
  resolvedActorId?: string
  resolvedActorKind?: ConversationActorKind
  resolvedActorLabel?: string
  resolvedTarget?: ResolvedExecutionTarget
  sessionId: string
  startedAt: number
  startedFromScopeSet: RuntimeConfigScope[]
  status: RunStatus
  updatedAt: number
}

export type RuntimeSecretReferenceState = "reference-present" | "reference-missing" | "inline-redacted"

export interface RuntimeSecretReferenceStatus {
  path: string
  reference?: string
  scope: RuntimeConfigScope
  status: RuntimeSecretReferenceState
}

export interface RuntimeSessionDetail {
  messages: RuntimeMessage[]
  pendingApproval?: RuntimeApprovalRequest
  run: RuntimeRunSnapshot
  summary: RuntimeSessionSummary
  trace: RuntimeTraceItem[]
}

export type RuntimeSessionKind = "project" | "pet"

export interface RuntimeSessionSummary {
  configSnapshotId: string
  conversationId: string
  effectiveConfigHash: string
  id: string
  lastMessagePreview?: string
  projectId: string
  sessionKind: RuntimeSessionKind
  startedFromScopeSet: RuntimeConfigScope[]
  status: RunStatus
  title: string
  updatedAt: number
}

export interface RuntimeTraceItem {
  actor: string
  actorId?: string
  actorKind?: ConversationActorKind
  conversationId: string
  detail: string
  id: string
  kind: TraceKind
  relatedMessageId?: string
  relatedToolName?: string
  runId: string
  sessionId: string
  timestamp: number
  title: string
  tone: TraceTone
}

export interface SavePetPresenceInput {
  chatOpen?: boolean
  isVisible?: boolean
  lastInteractionAt?: number
  motionState?: PetMotionState
  petId: string
  position?: PetPosition
  unreadCount?: number
}

export interface SessionRecord {
  clientAppId: string
  createdAt: number
  expiresAt?: number
  id: string
  roleIds: string[]
  scopeProjectIds: string[]
  status: SessionStatus
  token: string
  userId: string
  workspaceId: string
}

export type SessionStatus = "active" | "revoked" | "expired"

export interface ShellBootstrap {
  backend?: HostBackendConnection
  connections: ConnectionProfile[]
  hostState: HostState
  preferences: ShellPreferences
  workspaceConnections?: WorkspaceConnectionRecord[]
  workspaceSessions?: WorkspaceSessionTokenEnvelope[]
}

export interface ShellPreferences {
  compactSidebar: boolean
  defaultWorkspaceId: string
  fontFamily: string
  fontSize: number
  fontStyle: string
  lastVisitedRoute: string
  leftSidebarCollapsed: boolean
  locale: Locale
  rightSidebarCollapsed: boolean
  theme: ThemeMode
  updateChannel: HostUpdateChannel
}

export interface SubmitRuntimeTurnInput {
  actorId?: string
  actorKind?: ConversationActorKind
  configuredModelId?: string
  content: string
  modelId?: string
  permissionMode: RuntimePermissionMode
}

export interface SystemBootstrapStatus {
  apiBasePath: string
  authMode: WorkspaceAuthMode
  capabilities: WorkspaceCapabilitySet
  ownerReady: boolean
  protocolVersion: string
  registeredApps: ClientAppRecord[]
  setupRequired: boolean
  transportSecurity: TransportSecurityLevel
  workspace: WorkspaceSummary
}

export interface TeamRecord {
  avatar?: string
  avatarPath?: string
  builtinToolKeys: string[]
  description: string
  id: string
  integrationSource?: {
  kind: "workspace-link"
  sourceId: string
}
  leaderAgentId?: string
  mcpServerNames: string[]
  memberAgentIds: string[]
  name: string
  personality: string
  projectId?: string
  prompt: string
  scope: TeamScope
  skillIds: string[]
  status: TeamStatus
  tags: string[]
  updatedAt: number
  workspaceId: string
}

export type TeamScope = "workspace" | "project"

export type TeamStatus = "active" | "archived"

export type ThemeMode = "light" | "dark" | "system"

export type ToolCatalogKind = "builtin" | "skill" | "mcp"

export interface ToolRecord {
  description: string
  id: string
  kind: ToolCatalogKind
  name: string
  permissionMode: WorkspaceToolPermissionMode
  status: WorkspaceToolStatus
  updatedAt: number
  workspaceId: string
}

export type TraceKind = "step" | "tool" | "approval" | "pause" | "resume" | "artifact" | "knowledge"

export type TraceTone = "info" | "success" | "warning" | "error" | "default"

export type TransportSecurityLevel = "loopback" | "trusted" | "public"

export interface UpdateCurrentUserProfileRequest {
  avatar?: AvatarUploadPayload
  displayName: string
  removeAvatar?: boolean
  username: string
}

export interface UpdateProjectRequest {
  assignments?: ProjectWorkspaceAssignments
  description: string
  name: string
  status: "active" | "archived"
}

export interface UpdateWorkspaceResourceInput {
  location?: string
  name?: string
  status?: ViewStatus
  tags?: string[]
}

export interface UpdateWorkspaceSkillFileInput {
  content: string
}

export interface UpdateWorkspaceSkillInput {
  content: string
}

export interface UpdateWorkspaceUserRequest {
  avatar?: AvatarUploadPayload
  confirmPassword?: string
  displayName: string
  password?: string
  removeAvatar?: boolean
  resetPasswordToDefault?: boolean
  roleIds: string[]
  scopeProjectIds: string[]
  status: UserStatus
  username: string
}

export interface UpsertAgentInput {
  avatar?: AvatarUploadPayload
  builtinToolKeys: string[]
  description: string
  mcpServerNames: string[]
  name: string
  personality: string
  projectId?: string
  prompt: string
  removeAvatar?: boolean
  scope: AgentScope
  skillIds: string[]
  status: AgentStatus
  tags: string[]
  workspaceId: string
}

export interface UpsertTeamInput {
  avatar?: AvatarUploadPayload
  builtinToolKeys: string[]
  description: string
  leaderAgentId?: string
  mcpServerNames: string[]
  memberAgentIds: string[]
  name: string
  personality: string
  projectId?: string
  prompt: string
  removeAvatar?: boolean
  scope: TeamScope
  skillIds: string[]
  status: TeamStatus
  tags: string[]
  workspaceId: string
}

export interface UpsertWorkspaceMcpServerInput {
  config: Record<string, unknown>
  serverName: string
}

export interface UserCenterAlertRecord {
  description: string
  id: string
  severity: RiskLevel
  title: string
}

export interface UserCenterOverviewSnapshot {
  alerts: UserCenterAlertRecord[]
  currentUser: UserRecordSummary
  metrics: WorkspaceMetricRecord[]
  quickLinks: MenuRecord[]
  roleNames: string[]
  workspaceId: string
}

export interface UserRecordSummary {
  avatar?: string
  displayName: string
  id: string
  passwordState: PasswordState
  roleIds: string[]
  scopeProjectIds: string[]
  status: UserStatus
  username: string
}

export type UserStatus = "active" | "disabled"

export type ViewStatus = "healthy" | "configured" | "attention"

export interface WorkspaceActivityRecord {
  description: string
  id: string
  timestamp: number
  title: string
}

export type WorkspaceAuthMode = "session-token"

export interface WorkspaceBuiltinToolCatalogEntry {
  availability: ViewStatus
  builtinKey: string
  description: string
  disabled: boolean
  displayPath: string
  id: string
  kind: "builtin"
  management: WorkspaceToolManagementCapabilities
  name: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  sourceKey: string
  workspaceId: string
}

export interface WorkspaceCapabilitySet {
  eventReplay: boolean
  idempotency: boolean
  polling: boolean
  reconnect: boolean
  sse: boolean
}

export interface WorkspaceConnectionRecord {
  authMode: WorkspaceAuthMode
  baseUrl: string
  label: string
  lastUsedAt?: number
  status: WorkspaceConnectionStatus
  transportSecurity: TransportSecurityLevel
  workspaceConnectionId: string
  workspaceId: string
}

export type WorkspaceConnectionStatus = "connected" | "disconnected" | "expired" | "unreachable"

export type WorkspaceDirectoryUploadEntry = WorkspaceFileUploadPayload & {
  relativePath: string
}

export interface WorkspaceFileUploadPayload {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
}

export interface WorkspaceMcpServerDocument {
  config: Record<string, unknown>
  displayPath: string
  scope: "workspace" | "project" | "user"
  serverName: string
  sourceKey: string
}

export interface WorkspaceMcpToolCatalogEntry {
  availability: ViewStatus
  description: string
  disabled: boolean
  displayPath: string
  endpoint: string
  id: string
  kind: "mcp"
  management: WorkspaceToolManagementCapabilities
  name: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  scope: "workspace" | "project" | "user"
  serverName: string
  sourceKey: string
  statusDetail?: string
  toolNames: string[]
  workspaceId: string
}

export interface WorkspaceMetricRecord {
  helper?: string
  id: string
  label: string
  tone?: WorkspaceMetricTone
  value: string
}

export type WorkspaceMetricTone = "default" | "success" | "warning" | "error" | "info" | "accent"

export interface WorkspaceOverviewSnapshot {
  metrics: WorkspaceMetricRecord[]
  projects: ProjectRecord[]
  recentActivity: WorkspaceActivityRecord[]
  recentConversations: ConversationRecord[]
  workspace: WorkspaceSummary
}

export interface WorkspaceResourceFolderUploadEntry {
  byteSize: number
  contentType: string
  dataBase64: string
  fileName: string
  relativePath: string
}

export interface WorkspaceResourceRecord {
  id: string
  kind: ProjectResourceKind
  location?: string
  name: string
  origin: ProjectResourceOrigin
  projectId?: string
  sourceArtifactId?: string
  status: ViewStatus
  tags: string[]
  updatedAt: number
  workspaceId: string
}

export interface WorkspaceSessionTokenEnvelope {
  expiresAt?: number
  issuedAt: number
  session: SessionRecord
  token: string
  workspaceConnectionId: string
}

export interface WorkspaceSkillDocument {
  content: string
  description: string
  displayPath: string
  id: string
  name: string
  relativePath?: string
  rootPath: string
  sourceKey: string
  sourceOrigin: "skills_dir" | "legacy_commands_dir"
  tree: WorkspaceSkillTreeNode[]
  workspaceOwned: boolean
}

export interface WorkspaceSkillFileDocument {
  byteSize: number
  content?: string | null
  contentType?: string
  displayPath: string
  isText: boolean
  language?: string
  path: string
  readonly: boolean
  skillId: string
  sourceKey: string
}

export interface WorkspaceSkillToolCatalogEntry {
  active: boolean
  availability: ViewStatus
  description: string
  disabled: boolean
  displayPath: string
  id: string
  kind: "skill"
  management: WorkspaceToolManagementCapabilities
  name: string
  relativePath?: string
  requiredPermission?: "readonly" | "workspace-write" | "danger-full-access" | "null"
  shadowedBy?: string
  sourceKey: string
  sourceOrigin: "skills_dir" | "legacy_commands_dir"
  workspaceId: string
  workspaceOwned: boolean
}

export interface WorkspaceSkillTreeDocument {
  displayPath: string
  rootPath: string
  skillId: string
  sourceKey: string
  tree: WorkspaceSkillTreeNode[]
}

export interface WorkspaceSkillTreeNode {
  byteSize?: number
  children?: WorkspaceSkillTreeNode[]
  isText?: boolean
  kind: "directory" | "file"
  name: string
  path: string
}

export interface WorkspaceSummary {
  bootstrapStatus: "setup_required" | "ready"
  defaultProjectId: string
  deployment: "local" | "remote"
  host: string
  id: string
  listenAddress: string
  name: string
  ownerUserId?: string
  slug: string
}

export type WorkspaceToolCatalogEntry = WorkspaceBuiltinToolCatalogEntry | WorkspaceSkillToolCatalogEntry | WorkspaceMcpToolCatalogEntry

export interface WorkspaceToolCatalogSnapshot {
  entries: WorkspaceToolCatalogEntry[]
}

export interface WorkspaceToolDisablePatch {
  disabled: boolean
  sourceKey: string
}

export interface WorkspaceToolManagementCapabilities {
  canDelete: boolean
  canDisable: boolean
  canEdit: boolean
}

export type WorkspaceToolPermissionMode = "allow" | "deny" | "ask" | "readonly"

export type WorkspaceToolStatus = "active" | "disabled"


export interface OctopusApiPaths {
  "/api/v1/apps": {
    get: { operationId: "listClientApps"; response: ClientAppRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "registerClientApp"; response: ClientAppRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/artifacts": {
    get: { operationId: "listArtifacts"; response: ArtifactRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/audit": {
    get: { operationId: "listAuditRecords"; response: AuditRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/auth/login": {
    post: { operationId: "login"; response: LoginResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/auth/logout": {
    post: { operationId: "logout"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/auth/register-owner": {
    post: { operationId: "registerOwner"; response: RegisterWorkspaceOwnerResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/auth/session": {
    get: { operationId: "getCurrentSession"; response: SessionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/bootstrap": {
    get: { operationId: "getHostBootstrap"; response: ShellBootstrap; error: ApiErrorEnvelope }
  }
  "/api/v1/host/health": {
    get: { operationId: "getHostHealthcheck"; response: HealthcheckStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications": {
    get: { operationId: "listHostNotifications"; response: NotificationListResponse; error: ApiErrorEnvelope }
    post: { operationId: "createHostNotification"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/{notificationId}/dismiss-toast": {
    post: { operationId: "dismissHostNotificationToast"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/{notificationId}/read": {
    post: { operationId: "markHostNotificationRead"; response: NotificationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/read-all": {
    post: { operationId: "markAllHostNotificationsRead"; response: NotificationUnreadSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/host/notifications/unread-summary": {
    get: { operationId: "getHostNotificationUnreadSummary"; response: NotificationUnreadSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/host/preferences": {
    get: { operationId: "getHostPreferences"; response: ShellPreferences; error: ApiErrorEnvelope }
    put: { operationId: "saveHostPreferences"; response: ShellPreferences; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-check": {
    post: { operationId: "checkHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-download": {
    post: { operationId: "downloadHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-install": {
    post: { operationId: "installHostUpdate"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/update-status": {
    get: { operationId: "getHostUpdateStatus"; response: HostUpdateStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/host/workspace-connections": {
    get: { operationId: "listHostWorkspaceConnections"; response: WorkspaceConnectionRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createHostWorkspaceConnection"; response: WorkspaceConnectionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/host/workspace-connections/{connectionId}": {
    delete: { operationId: "deleteHostWorkspaceConnection"; response: null; error: ApiErrorEnvelope }
  }
  "/api/v1/inbox": {
    get: { operationId: "listInboxItems"; response: InboxItemRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/knowledge": {
    get: { operationId: "listKnowledgeEntries"; response: KnowledgeEntryRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects": {
    get: { operationId: "listProjects"; response: ProjectRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProject"; response: ProjectRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}": {
    patch: { operationId: "updateProject"; response: ProjectRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agent-links": {
    get: { operationId: "listProjectAgentLinks"; response: ProjectAgentLinkRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectAgentLink"; response: ProjectAgentLinkRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/agent-links/{agentId}": {
    delete: { operationId: "deleteProjectAgentLink"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/dashboard": {
    get: { operationId: "getProjectDashboard"; response: ProjectDashboardSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/knowledge": {
    get: { operationId: "listProjectKnowledge"; response: KnowledgeRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet": {
    get: { operationId: "getProjectPetSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet/conversation": {
    put: { operationId: "bindProjectPetConversation"; response: PetConversationBinding; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/pet/presence": {
    patch: { operationId: "saveProjectPetPresence"; response: PetPresenceState; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources": {
    get: { operationId: "listProjectResources"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources/{resourceId}": {
    patch: { operationId: "updateProjectResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteProjectResource"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/resources/folder": {
    post: { operationId: "createProjectResourceFolder"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/runtime-config": {
    get: { operationId: "getProjectRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
    patch: { operationId: "saveProjectRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/runtime-config/validate": {
    post: { operationId: "validateProjectRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/team-links": {
    get: { operationId: "listProjectTeamLinks"; response: ProjectTeamLinkRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createProjectTeamLink"; response: ProjectTeamLinkRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/projects/{projectId}/team-links/{teamId}": {
    delete: { operationId: "deleteProjectTeamLink"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/bootstrap": {
    get: { operationId: "getRuntimeBootstrap"; response: RuntimeBootstrap; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config": {
    get: { operationId: "getRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/configured-models/probe": {
    post: { operationId: "probeRuntimeConfiguredModel"; response: RuntimeConfiguredModelProbeResult; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/scopes/{scope}": {
    patch: { operationId: "saveRuntimeConfigScope"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/config/validate": {
    post: { operationId: "validateRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions": {
    get: { operationId: "listRuntimeSessions"; response: RuntimeSessionSummary[]; error: ApiErrorEnvelope }
    post: { operationId: "createRuntimeSession"; response: RuntimeSessionDetail; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}": {
    get: { operationId: "getRuntimeSession"; response: RuntimeSessionDetail; error: ApiErrorEnvelope }
    delete: { operationId: "deleteRuntimeSession"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}": {
    post: { operationId: "resolveRuntimeApproval"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/events": {
    get: { operationId: "listRuntimeSessionEvents"; response: RuntimeEventEnvelope[]; error: ApiErrorEnvelope }
  }
  "/api/v1/runtime/sessions/{sessionId}/turns": {
    post: { operationId: "submitRuntimeTurn"; response: RuntimeRunSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/system/bootstrap": {
    get: { operationId: "getSystemBootstrap"; response: SystemBootstrapStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/system/health": {
    get: { operationId: "getSystemHealthcheck"; response: HealthcheckStatus; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace": {
    get: { operationId: "getWorkspaceSummary"; response: WorkspaceSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents": {
    get: { operationId: "listWorkspaceAgents"; response: AgentRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceAgent"; response: AgentRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/{agentId}": {
    patch: { operationId: "updateWorkspaceAgent"; response: AgentRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceAgent"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/import": {
    post: { operationId: "importWorkspaceAgentBundle"; response: ImportWorkspaceAgentBundleResult; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/agents/import-preview": {
    post: { operationId: "previewWorkspaceAgentBundleImport"; response: ImportWorkspaceAgentBundlePreview; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/automations": {
    get: { operationId: "listWorkspaceAutomations"; response: AutomationRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceAutomation"; response: AutomationRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/automations/{automationId}": {
    patch: { operationId: "updateWorkspaceAutomation"; response: AutomationRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceAutomation"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/mcp-servers": {
    post: { operationId: "createWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/mcp-servers/{serverName}": {
    get: { operationId: "getWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceMcpServer"; response: WorkspaceMcpServerDocument; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceMcpServer"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/models": {
    get: { operationId: "getWorkspaceCatalogModels"; response: ModelCatalogSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/provider-credentials": {
    get: { operationId: "listWorkspaceProviderCredentials"; response: CredentialBinding[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills": {
    post: { operationId: "createWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}": {
    get: { operationId: "getWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceSkill"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceSkill"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/copy-to-managed": {
    post: { operationId: "copyWorkspaceSkillToManaged"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}": {
    get: { operationId: "getWorkspaceSkillFile"; response: WorkspaceSkillFileDocument; error: ApiErrorEnvelope }
    patch: { operationId: "updateWorkspaceSkillFile"; response: WorkspaceSkillFileDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/{skillId}/tree": {
    get: { operationId: "getWorkspaceSkillTree"; response: WorkspaceSkillTreeDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/import-archive": {
    post: { operationId: "importWorkspaceSkillArchive"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/skills/import-folder": {
    post: { operationId: "importWorkspaceSkillFolder"; response: WorkspaceSkillDocument; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tool-catalog": {
    get: { operationId: "getWorkspaceToolCatalog"; response: WorkspaceToolCatalogSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tool-catalog/disable": {
    patch: { operationId: "setWorkspaceToolCatalogDisabled"; response: WorkspaceToolCatalogSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tools": {
    get: { operationId: "listWorkspaceTools"; response: ToolRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceTool"; response: ToolRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/catalog/tools/{toolId}": {
    patch: { operationId: "updateWorkspaceTool"; response: ToolRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceTool"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/knowledge": {
    get: { operationId: "listWorkspaceKnowledge"; response: KnowledgeRecord[]; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/overview": {
    get: { operationId: "getWorkspaceOverview"; response: WorkspaceOverviewSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet": {
    get: { operationId: "getWorkspacePetSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet/conversation": {
    put: { operationId: "bindWorkspacePetConversation"; response: PetConversationBinding; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/pet/presence": {
    patch: { operationId: "saveWorkspacePetPresence"; response: PetPresenceState; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/menus": {
    get: { operationId: "listWorkspaceMenus"; response: MenuRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceMenu"; response: MenuRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/menus/{menuId}": {
    patch: { operationId: "updateWorkspaceMenu"; response: MenuRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/permissions": {
    get: { operationId: "listWorkspacePermissions"; response: PermissionRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspacePermission"; response: PermissionRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/permissions/{permissionId}": {
    patch: { operationId: "updateWorkspacePermission"; response: PermissionRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspacePermission"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/roles": {
    get: { operationId: "listWorkspaceRoles"; response: RoleRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceRole"; response: RoleRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/roles/{roleId}": {
    patch: { operationId: "updateWorkspaceRole"; response: RoleRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceRole"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/users": {
    get: { operationId: "listWorkspaceUsers"; response: UserRecordSummary[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceUser"; response: UserRecordSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/rbac/users/{userId}": {
    patch: { operationId: "updateWorkspaceUser"; response: UserRecordSummary; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceUser"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/resources": {
    get: { operationId: "listWorkspaceResources"; response: WorkspaceResourceRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/resources/{resourceId}": {
    patch: { operationId: "updateWorkspaceResource"; response: WorkspaceResourceRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceResource"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/teams": {
    get: { operationId: "listWorkspaceTeams"; response: TeamRecord[]; error: ApiErrorEnvelope }
    post: { operationId: "createWorkspaceTeam"; response: TeamRecord; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/teams/{teamId}": {
    patch: { operationId: "updateWorkspaceTeam"; response: TeamRecord; error: ApiErrorEnvelope }
    delete: { operationId: "deleteWorkspaceTeam"; response: void; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/user-center/overview": {
    get: { operationId: "getUserCenterOverview"; response: UserCenterOverviewSnapshot; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/user-center/profile": {
    patch: { operationId: "updateCurrentUserProfile"; response: UserRecordSummary; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/user-center/profile/password": {
    post: { operationId: "changeCurrentUserPassword"; response: ChangeCurrentUserPasswordResponse; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/user-center/profile/runtime-config": {
    get: { operationId: "getUserRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
    patch: { operationId: "saveUserRuntimeConfig"; response: RuntimeEffectiveConfig; error: ApiErrorEnvelope }
  }
  "/api/v1/workspace/user-center/profile/runtime-config/validate": {
    post: { operationId: "validateUserRuntimeConfig"; response: RuntimeConfigValidationResult; error: ApiErrorEnvelope }
  }
}

