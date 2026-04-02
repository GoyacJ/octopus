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
