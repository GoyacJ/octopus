import { computed, h, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type {
  ConfiguredModelRecord,
  JsonValue,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  RuntimeConfiguredModelCredentialInput,
  RuntimeConfigSource,
} from '@octopus/schema'

import { enumLabel, formatList } from '@/i18n/copy'
import type { CatalogConfiguredModelRow, CatalogFilterOption } from '@/stores/catalog'
import { useCatalogStore } from '@/stores/catalog'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'

import {
  buildManagedModelOverride,
  buildManagedProviderOverride,
  cloneJson,
  createConfiguredModelId,
  createCustomProviderId,
  CUSTOM_BASE_URL_PLACEHOLDER,
  CUSTOM_PROVIDER_MODE,
  isManagedByPage,
  isSpecialProviderType,
  OLLAMA_PROVIDER_ID,
  PAGE_SIZE,
  PROVIDER_TYPE_METADATA_KEY,
  resolveProviderType,
  toConfiguredModelMap,
  toMergePatch,
  toModelRegistryMap,
  toPersistedConfiguredModelMap,
  toProviderRegistryMap,
  toRecord,
  updateProviderBaseUrl,
  VLLM_PROVIDER_ID,
} from './models-runtime-helpers'
import {
  getConfiguredModelCredentialStatus,
  isModelCredentialBlocked,
  localizeModelCredentialDescription,
  localizeModelCredentialLabel,
  localizeModelRuntimeMessage,
  localizeModelRuntimeMessages,
  resolveModelCredentialSecurityState,
  resolveModelCredentialTone,
} from './models-security'
import { useWorkspaceModelNotifications } from './useWorkspaceModelNotifications'

function genericTableFallback(value?: string | null) {
  return value?.trim() || '—'
}

export function useModelsDraft() {
  const { t } = useI18n()
  const catalogStore = useCatalogStore()
  const runtime = useRuntimeStore()
  const shell = useShellStore()
  const notifications = useWorkspaceModelNotifications()

  const searchQuery = ref('')
  const providerFilter = ref('')
  const surfaceFilter = ref('')
  const capabilityFilter = ref('')
  const page = ref(1)
  const selectedConfiguredModelId = ref('')
  const detailDialogOpen = ref(false)
  const selectedApiKey = ref('')

  const createDialogOpen = ref(false)
  const createName = ref('')
  const createProviderType = ref('')
  const createCustomProviderLabel = ref('')
  const createModelId = ref('')
  const createApiKey = ref('')
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
  const secretReferences = computed(() => runtime.config?.secretReferences ?? [])

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
        const credentialSecurityState = resolveModelCredentialSecurityState({
          credentialRef: configuredModel.credentialRef,
          secretStatus: getConfiguredModelCredentialStatus(
            secretReferences.value,
            configuredModel.configuredModelId,
          ),
        })

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
          credentialDisplayLabel: localizeModelCredentialLabel(t, credentialSecurityState),
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
        surfacesMap.set(surface, enumLabel('modelSurface', surface))
      }
      for (const capability of row.capabilities) {
        capabilitiesMap.set(capability, enumLabel('modelCapability', capability))
      }
    }

    return {
      providers: [...providersMap.entries()]
        .sort((left, right) => left[1].localeCompare(right[1]))
        .map(([value, label]) => ({ value, label })),
      surfaces: [...surfacesMap.entries()]
        .sort((left, right) => left[1].localeCompare(right[1]))
        .map(([value, label]) => ({ value, label })),
      capabilities: [...capabilitiesMap.entries()]
        .sort((left, right) => left[1].localeCompare(right[1]))
        .map(([value, label]) => ({ value, label })),
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
  const selectedCredentialStatusEntry = computed(() =>
    getConfiguredModelCredentialStatus(
      secretReferences.value,
      selectedConfiguredModel.value?.configuredModelId,
    ),
  )
  const selectedCredentialSecurityState = computed(() =>
    resolveModelCredentialSecurityState({
      credentialRef: selectedConfiguredModel.value?.credentialRef,
      secretStatus: selectedCredentialStatusEntry.value,
      hasPendingApiKey: Boolean(selectedApiKey.value.trim()),
    }),
  )
  const selectedCredentialStatusLabel = computed(() =>
    localizeModelCredentialLabel(t, selectedCredentialSecurityState.value))
  const selectedCredentialStatusDescription = computed(() =>
    localizeModelCredentialDescription(t, selectedCredentialSecurityState.value))
  const selectedCredentialStatusTone = computed(() =>
    resolveModelCredentialTone(selectedCredentialSecurityState.value))
  const selectedCredentialBlocked = computed(() =>
    isModelCredentialBlocked(selectedCredentialSecurityState.value))

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
  const hasPendingSaveChanges = computed(() =>
    hasPendingPatch.value || Boolean(selectedApiKey.value.trim()))
  const validationErrors = computed(() =>
    localizeModelRuntimeMessages(
      t,
      runtime.configValidation.workspace?.errors ?? [],
      'models.messages.validateFailed',
    ),
  )
  const validationWarnings = computed(() =>
    localizeModelRuntimeMessages(
      t,
      runtime.configValidation.workspace?.warnings ?? [],
      'models.messages.runtimeWarning',
    ),
  )

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

  function resolveWorkspaceMessage(fallbackKey: string) {
    const localizedErrors = localizeModelRuntimeMessages(
      t,
      runtime.configValidation.workspace?.errors ?? [],
      fallbackKey,
    )
    return localizedErrors[0]
      || localizeModelRuntimeMessage(t, runtime.configError, fallbackKey)
      || t(fallbackKey)
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

  function updateSelectedApiKey(value: string) {
    runtime.clearConfiguredModelProbeResult()
    selectedApiKey.value = value
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

  function buildManagedCredentialInputs(
    configuredModelId: string,
    apiKey: string,
  ): RuntimeConfiguredModelCredentialInput[] {
    const trimmedApiKey = apiKey.trim()
    if (!trimmedApiKey) {
      return []
    }

    return [{
      configuredModelId,
      apiKey: trimmedApiKey,
    }]
  }

  async function deleteSelectedConfiguredModel() {
    if (!selectedConfiguredModel.value) {
      return
    }

    const removedModelName = selectedConfiguredModel.value.name
    const removedConfiguredModelId = selectedConfiguredModel.value.configuredModelId
    const previousConfiguredModels = cloneJson(draftConfiguredModels.value)
    const previousProviderOverrides = cloneJson(draftProviderOverrides.value)
    const previousModelOverrides = cloneJson(draftModelOverrides.value)
    const previousSelectedConfiguredModelId = selectedConfiguredModelId.value
    const next = { ...draftConfiguredModels.value }
    const removed = next[removedConfiguredModelId]
    delete next[removedConfiguredModelId]
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
      await notifications.notifyDeleteFailed(
        removedModelName,
        resolveWorkspaceMessage('models.messages.deleteFailed'),
      )
      return
    }

    detailDialogOpen.value = false
    await refreshWorkspaceModels()
    await notifications.notifyDeleteSuccess(removedModelName)
  }

  function openCreateDialog() {
    createName.value = ''
    createProviderType.value = createProviderOptions.value[0]?.value ?? ''
    createCustomProviderLabel.value = ''
    createModelId.value = ''
    createApiKey.value = ''
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
    const trimmedApiKey = createApiKey.value.trim()
    const configuredModelCredentials = buildManagedCredentialInputs(configuredModelId, trimmedApiKey)

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
        baseUrl,
        tokenQuota: totalTokens && totalTokens > 0 ? { totalTokens } : undefined,
        tokenUsage: {
          usedTokens: 0,
          remainingTokens: totalTokens && totalTokens > 0 ? totalTokens : undefined,
          exhausted: false,
        },
        enabled: createEnabled.value,
        source: 'workspace',
        status: 'unconfigured',
        configured: false,
      },
    }
    selectedConfiguredModelId.value = configuredModelId
    syncRuntimeDraft()

    const saved = await runtime.saveConfig('workspace', {
      configuredModelCredentials,
    })
    if (!saved) {
      draftConfiguredModels.value = previousConfiguredModels
      draftProviderOverrides.value = previousProviderOverrides
      draftModelOverrides.value = previousModelOverrides
      selectedConfiguredModelId.value = previousSelectedConfiguredModelId
      syncRuntimeDraft()
      createFormError.value = resolveWorkspaceMessage('models.messages.createFailed')
      await notifications.notifyCreateFailed(name, createFormError.value)
      return
    }

    await refreshWorkspaceModels()
    createDialogOpen.value = false
    detailDialogOpen.value = true
    createApiKey.value = ''
    createFormError.value = ''
    await notifications.notifyCreateSuccess(name)
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

  async function validateSelectedConfiguredModel() {
    if (!selectedConfiguredModel.value) {
      return
    }

    if (selectedCredentialBlocked.value) {
      await notifications.notifyValidationFailed(
        selectedConfiguredModel.value.name,
        t('models.messages.migrationBlocked'),
      )
      return
    }

    syncRuntimeDraft()
    const result = await runtime.probeConfiguredModel(
      'workspace',
      selectedConfiguredModel.value.configuredModelId,
      selectedApiKey.value.trim() || undefined,
    )

    if (result.valid && result.reachable) {
      await notifications.notifyValidationSuccess(
        result.configuredModelName ?? selectedConfiguredModel.value.name,
        result.consumedTokens ?? 0,
      )
      await refreshWorkspaceModels()
      return
    }

    const message = localizeModelRuntimeMessages(
      t,
      result.errors.length ? result.errors : result.warnings,
      'models.messages.validateFailed',
    )[0] ?? t('models.messages.validateFailed')

    await notifications.notifyValidationFailed(selectedConfiguredModel.value.name, message)
  }

  async function saveWorkspacePatch() {
    if (!selectedConfiguredModel.value) {
      return
    }

    if (!hasPendingSaveChanges.value) {
      return
    }

    if (selectedCredentialBlocked.value) {
      await notifications.notifySaveFailed(
        selectedConfiguredModel.value.name,
        t('models.messages.migrationBlocked'),
      )
      return
    }

    const trimmedApiKey = selectedApiKey.value.trim()
    const configuredModelCredentials = buildManagedCredentialInputs(
      selectedConfiguredModel.value.configuredModelId,
      trimmedApiKey,
    )

    syncRuntimeDraft()

    const saved = await runtime.saveConfig('workspace', {
      configuredModelCredentials,
    })
    if (!saved) {
      await notifications.notifySaveFailed(
        selectedConfiguredModel.value.name,
        resolveWorkspaceMessage('models.messages.saveFailed'),
      )
      return
    }

    await refreshWorkspaceModels()
    selectedApiKey.value = ''
    await notifications.notifySaveSuccess(selectedConfiguredModel.value.name)
  }

  function selectRow(row: CatalogConfiguredModelRow) {
    runtime.clearConfiguredModelProbeResult()
    selectedApiKey.value = ''
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
      accessorFn: (row: CatalogConfiguredModelRow) => formatList(
        row.surfaces.map(surface => enumLabel('modelSurface', surface)),
      ) || '—',
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
      accessorFn: (row: CatalogConfiguredModelRow) => genericTableFallback(String(row.credentialDisplayLabel ?? '')),
    },
    {
      id: 'baseUrl',
      header: t('models.table.columns.baseUrl'),
      accessorFn: (row: CatalogConfiguredModelRow) => genericTableFallback(row.baseUrl),
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

  watch(selectedConfiguredModelId, () => {
    selectedApiKey.value = ''
  })

  watch(detailDialogOpen, (open) => {
    if (!open) {
      selectedApiKey.value = ''
    }
  })

  return {
    t,
    catalogStore,
    runtime,
    searchQuery,
    providerFilter,
    surfaceFilter,
    capabilityFilter,
    page,
    detailDialogOpen,
    createDialogOpen,
    createName,
    createProviderType,
    createCustomProviderLabel,
    createModelId,
    createApiKey,
    createBaseUrl,
    createTotalTokens,
    createEnabled,
    createFormError,
    localFilterOptions,
    pagedRows,
    filteredRows,
    pageCount,
    selectedRow,
    selectedConfiguredModel,
    selectedModel,
    selectedProvider,
    selectedApiKey,
    selectedCredentialStatusLabel,
    selectedCredentialStatusDescription,
    selectedCredentialStatusTone,
    selectedCredentialBlocked,
    selectedIsCustomManaged,
    selectedProbeResult,
    validationErrors,
    validationWarnings,
    hasPendingSaveChanges,
    createProviderOptions,
    createUsesFreeformModel,
    createRequiresCustomProviderName,
    createUpstreamModelOptions,
    columns,
    updateSelectedConfiguredModel,
    updateSelectedApiKey,
    updateSelectedTokenQuota,
    updateSelectedBaseUrl,
    updateSelectedCustomProviderLabel,
    deleteSelectedConfiguredModel,
    openCreateDialog,
    createConfiguredModel,
    validateSelectedConfiguredModel,
    saveWorkspacePatch,
    selectRow,
  }
}
