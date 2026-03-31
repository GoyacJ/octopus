export type ThemeMode = 'light' | 'dark' | 'system'
export type Locale = 'zh-CN' | 'en-US'
export type RiskLevel = 'low' | 'medium' | 'high'
export type ViewStatus = 'healthy' | 'configured' | 'attention'
export type DecisionAction = 'approve' | 'reject'

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
export type AgentStatus = 'active' | 'archived'
export type TeamStatus = 'active' | 'archived'
export type ArtifactStatus = 'draft' | 'review' | 'approved'
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
  agentIds: string[]
  teamIds: string[]
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
  persona: string[]
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
  avatar: string
  mode: TeamMode
  members: string[]
  defaultKnowledgeScope: string[]
  defaultOutput: string
  useScope: TeamScope
  projectNotes: string
  approvalPreferences: string[]
  status: TeamStatus
  isProjectCopy: boolean
  sourceTeamId?: string
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

export interface Message {
  id: string
  conversationId: string
  senderId: string
  senderType: 'user' | 'agent' | 'system'
  content: string
  artifacts?: string[]
  timestamp: number
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
}

export interface TraceRecord {
  id: string
  runId: string
  kind: TraceKind
  title: string
  detail: string
  status: TraceTone
  timestamp: number
  actor: string
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

export interface KnowledgeEntry {
  id: string
  workspaceId: string
  projectId?: string
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

export interface ShellPreferences {
  theme: ThemeMode
  locale: Locale
  compactSidebar: boolean
  defaultWorkspaceId: string
  lastVisitedRoute: string
}

export interface ShellBootstrap {
  hostState: HostState
  preferences: ShellPreferences
  connections: ConnectionProfile[]
}
