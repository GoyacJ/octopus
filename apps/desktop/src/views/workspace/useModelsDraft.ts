import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type {
  ConfiguredModelBudgetPolicy,
  ConfiguredModelRecord,
  JsonValue,
  ModelRegistryRecord,
  ProviderRegistryRecord,
  RuntimeConfiguredModelCredentialInput,
  RuntimeConfigSource,
} from '@octopus/schema'

import { enumLabel } from '@/i18n/copy'
import type { CatalogConfiguredModelRow, CatalogFilterOption } from '@/stores/catalog'
import { summarizeModelExecution } from '@/stores/catalog_normalizers'
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
} from './models-runtime-helpers'
import {
  getConfiguredModelCredentialStatus,
  isModelCredentialBlocked,
  localizeModelCredentialDescription,
  localizeModelCredentialLabel,
  localizeModelCredentialSourceDescription,
  localizeModelCredentialSourceLabel,
  localizeModelRuntimeMessage,
  localizeModelRuntimeMessages,
  resolveModelCredentialSecurityState,
  resolveModelCredentialSourceKind,
  resolveModelCredentialTone,
} from './models-security'
import { useWorkspaceModelNotifications } from './useWorkspaceModelNotifications'

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
  const selectedApiKey = ref('')

  const createDialogOpen = ref(false)
  const createName = ref('')
  const createProviderType = ref('')
  const createCustomProviderLabel = ref('')
  const createModelId = ref('')
  const createApiKey = ref('')
  const createBaseUrl = ref('')
  const createBudgetTotal = ref('')
  const createBudgetAccountingMode = ref('')
  const createBudgetTrafficClasses = ref('')
  const createBudgetWarningThresholds = ref('')
  const createBudgetReservationStrategy = ref('')
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
  const budgetAccountingModeOptions = computed<CatalogFilterOption[]>(() => [
    { value: '', label: t('models.budget.accountingModes.unset') },
    { value: 'provider_reported', label: t('models.budget.accountingModes.provider_reported') },
    { value: 'estimated', label: t('models.budget.accountingModes.estimated') },
    { value: 'non_billable', label: t('models.budget.accountingModes.non_billable') },
  ])
  const budgetReservationStrategyOptions = computed<CatalogFilterOption[]>(() => [
    { value: '', label: t('models.budget.reservationStrategies.unset') },
    { value: 'none', label: t('models.budget.reservationStrategies.none') },
    { value: 'fixed', label: t('models.budget.reservationStrategies.fixed') },
  ])

  const localRows = computed<CatalogConfiguredModelRow[]>(() =>
    Object.values(draftConfiguredModels.value)
      .filter(configuredModel => configuredModel.source === 'workspace')
      .map((configuredModel) => {
        const liveConfiguredModel = catalogStore.getConfiguredModelById(configuredModel.configuredModelId)
        const provider = draftProviderOverrides.value[configuredModel.providerId]
          ?? catalogStore.getProviderById(configuredModel.providerId)
        const model = draftModelOverrides.value[configuredModel.modelId]
          ?? catalogStore.getModelById(configuredModel.modelId)
        const providerCredential = catalogStore.getCredentialByProviderId(configuredModel.providerId)
        const totalTokens = configuredModel.budgetPolicy?.totalBudgetTokens ?? liveConfiguredModel?.budgetPolicy?.totalBudgetTokens
        const usedTokens = liveConfiguredModel?.tokenUsage.usedTokens ?? configuredModel.tokenUsage.usedTokens ?? 0
        const runtimeSummary = summarizeModelExecution(model)
        const hasDiagnostics = catalogStore.diagnostics.warnings.some(item =>
          item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.providerId) || item.includes(configuredModel.modelId))
          || catalogStore.diagnostics.errors.some(item =>
            item.includes(configuredModel.configuredModelId) || item.includes(configuredModel.providerId) || item.includes(configuredModel.modelId))
        const credentialSourceKind = resolveModelCredentialSourceKind({
          configuredCredentialRef: configuredModel.credentialRef,
          providerCredential,
        })
        const effectiveCredentialRef = configuredModel.credentialRef ?? providerCredential?.credentialRef
        const credentialSecurityState = resolveModelCredentialSecurityState({
          credentialRef: effectiveCredentialRef,
          referenceStatus: configuredModel.credentialRef
            ? getConfiguredModelCredentialStatus(
                secretReferences.value,
                configuredModel.configuredModelId,
              )?.status
            : providerCredential?.status,
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
          surfaces: runtimeSummary.enabledSurfaces,
          conversationSurfaces: runtimeSummary.conversationSurfaces,
          capabilities: model?.capabilities.map(capability => capability.capabilityId) ?? [],
          defaultSurfaces: [],
          contextWindow: model?.contextWindow,
          maxOutputTokens: model?.maxOutputTokens,
          executionClass: runtimeSummary.executionClass,
          upstreamStreaming: runtimeSummary.upstreamStreaming,
          toolLoop: runtimeSummary.toolLoop,
          supportsConversationExecution: runtimeSummary.supportsConversationExecution,
          credentialRef: configuredModel.credentialRef,
          credentialStatus: (providerCredential?.status ?? (effectiveCredentialRef ? 'configured' : 'unconfigured')) as CatalogConfiguredModelRow['credentialStatus'],
          credentialConfigured: Boolean(effectiveCredentialRef),
          credentialDisplayLabel: localizeModelCredentialSourceLabel(t, credentialSourceKind),
          credentialHealthLabel: localizeModelCredentialLabel(t, credentialSecurityState),
          baseUrl: configuredModel.baseUrl,
          budgetAccountingMode: configuredModel.budgetPolicy?.accountingMode,
          budgetReservationStrategy: configuredModel.budgetPolicy?.reservationStrategy,
          budgetTrafficClasses: configuredModel.budgetPolicy?.trafficClasses ?? [],
          totalTokens,
          usedTokens,
          remainingTokens: totalTokens ? Math.max(0, totalTokens - usedTokens) : undefined,
          budgetExhausted: totalTokens ? usedTokens >= totalTokens : false,
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
  const selectedProviderCredential = computed(() =>
    selectedProvider.value
      ? catalogStore.getCredentialByProviderId(selectedProvider.value.providerId) ?? null
      : null,
  )
  const selectedCredentialStatusEntry = computed(() =>
    getConfiguredModelCredentialStatus(
      secretReferences.value,
      selectedConfiguredModel.value?.configuredModelId,
    ),
  )
  const selectedCredentialSourceKind = computed(() =>
    resolveModelCredentialSourceKind({
      configuredCredentialRef: selectedConfiguredModel.value?.credentialRef,
      providerCredential: selectedProviderCredential.value,
    }),
  )
  const selectedEffectiveCredentialRef = computed(() =>
    selectedConfiguredModel.value?.credentialRef ?? selectedProviderCredential.value?.credentialRef ?? '')
  const selectedCredentialSecurityState = computed(() =>
    resolveModelCredentialSecurityState({
      credentialRef: selectedEffectiveCredentialRef.value,
      referenceStatus: selectedConfiguredModel.value?.credentialRef
        ? selectedCredentialStatusEntry.value?.status
        : selectedProviderCredential.value?.status,
      hasPendingApiKey: Boolean(selectedApiKey.value.trim()),
    }),
  )
  const selectedCredentialSourceLabel = computed(() =>
    localizeModelCredentialSourceLabel(t, selectedCredentialSourceKind.value))
  const selectedCredentialSourceDescription = computed(() =>
    localizeModelCredentialSourceDescription(
      t,
      selectedCredentialSourceKind.value,
      selectedProvider.value ? { provider: selectedProvider.value.label } : undefined,
    ))
  const selectedCredentialStatusLabel = computed(() =>
    localizeModelCredentialLabel(t, selectedCredentialSecurityState.value))
  const selectedCredentialStatusDescription = computed(() =>
    localizeModelCredentialDescription(t, selectedCredentialSecurityState.value))
  const selectedCredentialStatusTone = computed(() =>
    resolveModelCredentialTone(selectedCredentialSecurityState.value))
  const selectedCredentialBlocked = computed(() =>
    isModelCredentialBlocked(selectedCredentialSecurityState.value))
  const selectedCanClearCredentialOverride = computed(() =>
    Boolean(selectedConfiguredModel.value?.credentialRef))
  const selectedBudgetAccountingMode = computed(() =>
    selectedConfiguredModel.value?.budgetPolicy?.accountingMode ?? '')
  const selectedBudgetTrafficClasses = computed(() =>
    formatBudgetStringValues(selectedConfiguredModel.value?.budgetPolicy?.trafficClasses))
  const selectedBudgetWarningThresholds = computed(() =>
    formatBudgetNumberValues(selectedConfiguredModel.value?.budgetPolicy?.warningThresholdPercentages))
  const selectedBudgetReservationStrategy = computed(() =>
    selectedConfiguredModel.value?.budgetPolicy?.reservationStrategy ?? '')

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

  function updateSelectedBudgetPolicy(patch: Partial<ConfiguredModelBudgetPolicy>) {
    updateSelectedConfiguredModel({
      budgetPolicy: normalizeBudgetPolicy({
        ...selectedConfiguredModel.value?.budgetPolicy,
        ...patch,
      }),
    })
  }

  function updateSelectedBudgetTotal(value: string) {
    updateSelectedBudgetPolicy({
      totalBudgetTokens: parseBudgetTokenCount(value),
    })
  }

  function updateSelectedBudgetAccountingMode(value: string) {
    updateSelectedBudgetPolicy({
      accountingMode: parseBudgetAccountingMode(value),
    })
  }

  function updateSelectedBudgetTrafficClasses(value: string) {
    updateSelectedBudgetPolicy({
      trafficClasses: parseBudgetTrafficClasses(value),
    })
  }

  function updateSelectedBudgetWarningThresholds(value: string) {
    updateSelectedBudgetPolicy({
      warningThresholdPercentages: parseBudgetWarningThresholds(value),
    })
  }

  function updateSelectedBudgetReservationStrategy(value: string) {
    updateSelectedBudgetPolicy({
      reservationStrategy: parseBudgetReservationStrategy(value),
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
    createBudgetTotal.value = ''
    createBudgetAccountingMode.value = ''
    createBudgetTrafficClasses.value = ''
    createBudgetWarningThresholds.value = ''
    createBudgetReservationStrategy.value = ''
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
    const budgetPolicy = normalizeBudgetPolicy({
      accountingMode: parseBudgetAccountingMode(createBudgetAccountingMode.value),
      trafficClasses: parseBudgetTrafficClasses(createBudgetTrafficClasses.value),
      totalBudgetTokens: parseBudgetTokenCount(createBudgetTotal.value),
      reservationStrategy: parseBudgetReservationStrategy(createBudgetReservationStrategy.value),
      warningThresholdPercentages: parseBudgetWarningThresholds(createBudgetWarningThresholds.value),
    })
    const totalBudgetTokens = budgetPolicy?.totalBudgetTokens
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
        budgetPolicy,
        tokenUsage: {
          usedTokens: 0,
          remainingTokens: totalBudgetTokens,
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

  function clearSelectedCredentialOverride() {
    if (!selectedConfiguredModel.value?.credentialRef) {
      return
    }

    runtime.clearConfiguredModelProbeResult()
    selectedApiKey.value = ''
    updateSelectedConfiguredModel({
      credentialRef: undefined,
      configured: false,
      status: 'unconfigured',
    })
  }

  function selectRow(row: CatalogConfiguredModelRow) {
    runtime.clearConfiguredModelProbeResult()
    selectedApiKey.value = ''
    selectedConfiguredModelId.value = row.configuredModelId
  }

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

  return {
    t,
    catalogStore,
    runtime,
    searchQuery,
    providerFilter,
    surfaceFilter,
    capabilityFilter,
    page,
    selectedConfiguredModelId,
    createDialogOpen,
    createName,
    createProviderType,
    createCustomProviderLabel,
    createModelId,
    createApiKey,
    createBaseUrl,
    createBudgetTotal,
    createBudgetAccountingMode,
    createBudgetTrafficClasses,
    createBudgetWarningThresholds,
    createBudgetReservationStrategy,
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
    selectedProviderCredential,
    selectedApiKey,
    selectedCredentialSourceKind,
    selectedCredentialSourceLabel,
    selectedCredentialSourceDescription,
    selectedCredentialStatusLabel,
    selectedCredentialStatusDescription,
    selectedCredentialStatusTone,
    selectedCredentialBlocked,
    selectedCanClearCredentialOverride,
    selectedIsCustomManaged,
    selectedProbeResult,
    selectedBudgetAccountingMode,
    selectedBudgetTrafficClasses,
    selectedBudgetWarningThresholds,
    selectedBudgetReservationStrategy,
    validationErrors,
    validationWarnings,
    hasPendingSaveChanges,
    createProviderOptions,
    budgetAccountingModeOptions,
    budgetReservationStrategyOptions,
    createUsesFreeformModel,
    createRequiresCustomProviderName,
    createUpstreamModelOptions,
    updateSelectedConfiguredModel,
    updateSelectedApiKey,
    updateSelectedBudgetTotal,
    updateSelectedBudgetAccountingMode,
    updateSelectedBudgetTrafficClasses,
    updateSelectedBudgetWarningThresholds,
    updateSelectedBudgetReservationStrategy,
    updateSelectedBaseUrl,
    updateSelectedCustomProviderLabel,
    clearSelectedCredentialOverride,
    deleteSelectedConfiguredModel,
    openCreateDialog,
    createConfiguredModel,
    validateSelectedConfiguredModel,
    saveWorkspacePatch,
    selectRow,
  }
}

function parseBudgetTokenCount(value: string) {
  const trimmed = value.trim()
  if (!trimmed) {
    return undefined
  }

  const parsed = Number(trimmed)
  return Number.isFinite(parsed) && parsed > 0 ? Math.trunc(parsed) : undefined
}

function parseBudgetAccountingMode(value: string): ConfiguredModelBudgetPolicy['accountingMode'] | undefined {
  return value === 'provider_reported' || value === 'estimated' || value === 'non_billable'
    ? value
    : undefined
}

function parseBudgetReservationStrategy(value: string): ConfiguredModelBudgetPolicy['reservationStrategy'] | undefined {
  return value === 'none' || value === 'fixed'
    ? value
    : undefined
}

function parseBudgetTrafficClasses(value: string) {
  return sanitizeBudgetStringList(value.split(',').map(entry => entry.trim()))
}

function parseBudgetWarningThresholds(value: string) {
  const values = value
    .split(',')
    .map(entry => Number(entry.trim()))
    .filter(entry => Number.isFinite(entry) && entry > 0)
  return sanitizeBudgetNumberList(values)
}

function sanitizeBudgetStringList(values: string[] | undefined) {
  if (!values?.length) {
    return undefined
  }

  const deduped = Array.from(new Set(values.filter(Boolean)))
  return deduped.length ? deduped : undefined
}

function sanitizeBudgetNumberList(values: number[] | undefined) {
  if (!values?.length) {
    return undefined
  }

  const deduped = Array.from(new Set(values.map(value => Math.trunc(value)).filter(value => value > 0)))
  return deduped.length ? deduped : undefined
}

function normalizeBudgetPolicy(policy: Partial<ConfiguredModelBudgetPolicy> | undefined): ConfiguredModelBudgetPolicy | undefined {
  if (!policy) {
    return undefined
  }

  const next: ConfiguredModelBudgetPolicy = {}
  if (policy.accountingMode) {
    next.accountingMode = policy.accountingMode
  }
  if (policy.trafficClasses?.length) {
    next.trafficClasses = sanitizeBudgetStringList(policy.trafficClasses)
  }
  if (typeof policy.totalBudgetTokens === 'number' && Number.isFinite(policy.totalBudgetTokens) && policy.totalBudgetTokens > 0) {
    next.totalBudgetTokens = Math.trunc(policy.totalBudgetTokens)
  }
  if (policy.reservationStrategy) {
    next.reservationStrategy = policy.reservationStrategy
  }
  if (policy.warningThresholdPercentages?.length) {
    next.warningThresholdPercentages = sanitizeBudgetNumberList(policy.warningThresholdPercentages)
  }

  return Object.keys(next).length ? next : undefined
}

function formatBudgetStringValues(values: string[] | undefined) {
  return values?.join(', ') ?? ''
}

function formatBudgetNumberValues(values: number[] | undefined) {
  return values?.join(', ') ?? ''
}
