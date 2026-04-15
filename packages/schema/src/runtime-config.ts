import type {
  RuntimeConfigPatch as OpenApiRuntimeConfigPatch,
  RuntimeConfigScope as OpenApiRuntimeConfigScope,
  RuntimeConfigSource as OpenApiRuntimeConfigSource,
  RuntimeConfigValidationResult as OpenApiRuntimeConfigValidationResult,
  RuntimeConfiguredModelProbeInput as OpenApiRuntimeConfiguredModelProbeInput,
  RuntimeConfiguredModelProbeResult as OpenApiRuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig as OpenApiRuntimeEffectiveConfig,
  RuntimeSecretReferenceStatus as OpenApiRuntimeSecretReferenceStatus,
} from './generated'

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
export type RuntimeConfigValidationResult = OpenApiRuntimeConfigValidationResult
export type RuntimeConfiguredModelProbeResult = OpenApiRuntimeConfiguredModelProbeResult
export type RuntimeEffectiveConfig = OpenApiRuntimeEffectiveConfig

export interface RuntimeConfigPatch extends Omit<OpenApiRuntimeConfigPatch, 'patch'> {
  patch: Record<string, JsonValue>
}

export interface RuntimeConfiguredModelProbeInput extends Omit<OpenApiRuntimeConfiguredModelProbeInput, 'patch'> {
  patch: Record<string, JsonValue>
}

export interface RuntimeConfigSnapshotSummary {
  id: string
  effectiveConfigHash: string
  startedFromScopeSet: RuntimeConfigScope[]
  sourceRefs: string[]
  createdAt: number
  effectiveConfig?: Record<string, JsonValue>
}
