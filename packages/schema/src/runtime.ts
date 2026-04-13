import type {
  RuntimeConfigScope as OpenApiRuntimeConfigScope,
  RuntimeConfigPatch as OpenApiRuntimeConfigPatch,
  RuntimeConfigSource as OpenApiRuntimeConfigSource,
  RuntimeConfigValidationResult as OpenApiRuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeInput as OpenApiRuntimeConfiguredModelProbeInput,
  RuntimeConfiguredModelProbeResult as OpenApiRuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig as OpenApiRuntimeEffectiveConfig,
  RuntimeSecretReferenceStatus as OpenApiRuntimeSecretReferenceStatus,
} from './generated'
import type { WorkspaceToolPermissionMode } from './shared'

export * from './actor-manifest'
export * from './agent-runtime'
export * from './capability-runtime'
export * from './memory-runtime'
export * from './runtime-policy'

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
