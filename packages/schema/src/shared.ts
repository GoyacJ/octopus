import type {
  ArtifactStatus as OpenApiArtifactStatus,
  BackendConnectionState as OpenApiBackendConnectionState,
  BackendTransport as OpenApiBackendTransport,
  ConnectionMode as OpenApiConnectionMode,
  ConnectionState as OpenApiConnectionState,
  HostExecutionMode as OpenApiHostExecutionMode,
  HostPlatform as OpenApiHostPlatform,
  KnowledgeKind as OpenApiKnowledgeKind,
  KnowledgePlaneScope as OpenApiKnowledgePlaneScope,
  KnowledgeSourceType as OpenApiKnowledgeSourceType,
  KnowledgeStatus as OpenApiKnowledgeStatus,
  KnowledgeVisibilityMode as OpenApiKnowledgeVisibilityMode,
  Locale as OpenApiLocale,
  PetContextScope as OpenApiPetContextScope,
  ProjectResourceKind as OpenApiProjectResourceKind,
  ProjectResourceOrigin as OpenApiProjectResourceOrigin,
  ThemeMode as OpenApiThemeMode,
  ViewStatus as OpenApiViewStatus,
} from './generated'

export type ThemeMode = OpenApiThemeMode
export type Locale = OpenApiLocale
export type RiskLevel = 'low' | 'medium' | 'high'
export type ViewStatus = OpenApiViewStatus
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
export type ArtifactStatus = OpenApiArtifactStatus
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
export type ProjectResourceKind = OpenApiProjectResourceKind
export type ProjectResourceOrigin = OpenApiProjectResourceOrigin
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
export type KnowledgeKind = OpenApiKnowledgeKind
export type KnowledgePlaneScope = OpenApiKnowledgePlaneScope
export type KnowledgeStatus = OpenApiKnowledgeStatus
export type KnowledgeSourceType = OpenApiKnowledgeSourceType
export type KnowledgeVisibilityMode = OpenApiKnowledgeVisibilityMode
export type PetContextScope = OpenApiPetContextScope
export type ConnectionMode = OpenApiConnectionMode
export type ConnectionState = OpenApiConnectionState
export type HostPlatform = OpenApiHostPlatform
export type HostExecutionMode = OpenApiHostExecutionMode
export type BackendTransport = OpenApiBackendTransport
export type BackendConnectionState = OpenApiBackendConnectionState
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
export type MenuSource = 'main-sidebar' | 'console' | 'access-control'
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
