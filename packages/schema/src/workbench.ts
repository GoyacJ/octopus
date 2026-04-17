import type {
  BindPetConversationInput as OpenApiBindPetConversationInput,
  PetConversationBinding as OpenApiPetConversationBinding,
  PetMessage as OpenApiPetMessage,
  PetPresenceState as OpenApiPetPresenceState,
  PetProfile as OpenApiPetProfile,
  PetWorkspaceSnapshot as OpenApiPetWorkspaceSnapshot,
  SavePetPresenceInput as OpenApiSavePetPresenceInput,
} from './generated'
import type { ArtifactVersionReference, DeliverableDetail } from './artifact'
import type {
  AgentAssetKind,
  AgentScope,
  AgentStatus,
  ConversationActorKind,
  ConversationIntent,
  PermissionMode,
  ProjectResourceKind,
  ProjectResourceOrigin,
  ProjectStatus,
  RiskLevel,
  RunStatus,
  RunType,
  TeamMode,
  TeamScope,
  TeamStatus,
  ToolCatalogKind,
  TraceKind,
  TraceTone,
  ViewStatus,
} from './shared'

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

export interface WorkbenchCapabilityPolicy {
  model: string
  tools: string[]
  externalBindings: string[]
  environment: string[]
  approvalRequired: string[]
  forbiddenActions: string[]
  defaultResultFormat: string
  riskLevel: RiskLevel
}

export interface AgentMetrics {
  completedToday?: number
  avgResponse?: string
  cost?: string
  activeTasks?: number
  successRate?: string
  efficiency?: string
}

export interface TeamMetrics {
  activeTasks?: number
  cost?: string
  successRate?: string
  handoffSpeed?: string
  throughput?: string
}

export interface Agent {
  id: string
  name: string
  avatar: string
  role: string
  title?: string
  summary: string
  description?: string
  persona: string[]
  skillTags: string[]
  tags?: string[]
  mcpBindings: string[]
  systemPrompt: string
  capabilityPolicy: WorkbenchCapabilityPolicy
  knowledgeScope: KnowledgeScope
  executionProfile: ExecutionProfile
  approvalPreferences: string[]
  recentTask?: string
  lastActiveAt?: number
  metrics?: AgentMetrics
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
  title?: string
  description: string
  summary: string
  avatar: string
  mode: TeamMode
  members: string[]
  lead?: string
  skillTags: string[]
  tags?: string[]
  mcpBindings: string[]
  defaultKnowledgeScope: string[]
  defaultOutput: string
  workflow?: string[]
  recentTask?: string
  recentOutcome?: string
  lastActiveAt?: number
  metrics?: TeamMetrics
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
  parentRunId?: string
  delegatedByToolCallId?: string
  subrunCount?: number
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
  type: 'thinking' | 'execution' | 'tool' | 'result'
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
  status?: RunStatus
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
  artifacts?: Array<string | ArtifactVersionReference>
  approval?: {
    id: string
    toolName: string
    summary: string
    detail: string
    riskLevel: RiskLevel
    status?: 'pending' | 'approved' | 'rejected'
  }
  timestamp: number
}

export type PetProfile = OpenApiPetProfile
export type PetMessage = OpenApiPetMessage
export type PetPresenceState = OpenApiPetPresenceState
export type PetConversationBinding = OpenApiPetConversationBinding
export type SavePetPresenceInput = OpenApiSavePetPresenceInput
export type BindPetConversationInput = OpenApiBindPetConversationInput
export type PetWorkspaceSnapshot = OpenApiPetWorkspaceSnapshot

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
  origin: ProjectResourceOrigin
  sourceArtifactId?: string
  linkedConversationIds: string[]
  createdAt: number
  createdByMessageId?: string
  sizeLabel?: string
  location?: string
  tags: string[]
}

export type Artifact = DeliverableDetail

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
  trend?: string
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

export interface LegacyWorkspaceOverviewSnapshot {
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

export interface LegacyProjectDashboardSnapshot {
  workspaceId: string
  project: Project
  resourceMetrics: DashboardMetric[]
  progress: ProjectDashboardProgress
  dataMetrics: DashboardMetric[]
  activity: ActivityRecord[]
  conversationTokenTop: UsageRankingItem[]
}

export interface Project {
  id: string
  workspaceId: string
  name: string
  status: ProjectStatus
  goal: string
  phase: string
  summary: string
  workingDir?: string
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

export type { AgentAssetKind }
