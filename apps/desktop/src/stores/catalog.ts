import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CapabilityDescriptor,
  CopyWorkspaceSkillToManagedInput,
  ConfiguredModelRecord,
  CredentialBinding,
  CreateWorkspaceSkillInput,
  DefaultSelection,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
  ModelCatalogSnapshot,
  ModelRegistryDiagnostics,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  ToolRecord,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpsertWorkspaceMcpServerInput,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceToolCatalogEntry,
  WorkspaceToolCatalogSnapshot,
  WorkspaceToolDisablePatch,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

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

const EMPTY_SNAPSHOT: ModelCatalogSnapshot = {
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

function normalizeSnapshot(snapshot?: Partial<ModelCatalogSnapshot> | null): ModelCatalogSnapshot {
  return {
    providers: snapshot?.providers ?? [],
    models: snapshot?.models ?? [],
    configuredModels: (snapshot?.configuredModels ?? []).map(normalizeConfiguredModel),
    credentialBindings: snapshot?.credentialBindings ?? [],
    defaultSelections: snapshot?.defaultSelections ?? {},
    diagnostics: {
      warnings: snapshot?.diagnostics?.warnings ?? [],
      errors: snapshot?.diagnostics?.errors ?? [],
    },
  }
}

function sortFilterOptions(values: Map<string, string>): CatalogFilterOption[] {
  return [...values.entries()]
    .sort((left, right) => left[1].localeCompare(right[1]))
    .map(([value, label]) => ({ value, label }))
}

function resolveCapabilityLabel(capability: CapabilityDescriptor): string {
  return capability.label || capability.capabilityId
}

function toModelRow(
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
      .filter(binding => binding.enabled)
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

function toConfiguredModelRow(
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
    surfaces: model?.surfaceBindings.filter(binding => binding.enabled).map(binding => binding.surface) ?? [],
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

export const useCatalogStore = defineStore('catalog', () => {
  const snapshots = ref<Record<string, ModelCatalogSnapshot>>({})
  const toolCatalogsByConnection = ref<Record<string, WorkspaceToolCatalogSnapshot>>({})
  const skillDocumentsByConnection = ref<Record<string, Record<string, WorkspaceSkillDocument>>>({})
  const skillTreesByConnection = ref<Record<string, Record<string, WorkspaceSkillTreeDocument>>>({})
  const skillFilesByConnection = ref<Record<string, Record<string, WorkspaceSkillFileDocument>>>({})
  const mcpDocumentsByConnection = ref<Record<string, Record<string, WorkspaceMcpServerDocument>>>({})
  const toolsByConnection = ref<Record<string, ToolRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const snapshot = computed<ModelCatalogSnapshot>(() => snapshots.value[activeConnectionId.value] ?? EMPTY_SNAPSHOT)
  const toolCatalog = computed<WorkspaceToolCatalogSnapshot>(() => toolCatalogsByConnection.value[activeConnectionId.value] ?? { entries: [] })
  const toolCatalogEntries = computed<WorkspaceToolCatalogEntry[]>(() => toolCatalog.value.entries)
  const providers = computed<ProviderRegistryRecord[]>(() => snapshot.value.providers)
  const models = computed<ModelRegistryRecord[]>(() => snapshot.value.models)
  const configuredModels = computed<ConfiguredModelRecord[]>(() => snapshot.value.configuredModels)
  const credentialBindings = computed<CredentialBinding[]>(() => snapshot.value.credentialBindings)
  const defaultSelections = computed<Record<string, DefaultSelection>>(() => snapshot.value.defaultSelections)
  const diagnostics = computed<ModelRegistryDiagnostics>(() => snapshot.value.diagnostics)
  const tools = computed(() => toolsByConnection.value[activeConnectionId.value] ?? [])
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  const providerMap = computed(() => new Map(providers.value.map(provider => [provider.providerId, provider])))
  const modelMap = computed(() => new Map(models.value.map(model => [model.modelId, model])))
  const credentialMap = computed(() => new Map(credentialBindings.value.map(binding => [binding.providerId, binding])))
  const diagnosticSummary = computed<CatalogDiagnosticSummary>(() => {
    const warningCount = diagnostics.value.warnings.length
    const errorCount = diagnostics.value.errors.length
    return {
      warningCount,
      errorCount,
      totalCount: warningCount + errorCount,
      hasIssues: warningCount + errorCount > 0,
    }
  })

  const modelRows = computed<CatalogModelRow[]>(() => models.value.map((model) => {
    const provider = providerMap.value.get(model.providerId)
    const binding = credentialMap.value.get(model.providerId)
    const hasDiagnostics = diagnostics.value.warnings.some(item => item.includes(model.modelId) || item.includes(model.providerId))
      || diagnostics.value.errors.some(item => item.includes(model.modelId) || item.includes(model.providerId))
    return toModelRow(
      model,
      provider?.label ?? model.providerId,
      defaultSelections.value,
      binding,
      hasDiagnostics,
    )
  }))

  const configuredModelRows = computed<CatalogConfiguredModelRow[]>(() => configuredModels.value.map((configuredModel) => {
    const provider = providerMap.value.get(configuredModel.providerId)
    const model = modelMap.value.get(configuredModel.modelId)
    const hasDiagnostics = diagnostics.value.warnings.some(item =>
      item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.modelId) || item.includes(configuredModel.providerId))
      || diagnostics.value.errors.some(item =>
        item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.modelId) || item.includes(configuredModel.providerId))
    return toConfiguredModelRow(
      configuredModel,
      provider?.label ?? configuredModel.providerId,
      model,
      defaultSelections.value,
      hasDiagnostics,
    )
  }))

  const workspaceConfiguredModelRows = computed<CatalogConfiguredModelRow[]>(() =>
    configuredModelRows.value.filter(model => model.source === 'workspace'))

  const providerSummaries = computed<CatalogProviderSummary[]>(() => providers.value.map((provider) => {
    const binding = credentialMap.value.get(provider.providerId)
    const relatedModels = configuredModelRows.value.filter(model => model.providerId === provider.providerId)
    const defaultSurfaces = Object.values(defaultSelections.value)
      .filter(selection => selection.providerId === provider.providerId)
      .map(selection => selection.surface)
    const hasIssues = !provider.enabled
      || (binding ? binding.status !== 'healthy' && binding.status !== 'configured' : relatedModels.length > 0)
      || diagnostics.value.warnings.some(item => item.includes(provider.providerId))
      || diagnostics.value.errors.some(item => item.includes(provider.providerId))

    return {
      providerId: provider.providerId,
      label: provider.label,
      enabled: provider.enabled,
      modelCount: relatedModels.length,
      defaultSurfaces,
      credentialStatus: binding?.status ?? 'missing',
      credentialConfigured: binding?.configured ?? false,
      hasIssues,
      baseUrl: binding?.baseUrl,
    }
  }))

  const credentialSummaries = computed<CatalogCredentialSummary[]>(() => providers.value.map((provider) => {
    const binding = credentialMap.value.get(provider.providerId)
    return {
      providerId: provider.providerId,
      providerLabel: provider.label,
      credentialRef: binding?.credentialRef,
      label: binding?.label ?? provider.label,
      status: binding?.status ?? 'missing',
      configured: binding?.configured ?? false,
      source: binding?.source ?? 'workspace',
      baseUrl: binding?.baseUrl,
      hasIssues: !binding || (binding.status !== 'healthy' && binding.status !== 'configured'),
    }
  }))

  const defaultSelectionRows = computed<CatalogDefaultSelectionRow[]>(() => Object.entries(defaultSelections.value)
    .map(([surface, selection]) => {
      const provider = providerMap.value.get(selection.providerId)
      const model = modelMap.value.get(selection.modelId)
      const configuredModel = configuredModels.value.find(item => item.configuredModelId === selection.configuredModelId)
      return {
        surface,
        configuredModelId: selection.configuredModelId,
        configuredModelName: configuredModel?.name,
        providerId: selection.providerId,
        providerLabel: provider?.label ?? selection.providerId,
        modelId: selection.modelId,
        modelLabel: configuredModel?.name ?? model?.label ?? selection.modelId,
      }
    })
    .sort((left, right) => left.surface.localeCompare(right.surface)))

  const filterOptions = computed(() => {
    const providersMap = new Map<string, string>()
    const surfacesMap = new Map<string, string>()
    const capabilitiesMap = new Map<string, string>()

    for (const provider of providers.value) {
      providersMap.set(provider.providerId, provider.label)
    }

    for (const model of models.value) {
      for (const binding of model.surfaceBindings) {
        surfacesMap.set(binding.surface, binding.surface)
      }
      for (const capability of model.capabilities) {
        capabilitiesMap.set(capability.capabilityId, resolveCapabilityLabel(capability))
      }
    }

    return {
      providers: sortFilterOptions(providersMap),
      surfaces: sortFilterOptions(surfacesMap),
      capabilities: sortFilterOptions(capabilitiesMap),
    }
  })

  const enabledProviderCount = computed(() => providerSummaries.value.filter(provider => provider.enabled).length)
  const defaultSurfaceCount = computed(() => defaultSelectionRows.value.length)
  const credentialIssueCount = computed(() => credentialSummaries.value.filter(item => item.hasIssues).length)
  const configuredModelOptions = computed<CatalogConfiguredModelOption[]>(() => configuredModelRows.value
    .filter(model => model.enabled)
    .map(model => ({
      value: model.configuredModelId,
      label: model.name,
      providerId: model.providerId,
      providerLabel: model.providerLabel,
      modelId: model.modelId,
      modelLabel: model.modelLabel,
    })))
  const workspaceConfiguredModelOptions = computed<CatalogConfiguredModelOption[]>(() => workspaceConfiguredModelRows.value
    .filter(model => model.enabled)
    .map(model => ({
      value: model.configuredModelId,
      label: model.name,
      providerId: model.providerId,
      providerLabel: model.providerLabel,
      modelId: model.modelId,
      modelLabel: model.modelLabel,
    })))
  const modelOptions = computed(() => configuredModelOptions.value.map(model => ({
    value: model.value,
    label: model.label,
  })))

  function getModelOptionsByProviderId(providerId: string): CatalogFilterOption[] {
    return models.value
      .filter(model => model.providerId === providerId)
      .map(model => ({
        value: model.modelId,
        label: model.label,
      }))
      .sort((left, right) => left.label.localeCompare(right.label))
  }

  function getProviderBaseUrl(providerId: string, preferredSurface = 'conversation') {
    const provider = getProviderById(providerId)
    if (!provider) {
      return undefined
    }
    return provider.surfaces.find(surface => surface.enabled && surface.surface === preferredSurface)?.baseUrl
      ?? provider.surfaces.find(surface => surface.enabled)?.baseUrl
  }

  function getModelById(modelId: string) {
    return models.value.find(model => model.modelId === modelId)
  }

  function getModelRowById(modelId: string) {
    return modelRows.value.find(model => model.modelId === modelId)
  }

  function getConfiguredModelById(configuredModelId: string) {
    return configuredModels.value.find(model => model.configuredModelId === configuredModelId)
  }

  function getConfiguredModelRowById(configuredModelId: string) {
    return configuredModelRows.value.find(model => model.configuredModelId === configuredModelId)
  }

  function getProviderById(providerId: string) {
    return providers.value.find(provider => provider.providerId === providerId)
  }

  function getCredentialByProviderId(providerId: string) {
    return credentialBindings.value.find(binding => binding.providerId === providerId)
  }

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const [nextSnapshot, nextToolCatalog, nextTools] = await Promise.all([
        client.catalog.getSnapshot(),
        client.catalog.getToolCatalog(),
        client.catalog.listTools(),
      ])
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      snapshots.value = {
        ...snapshots.value,
        [connectionId]: normalizeSnapshot(nextSnapshot),
      }
      toolCatalogsByConnection.value = {
        ...toolCatalogsByConnection.value,
        [connectionId]: nextToolCatalog,
      }
      toolsByConnection.value = {
        ...toolsByConnection.value,
        [connectionId]: nextTools,
      }
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace catalog',
        }
      }
    }
  }

  function replaceToolCatalog(connectionId: string, nextToolCatalog: WorkspaceToolCatalogSnapshot) {
    toolCatalogsByConnection.value = {
      ...toolCatalogsByConnection.value,
      [connectionId]: nextToolCatalog,
    }
  }

  async function refreshToolCatalog(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return { entries: [] } satisfies WorkspaceToolCatalogSnapshot
    }
    const snapshot = await resolvedClient.client.catalog.getToolCatalog()
    replaceToolCatalog(resolvedClient.connectionId, snapshot)
    return snapshot
  }

  async function setToolDisabled(patch: WorkspaceToolDisablePatch) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const snapshot = await client.catalog.setToolDisabled(patch)
    replaceToolCatalog(connectionId, snapshot)
    return snapshot
  }

  async function getSkillDocument(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkill(skillId)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    return document
  }

  async function getSkillTreeDocument(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkillTree(skillId)
    skillTreesByConnection.value = {
      ...skillTreesByConnection.value,
      [connectionId]: {
        ...(skillTreesByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    return document
  }

  function skillFileCacheKey(skillId: string, relativePath: string) {
    return `${skillId}:${relativePath}`
  }

  async function getSkillFileDocument(skillId: string, relativePath: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkillFile(skillId, relativePath)
    skillFilesByConnection.value = {
      ...skillFilesByConnection.value,
      [connectionId]: {
        ...(skillFilesByConnection.value[connectionId] ?? {}),
        [skillFileCacheKey(skillId, relativePath)]: document,
      },
    }
    return document
  }

  async function createSkill(input: CreateWorkspaceSkillInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.createSkill(input)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function updateSkill(skillId: string, input: UpdateWorkspaceSkillInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateSkill(skillId, input)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function updateSkillFile(
    skillId: string,
    relativePath: string,
    input: UpdateWorkspaceSkillFileInput,
  ) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateSkillFile(skillId, relativePath, input)
    skillFilesByConnection.value = {
      ...skillFilesByConnection.value,
      [connectionId]: {
        ...(skillFilesByConnection.value[connectionId] ?? {}),
        [skillFileCacheKey(skillId, relativePath)]: document,
      },
    }
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: await client.catalog.getSkill(skillId),
      },
    }
    skillTreesByConnection.value = {
      ...skillTreesByConnection.value,
      [connectionId]: {
        ...(skillTreesByConnection.value[connectionId] ?? {}),
        [skillId]: await client.catalog.getSkillTree(skillId),
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function importSkillArchive(input: ImportWorkspaceSkillArchiveInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.importSkillArchive(input)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function importSkillFolder(input: ImportWorkspaceSkillFolderInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.importSkillFolder(input)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function copySkillToManaged(skillId: string, input: CopyWorkspaceSkillToManagedInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.copySkillToManaged(skillId, input)
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: {
        ...(skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function deleteSkill(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteSkill(skillId)
    const nextDocuments = { ...(skillDocumentsByConnection.value[connectionId] ?? {}) }
    delete nextDocuments[skillId]
    const nextTrees = { ...(skillTreesByConnection.value[connectionId] ?? {}) }
    delete nextTrees[skillId]
    const nextFiles = Object.fromEntries(
      Object.entries(skillFilesByConnection.value[connectionId] ?? {})
        .filter(([key]) => !key.startsWith(`${skillId}:`)),
    )
    skillDocumentsByConnection.value = {
      ...skillDocumentsByConnection.value,
      [connectionId]: nextDocuments,
    }
    skillTreesByConnection.value = {
      ...skillTreesByConnection.value,
      [connectionId]: nextTrees,
    }
    skillFilesByConnection.value = {
      ...skillFilesByConnection.value,
      [connectionId]: nextFiles,
    }
    await refreshToolCatalog(connectionId)
  }

  async function getMcpServerDocument(serverName: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getMcpServer(serverName)
    mcpDocumentsByConnection.value = {
      ...mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(mcpDocumentsByConnection.value[connectionId] ?? {}),
        [serverName]: document,
      },
    }
    return document
  }

  async function createMcpServer(input: UpsertWorkspaceMcpServerInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.createMcpServer(input)
    mcpDocumentsByConnection.value = {
      ...mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(mcpDocumentsByConnection.value[connectionId] ?? {}),
        [document.serverName]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function updateMcpServer(serverName: string, input: UpsertWorkspaceMcpServerInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateMcpServer(serverName, input)
    mcpDocumentsByConnection.value = {
      ...mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(mcpDocumentsByConnection.value[connectionId] ?? {}),
        [document.serverName]: document,
      },
    }
    await refreshToolCatalog(connectionId)
    return document
  }

  async function deleteMcpServer(serverName: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteMcpServer(serverName)
    const nextDocuments = { ...(mcpDocumentsByConnection.value[connectionId] ?? {}) }
    delete nextDocuments[serverName]
    mcpDocumentsByConnection.value = {
      ...mcpDocumentsByConnection.value,
      [connectionId]: nextDocuments,
    }
    await refreshToolCatalog(connectionId)
  }

  async function createTool(record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.catalog.createTool(record)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: [...(toolsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateTool(toolId: string, record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.catalog.updateTool(toolId, record)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: (toolsByConnection.value[connectionId] ?? []).map(item => item.id === toolId ? updated : item),
    }
    return updated
  }

  async function removeTool(toolId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteTool(toolId)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: (toolsByConnection.value[connectionId] ?? []).filter(item => item.id !== toolId),
    }
  }

  return {
    snapshot,
    toolCatalog,
    toolCatalogEntries,
    providers,
    models,
    configuredModels,
    credentialBindings,
    defaultSelections,
    diagnostics,
    tools,
    error,
    modelRows,
    configuredModelRows,
    workspaceConfiguredModelRows,
    providerSummaries,
    credentialSummaries,
    defaultSelectionRows,
    diagnosticSummary,
    filterOptions,
    enabledProviderCount,
    defaultSurfaceCount,
    credentialIssueCount,
    configuredModelOptions,
    workspaceConfiguredModelOptions,
    modelOptions,
    load,
    refreshToolCatalog,
    setToolDisabled,
    getSkillDocument,
    getSkillTreeDocument,
    getSkillFileDocument,
    createSkill,
    updateSkill,
    updateSkillFile,
    importSkillArchive,
    importSkillFolder,
    copySkillToManaged,
    deleteSkill,
    getMcpServerDocument,
    createMcpServer,
    updateMcpServer,
    deleteMcpServer,
    createTool,
    updateTool,
    removeTool,
    getModelById,
    getModelRowById,
    getConfiguredModelById,
    getConfiguredModelRowById,
    getModelOptionsByProviderId,
    getProviderById,
    getProviderBaseUrl,
    getCredentialByProviderId,
  }
})
