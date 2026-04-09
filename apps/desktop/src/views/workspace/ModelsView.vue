<script setup lang="ts">
import { computed, h, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus, Trash2 } from 'lucide-vue-next'

import type {
  ConfiguredModelRecord,
  JsonValue,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  RuntimeConfigSource,
  SurfaceDescriptor,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDataTable,
  UiDialog,
  UiEmptyState,
  UiInput,
  UiPageHeader,
  UiPageShell,
  UiPagination,
  UiSelect,
  UiStatusCallout,
  UiSurface,
} from '@octopus/ui'

import type { CatalogConfiguredModelRow, CatalogFilterOption } from '@/stores/catalog'
import { useCatalogStore } from '@/stores/catalog'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'

const PAGE_SIZE = 10
const CUSTOM_PROVIDER_MODE = 'custom'
const OLLAMA_PROVIDER_ID = 'ollama'
const VLLM_PROVIDER_ID = 'vllm'
const MANAGED_BY_METADATA_KEY = 'managedBy'
const MANAGED_BY_METADATA_VALUE = 'workspace-models-page'
const PROVIDER_TYPE_METADATA_KEY = 'providerType'
const UPSTREAM_MODEL_ID_METADATA_KEY = 'upstreamModelId'
const CUSTOM_BASE_URL_PLACEHOLDER = 'https://api.example.com/v1'

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function toRecord(value: unknown): Record<string, JsonValue> {
  return isObjectRecord(value) ? cloneJson(value) as Record<string, JsonValue> : {}
}

function toMetadata(value: unknown): Record<string, unknown> {
  return isObjectRecord(value) ? cloneJson(value) as Record<string, unknown> : {}
}

function toConfiguredModelMap(value: unknown): Record<string, ConfiguredModelRecord> {
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
      tokenQuota: isObjectRecord(entry.tokenQuota) && typeof entry.tokenQuota.totalTokens === 'number' && entry.tokenQuota.totalTokens > 0
        ? { totalTokens: entry.tokenQuota.totalTokens }
        : undefined,
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

function toSurfaceDescriptors(value: unknown): SurfaceDescriptor[] {
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
    }))
}

function toProviderRegistryMap(value: unknown): Record<string, ProviderRegistryRecord> {
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

function toModelRegistryMap(value: unknown): Record<string, ModelRegistryRecord> {
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

function toMergePatch(current: unknown, next: unknown): JsonValue | undefined {
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

function toPersistedConfiguredModelMap(
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
    if (configuredModel.tokenQuota?.totalTokens) {
      persisted.tokenQuota = {
        totalTokens: configuredModel.tokenQuota.totalTokens,
      }
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

function createConfiguredModelId(providerId: string, modelId: string) {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID().slice(0, 8)
    : `${Date.now()}`
  return `${providerId}-${slugify(modelId) || 'model'}-${suffix}`
}

function createCustomProviderId(providerLabel: string) {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID().slice(0, 8)
    : `${Date.now()}`
  return `custom-${slugify(providerLabel) || 'provider'}-${suffix}`
}

function createManagedMetadata(providerType: string, upstreamModelId?: string) {
  return {
    [MANAGED_BY_METADATA_KEY]: MANAGED_BY_METADATA_VALUE,
    [PROVIDER_TYPE_METADATA_KEY]: providerType,
    ...(upstreamModelId ? { [UPSTREAM_MODEL_ID_METADATA_KEY]: upstreamModelId } : {}),
  } satisfies Record<string, unknown>
}

function isManagedByPage(metadata: Record<string, unknown> | undefined) {
  return metadata?.[MANAGED_BY_METADATA_KEY] === MANAGED_BY_METADATA_VALUE
}

function resolveProviderType(
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

function isSpecialProviderType(providerType: string) {
  return providerType === CUSTOM_PROVIDER_MODE
    || providerType === OLLAMA_PROVIDER_ID
    || providerType === VLLM_PROVIDER_ID
}

function buildManagedProviderOverride(
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
      },
    ],
    metadata: createManagedMetadata(providerType),
  }
}

function buildManagedModelOverride(
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
      },
    ],
    capabilities: [],
    metadata: createManagedMetadata(providerType, upstreamModelId),
  }
}

function updateProviderBaseUrl(provider: ProviderRegistryRecord, baseUrl: string): ProviderRegistryRecord {
  const surfaces = provider.surfaces.length
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
      }]
  return {
    ...provider,
    surfaces,
  }
}

const { t } = useI18n()
const catalogStore = useCatalogStore()
const runtime = useRuntimeStore()
const shell = useShellStore()

const searchQuery = ref('')
const providerFilter = ref('')
const surfaceFilter = ref('')
const capabilityFilter = ref('')
const page = ref(1)
const selectedConfiguredModelId = ref('')
const detailDialogOpen = ref(false)

const createDialogOpen = ref(false)
const createName = ref('')
const createProviderType = ref('')
const createCustomProviderLabel = ref('')
const createModelId = ref('')
const createCredentialRef = ref('')
const createBaseUrl = ref('')
const createTotalTokens = ref('')
const createEnabled = ref(true)
const createFormError = ref('')

const draftConfiguredModels = ref<Record<string, ConfiguredModelRecord>>({})
const draftProviderOverrides = ref<Record<string, ProviderRegistryRecord>>({})
const draftModelOverrides = ref<Record<string, ModelRegistryRecord>>({})

const workspaceRuntimeSource = computed<RuntimeConfigSource | undefined>(() =>
  runtime.config?.sources.filter(source => source.scope === 'workspace').at(-1),
)
const workspaceDocument = computed<Record<string, JsonValue>>(() => toRecord(workspaceRuntimeSource.value?.document))
const workspaceConfiguredModelsCurrent = computed<Record<string, ConfiguredModelRecord>>(() =>
  toConfiguredModelMap(workspaceDocument.value.configuredModels),
)
const workspaceModelRegistryDocument = computed<Record<string, JsonValue>>(() =>
  toRecord(workspaceDocument.value.modelRegistry),
)
const workspaceProviderOverridesCurrent = computed<Record<string, ProviderRegistryRecord>>(() =>
  toProviderRegistryMap(workspaceModelRegistryDocument.value.providers),
)
const workspaceModelOverridesCurrent = computed<Record<string, ModelRegistryRecord>>(() =>
  toModelRegistryMap(workspaceModelRegistryDocument.value.models),
)

const createProviderOptions = computed<CatalogFilterOption[]>(() => {
  const providerOptions = catalogStore.providers
    .filter(provider => provider.metadata?.[PROVIDER_TYPE_METADATA_KEY] !== CUSTOM_PROVIDER_MODE)
    .map(provider => ({
      value: provider.providerId,
      label: provider.label,
    }))
    .sort((left, right) => left.label.localeCompare(right.label))

  return [
    ...providerOptions,
    {
      value: CUSTOM_PROVIDER_MODE,
      label: t('models.providerModes.custom'),
    },
  ]
})

const createUsesFreeformModel = computed(() => isSpecialProviderType(createProviderType.value))
const createRequiresCustomProviderName = computed(() => createProviderType.value === CUSTOM_PROVIDER_MODE)
const createUpstreamModelOptions = computed(() =>
  createUsesFreeformModel.value ? [] : catalogStore.getModelOptionsByProviderId(createProviderType.value))
const createBaseUrlDefault = computed(() => {
  if (createProviderType.value === CUSTOM_PROVIDER_MODE) {
    return CUSTOM_BASE_URL_PLACEHOLDER
  }
  return catalogStore.getProviderBaseUrl(createProviderType.value) ?? ''
})

const localRows = computed<CatalogConfiguredModelRow[]>(() =>
  Object.values(draftConfiguredModels.value)
    .filter(configuredModel => configuredModel.source === 'workspace')
    .map((configuredModel) => {
      const liveConfiguredModel = catalogStore.getConfiguredModelById(configuredModel.configuredModelId)
      const provider = draftProviderOverrides.value[configuredModel.providerId]
        ?? catalogStore.getProviderById(configuredModel.providerId)
      const model = draftModelOverrides.value[configuredModel.modelId]
        ?? catalogStore.getModelById(configuredModel.modelId)
      const totalTokens = configuredModel.tokenQuota?.totalTokens ?? liveConfiguredModel?.tokenQuota?.totalTokens
      const usedTokens = liveConfiguredModel?.tokenUsage.usedTokens ?? configuredModel.tokenUsage.usedTokens ?? 0
      const hasDiagnostics = catalogStore.diagnostics.warnings.some(item =>
        item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.providerId) || item.includes(configuredModel.modelId))
        || catalogStore.diagnostics.errors.some(item =>
          item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.providerId) || item.includes(configuredModel.modelId))

      return {
        configuredModelId: configuredModel.configuredModelId,
        name: configuredModel.name,
        providerId: configuredModel.providerId,
        providerLabel: provider?.label ?? configuredModel.providerId,
        modelId: configuredModel.modelId,
        modelLabel: model?.label ?? configuredModel.modelId,
        description: model?.description ?? '',
        family: model?.family ?? '',
        track: model?.track ?? '',
        enabled: configuredModel.enabled,
        source: configuredModel.source,
        surfaces: model?.surfaceBindings.filter(binding => binding.enabled).map(binding => binding.surface) ?? [],
        capabilities: model?.capabilities.map(capability => capability.capabilityId) ?? [],
        defaultSurfaces: [],
        contextWindow: model?.contextWindow,
        maxOutputTokens: model?.maxOutputTokens,
        credentialRef: configuredModel.credentialRef,
        credentialStatus: (configuredModel.credentialRef ? 'configured' : 'unconfigured') as CatalogConfiguredModelRow['credentialStatus'],
        credentialConfigured: Boolean(configuredModel.credentialRef),
        baseUrl: configuredModel.baseUrl,
        totalTokens,
        usedTokens,
        remainingTokens: totalTokens ? Math.max(0, totalTokens - usedTokens) : undefined,
        quotaExhausted: totalTokens ? usedTokens >= totalTokens : false,
        hasDiagnostics,
      }
    })
    .sort((left, right) => left.name.localeCompare(right.name)),
)

const localFilterOptions = computed(() => {
  const providersMap = new Map<string, string>()
  const surfacesMap = new Map<string, string>()
  const capabilitiesMap = new Map<string, string>()

  for (const row of localRows.value) {
    providersMap.set(row.providerId, row.providerLabel)
    for (const surface of row.surfaces) {
      surfacesMap.set(surface, surface)
    }
    for (const capability of row.capabilities) {
      capabilitiesMap.set(capability, capability)
    }
  }

  return {
    providers: [...providersMap.entries()]
      .sort((left, right) => left[1].localeCompare(right[1]))
      .map(([value, label]) => ({ value, label })),
    surfaces: [...surfacesMap.keys()]
      .sort((left, right) => left.localeCompare(right))
      .map(value => ({ value, label: value })),
    capabilities: [...capabilitiesMap.keys()]
      .sort((left, right) => left.localeCompare(right))
      .map(value => ({ value, label: value })),
  }
})

const filteredRows = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return localRows.value.filter((row) => {
    if (providerFilter.value && row.providerId !== providerFilter.value) {
      return false
    }
    if (surfaceFilter.value && !row.surfaces.includes(surfaceFilter.value)) {
      return false
    }
    if (capabilityFilter.value && !row.capabilities.includes(capabilityFilter.value)) {
      return false
    }
    if (!query) {
      return true
    }
    const haystack = [
      row.name,
      row.providerId,
      row.providerLabel,
      row.modelId,
      row.modelLabel,
      row.family,
      row.track,
      row.surfaces.join(' '),
      row.capabilities.join(' '),
    ].join(' ').toLowerCase()
    return haystack.includes(query)
  })
})

const pageCount = computed(() => Math.max(1, Math.ceil(filteredRows.value.length / PAGE_SIZE)))
const pagedRows = computed(() => {
  const start = (page.value - 1) * PAGE_SIZE
  return filteredRows.value.slice(start, start + PAGE_SIZE)
})

const selectedRow = computed(() =>
  localRows.value.find(row => row.configuredModelId === selectedConfiguredModelId.value)
  ?? filteredRows.value[0]
  ?? localRows.value[0]
  ?? null,
)
const selectedConfiguredModel = computed(() =>
  selectedRow.value ? draftConfiguredModels.value[selectedRow.value.configuredModelId] ?? null : null,
)
const selectedModel = computed(() =>
  selectedRow.value
    ? draftModelOverrides.value[selectedRow.value.modelId] ?? catalogStore.getModelById(selectedRow.value.modelId) ?? null
    : null,
)
const selectedProvider = computed(() =>
  selectedRow.value
    ? draftProviderOverrides.value[selectedRow.value.providerId] ?? catalogStore.getProviderById(selectedRow.value.providerId) ?? null
    : null,
)
const selectedProviderType = computed(() =>
  selectedRow.value && selectedProvider.value && selectedModel.value
    ? resolveProviderType(selectedRow.value.providerId, selectedProvider.value, selectedModel.value)
    : 'standard',
)
const selectedIsCustomManaged = computed(() =>
  selectedProviderType.value === CUSTOM_PROVIDER_MODE
  && Boolean(selectedProvider.value)
  && isManagedByPage(selectedProvider.value?.metadata),
)
const selectedProbeResult = computed(() =>
  runtime.configuredModelProbeResult?.configuredModelId === selectedConfiguredModelId.value
    ? runtime.configuredModelProbeResult
    : null,
)

const patchDocument = computed<Record<string, JsonValue>>(() => {
  const patch: Record<string, JsonValue> = {}
  const configuredModelsPatch = toMergePatch(
    toPersistedConfiguredModelMap(workspaceConfiguredModelsCurrent.value),
    toPersistedConfiguredModelMap(draftConfiguredModels.value),
  )
  if (configuredModelsPatch !== undefined) {
    patch.configuredModels = configuredModelsPatch
  }

  const modelRegistryPatch: Record<string, JsonValue> = {}
  const providersPatch = toMergePatch(workspaceProviderOverridesCurrent.value, draftProviderOverrides.value)
  if (providersPatch !== undefined) {
    modelRegistryPatch.providers = providersPatch
  }
  const modelsPatch = toMergePatch(workspaceModelOverridesCurrent.value, draftModelOverrides.value)
  if (modelsPatch !== undefined) {
    modelRegistryPatch.models = modelsPatch
  }
  if (Object.keys(modelRegistryPatch).length > 0) {
    patch.modelRegistry = modelRegistryPatch
  }

  return patch
})

const patchPreview = computed(() => JSON.stringify(patchDocument.value, null, 2))
const hasPendingPatch = computed(() => Object.keys(patchDocument.value).length > 0)

function initializeDrafts() {
  draftConfiguredModels.value = cloneJson(workspaceConfiguredModelsCurrent.value)
  draftProviderOverrides.value = cloneJson(workspaceProviderOverridesCurrent.value)
  draftModelOverrides.value = cloneJson(workspaceModelOverridesCurrent.value)
}

function ensureSelectedRow() {
  if (!selectedConfiguredModelId.value || !draftConfiguredModels.value[selectedConfiguredModelId.value]) {
    selectedConfiguredModelId.value = filteredRows.value[0]?.configuredModelId ?? localRows.value[0]?.configuredModelId ?? ''
  }
}

function updateSelectedConfiguredModel(patch: Partial<ConfiguredModelRecord>) {
  const current = selectedConfiguredModel.value
  if (!current) {
    return
  }
  runtime.clearConfiguredModelProbeResult()
  draftConfiguredModels.value = {
    ...draftConfiguredModels.value,
    [current.configuredModelId]: {
      ...current,
      ...patch,
    },
  }
}

function updateSelectedTokenQuota(value: string) {
  const trimmed = value.trim()
  const totalTokens = trimmed ? Number(trimmed) : undefined
  updateSelectedConfiguredModel({
    tokenQuota: totalTokens && totalTokens > 0
      ? { totalTokens }
      : undefined,
  })
}

function updateSelectedBaseUrl(value: string) {
  const trimmed = value.trim()
  updateSelectedConfiguredModel({ baseUrl: trimmed || undefined })

  if (selectedIsCustomManaged.value && selectedProvider.value) {
    draftProviderOverrides.value = {
      ...draftProviderOverrides.value,
      [selectedProvider.value.providerId]: updateProviderBaseUrl(selectedProvider.value, trimmed || CUSTOM_BASE_URL_PLACEHOLDER),
    }
  }
}

function updateSelectedCustomProviderLabel(value: string) {
  if (!selectedProvider.value || !selectedIsCustomManaged.value) {
    return
  }
  runtime.clearConfiguredModelProbeResult()
  draftProviderOverrides.value = {
    ...draftProviderOverrides.value,
    [selectedProvider.value.providerId]: {
      ...selectedProvider.value,
      label: value,
    },
  }
}

function cleanupUnusedManagedEntries(
  nextConfiguredModels: Record<string, ConfiguredModelRecord>,
  removedModelId: string,
  removedProviderId: string,
) {
  const hasModelReference = Object.values(nextConfiguredModels).some(model => model.modelId === removedModelId)
  if (!hasModelReference) {
    const draftModel = draftModelOverrides.value[removedModelId]
    if (draftModel && isManagedByPage(draftModel.metadata)) {
      const nextModels = { ...draftModelOverrides.value }
      delete nextModels[removedModelId]
      draftModelOverrides.value = nextModels
    }
  }

  const hasProviderReference = Object.values(nextConfiguredModels).some(model => model.providerId === removedProviderId)
  if (!hasProviderReference) {
    const draftProvider = draftProviderOverrides.value[removedProviderId]
    if (draftProvider && isManagedByPage(draftProvider.metadata)) {
      const nextProviders = { ...draftProviderOverrides.value }
      delete nextProviders[removedProviderId]
      draftProviderOverrides.value = nextProviders
    }
  }
}

async function deleteSelectedConfiguredModel() {
  if (!selectedConfiguredModel.value) {
    return
  }
  const previousConfiguredModels = cloneJson(draftConfiguredModels.value)
  const previousProviderOverrides = cloneJson(draftProviderOverrides.value)
  const previousModelOverrides = cloneJson(draftModelOverrides.value)
  const previousSelectedConfiguredModelId = selectedConfiguredModelId.value
  const next = { ...draftConfiguredModels.value }
  const removed = next[selectedConfiguredModel.value.configuredModelId]
  delete next[selectedConfiguredModel.value.configuredModelId]
  draftConfiguredModels.value = next
  cleanupUnusedManagedEntries(next, removed.modelId, removed.providerId)
  selectedConfiguredModelId.value = ''

  syncRuntimeDraft()
  const saved = await runtime.saveConfig('workspace')
  if (!saved) {
    draftConfiguredModels.value = previousConfiguredModels
    draftProviderOverrides.value = previousProviderOverrides
    draftModelOverrides.value = previousModelOverrides
    selectedConfiguredModelId.value = previousSelectedConfiguredModelId
    syncRuntimeDraft()
    return
  }

  detailDialogOpen.value = false
  await refreshWorkspaceModels()
}

function openCreateDialog() {
  createName.value = ''
  createProviderType.value = createProviderOptions.value[0]?.value ?? ''
  createCustomProviderLabel.value = ''
  createModelId.value = ''
  createCredentialRef.value = ''
  createBaseUrl.value = createBaseUrlDefault.value
  createTotalTokens.value = ''
  createEnabled.value = true
  createFormError.value = ''
  createDialogOpen.value = true
}

function createManagedRegistryModelId(providerId: string, upstreamModelId: string) {
  return `${providerId}::${upstreamModelId.trim()}`
}

async function createConfiguredModel() {
  createFormError.value = ''
  const name = createName.value.trim()
  const providerType = createProviderType.value.trim()
  const customProviderLabel = createCustomProviderLabel.value.trim()
  const upstreamModelId = createModelId.value.trim()

  if (!name || !providerType || !upstreamModelId) {
    createFormError.value = t('models.create.errors.required')
    return
  }
  if (providerType === CUSTOM_PROVIDER_MODE && !customProviderLabel) {
    createFormError.value = t('models.create.errors.customProviderNameRequired')
    return
  }
  const duplicate = Object.values(draftConfiguredModels.value).some(model =>
    model.source === 'workspace'
    && model.name.trim().toLowerCase() === name.toLowerCase())
  if (duplicate) {
    createFormError.value = t('models.create.errors.duplicateName')
    return
  }

  const providerId = providerType === CUSTOM_PROVIDER_MODE
    ? createCustomProviderId(customProviderLabel)
    : providerType
  const registryModelId = isSpecialProviderType(providerType)
    ? createManagedRegistryModelId(providerId, upstreamModelId)
    : upstreamModelId
  const baseUrl = createBaseUrl.value.trim() || createBaseUrlDefault.value || undefined
  const totalTokensValue = createTotalTokens.value.trim()
  const totalTokens = totalTokensValue ? Number(totalTokensValue) : undefined
  const configuredModelId = createConfiguredModelId(providerId, registryModelId)
  const previousConfiguredModels = cloneJson(draftConfiguredModels.value)
  const previousProviderOverrides = cloneJson(draftProviderOverrides.value)
  const previousModelOverrides = cloneJson(draftModelOverrides.value)
  const previousSelectedConfiguredModelId = selectedConfiguredModelId.value

  if (providerType === CUSTOM_PROVIDER_MODE) {
    draftProviderOverrides.value = {
      ...draftProviderOverrides.value,
      [providerId]: buildManagedProviderOverride(
        providerId,
        customProviderLabel,
        baseUrl ?? CUSTOM_BASE_URL_PLACEHOLDER,
        CUSTOM_PROVIDER_MODE,
      ),
    }
  }

  if (isSpecialProviderType(providerType)) {
    draftModelOverrides.value = {
      ...draftModelOverrides.value,
      [registryModelId]: buildManagedModelOverride(
        registryModelId,
        providerId,
        upstreamModelId,
        providerType,
        upstreamModelId,
      ),
    }
  }

  draftConfiguredModels.value = {
    ...draftConfiguredModels.value,
    [configuredModelId]: {
      configuredModelId,
      name,
      providerId,
      modelId: registryModelId,
      credentialRef: createCredentialRef.value.trim() || undefined,
      baseUrl,
      tokenQuota: totalTokens && totalTokens > 0 ? { totalTokens } : undefined,
      tokenUsage: {
        usedTokens: 0,
        remainingTokens: totalTokens && totalTokens > 0 ? totalTokens : undefined,
        exhausted: false,
      },
      enabled: createEnabled.value,
      source: 'workspace',
      status: createCredentialRef.value.trim() ? 'configured' : 'unconfigured',
      configured: Boolean(createCredentialRef.value.trim()),
    },
  }
  selectedConfiguredModelId.value = configuredModelId
  syncRuntimeDraft()

  const saved = await runtime.saveConfig('workspace')
  if (!saved) {
    draftConfiguredModels.value = previousConfiguredModels
    draftProviderOverrides.value = previousProviderOverrides
    draftModelOverrides.value = previousModelOverrides
    selectedConfiguredModelId.value = previousSelectedConfiguredModelId
    syncRuntimeDraft()
    createFormError.value = runtime.configError
      || runtime.configValidation.workspace?.errors[0]
      || t('models.create.errors.saveFailed')
    return
  }

  await refreshWorkspaceModels()
  createDialogOpen.value = false
  detailDialogOpen.value = true
}

function syncRuntimeDraft() {
  runtime.setConfigDraft('workspace', patchPreview.value)
}

async function refreshWorkspaceModels() {
  const connectionId = shell.activeWorkspaceConnectionId
  await Promise.all([
    runtime.loadConfig(true),
    connectionId ? catalogStore.load(connectionId) : Promise.resolve(),
  ])
}

async function validateWorkspacePatch() {
  syncRuntimeDraft()
  return await runtime.validateConfig('workspace')
}

async function validateSelectedConfiguredModel() {
  if (!selectedConfiguredModel.value) {
    return
  }
  syncRuntimeDraft()
  const result = await runtime.probeConfiguredModel('workspace', selectedConfiguredModel.value.configuredModelId)
  if (result.valid && result.reachable) {
    await refreshWorkspaceModels()
  }
}

async function saveWorkspacePatch() {
  const validation = await validateWorkspacePatch()
  if (!validation.valid) {
    return
  }

  const saved = await runtime.saveConfig('workspace')
  if (!saved) {
    return
  }

  await refreshWorkspaceModels()
}

function selectRow(row: CatalogConfiguredModelRow) {
  runtime.clearConfiguredModelProbeResult()
  selectedConfiguredModelId.value = row.configuredModelId
  detailDialogOpen.value = true
}

const columns = computed(() => [
  {
    id: 'name',
    header: t('models.table.columns.name'),
    accessorKey: 'name',
    cell: ({ row }: { row: { original: CatalogConfiguredModelRow } }) => h('div', { class: 'space-y-1 min-w-[220px]' }, [
      h('div', { class: 'font-semibold text-text-primary' }, row.original.name),
      h('div', { class: 'text-xs text-text-secondary' }, `${row.original.modelLabel} · ${row.original.providerLabel}`),
    ]),
  },
  {
    id: 'provider',
    header: t('models.table.columns.provider'),
    accessorKey: 'providerLabel',
  },
  {
    id: 'upstream',
    header: t('models.table.columns.upstreamModel'),
    accessorKey: 'modelLabel',
  },
  {
    id: 'surfaces',
    header: t('models.table.columns.surfaces'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.surfaces.join(', ') || '—',
  },
  {
    id: 'usedTokens',
    header: t('models.table.columns.usedTokens'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.usedTokens.toLocaleString(),
  },
  {
    id: 'totalTokens',
    header: t('models.table.columns.totalTokens'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.totalTokens?.toLocaleString() || t('models.quota.unlimited'),
  },
  {
    id: 'quotaStatus',
    header: t('models.table.columns.quotaStatus'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.totalTokens
      ? (row.quotaExhausted ? t('models.quota.exhausted') : t('models.quota.available'))
      : t('models.quota.unlimited'),
  },
  {
    id: 'credentialRef',
    header: t('models.table.columns.credentialRef'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.credentialRef || '—',
  },
  {
    id: 'baseUrl',
    header: t('models.table.columns.baseUrl'),
    accessorFn: (row: CatalogConfiguredModelRow) => row.baseUrl || '—',
  },
])

watch(
  () => shell.activeWorkspaceConnectionId,
  async (connectionId, previousConnectionId) => {
    if (!connectionId) {
      return
    }

    await Promise.all([
      catalogStore.load(connectionId),
      runtime.loadConfig(previousConnectionId !== connectionId),
    ])
  },
  { immediate: true },
)

watch(
  () => ({
    configHash: runtime.config?.effectiveConfigHash ?? '',
    catalogHash: JSON.stringify(catalogStore.snapshot),
  }),
  () => {
    initializeDrafts()
    ensureSelectedRow()
  },
  { immediate: true },
)

watch(filteredRows, () => {
  if (page.value > pageCount.value) {
    page.value = pageCount.value
  }
  ensureSelectedRow()
})

watch(
  () => [searchQuery.value, providerFilter.value, surfaceFilter.value, capabilityFilter.value],
  () => {
    page.value = 1
    runtime.clearConfiguredModelProbeResult()
  },
)

watch(createProviderType, () => {
  createFormError.value = ''
  if (!createUsesFreeformModel.value && !createUpstreamModelOptions.value.some(option => option.value === createModelId.value)) {
    createModelId.value = createUpstreamModelOptions.value[0]?.value ?? ''
  }
  if (createUsesFreeformModel.value) {
    createModelId.value = ''
  }
  createBaseUrl.value = createBaseUrlDefault.value
}, { immediate: true })
</script>

<template>
  <UiPageShell width="wide" test-id="workspace-models-view">
    <UiPageHeader
      :eyebrow="t('models.header.eyebrow')"
      :title="t('models.header.title')"
      :description="t('models.header.subtitle')"
    >
      <template #actions>
        <UiButton data-testid="models-create-button" size="sm" @click="openCreateDialog">
          <Plus :size="14" />
          {{ t('models.actions.create') }}
        </UiButton>
      </template>
    </UiPageHeader>

    <section>
      <UiSurface variant="raised" padding="md">
        <UiDataTable
          :data="pagedRows"
          :columns="columns"
          row-test-id="models-table-row"
          :empty-title="t('models.empty.title')"
          :empty-description="t('models.empty.description')"
          :on-row-click="selectRow"
        >
          <template #toolbar>
            <div
              data-testid="models-filters"
              class="grid min-w-0 w-full gap-3 pb-3 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-center"
            >
              <div class="flex min-w-0 flex-wrap items-center gap-3 xl:flex-nowrap">
                <UiInput
                  v-model="searchQuery"
                  data-testid="models-search-input"
                  class="min-w-[260px] flex-[1.35_1_320px]"
                  :placeholder="t('models.filters.searchPlaceholder')"
                />
                <UiSelect
                  v-model="providerFilter"
                  data-testid="models-provider-filter"
                  class="min-w-[150px] flex-[0_0_180px]"
                  :options="[{ value: '', label: t('models.filters.allProviders') }, ...localFilterOptions.providers]"
                />
                <UiSelect
                  v-model="surfaceFilter"
                  data-testid="models-surface-filter"
                  class="min-w-[150px] flex-[0_0_180px]"
                  :options="[{ value: '', label: t('models.filters.allSurfaces') }, ...localFilterOptions.surfaces]"
                />
                <UiSelect
                  v-model="capabilityFilter"
                  data-testid="models-capability-filter"
                  class="min-w-[150px] flex-[0_0_180px]"
                  :options="[{ value: '', label: t('models.filters.allCapabilities') }, ...localFilterOptions.capabilities]"
                />
              </div>
              <div class="flex justify-end text-[12px] text-text-tertiary">
                {{ t('models.pagination.summary', { count: filteredRows.length, page, total: pageCount }) }}
              </div>
            </div>
          </template>
        </UiDataTable>

        <div class="mt-4">
          <UiPagination
            v-model:page="page"
            data-testid="models-pagination"
            root-test-id="models-pagination"
            :page-count="pageCount"
            :summary-label="t('models.pagination.summary', { count: filteredRows.length, page, total: pageCount })"
          />
        </div>
      </UiSurface>
    </section>

    <UiDialog
      v-model:open="detailDialogOpen"
      :title="selectedConfiguredModel?.name ?? t('models.empty.selectionTitle')"
      :description="selectedProvider && selectedModel ? `${selectedProvider.label} · ${selectedModel.label}` : t('models.empty.selectionDescription')"
      content-test-id="models-detail-dialog"
    >
      <div
        v-if="selectedRow && selectedConfiguredModel && selectedModel && selectedProvider"
        data-testid="models-detail-panel"
        class="space-y-5"
      >
        <div class="space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <UiBadge :label="selectedProvider.label" subtle />
            <UiBadge :label="selectedModel.label" subtle />
            <UiBadge :label="selectedConfiguredModel.enabled ? t('models.states.enabled') : t('models.states.disabled')" subtle />
          </div>
          <p class="text-sm text-text-secondary">
            {{ selectedModel.description || t('models.detail.noDescription') }}
          </p>
        </div>

        <div class="grid gap-3 sm:grid-cols-2">
          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.name') }}
            </p>
            <UiInput
              :model-value="selectedConfiguredModel.name"
              data-testid="models-detail-name-input"
              @update:model-value="updateSelectedConfiguredModel({ name: String($event) })"
            />
          </div>

          <div v-if="selectedIsCustomManaged" class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.customProviderName') }}
            </p>
            <UiInput
              :model-value="selectedProvider.label"
              data-testid="models-detail-provider-label-input"
              @update:model-value="updateSelectedCustomProviderLabel(String($event))"
            />
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.provider') }}
            </p>
            <p class="text-sm text-text-primary">{{ selectedProvider.label }}</p>
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.upstreamModel') }}
            </p>
            <p class="text-sm text-text-primary">{{ selectedModel.label }}</p>
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.credentialRef') }}
            </p>
            <UiInput
              :model-value="selectedConfiguredModel.credentialRef ?? ''"
              data-testid="models-detail-credential-ref"
              :placeholder="t('models.detail.credentialRefPlaceholder')"
              @update:model-value="updateSelectedConfiguredModel({ credentialRef: String($event).trim() || undefined })"
            />
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.baseUrl') }}
            </p>
            <UiInput
              :model-value="selectedConfiguredModel.baseUrl ?? selectedProvider.surfaces[0]?.baseUrl ?? ''"
              data-testid="models-detail-base-url"
              :placeholder="t('models.detail.baseUrlPlaceholder')"
              @update:model-value="updateSelectedBaseUrl(String($event))"
            />
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.totalTokens') }}
            </p>
            <UiInput
              :model-value="selectedConfiguredModel.tokenQuota?.totalTokens ? String(selectedConfiguredModel.tokenQuota.totalTokens) : ''"
              data-testid="models-detail-total-tokens"
              type="number"
              :placeholder="t('models.detail.totalTokensPlaceholder')"
              @update:model-value="updateSelectedTokenQuota(String($event))"
            />
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.usedTokens') }}
            </p>
            <p class="text-sm text-text-primary">
              {{ selectedRow.usedTokens.toLocaleString() }}
            </p>
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.remainingTokens') }}
            </p>
            <p class="text-sm text-text-primary">
              {{ selectedRow.remainingTokens?.toLocaleString() ?? t('models.quota.unlimited') }}
            </p>
          </div>

          <div class="space-y-1">
            <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
              {{ t('models.detail.quotaStatus') }}
            </p>
            <UiBadge
              :label="selectedRow.totalTokens
                ? (selectedRow.quotaExhausted ? t('models.quota.exhausted') : t('models.quota.available'))
                : t('models.quota.unlimited')"
              subtle
            />
          </div>
        </div>

        <UiSurface variant="subtle" padding="sm">
          <div class="space-y-3">
            <div class="space-y-2">
              <h4 class="text-sm font-semibold text-text-primary">{{ t('models.detail.capabilities') }}</h4>
              <div class="flex flex-wrap gap-2">
                <UiBadge
                  v-for="capability in selectedModel.capabilities"
                  :key="capability.capabilityId"
                  :label="capability.label || capability.capabilityId"
                  subtle
                />
                <p v-if="!selectedModel.capabilities.length" class="text-sm text-text-secondary">
                  {{ t('models.detail.noCapabilities') }}
                </p>
              </div>
            </div>

            <div class="space-y-2">
              <h4 class="text-sm font-semibold text-text-primary">{{ t('models.detail.surfaces') }}</h4>
              <div class="flex flex-wrap gap-2">
                <UiBadge
                  v-for="binding in selectedModel.surfaceBindings.filter(item => item.enabled)"
                  :key="binding.surface"
                  :label="binding.surface"
                  subtle
                />
                <p v-if="!selectedModel.surfaceBindings.some(item => item.enabled)" class="text-sm text-text-secondary">
                  {{ t('models.detail.noSurfaces') }}
                </p>
              </div>
            </div>
          </div>
        </UiSurface>

        <UiSurface variant="subtle" padding="sm">
          <div class="space-y-3">
            <UiCheckbox
              :model-value="selectedConfiguredModel.enabled"
              :label="t('models.detail.enabled')"
              data-testid="models-detail-enabled"
              @update:model-value="updateSelectedConfiguredModel({ enabled: Boolean($event) })"
            />

            <div class="flex flex-wrap items-center gap-2">
              <UiButton
                data-testid="models-validate-button"
                variant="ghost"
                size="sm"
                :disabled="runtime.configValidating || runtime.configuredModelProbing"
                @click="validateSelectedConfiguredModel"
              >
                {{ t('models.actions.validate') }}
              </UiButton>
              <UiButton
                data-testid="models-save-button"
                size="sm"
                :disabled="runtime.configSaving || !hasPendingPatch"
                @click="saveWorkspacePatch"
              >
                {{ t('models.actions.save') }}
              </UiButton>
              <UiButton
                variant="ghost"
                size="sm"
                class="justify-start text-status-error"
                data-testid="models-delete-button"
                @click="deleteSelectedConfiguredModel"
              >
                <Trash2 :size="14" />
                {{ t('models.actions.delete') }}
              </UiButton>
            </div>

            <UiStatusCallout
              v-if="runtime.configValidation.workspace?.errors.length"
              tone="error"
              :description="runtime.configValidation.workspace.errors.join(' ')"
            />
            <UiStatusCallout
              v-if="runtime.configValidation.workspace?.warnings.length"
              tone="warning"
              :description="runtime.configValidation.workspace.warnings.join(' ')"
            />
            <p
              v-if="selectedProbeResult?.reachable"
              data-testid="models-validate-success"
              class="text-sm text-status-success"
            >
              {{ t('models.validation.success', {
                name: selectedProbeResult.configuredModelName ?? selectedConfiguredModel.name,
                tokens: selectedProbeResult.consumedTokens ?? 0,
              }) }}
              <span v-if="selectedProbeResult.requestId">
                {{ t('models.validation.requestId', { requestId: selectedProbeResult.requestId }) }}
              </span>
            </p>
          </div>
        </UiSurface>
      </div>

      <UiEmptyState
        v-else
        :title="t('models.empty.selectionTitle')"
        :description="t('models.empty.selectionDescription')"
      />
    </UiDialog>

    <UiDialog
      v-model:open="createDialogOpen"
      :title="t('models.create.title')"
      :description="t('models.create.description')"
      content-test-id="models-create-dialog"
    >
      <div class="grid gap-3">
        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.name') }}</p>
          <UiInput v-model="createName" data-testid="models-create-name-input" />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.provider') }}</p>
          <UiSelect
            v-model="createProviderType"
            data-testid="models-create-provider-select"
            :options="createProviderOptions"
          />
        </div>

        <div v-if="createRequiresCustomProviderName" class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.customProviderName') }}</p>
          <UiInput
            v-model="createCustomProviderLabel"
            data-testid="models-create-custom-provider-name-input"
            :placeholder="t('models.create.placeholders.customProviderName')"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.model') }}</p>
          <UiSelect
            v-if="!createUsesFreeformModel"
            v-model="createModelId"
            data-testid="models-create-upstream-model-select"
            :options="createUpstreamModelOptions"
          />
          <UiInput
            v-else
            v-model="createModelId"
            data-testid="models-create-upstream-model-input"
            :placeholder="t('models.create.placeholders.modelId')"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.credentialRef') }}</p>
          <UiInput
            v-model="createCredentialRef"
            data-testid="models-create-credential-ref-input"
            :placeholder="t('models.detail.credentialRefPlaceholder')"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.baseUrl') }}</p>
          <UiInput
            v-model="createBaseUrl"
            data-testid="models-create-base-url-input"
            :placeholder="t('models.detail.baseUrlPlaceholder')"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.totalTokens') }}</p>
          <UiInput
            v-model="createTotalTokens"
            data-testid="models-create-total-tokens-input"
            type="number"
            :placeholder="t('models.create.placeholders.totalTokens')"
          />
        </div>

        <UiCheckbox
          :model-value="createEnabled"
          :label="t('models.create.fields.enabled')"
          data-testid="models-create-enabled"
          @update:model-value="createEnabled = Boolean($event)"
        />

        <UiStatusCallout v-if="createFormError" tone="error" :description="createFormError" />
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="runtime.configSaving || runtime.configValidating" @click="createDialogOpen = false">
          {{ t('models.actions.cancel') }}
        </UiButton>
        <UiButton :disabled="runtime.configSaving || runtime.configValidating" @click="createConfiguredModel">
          {{ t('models.actions.confirmCreate') }}
        </UiButton>
      </template>
    </UiDialog>
  </UiPageShell>
</template>
