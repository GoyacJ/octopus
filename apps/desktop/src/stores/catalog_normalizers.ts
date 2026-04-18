import type {
  CapabilityDescriptor,
  ConfiguredModelRecord,
  CredentialBinding,
  DefaultSelection,
  ModelCatalogSnapshot,
  ModelRegistryRecord,
  RuntimeExecutionSupport,
} from '@octopus/schema'

export interface CatalogFilterOption {
  value: string
  label: string
}

export interface CatalogModelRow extends Record<string, unknown> {
  modelId: string
  label: string
  providerId: string
  providerLabel: string
  description: string
  family: string
  track: string
  enabled: boolean
  availability: string
  defaultPermission: string
  recommendedFor: string
  surfaces: string[]
  capabilities: string[]
  defaultSurfaces: string[]
  contextWindow?: number
  maxOutputTokens?: number
  credentialStatus: CredentialBinding['status'] | 'missing'
  credentialConfigured: boolean
  credentialLabel?: string
  hasDiagnostics: boolean
  metadata: Record<string, unknown>
}

export interface CatalogProviderSummary {
  providerId: string
  label: string
  enabled: boolean
  modelCount: number
  defaultSurfaces: string[]
  credentialStatus: CredentialBinding['status'] | 'missing'
  credentialConfigured: boolean
  hasIssues: boolean
  baseUrl?: string
}

export interface CatalogConfiguredModelRow extends Record<string, unknown> {
  configuredModelId: string
  name: string
  providerId: string
  providerLabel: string
  modelId: string
  modelLabel: string
  description: string
  family: string
  track: string
  enabled: boolean
  source: string
  surfaces: string[]
  capabilities: string[]
  defaultSurfaces: string[]
  contextWindow?: number
  maxOutputTokens?: number
  credentialRef?: string
  credentialStatus: CredentialBinding['status'] | 'missing'
  credentialConfigured: boolean
  credentialDisplayLabel?: string
  credentialHealthLabel?: string
  baseUrl?: string
  totalTokens?: number
  usedTokens: number
  remainingTokens?: number
  quotaExhausted: boolean
  hasDiagnostics: boolean
}

export interface CatalogConfiguredModelOption {
  value: string
  label: string
  providerId: string
  providerLabel: string
  modelId: string
  modelLabel: string
}

export interface CatalogCredentialSummary {
  providerId: string
  providerLabel: string
  credentialRef?: string
  label: string
  status: CredentialBinding['status'] | 'missing'
  configured: boolean
  source: string
  baseUrl?: string
  hasIssues: boolean
}

export interface CatalogDefaultSelectionRow {
  surface: string
  configuredModelId?: string
  configuredModelName?: string
  providerId: string
  providerLabel: string
  modelId: string
  modelLabel: string
}

export interface CatalogDiagnosticSummary {
  warningCount: number
  errorCount: number
  totalCount: number
  hasIssues: boolean
}

export const EMPTY_SNAPSHOT: ModelCatalogSnapshot = {
  providers: [],
  models: [],
  configuredModels: [],
  credentialBindings: [],
  defaultSelections: {},
  diagnostics: {
    warnings: [],
    errors: [],
  },
}

function normalizeRuntimeExecutionSupport(
  runtimeSupport?: Partial<RuntimeExecutionSupport> | null,
): RuntimeExecutionSupport {
  return {
    prompt: Boolean(runtimeSupport?.prompt),
    conversation: Boolean(runtimeSupport?.conversation),
    toolLoop: Boolean(runtimeSupport?.toolLoop),
    streaming: Boolean(runtimeSupport?.streaming),
  }
}

function isRuntimeExecutable(runtimeSupport: RuntimeExecutionSupport): boolean {
  return runtimeSupport.prompt
    || runtimeSupport.conversation
    || runtimeSupport.toolLoop
    || runtimeSupport.streaming
}

function normalizeConfiguredModel(record: ConfiguredModelRecord): ConfiguredModelRecord {
  const totalTokens = record.tokenQuota?.totalTokens
  const usedTokens = Number.isFinite(record.tokenUsage?.usedTokens)
    ? Math.max(0, record.tokenUsage.usedTokens)
    : 0
  return {
    ...record,
    tokenQuota: totalTokens && totalTokens > 0
      ? { totalTokens }
      : undefined,
    tokenUsage: {
      usedTokens,
      remainingTokens: totalTokens && totalTokens > 0
        ? Math.max(0, totalTokens - usedTokens)
        : undefined,
      exhausted: totalTokens ? usedTokens >= totalTokens : false,
    },
  }
}

export function normalizeSnapshot(snapshot?: Partial<ModelCatalogSnapshot> | null): ModelCatalogSnapshot {
  return {
    providers: (snapshot?.providers ?? []).map(provider => ({
      ...provider,
      surfaces: provider.surfaces.map(surface => ({
        ...surface,
        runtimeSupport: normalizeRuntimeExecutionSupport(surface.runtimeSupport),
      })),
    })),
    models: (snapshot?.models ?? []).map(model => ({
      ...model,
      surfaceBindings: model.surfaceBindings.map(binding => ({
        ...binding,
        runtimeSupport: normalizeRuntimeExecutionSupport(binding.runtimeSupport),
      })),
    })),
    configuredModels: (snapshot?.configuredModels ?? []).map(normalizeConfiguredModel),
    credentialBindings: snapshot?.credentialBindings ?? [],
    defaultSelections: snapshot?.defaultSelections ?? {},
    diagnostics: {
      warnings: snapshot?.diagnostics?.warnings ?? [],
      errors: snapshot?.diagnostics?.errors ?? [],
    },
  }
}

export function sortFilterOptions(values: Map<string, string>): CatalogFilterOption[] {
  return [...values.entries()]
    .sort((left, right) => left[1].localeCompare(right[1]))
    .map(([value, label]) => ({ value, label }))
}

export function resolveCapabilityLabel(capability: CapabilityDescriptor): string {
  return capability.label || capability.capabilityId
}

export function toModelRow(
  model: ModelRegistryRecord,
  providerLabel: string,
  defaultSelections: Record<string, DefaultSelection>,
  credentialBinding?: CredentialBinding,
  hasDiagnostics = false,
): CatalogModelRow {
  const defaultSurfaces = Object.values(defaultSelections)
    .filter(selection => selection.modelId === model.modelId)
    .map(selection => selection.surface)

  return {
    modelId: model.modelId,
    label: model.label,
    providerId: model.providerId,
    providerLabel,
    description: model.description,
    family: model.family,
    track: model.track,
    enabled: model.enabled,
    availability: String(model.availability),
    defaultPermission: model.defaultPermission,
    recommendedFor: model.recommendedFor,
    surfaces: model.surfaceBindings
      .filter(binding => binding.enabled && isRuntimeExecutable(binding.runtimeSupport))
      .map(binding => binding.surface),
    capabilities: model.capabilities.map(capability => capability.capabilityId),
    defaultSurfaces,
    contextWindow: model.contextWindow,
    maxOutputTokens: model.maxOutputTokens,
    credentialStatus: credentialBinding?.status ?? 'missing',
    credentialConfigured: credentialBinding?.configured ?? false,
    credentialLabel: credentialBinding?.label,
    hasDiagnostics,
    metadata: model.metadata,
  }
}

export function toConfiguredModelRow(
  configuredModel: ConfiguredModelRecord,
  providerLabel: string,
  model: ModelRegistryRecord | undefined,
  defaultSelections: Record<string, DefaultSelection>,
  hasDiagnostics = false,
): CatalogConfiguredModelRow {
  const defaultSurfaces = Object.values(defaultSelections)
    .filter(selection => selection.configuredModelId === configuredModel.configuredModelId)
    .map(selection => selection.surface)

  return {
    configuredModelId: configuredModel.configuredModelId,
    name: configuredModel.name,
    providerId: configuredModel.providerId,
    providerLabel,
    modelId: configuredModel.modelId,
    modelLabel: model?.label ?? configuredModel.modelId,
    description: model?.description ?? '',
    family: model?.family ?? '',
    track: model?.track ?? '',
    enabled: configuredModel.enabled && (model?.enabled ?? true),
    source: configuredModel.source,
    surfaces: model?.surfaceBindings
      .filter(binding => binding.enabled && isRuntimeExecutable(binding.runtimeSupport))
      .map(binding => binding.surface) ?? [],
    capabilities: model?.capabilities.map(capability => capability.capabilityId) ?? [],
    defaultSurfaces,
    contextWindow: model?.contextWindow,
    maxOutputTokens: model?.maxOutputTokens,
    credentialRef: configuredModel.credentialRef,
    credentialStatus: configuredModel.status,
    credentialConfigured: configuredModel.configured,
    baseUrl: configuredModel.baseUrl,
    totalTokens: configuredModel.tokenQuota?.totalTokens,
    usedTokens: configuredModel.tokenUsage.usedTokens,
    remainingTokens: configuredModel.tokenUsage.remainingTokens,
    quotaExhausted: configuredModel.tokenUsage.exhausted,
    hasDiagnostics,
  }
}
