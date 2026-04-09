import type {
  CreateRuntimeSessionInput as OpenApiCreateRuntimeSessionInput,
  ProviderConfig as OpenApiProviderConfig,
  ResolvedExecutionTarget as OpenApiResolvedExecutionTarget,
  ResolveRuntimeApprovalInput as OpenApiResolveRuntimeApprovalInput,
  RuntimeActorType as OpenApiRuntimeActorType,
  RuntimeApprovalRequest as OpenApiRuntimeApprovalRequest,
  RuntimeBootstrap as OpenApiRuntimeBootstrap,
  RuntimeConfigScope as OpenApiRuntimeConfigScope,
  RuntimeConfigPatch as OpenApiRuntimeConfigPatch,
  RuntimeConfigSource as OpenApiRuntimeConfigSource,
  RuntimeConfigValidationResult as OpenApiRuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeInput as OpenApiRuntimeConfiguredModelProbeInput,
  RuntimeConfiguredModelProbeResult as OpenApiRuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig as OpenApiRuntimeEffectiveConfig,
  RuntimeEventEnvelope as OpenApiRuntimeEventEnvelope,
  RuntimeEventKind as OpenApiRuntimeEventKind,
  RuntimeMessage as OpenApiRuntimeMessage,
  RuntimeRunSnapshot as OpenApiRuntimeRunSnapshot,
  RuntimeSecretReferenceStatus as OpenApiRuntimeSecretReferenceStatus,
  RuntimeSessionDetail as OpenApiRuntimeSessionDetail,
  RuntimeSessionKind as OpenApiRuntimeSessionKind,
  RuntimeSessionSummary as OpenApiRuntimeSessionSummary,
  RuntimeTraceItem as OpenApiRuntimeTraceItem,
  SubmitRuntimeTurnInput as OpenApiSubmitRuntimeTurnInput,
} from './generated'
import type {
  DecisionAction,
  PermissionMode,
  WorkspaceToolPermissionMode,
} from './shared'
import type { RuntimePermissionMode } from './permissions'

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JsonValue }
  | JsonValue[]

export type RuntimeConfigScope = OpenApiRuntimeConfigScope
export type RuntimeConfigSource = OpenApiRuntimeConfigSource
export type RuntimeSecretReferenceStatus = OpenApiRuntimeSecretReferenceStatus

export interface RuntimeConfigPatch extends Omit<OpenApiRuntimeConfigPatch, 'patch'> {
  patch: Record<string, JsonValue>
}

export interface ProjectModelSettings {
  allowedConfiguredModelIds: string[]
  defaultConfiguredModelId: string
}

export interface ProjectToolPermissionOverride {
  permissionMode: WorkspaceToolPermissionMode
}

export interface ProjectToolSettings {
  enabledSourceKeys: string[]
  overrides: Record<string, ProjectToolPermissionOverride>
}

export interface ProjectAgentSettings {
  enabledAgentIds: string[]
  enabledTeamIds: string[]
}

export interface ProjectSettingsConfig {
  models?: ProjectModelSettings
  tools?: ProjectToolSettings
  agents?: ProjectAgentSettings
}

export type RuntimeConfigValidationResult = OpenApiRuntimeConfigValidationResult

export interface RuntimeConfiguredModelProbeInput extends Omit<OpenApiRuntimeConfiguredModelProbeInput, 'patch'> {
  patch: Record<string, JsonValue>
}

export type RuntimeConfiguredModelProbeResult = OpenApiRuntimeConfiguredModelProbeResult

export type RuntimeEffectiveConfig = OpenApiRuntimeEffectiveConfig

export interface RuntimeConfigSnapshotSummary {
  id: string
  effectiveConfigHash: string
  startedFromScopeSet: RuntimeConfigScope[]
  sourceRefs: string[]
  createdAt: number
  effectiveConfig?: Record<string, JsonValue>
}

export type ResolvedExecutionTarget = OpenApiResolvedExecutionTarget
export type RuntimeActorType = OpenApiRuntimeActorType
export type RuntimeEventKind = OpenApiRuntimeEventKind
export type RuntimeSessionKind = OpenApiRuntimeSessionKind
export type RuntimeSessionSummary = OpenApiRuntimeSessionSummary
export type RuntimeRunSnapshot = OpenApiRuntimeRunSnapshot
export type RuntimeMessage = OpenApiRuntimeMessage
export type RuntimeTraceItem = OpenApiRuntimeTraceItem
export type RuntimeApprovalRequest = OpenApiRuntimeApprovalRequest

export type RuntimeDecisionAction = DecisionAction

export type RuntimeEventEnvelope = OpenApiRuntimeEventEnvelope
export type RuntimeSessionDetail = OpenApiRuntimeSessionDetail
export type ProviderConfig = OpenApiProviderConfig
export type RuntimeBootstrap = OpenApiRuntimeBootstrap
export type CreateRuntimeSessionInput = OpenApiCreateRuntimeSessionInput

export interface SubmitRuntimeTurnInput extends Omit<OpenApiSubmitRuntimeTurnInput, 'permissionMode'> {
  permissionMode: PermissionMode | RuntimePermissionMode
}

export type ResolveRuntimeApprovalInput = OpenApiResolveRuntimeApprovalInput
