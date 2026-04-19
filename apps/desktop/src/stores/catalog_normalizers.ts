import type {
  ConfiguredModelBudgetPolicy,
  CapabilityDescriptor,
  ConfiguredModelRecord,
  CredentialBinding,
  DefaultSelection,
  ModelCatalogSnapshot,
  ModelRegistryRecord,
  RuntimeExecutionClass,
  RuntimeExecutionProfile,
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
  conversationSurfaces: string[]
  capabilities: string[]
  defaultSurfaces: string[]
  contextWindow?: number
  maxOutputTokens?: number
  executionClass: RuntimeExecutionClass
  upstreamStreaming: boolean
  toolLoop: boolean
  supportsConversationExecution: boolean
  credentialRef?: string
  credentialStatus: CredentialBinding['status'] | 'missing'
  credentialConfigured: boolean
  credentialDisplayLabel?: string
  credentialHealthLabel?: string
  baseUrl?: string
  budgetAccountingMode?: ConfiguredModelBudgetPolicy['accountingMode']
  budgetReservationStrategy?: ConfiguredModelBudgetPolicy['reservationStrategy']
  budgetTrafficClasses: string[]
  totalTokens?: number
  usedTokens: number
  remainingTokens?: number
  budgetExhausted: boolean
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

function normalizeExecutionProfile(
  executionProfile?: Partial<RuntimeExecutionProfile> | null,
): RuntimeExecutionProfile {
  return {
    executionClass: (executionProfile?.executionClass ?? 'unsupported') as RuntimeExecutionClass,
    toolLoop: Boolean(executionProfile?.toolLoop),
    upstreamStreaming: Boolean(executionProfile?.upstreamStreaming),
  }
}

function isConversationExecutable(executionProfile: RuntimeExecutionProfile): boolean {
  return executionProfile.executionClass === 'agent_conversation'
}

export interface CatalogModelExecutionSummary {
  executionClass: RuntimeExecutionClass
  upstreamStreaming: boolean
  toolLoop: boolean
  supportsConversationExecution: boolean
  enabledSurfaces: string[]
  conversationSurfaces: string[]
}

export function summarizeModelExecution(
  model?: Pick<ModelRegistryRecord, 'surfaceBindings'> | null,
): CatalogModelExecutionSummary {
  const enabledBindings = (model?.surfaceBindings ?? []).filter(binding => binding.enabled)
  const conversationBindings = enabledBindings.filter(binding =>
    isConversationExecutable(normalizeExecutionProfile(binding.executionProfile)))
  const generationBindings = enabledBindings.filter(binding =>
    normalizeExecutionProfile(binding.executionProfile).executionClass === 'single_shot_generation')
  const primaryBinding = conversationBindings[0] ?? generationBindings[0] ?? enabledBindings[0]
  const primaryProfile = normalizeExecutionProfile(primaryBinding?.executionProfile)

  return {
    executionClass: primaryProfile.executionClass,
    upstreamStreaming: primaryProfile.upstreamStreaming,
    toolLoop: primaryProfile.toolLoop,
    supportsConversationExecution: conversationBindings.length > 0,
    enabledSurfaces: enabledBindings.map(binding => binding.surface),
    conversationSurfaces: conversationBindings.map(binding => binding.surface),
  }
}

function normalizeBudgetPolicy(
  budgetPolicy?: ConfiguredModelBudgetPolicy | null,
): ConfiguredModelBudgetPolicy | undefined {
  const totalBudgetTokens = budgetPolicy?.totalBudgetTokens
  if (!budgetPolicy || !totalBudgetTokens || totalBudgetTokens <= 0) {
    return budgetPolicy
      ? {
          ...budgetPolicy,
          totalBudgetTokens: undefined,
        }
      : undefined
  }

  return {
    ...budgetPolicy,
    totalBudgetTokens,
  }
}

function normalizeConfiguredModel(record: ConfiguredModelRecord): ConfiguredModelRecord {
  const totalTokens = record.budgetPolicy?.totalBudgetTokens
  const usedTokens = Number.isFinite(record.tokenUsage?.usedTokens)
    ? Math.max(0, record.tokenUsage.usedTokens)
    : 0
  return {
    ...record,
    budgetPolicy: normalizeBudgetPolicy(record.budgetPolicy),
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
        executionProfile: normalizeExecutionProfile(surface.executionProfile),
      })),
    })),
    models: (snapshot?.models ?? []).map(model => ({
      ...model,
      surfaceBindings: model.surfaceBindings.map(binding => ({
        ...binding,
        executionProfile: normalizeExecutionProfile(binding.executionProfile),
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
      .filter(binding => binding.enabled && isConversationExecutable(binding.executionProfile))
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
  const runtime = summarizeModelExecution(model)
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
    surfaces: runtime.enabledSurfaces,
    conversationSurfaces: runtime.conversationSurfaces,
    capabilities: model?.capabilities.map(capability => capability.capabilityId) ?? [],
    defaultSurfaces,
    contextWindow: model?.contextWindow,
    maxOutputTokens: model?.maxOutputTokens,
    executionClass: runtime.executionClass,
    upstreamStreaming: runtime.upstreamStreaming,
    toolLoop: runtime.toolLoop,
    supportsConversationExecution: runtime.supportsConversationExecution,
    credentialRef: configuredModel.credentialRef,
    credentialStatus: configuredModel.status,
    credentialConfigured: configuredModel.configured,
    baseUrl: configuredModel.baseUrl,
    budgetAccountingMode: configuredModel.budgetPolicy?.accountingMode,
    budgetReservationStrategy: configuredModel.budgetPolicy?.reservationStrategy,
    budgetTrafficClasses: configuredModel.budgetPolicy?.trafficClasses ?? [],
    totalTokens: configuredModel.budgetPolicy?.totalBudgetTokens,
    usedTokens: configuredModel.tokenUsage.usedTokens,
    remainingTokens: configuredModel.tokenUsage.remainingTokens,
    budgetExhausted: configuredModel.tokenUsage.exhausted,
    hasDiagnostics,
  }
}
