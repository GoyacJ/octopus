<script setup lang="ts">
import type { CatalogConfiguredModelOption } from '@/stores/catalog'
import { UiButton, UiCheckbox, UiEmptyState, UiField, UiRecordCard, UiSelect, UiStatusCallout } from '@octopus/ui'

defineProps<{
  modelTabReady: boolean
  allowedWorkspaceConfiguredModels: CatalogConfiguredModelOption[]
  modelsForm: { allowedConfiguredModelIds: string[], defaultConfiguredModelId: string }
  modelsError: string
  savingModels: boolean
}>()

const emit = defineEmits<{
  reset: []
  save: []
}>()
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.models.title')"
    :description="$t('projectSettings.models.description')"
  >
    <template #eyebrow>
      {{ $t('projectSettings.tabs.models') }}
    </template>

    <div v-if="!modelTabReady" class="text-sm text-text-secondary">
      {{ $t('projectSettings.loading') }}
    </div>

    <UiEmptyState
      v-else-if="!allowedWorkspaceConfiguredModels.length"
      :title="$t('projectSettings.models.emptyTitle')"
      :description="$t('projectSettings.models.emptyDescription')"
    />

    <div v-else class="space-y-5">
      <UiField
        :label="$t('projectSettings.models.allowedLabel')"
        :hint="$t('projectSettings.models.allowedHint')"
      >
        <div class="space-y-3">
          <label
            v-for="modelOption in allowedWorkspaceConfiguredModels"
            :key="modelOption.value"
            class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 transition-colors"
          >
            <div class="min-w-0 space-y-1">
              <div class="text-sm font-semibold text-text-primary">
                {{ modelOption.label }}
              </div>
              <div class="text-xs text-text-secondary">
                {{ modelOption.providerLabel }} · {{ modelOption.modelLabel }}
              </div>
            </div>
            <UiCheckbox
              v-model="modelsForm.allowedConfiguredModelIds"
              :value="modelOption.value"
              :aria-label="modelOption.label"
            />
          </label>
        </div>
      </UiField>

      <UiField
        :label="$t('projectSettings.models.defaultLabel')"
        :hint="$t('projectSettings.models.defaultHint')"
      >
        <UiSelect
          v-model="modelsForm.defaultConfiguredModelId"
          :disabled="!modelsForm.allowedConfiguredModelIds.length"
          :options="allowedWorkspaceConfiguredModels
            .filter(option => modelsForm.allowedConfiguredModelIds.includes(option.value))
            .map(option => ({
              value: option.value,
              label: `${option.label} · ${option.providerLabel}`,
            }))"
        />
      </UiField>

      <UiStatusCallout v-if="modelsError" tone="error" :description="modelsError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingModels" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton :disabled="savingModels || !allowedWorkspaceConfiguredModels.length" @click="emit('save')">
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
