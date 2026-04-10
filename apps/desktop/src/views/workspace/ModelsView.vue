<script setup lang="ts">
import { Plus } from 'lucide-vue-next'

import { UiButton, UiPageHeader, UiPageShell } from '@octopus/ui'

import CreateModelDialog from './CreateModelDialog.vue'
import ModelDetailsDialog from './ModelDetailsDialog.vue'
import ModelsTablePanel from './ModelsTablePanel.vue'
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
  detailDialogOpen,
  createDialogOpen,
  createName,
  createProviderType,
  createCustomProviderLabel,
  createModelId,
  createCredentialRef,
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
  selectedIsCustomManaged,
  selectedProbeResult,
  hasPendingPatch,
  createProviderOptions,
  createUsesFreeformModel,
  createRequiresCustomProviderName,
  createUpstreamModelOptions,
  columns,
  updateSelectedConfiguredModel,
  updateSelectedTokenQuota,
  updateSelectedBaseUrl,
  updateSelectedCustomProviderLabel,
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
    >
      <template #actions>
        <UiButton data-testid="models-create-button" size="sm" @click="openCreateDialog">
          <Plus :size="14" />
          {{ t('models.actions.create') }}
        </UiButton>
      </template>
    </UiPageHeader>

    <div v-else class="flex justify-end">
      <UiButton data-testid="models-create-button" size="sm" @click="openCreateDialog">
        <Plus :size="14" />
        {{ t('models.actions.create') }}
      </UiButton>
    </div>

    <ModelsTablePanel
      :paged-rows="pagedRows"
      :columns="columns"
      :search-query="searchQuery"
      :provider-filter="providerFilter"
      :surface-filter="surfaceFilter"
      :capability-filter="capabilityFilter"
      :local-filter-options="localFilterOptions"
      :filtered-rows-length="filteredRows.length"
      :page="page"
      :page-count="pageCount"
      :t="t"
      @update:search-query="searchQuery = $event"
      @update:provider-filter="providerFilter = $event"
      @update:surface-filter="surfaceFilter = $event"
      @update:capability-filter="capabilityFilter = $event"
      @update:page="page = $event"
      @select-row="selectRow"
    />

    <ModelDetailsDialog
      :open="detailDialogOpen"
      :selected-row="selectedRow"
      :selected-configured-model="selectedConfiguredModel"
      :selected-model="selectedModel"
      :selected-provider="selectedProvider"
      :selected-is-custom-managed="selectedIsCustomManaged"
      :selected-probe-result="selectedProbeResult"
      :has-pending-patch="hasPendingPatch"
      :runtime-config-validating="runtime.configValidating"
      :runtime-configured-model-probing="runtime.configuredModelProbing"
      :runtime-config-saving="runtime.configSaving"
      :validation-errors="runtime.configValidation.workspace?.errors ?? []"
      :validation-warnings="runtime.configValidation.workspace?.warnings ?? []"
      :t="t"
      @update:open="detailDialogOpen = $event"
      @update:name="updateSelectedConfiguredModel({ name: $event })"
      @update:custom-provider-label="updateSelectedCustomProviderLabel"
      @update:credential-ref="updateSelectedConfiguredModel({ credentialRef: $event.trim() || undefined })"
      @update:base-url="updateSelectedBaseUrl"
      @update:total-tokens="updateSelectedTokenQuota"
      @update:enabled="updateSelectedConfiguredModel({ enabled: $event })"
      @validate="validateSelectedConfiguredModel"
      @save="saveWorkspacePatch"
      @delete="deleteSelectedConfiguredModel"
    />

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
      :create-credential-ref="createCredentialRef"
      :create-base-url="createBaseUrl"
      :create-total-tokens="createTotalTokens"
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
      @update:create-credential-ref="createCredentialRef = $event"
      @update:create-base-url="createBaseUrl = $event"
      @update:create-total-tokens="createTotalTokens = $event"
      @update:create-enabled="createEnabled = $event"
      @submit="createConfiguredModel"
    />
  </component>
</template>
