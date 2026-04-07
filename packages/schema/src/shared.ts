export type ThemeMode = 'light' | 'dark' | 'system'
export type Locale = 'zh-CN' | 'en-US'
export type RiskLevel = 'low' | 'medium' | 'high'
export type ViewStatus = 'healthy' | 'configured' | 'attention'
export type DecisionAction = 'approve' | 'reject'
export type PermissionMode = 'auto' | 'readonly' | 'danger-full-access'
export type WorkspaceToolPermissionMode = 'allow' | 'deny' | 'ask' | 'readonly'
export type WorkspaceToolStatus = 'active' | 'disabled'

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
export type ArtifactStatus = 'draft' | 'review' | 'approved' | 'published'
export type PetSpecies =
  | 'duck'
  | 'goose'
  | 'blob'
  | 'cat'
  | 'dragon'
  | 'octopus'
  | 'owl'
  | 'penguin'
  | 'turtle'
  | 'snail'
  | 'ghost'
  | 'axolotl'
  | 'capybara'
  | 'cactus'
  | 'robot'
  | 'rabbit'
  | 'mushroom'
  | 'chonk'
export type PetMood = 'curious' | 'happy' | 'sleepy' | 'playful' | 'focused'
export type PetMotionState = 'idle' | 'hover' | 'walk' | 'chat' | 'sleep'
export type PetChatSender = 'user' | 'pet'
export type ProjectResourceKind = 'file' | 'folder' | 'artifact' | 'url'
export type ProjectResourceOrigin = 'source' | 'generated'
export type RunType = 'conversation_run' | 'review_run' | 'execution_run' | 'automation_run'
export type RunStatus =
  | 'idle'
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
export type HostPlatform = 'tauri' | 'web'
export type HostExecutionMode = 'local' | 'remote'
export type BackendTransport = 'http' | 'sse' | 'ws'
export type BackendConnectionState = 'ready' | 'unavailable'
export type AutomationStatus = 'active' | 'paused' | 'error'
export type SettingsSectionId = 'connections' | 'roles' | 'policies' | 'audit' | 'integrations' | 'logs'
export type DesktopSettingsTabId = 'general' | 'theme' | 'i18n' | 'version'
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
export type RbacPermissionTargetType = 'workspace' | 'project' | 'user' | 'role' | 'permission' | 'menu' | 'resource' | 'agent' | 'knowledge' | 'tool' | 'skill' | 'mcp'
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
