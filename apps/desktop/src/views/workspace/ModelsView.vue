<script setup lang="ts">
import { Plus } from 'lucide-vue-next'

import {
  UiButton,
  UiInput,
  UiListDetailWorkspace,
  UiPageHeader,
  UiPageShell,
  UiSelect,
  UiToolbarRow,
} from '@octopus/ui'

import CreateModelDialog from './CreateModelDialog.vue'
import ModelDetailsPanel from './ModelDetailsPanel.vue'
import ModelsListPane from './ModelsListPane.vue'
import { useModelsDraft } from './useModelsDraft'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const {
  t,
  runtime,
  searchQuery,
  providerFilter,
  surfaceFilter,
  capabilityFilter,
  page,
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
  selectedConfiguredModelId,
  selectedRow,
  selectedConfiguredModel,
  selectedModel,
  selectedProvider,
  selectedApiKey,
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
} = useModelsDraft()
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    :width="props.embedded ? undefined : 'wide'"
    :test-id="props.embedded ? undefined : 'workspace-models-view'"
    :data-testid="props.embedded ? 'workspace-models-embedded' : undefined"
    class="space-y-6"
  >
    <UiPageHeader
      v-if="!props.embedded"
      :eyebrow="t('models.header.eyebrow')"
      :title="t('models.header.title')"
      :description="t('models.header.subtitle')"
    />

    <UiListDetailWorkspace
      :has-selection="true"
      :detail-title="selectedConfiguredModel?.name ?? ''"
      :detail-subtitle="selectedProvider && selectedModel ? `${selectedProvider.label} · ${selectedModel.label}` : ''"
      list-class="p-3"
      detail-class="p-3"
    >
      <template #toolbar>
        <UiToolbarRow test-id="workspace-models-toolbar" layout="inline">
          <template #search>
            <UiInput
              v-model="searchQuery"
              data-testid="models-search-input"
              :placeholder="t('models.filters.searchPlaceholder')"
            />
          </template>

          <template #filters>
            <UiSelect
              v-model="providerFilter"
              data-testid="models-provider-filter"
              class="min-w-[150px]"
              :options="[{ value: '', label: t('models.filters.allProviders') }, ...localFilterOptions.providers]"
            />
            <UiSelect
              v-model="surfaceFilter"
              data-testid="models-surface-filter"
              class="min-w-[150px]"
              :options="[{ value: '', label: t('models.filters.allSurfaces') }, ...localFilterOptions.surfaces]"
            />
            <UiSelect
              v-model="capabilityFilter"
              data-testid="models-capability-filter"
              class="min-w-[150px]"
              :options="[{ value: '', label: t('models.filters.allCapabilities') }, ...localFilterOptions.capabilities]"
            />
          </template>

          <template #actions>
            <UiButton data-testid="models-create-button" size="sm" @click="openCreateDialog">
              <Plus :size="14" />
              {{ t('models.actions.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <ModelsListPane
          :paged-rows="pagedRows"
          :selected-configured-model-id="selectedConfiguredModelId"
          :filtered-rows-length="filteredRows.length"
          :page="page"
          :page-count="pageCount"
          :t="t"
          @update:page="page = $event"
          @select-row="selectRow"
        />
      </template>

      <template #detail>
        <ModelDetailsPanel
          :selected-row="selectedRow"
          :selected-configured-model="selectedConfiguredModel"
          :selected-model="selectedModel"
          :selected-provider="selectedProvider"
          :selected-api-key="selectedApiKey"
          :selected-credential-source-label="selectedCredentialSourceLabel"
          :selected-credential-source-description="selectedCredentialSourceDescription"
          :selected-credential-status-label="selectedCredentialStatusLabel"
          :selected-credential-status-description="selectedCredentialStatusDescription"
          :selected-credential-status-tone="selectedCredentialStatusTone"
          :selected-credential-blocked="selectedCredentialBlocked"
          :selected-can-clear-credential-override="selectedCanClearCredentialOverride"
          :selected-is-custom-managed="selectedIsCustomManaged"
          :selected-probe-result="selectedProbeResult"
          :runtime-config-validating="runtime.configValidating"
          :runtime-configured-model-probing="runtime.configuredModelProbing"
          :runtime-config-saving="runtime.configSaving"
          :validation-errors="validationErrors"
          :validation-warnings="validationWarnings"
          :selected-budget-accounting-mode="selectedBudgetAccountingMode"
          :selected-budget-traffic-classes="selectedBudgetTrafficClasses"
          :selected-budget-warning-thresholds="selectedBudgetWarningThresholds"
          :selected-budget-reservation-strategy="selectedBudgetReservationStrategy"
          :budget-accounting-mode-options="budgetAccountingModeOptions"
          :budget-reservation-strategy-options="budgetReservationStrategyOptions"
          :t="t"
          @update:name="updateSelectedConfiguredModel({ name: $event })"
          @update:custom-provider-label="updateSelectedCustomProviderLabel"
          @update:api-key="updateSelectedApiKey"
          @update:base-url="updateSelectedBaseUrl"
          @update:budget-total="updateSelectedBudgetTotal"
          @update:budget-accounting-mode="updateSelectedBudgetAccountingMode"
          @update:budget-traffic-classes="updateSelectedBudgetTrafficClasses"
          @update:budget-warning-thresholds="updateSelectedBudgetWarningThresholds"
          @update:budget-reservation-strategy="updateSelectedBudgetReservationStrategy"
          @update:enabled="updateSelectedConfiguredModel({ enabled: $event })"
          @clear-credential-override="clearSelectedCredentialOverride"
          @validate="validateSelectedConfiguredModel"
          @save="saveWorkspacePatch"
          @delete="deleteSelectedConfiguredModel"
        />
      </template>
    </UiListDetailWorkspace>

    <CreateModelDialog
      :open="createDialogOpen"
      :create-name="createName"
      :create-provider-type="createProviderType"
      :create-provider-options="createProviderOptions"
      :create-requires-custom-provider-name="createRequiresCustomProviderName"
      :create-custom-provider-label="createCustomProviderLabel"
      :create-uses-freeform-model="createUsesFreeformModel"
      :create-model-id="createModelId"
      :create-upstream-model-options="createUpstreamModelOptions"
      :create-api-key="createApiKey"
      :create-base-url="createBaseUrl"
      :create-budget-total="createBudgetTotal"
      :create-budget-accounting-mode="createBudgetAccountingMode"
      :create-budget-traffic-classes="createBudgetTrafficClasses"
      :create-budget-warning-thresholds="createBudgetWarningThresholds"
      :create-budget-reservation-strategy="createBudgetReservationStrategy"
      :budget-accounting-mode-options="budgetAccountingModeOptions"
      :budget-reservation-strategy-options="budgetReservationStrategyOptions"
      :create-enabled="createEnabled"
      :create-form-error="createFormError"
      :runtime-config-saving="runtime.configSaving"
      :runtime-config-validating="runtime.configValidating"
      :t="t"
      @update:open="createDialogOpen = $event"
      @update:create-name="createName = $event"
      @update:create-provider-type="createProviderType = $event"
      @update:create-custom-provider-label="createCustomProviderLabel = $event"
      @update:create-model-id="createModelId = $event"
      @update:create-api-key="createApiKey = $event"
      @update:create-base-url="createBaseUrl = $event"
      @update:create-budget-total="createBudgetTotal = $event"
      @update:create-budget-accounting-mode="createBudgetAccountingMode = $event"
      @update:create-budget-traffic-classes="createBudgetTrafficClasses = $event"
      @update:create-budget-warning-thresholds="createBudgetWarningThresholds = $event"
      @update:create-budget-reservation-strategy="createBudgetReservationStrategy = $event"
      @update:create-enabled="createEnabled = $event"
      @submit="createConfiguredModel"
    />
  </component>
</template>
