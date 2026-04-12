import { computed, type ComputedRef, type Ref } from 'vue'

import type {
  CapabilityManagementProjection,
  ConfiguredModelRecord,
  CredentialBinding,
  DefaultSelection,
  ModelCatalogSnapshot,
  ModelRegistryDiagnostics,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  ToolRecord,
} from '@octopus/schema'

import {
  EMPTY_SNAPSHOT,
  resolveCapabilityLabel,
  sortFilterOptions,
  toConfiguredModelRow,
  toModelRow,
  type CatalogConfiguredModelOption,
  type CatalogConfiguredModelRow,
  type CatalogCredentialSummary,
  type CatalogDefaultSelectionRow,
  type CatalogDiagnosticSummary,
  type CatalogFilterOption,
  type CatalogModelRow,
  type CatalogProviderSummary,
} from './catalog_normalizers'

interface CatalogFilterContext {
  activeConnectionId: ComputedRef<string>
  snapshots: Ref<Record<string, ModelCatalogSnapshot>>
  managementProjectionsByConnection: Ref<Record<string, CapabilityManagementProjection>>
  toolsByConnection: Ref<Record<string, ToolRecord[]>>
}

export function createCatalogFilters(context: CatalogFilterContext) {
  const snapshot = computed<ModelCatalogSnapshot>(() => context.snapshots.value[context.activeConnectionId.value] ?? EMPTY_SNAPSHOT)
  const managementProjection = computed<CapabilityManagementProjection>(() =>
    context.managementProjectionsByConnection.value[context.activeConnectionId.value]
    ?? {
      entries: [],
      assets: [],
      skillPackages: [],
      mcpServerPackages: [],
    },
  )
  const managementEntries = computed(() => managementProjection.value.entries)
  const providers = computed<ProviderRegistryRecord[]>(() => snapshot.value.providers)
  const models = computed<ModelRegistryRecord[]>(() => snapshot.value.models)
  const configuredModels = computed<ConfiguredModelRecord[]>(() => snapshot.value.configuredModels)
  const credentialBindings = computed<CredentialBinding[]>(() => snapshot.value.credentialBindings)
  const defaultSelections = computed<Record<string, DefaultSelection>>(() => snapshot.value.defaultSelections)
  const diagnostics = computed<ModelRegistryDiagnostics>(() => snapshot.value.diagnostics)
  const tools = computed(() => context.toolsByConnection.value[context.activeConnectionId.value] ?? [])
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

  return {
    snapshot,
    managementProjection,
    managementEntries,
    providers,
    models,
    configuredModels,
    credentialBindings,
    defaultSelections,
    diagnostics,
    tools,
    diagnosticSummary,
    modelRows,
    configuredModelRows,
    workspaceConfiguredModelRows,
    providerSummaries,
    credentialSummaries,
    defaultSelectionRows,
    filterOptions,
    enabledProviderCount,
    defaultSurfaceCount,
    credentialIssueCount,
    configuredModelOptions,
    workspaceConfiguredModelOptions,
    modelOptions,
    getModelOptionsByProviderId,
    getProviderBaseUrl,
    getModelById,
    getModelRowById,
    getConfiguredModelById,
    getConfiguredModelRowById,
    getProviderById,
    getCredentialByProviderId,
  }
}
