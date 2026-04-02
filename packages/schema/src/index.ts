export type ThemeMode = 'light' | 'dark' | 'system'
export type Locale = 'zh-CN' | 'en-US'
export type RiskLevel = 'low' | 'medium' | 'high'
export type ViewStatus = 'healthy' | 'configured' | 'attention'
export type DecisionAction = 'approve' | 'reject'
export type PermissionMode = 'auto' | 'readonly'

export enum ConversationIntent {
  EXPLORE = 'explore',
  CLARIFY = 'clarify',
  PLAN = 'plan',
  DRAFT = 'draft',
  EXECUTE = 'execute',
  REVIEW = 'review',
  APPROVE = 'approve',
  BLOCKED = 'blocked',
  PAUSED = 'paused',
  AUTOMATED = 'automated',
}

export enum TeamMode {
  LEADERED = 'leadered',
  HYBRID = 'hybrid',
  MESH = 'mesh',
}

export type ProjectStatus = 'active' | 'archived'
export type AgentScope = 'personal' | 'workspace' | 'project'
export type TeamScope = 'workspace' | 'project'
export type AgentAssetKind = 'agent' | 'team'
export type AgentStatus = 'active' | 'archived'
export type TeamStatus = 'active' | 'archived'
export type ArtifactStatus = 'draft' | 'review' | 'approved'
export type ProjectResourceKind = 'file' | 'folder' | 'artifact' | 'url'
export type RunType = 'conversation_run' | 'review_run' | 'execution_run' | 'automation_run'
export type RunStatus =
  | 'draft'
  | 'planned'
  | 'running'
  | 'waiting_input'
  | 'waiting_approval'
  | 'blocked'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'terminated'
export type TraceKind = 'step' | 'tool' | 'approval' | 'pause' | 'resume' | 'artifact' | 'knowledge'
export type TraceTone = 'info' | 'success' | 'warning' | 'error'
export type KnowledgeKind = 'private' | 'shared' | 'candidate'
export type KnowledgeStatus = 'candidate' | 'reviewed' | 'shared' | 'archived'
export type KnowledgeSourceType = 'conversation' | 'artifact' | 'run'
export type ConnectionMode = 'local' | 'shared' | 'remote'
export type ConnectionState = 'local-ready' | 'connected' | 'disconnected'
export type AutomationStatus = 'active' | 'paused' | 'error'
export type SettingsSectionId = 'connections' | 'roles' | 'policies' | 'audit' | 'integrations' | 'logs'
export type ToolCatalogKind = 'builtin' | 'skill' | 'mcp'
export type ConversationActorKind = 'agent' | 'team'
export type MessageProcessEntryType = 'thinking' | 'execution' | 'tool' | 'result'
export type ConversationMemorySource = 'agent' | 'conversation'
export type UserStatus = 'active' | 'disabled'
export type UserGender = 'male' | 'female' | 'unknown'
export type PasswordState = 'set' | 'reset-required' | 'temporary'
export type WorkspaceScopeMode = 'all-projects' | 'selected-projects'
export type RbacRoleStatus = 'active' | 'disabled'
export type RbacPermissionStatus = 'active' | 'disabled'
export type RbacPermissionKind = 'atomic' | 'bundle'
export type RbacPermissionTargetType = 'project' | 'agent' | 'tool' | 'skill' | 'mcp'
export type MenuSource = 'main-sidebar' | 'user-center'
export type MenuStatus = 'active' | 'disabled'
export type InboxItemType =
  | 'execution_approval'
  | 'sending_approval'
  | 'knowledge_promotion_approval'
  | 'high_risk_tool_approval'
  | 'resource_exceed_approval'
  | 'confirmation'
  | 'blocked'
  | 'resume'
  | 'automation_error'

export interface Workspace {
  id: string
  name: string
  avatar?: string
  isLocal: boolean
  description: string
  roleSummary: string
  memberCount: number
  projectIds: string[]
}

export interface UserAccount {
  id: string
  username: string
  nickname: string
  gender: UserGender
  avatar: string
  phone: string
  email: string
  status: UserStatus
  passwordState: PasswordState
  passwordUpdatedAt: number
}

export interface WorkspaceMembership {
  workspaceId: string
  userId: string
  roleIds: string[]
  scopeMode: WorkspaceScopeMode
  scopeProjectIds: string[]
}

export interface RbacRole {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacRoleStatus
  permissionIds: string[]
  menuIds: string[]
}

export interface RbacPermission {
  id: string
  workspaceId: string
  name: string
  code: string
  description: string
  status: RbacPermissionStatus
  kind: RbacPermissionKind
  targetType?: RbacPermissionTargetType
  targetIds?: string[]
  action?: string
  memberPermissionIds?: string[]
}

export interface MenuNode {
  id: string
  workspaceId: string
  parentId?: string
  source: MenuSource
  label: string
  routeName?: string
  status: MenuStatus
  order: number
}

export interface Project {
  id: string
  workspaceId: string
  name: string
  status: ProjectStatus
  goal: string
  phase: string
  summary: string
  blockerIds: string[]
  recentDecision: string
  conversationIds: string[]
  artifactIds: string[]
  resourceIds: string[]
  agentIds: string[]
  teamIds: string[]
  defaultActorKind?: ConversationActorKind
  defaultActorId?: string
}

export interface KnowledgeScope {
  privateMemories: string[]
  sharedSources: string[]
  accessibleProjects: string[]
}

export interface ExecutionProfile {
  planningStyle: string
  verificationStyle: string
  autonomyLevel: string
  interruptPreference: string
}

export interface CapabilityPolicy {
  model: string
  tools: string[]
  externalBindings: string[]
  environment: string[]
  approvalRequired: string[]
  forbiddenActions: string[]
  defaultResultFormat: string
  riskLevel: RiskLevel
}

export interface Agent {
  id: string
  name: string
  avatar: string
  role: string
  summary: string
  persona: string[]
  skillTags: string[]
  mcpBindings: string[]
  systemPrompt: string
  capabilityPolicy: CapabilityPolicy
  knowledgeScope: KnowledgeScope
  executionProfile: ExecutionProfile
  approvalPreferences: string[]
  scope: AgentScope
  owner: string
  status: AgentStatus
  isProjectCopy: boolean
  sourceAgentId?: string
}

export interface Team {
  id: string
  workspaceId: string
  projectId?: string
  name: string
  description: string
  summary: string
  avatar: string
  mode: TeamMode
  members: string[]
  skillTags: string[]
  mcpBindings: string[]
  defaultKnowledgeScope: string[]
  defaultOutput: string
  useScope: TeamScope
  projectNotes: string
  approvalPreferences: string[]
  structureMode: 'org-chart' | 'flow'
  structureNodes: TeamStructureNode[]
  structureEdges: TeamStructureEdge[]
  status: TeamStatus
  isProjectCopy: boolean
  sourceTeamId?: string
}

export interface TeamStructureNode {
  id: string
  label: string
  role: string
  memberId?: string
  level: number
  position: {
    x: number
    y: number
  }
}

export interface TeamStructureEdge {
  id: string
  source: string
  target: string
  relation: string
}

export interface ResumePoint {
  id: string
  label: string
  timestamp: number
}

export interface BranchLink {
  id: string
  label: string
  targetConversationId: string
  direction: 'source' | 'derived'
}

export interface RunSummary {
  id: string
  conversationId: string
  projectId: string
  title: string
  type: RunType
  status: RunStatus
  currentStep: string
  startedAt: number
  updatedAt: number
  ownerType: 'agent' | 'team'
  ownerId: string
  blockers: string[]
  nextAction: string
}

export interface Conversation {
  id: string
  projectId: string
  title: string
  intent: ConversationIntent
  activeAgentId?: string
  activeTeamId?: string
  summary: string
  currentGoal: string
  constraints: string[]
  blockerIds: string[]
  pendingInboxIds: string[]
  resumePoints: ResumePoint[]
  branchLinks: BranchLink[]
  artifactIds: string[]
  recentRun?: RunSummary
  stageProgress: number
  statusNote: string
}

export type MessageSenderType = 'user' | 'agent' | 'system'
export type ConversationAttachmentKind = 'artifact' | 'file' | 'folder' | 'image'

export interface ConversationAttachment {
  id: string
  name: string
  kind: ConversationAttachmentKind
}

export interface ConversationComposerDraft {
  content: string
  modelId: string
  permissionMode: PermissionMode
  actorKind?: ConversationActorKind
  actorId?: string
  resourceIds: string[]
  attachments: ConversationAttachment[]
}

export interface MessageUsage {
  inputTokens: number
  outputTokens: number
  totalTokens: number
}

export interface MessageToolCall {
  toolId: string
  label: string
  kind: ToolCatalogKind
  count: number
}

export interface MessageProcessEntry {
  id: string
  type: MessageProcessEntryType
  title: string
  detail: string
  timestamp: number
  toolId?: string
}

export interface Message {
  id: string
  conversationId: string
  senderId: string
  senderType: MessageSenderType
  content: string
  modelId?: string
  permissionMode?: PermissionMode
  actorKind?: ConversationActorKind
  actorId?: string
  requestedActorKind?: ConversationActorKind
  requestedActorId?: string
  usedDefaultActor?: boolean
  resourceIds?: string[]
  toolIds?: string[]
  toolCalls?: MessageToolCall[]
  usage?: MessageUsage
  processEntries?: MessageProcessEntry[]
  attachments?: ConversationAttachment[]
  artifacts?: string[]
  timestamp: number
}

export interface ConversationQueueItem {
  id: string
  conversationId: string
  content: string
  modelId: string
  permissionMode: PermissionMode
  requestedActorKind?: ConversationActorKind
  requestedActorId?: string
  resolvedActorKind: ConversationActorKind
  resolvedActorId: string
  resourceIds: string[]
  attachments: ConversationAttachment[]
  createdAt: number
}

export interface ProjectResource {
  id: string
  projectId: string
  workspaceId: string
  name: string
  kind: ProjectResourceKind
  sourceArtifactId?: string
  linkedConversationIds: string[]
  createdAt: number
  createdByMessageId?: string
  sizeLabel?: string
  location?: string
  tags: string[]
}

export interface Artifact {
  id: string
  projectId: string
  conversationId: string
  type: string
  title: string
  content: string
  excerpt: string
  tags: string[]
  version: number
  status: ArtifactStatus
  authorId: string
  updatedAt: number
  createdByMessageId?: string
}

export interface TraceRecord {
  id: string
  runId: string
  conversationId: string
  kind: TraceKind
  title: string
  detail: string
  status: TraceTone
  timestamp: number
  actor: string
  messageId?: string
  toolId?: string
  createdByMessageId?: string
  relatedArtifactId?: string
  relatedInboxId?: string
}

export interface RunDetail extends RunSummary {
  timeline: TraceRecord[]
}

export interface DashboardMetric {
  label: string
  value: string
  tone?: TraceTone | 'default'
}

export interface DashboardHighlight {
  id: string
  title: string
  description: string
  route: string
  surface: 'conversation' | 'artifact' | 'inbox' | 'trace'
}

export interface DashboardSnapshot {
  workspaceId: string
  projectId?: string
  conversationId?: string
  workspaceMetrics: DashboardMetric[]
  projectMetrics: DashboardMetric[]
  conversationMetrics: DashboardMetric[]
  highlights: DashboardHighlight[]
}

export interface ActivityRecord {
  id: string
  workspaceId: string
  projectId?: string
  userId?: string
  scope: 'user' | 'project' | 'workspace'
  type: 'login' | 'operation'
  title: string
  description: string
  timestamp: number
  tokenCount?: number
}

export interface UsageRankingItem {
  id: string
  label: string
  value: number
  secondary?: string
}

export interface ProjectOverviewSummary {
  projectId: string
  metrics: DashboardMetric[]
  activity: ActivityRecord[]
  conversationTokenTop: UsageRankingItem[]
}

export interface WorkspaceOverviewSnapshot {
  workspaceId: string
  projectId?: string
  userMetrics: DashboardMetric[]
  userActivity: ActivityRecord[]
  projectSummary: ProjectOverviewSummary
  workspaceMetrics: DashboardMetric[]
  projectTokenTop: UsageRankingItem[]
  agentUsage: UsageRankingItem[]
  teamUsage: UsageRankingItem[]
  toolUsage: UsageRankingItem[]
  modelUsage: UsageRankingItem[]
}

export interface ProjectDashboardProgress {
  phase: string
  progress: number
  runStatus?: RunStatus
  currentStep: string
  blockerCount: number
  pendingInboxCount: number
}

export interface ProjectDashboardSnapshot {
  workspaceId: string
  project: Project
  resourceMetrics: DashboardMetric[]
  progress: ProjectDashboardProgress
  dataMetrics: DashboardMetric[]
  activity: ActivityRecord[]
  conversationTokenTop: UsageRankingItem[]
}

export interface KnowledgeEntry {
  id: string
  workspaceId: string
  projectId?: string
  conversationId?: string
  title: string
  kind: KnowledgeKind
  status: KnowledgeStatus
  sourceType: KnowledgeSourceType
  sourceId: string
  summary: string
  ownerId?: string
  lastUsedAt: number
  trustLevel: RiskLevel
  lineage: string[]
  createdByMessageId?: string
}

export interface KnowledgeCandidate extends KnowledgeEntry {
  reviewNote: string
}

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

export interface ConversationMemoryItem {
  id: string
  conversationId: string
  title: string
  summary: string
  source: ConversationMemorySource
  ownerId?: string
  createdAt: number
  createdByMessageId?: string
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

export interface RuntimeBootstrap {
  provider: ProviderConfig
  sessions: RuntimeSessionSummary[]
}

export interface ProviderConfig {
  provider: string
  apiKey?: string
  baseUrl?: string
  defaultModel?: string
}

export interface RuntimeSessionSummary {
  id: string
  conversationId: string
  projectId: string
  title: string
  status: string
  updatedAt: number
  lastMessagePreview?: string
}

export interface RuntimeRunSnapshot {
  id: string
  sessionId: string
  conversationId: string
  status: string
  currentStep: string
  startedAt: number
  updatedAt: number
  modelId?: string
  nextAction?: string
}

export interface RuntimeMessage {
  id: string
  sessionId: string
  conversationId: string
  senderType: 'user' | 'assistant' | 'system'
  senderLabel: string
  content: string
  timestamp: number
  modelId?: string
  status: string
}

export interface RuntimeTraceItem {
  id: string
  sessionId: string
  runId: string
  conversationId: string
  kind: string
  title: string
  detail: string
  tone: TraceTone | 'default' | string
  timestamp: number
  actor: string
  relatedMessageId?: string
  relatedToolName?: string
}

export interface RuntimeApprovalRequest {
  id: string
  sessionId: string
  conversationId: string
  runId: string
  toolName: string
  summary: string
  detail: string
  riskLevel: RiskLevel | string
  createdAt: number
}

export type RuntimeDecisionAction = 'approve' | 'reject'

export interface RuntimeEventEnvelope {
  id: string
  kind: string
  sessionId: string
  conversationId: string
  runId?: string
  emittedAt: number
  run?: RuntimeRunSnapshot
  message?: RuntimeMessage
  trace?: RuntimeTraceItem
  approval?: RuntimeApprovalRequest
  decision?: RuntimeDecisionAction
  summary?: RuntimeSessionSummary
  error?: string
}

export interface RuntimeSessionDetail {
  summary: RuntimeSessionSummary
  run: RuntimeRunSnapshot
  messages: RuntimeMessage[]
  trace: RuntimeTraceItem[]
  pendingApproval?: RuntimeApprovalRequest
}
