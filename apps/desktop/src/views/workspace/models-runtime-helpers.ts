import type {
  ConfiguredModelBudgetPolicy,
  ConfiguredModelRecord,
  JsonValue,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  RuntimeExecutionClass,
  RuntimeExecutionProfile,
  SurfaceDescriptor,
} from '@octopus/schema'

export const PAGE_SIZE = 10
export const CUSTOM_PROVIDER_MODE = 'custom'
export const OLLAMA_PROVIDER_ID = 'ollama'
export const VLLM_PROVIDER_ID = 'vllm'
export const MANAGED_BY_METADATA_KEY = 'managedBy'
export const MANAGED_BY_METADATA_VALUE = 'workspace-models-page'
export const PROVIDER_TYPE_METADATA_KEY = 'providerType'
export const UPSTREAM_MODEL_ID_METADATA_KEY = 'upstreamModelId'
export const CUSTOM_BASE_URL_PLACEHOLDER = 'https://api.example.com/v1'

export function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function toRecord(value: unknown): Record<string, JsonValue> {
  return isObjectRecord(value) ? cloneJson(value) as Record<string, JsonValue> : {}
}

export function toMetadata(value: unknown): Record<string, unknown> {
  return isObjectRecord(value) ? cloneJson(value) as Record<string, unknown> : {}
}

function toExecutionProfile(value: unknown): RuntimeExecutionProfile {
  if (!isObjectRecord(value)) {
    return {
      executionClass: 'unsupported',
      toolLoop: false,
      upstreamStreaming: false,
    }
  }
  return {
    executionClass: (typeof value.executionClass === 'string'
      ? value.executionClass
      : 'unsupported') as RuntimeExecutionClass,
    toolLoop: Boolean(value.toolLoop),
    upstreamStreaming: Boolean(value.upstreamStreaming),
  }
}

export function supportsConversationExecution(executionProfile?: Partial<RuntimeExecutionProfile> | null) {
  return executionProfile?.executionClass === 'agent_conversation'
}

function toBudgetPolicy(value: unknown): ConfiguredModelBudgetPolicy | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  return {
    accountingMode: typeof value.accountingMode === 'string'
      ? value.accountingMode as ConfiguredModelBudgetPolicy['accountingMode']
      : undefined,
    trafficClasses: Array.isArray(value.trafficClasses)
      ? value.trafficClasses.filter((entry): entry is string => typeof entry === 'string')
      : undefined,
    totalBudgetTokens: typeof value.totalBudgetTokens === 'number' && value.totalBudgetTokens > 0
      ? value.totalBudgetTokens
      : undefined,
    reservationStrategy: typeof value.reservationStrategy === 'string'
      ? value.reservationStrategy as ConfiguredModelBudgetPolicy['reservationStrategy']
      : undefined,
    warningThresholdPercentages: Array.isArray(value.warningThresholdPercentages)
      ? value.warningThresholdPercentages.filter((entry): entry is number => typeof entry === 'number')
      : undefined,
  }
}

export function toConfiguredModelMap(value: unknown): Record<string, ConfiguredModelRecord> {
  const record = toRecord(value)
  const next: Record<string, ConfiguredModelRecord> = {}
  for (const [key, entry] of Object.entries(record)) {
    if (!isObjectRecord(entry)) {
      continue
    }
    const configuredModelId = typeof entry.configuredModelId === 'string' ? entry.configuredModelId : key
    const name = typeof entry.name === 'string' ? entry.name : configuredModelId
    const providerId = typeof entry.providerId === 'string' ? entry.providerId : ''
    const modelId = typeof entry.modelId === 'string' ? entry.modelId : ''
    if (!providerId || !modelId) {
      continue
    }
    next[configuredModelId] = {
      configuredModelId,
      name,
      providerId,
      modelId,
      credentialRef: typeof entry.credentialRef === 'string' ? entry.credentialRef : undefined,
      baseUrl: typeof entry.baseUrl === 'string' ? entry.baseUrl : undefined,
      budgetPolicy: toBudgetPolicy(entry.budgetPolicy),
      tokenUsage: {
        usedTokens: 0,
        remainingTokens: undefined,
        exhausted: false,
      },
      enabled: typeof entry.enabled === 'boolean' ? entry.enabled : true,
      source: typeof entry.source === 'string' ? entry.source : 'workspace',
      status: typeof entry.status === 'string' ? entry.status as ConfiguredModelRecord['status'] : (entry.credentialRef ? 'configured' : 'unconfigured'),
      configured: typeof entry.configured === 'boolean' ? entry.configured : Boolean(entry.credentialRef),
    }
  }
  return next
}

export function toSurfaceDescriptors(value: unknown): SurfaceDescriptor[] {
  if (!Array.isArray(value)) {
    return []
  }
  return value
    .filter(isObjectRecord)
    .map(entry => ({
      surface: typeof entry.surface === 'string' ? entry.surface : 'conversation',
      protocolFamily: typeof entry.protocolFamily === 'string' ? entry.protocolFamily : 'openai_chat',
      transport: Array.isArray(entry.transport) ? entry.transport.filter(item => typeof item === 'string') : ['request_response', 'sse'],
      authStrategy: typeof entry.authStrategy === 'string' ? entry.authStrategy : 'bearer',
      baseUrl: typeof entry.baseUrl === 'string' ? entry.baseUrl : '',
      baseUrlPolicy: typeof entry.baseUrlPolicy === 'string' ? entry.baseUrlPolicy : 'allow_override',
      enabled: typeof entry.enabled === 'boolean' ? entry.enabled : true,
      capabilities: Array.isArray(entry.capabilities)
        ? entry.capabilities
            .filter(isObjectRecord)
            .map(capability => ({
              capabilityId: typeof capability.capabilityId === 'string' ? capability.capabilityId : '',
              label: typeof capability.label === 'string'
                ? capability.label
                : (typeof capability.capabilityId === 'string' ? capability.capabilityId : ''),
            }))
            .filter(capability => capability.capabilityId)
        : [],
      executionProfile: toExecutionProfile(entry.executionProfile),
    }))
}

export function toProviderRegistryMap(value: unknown): Record<string, ProviderRegistryRecord> {
  const record = toRecord(value)
  const next: Record<string, ProviderRegistryRecord> = {}
  for (const [providerId, entry] of Object.entries(record)) {
    if (!isObjectRecord(entry)) {
      continue
    }
    next[providerId] = {
      providerId,
      label: typeof entry.label === 'string' ? entry.label : providerId,
      enabled: typeof entry.enabled === 'boolean' ? entry.enabled : true,
      surfaces: toSurfaceDescriptors(entry.surfaces),
      metadata: toMetadata(entry.metadata),
    }
  }
  return next
}

export function toModelRegistryMap(value: unknown): Record<string, ModelRegistryRecord> {
  const record = toRecord(value)
  const next: Record<string, ModelRegistryRecord> = {}
  for (const [modelId, entry] of Object.entries(record)) {
    if (!isObjectRecord(entry)) {
      continue
    }
    const providerId = typeof entry.providerId === 'string' ? entry.providerId : ''
    if (!providerId) {
      continue
    }
    next[modelId] = {
      modelId,
      providerId,
      label: typeof entry.label === 'string' ? entry.label : modelId,
      description: typeof entry.description === 'string' ? entry.description : '',
      family: typeof entry.family === 'string' ? entry.family : providerId,
      track: typeof entry.track === 'string' ? entry.track : 'custom',
      enabled: typeof entry.enabled === 'boolean' ? entry.enabled : true,
      recommendedFor: typeof entry.recommendedFor === 'string' ? entry.recommendedFor : '',
      availability: typeof entry.availability === 'string' ? entry.availability : 'configured',
      defaultPermission: (typeof entry.defaultPermission === 'string' ? entry.defaultPermission : 'auto') as ModelRegistryRecord['defaultPermission'],
      surfaceBindings: Array.isArray(entry.surfaceBindings)
        ? entry.surfaceBindings
            .flatMap((binding) => {
              if (!isObjectRecord(binding)) {
                return []
              }
              return [{
                surface: typeof binding.surface === 'string' ? binding.surface : 'conversation',
                protocolFamily: typeof binding.protocolFamily === 'string' ? binding.protocolFamily : 'openai_chat',
                enabled: typeof binding.enabled === 'boolean' ? binding.enabled : true,
                executionProfile: toExecutionProfile(binding.executionProfile),
              }]
            })
        : [],
      capabilities: Array.isArray(entry.capabilities)
        ? entry.capabilities
            .map(capability => {
              if (typeof capability === 'string') {
                return {
                  capabilityId: capability,
                  label: capability,
                }
              }
              if (!isObjectRecord(capability) || typeof capability.capabilityId !== 'string') {
                return null
              }
              return {
                capabilityId: capability.capabilityId,
                label: typeof capability.label === 'string' ? capability.label : capability.capabilityId,
              }
            })
            .filter((capability): capability is NonNullable<typeof capability> => capability !== null)
        : [],
      contextWindow: typeof entry.contextWindow === 'number' ? entry.contextWindow : undefined,
      maxOutputTokens: typeof entry.maxOutputTokens === 'number' ? entry.maxOutputTokens : undefined,
      metadata: toMetadata(entry.metadata),
    }
  }
  return next
}

export function toMergePatch(current: unknown, next: unknown): JsonValue | undefined {
  if (next === undefined) {
    return current === undefined ? undefined : null
  }

  if (JSON.stringify(current) === JSON.stringify(next)) {
    return undefined
  }

  if (isObjectRecord(current) && isObjectRecord(next)) {
    const patch: Record<string, JsonValue> = {}
    const keys = new Set([...Object.keys(current), ...Object.keys(next)])

    for (const key of keys) {
      const childPatch = toMergePatch(
        (current as Record<string, unknown>)[key],
        (next as Record<string, unknown>)[key],
      )
      if (childPatch !== undefined) {
        patch[key] = childPatch
      }
    }

    return Object.keys(patch).length ? patch : undefined
  }

  return cloneJson(next) as JsonValue
}

export function toPersistedConfiguredModelMap(
  value: Record<string, ConfiguredModelRecord>,
): Record<string, JsonValue> {
  const next: Record<string, JsonValue> = {}
  for (const [configuredModelId, configuredModel] of Object.entries(value)) {
    const persisted: Record<string, JsonValue> = {
      configuredModelId: configuredModel.configuredModelId,
      name: configuredModel.name,
      providerId: configuredModel.providerId,
      modelId: configuredModel.modelId,
      enabled: configuredModel.enabled,
      source: configuredModel.source,
    }
    if (configuredModel.credentialRef) {
      persisted.credentialRef = configuredModel.credentialRef
    }
    if (configuredModel.baseUrl) {
      persisted.baseUrl = configuredModel.baseUrl
    }
    if (configuredModel.budgetPolicy) {
      persisted.budgetPolicy = cloneJson(configuredModel.budgetPolicy) as JsonValue
    }
    next[configuredModelId] = persisted
  }
  return next
}

function slugify(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

export function createConfiguredModelId(providerId: string, modelId: string) {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID().slice(0, 8)
    : `${Date.now()}`
  return `${providerId}-${slugify(modelId) || 'model'}-${suffix}`
}

export function createCustomProviderId(providerLabel: string) {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID().slice(0, 8)
    : `${Date.now()}`
  return `custom-${slugify(providerLabel) || 'provider'}-${suffix}`
}

export function createManagedMetadata(providerType: string, upstreamModelId?: string) {
  return {
    [MANAGED_BY_METADATA_KEY]: MANAGED_BY_METADATA_VALUE,
    [PROVIDER_TYPE_METADATA_KEY]: providerType,
    ...(upstreamModelId ? { [UPSTREAM_MODEL_ID_METADATA_KEY]: upstreamModelId } : {}),
  } satisfies Record<string, unknown>
}

export function isManagedByPage(metadata: Record<string, unknown> | undefined) {
  return metadata?.[MANAGED_BY_METADATA_KEY] === MANAGED_BY_METADATA_VALUE
}

export function resolveProviderType(
  providerId: string,
  provider: ProviderRegistryRecord | null,
  model: ModelRegistryRecord | null,
) {
  const providerType = provider?.metadata?.[PROVIDER_TYPE_METADATA_KEY]
  if (typeof providerType === 'string') {
    return providerType
  }
  const modelType = model?.metadata?.[PROVIDER_TYPE_METADATA_KEY]
  if (typeof modelType === 'string') {
    return modelType
  }
  if (providerId === OLLAMA_PROVIDER_ID || providerId === VLLM_PROVIDER_ID) {
    return providerId
  }
  return 'standard'
}

export function isSpecialProviderType(providerType: string) {
  return providerType === CUSTOM_PROVIDER_MODE
    || providerType === OLLAMA_PROVIDER_ID
    || providerType === VLLM_PROVIDER_ID
}

export function buildManagedProviderOverride(
  providerId: string,
  label: string,
  baseUrl: string,
  providerType: string,
): ProviderRegistryRecord {
  return {
    providerId,
    label,
    enabled: true,
    surfaces: [
      {
        surface: 'conversation',
        protocolFamily: 'openai_chat',
        transport: ['request_response', 'sse'],
        authStrategy: 'bearer',
        baseUrl,
        baseUrlPolicy: 'allow_override',
        enabled: true,
        capabilities: [],
        executionProfile: {
          executionClass: 'unsupported',
          toolLoop: false,
          upstreamStreaming: false,
        },
      },
    ],
    metadata: createManagedMetadata(providerType),
  }
}

export function buildManagedModelOverride(
  modelId: string,
  providerId: string,
  label: string,
  providerType: string,
  upstreamModelId: string,
): ModelRegistryRecord {
  return {
    modelId,
    providerId,
    label,
    description: '',
    family: providerType,
    track: providerType === CUSTOM_PROVIDER_MODE ? 'custom' : 'local',
    enabled: true,
    recommendedFor: '',
    availability: 'configured',
    defaultPermission: 'auto',
    surfaceBindings: [
      {
        surface: 'conversation',
        protocolFamily: 'openai_chat',
        enabled: true,
        executionProfile: {
          executionClass: 'unsupported',
          toolLoop: false,
          upstreamStreaming: false,
        },
      },
    ],
    capabilities: [],
    metadata: createManagedMetadata(providerType, upstreamModelId),
  }
}

export function updateProviderBaseUrl(provider: ProviderRegistryRecord, baseUrl: string): ProviderRegistryRecord {
  const surfaces: SurfaceDescriptor[] = provider.surfaces.length
    ? provider.surfaces.map((surface, index) => (index === 0 ? { ...surface, baseUrl } : surface))
    : [{
        surface: 'conversation',
        protocolFamily: 'openai_chat',
        transport: ['request_response', 'sse'],
        authStrategy: 'bearer',
        baseUrl,
        baseUrlPolicy: 'allow_override',
        enabled: true,
        capabilities: [],
        executionProfile: {
          executionClass: 'unsupported',
          toolLoop: false,
          upstreamStreaming: false,
        },
      }]
  return {
    ...provider,
    surfaces,
  }
}
