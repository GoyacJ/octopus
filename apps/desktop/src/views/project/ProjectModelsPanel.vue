<script setup lang="ts">
import type { CatalogConfiguredModelOption } from '@/stores/catalog'
import { UiButton, UiCheckbox, UiEmptyState, UiField, UiInput, UiRecordCard, UiSelect, UiSkeleton, UiStatusCallout } from '@octopus/ui'

defineProps<{
  modelTabReady: boolean
  allowedWorkspaceConfiguredModels: CatalogConfiguredModelOption[]
  modelsForm: { allowedConfiguredModelIds: string[], defaultConfiguredModelId: string, totalTokens: string }
  projectUsedTokens: number
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

    <div
      v-if="!modelTabReady"
      data-testid="project-settings-models-skeleton"
      class="space-y-4"
    >
      <UiSkeleton variant="line" :count="2" />
      <UiSkeleton variant="card" :count="3" />
      <UiSkeleton variant="line" :count="2" />
    </div>

    <div v-else class="space-y-5">
      <UiField
        :label="$t('projectSettings.models.allowedLabel')"
        :hint="$t('projectSettings.models.allowedHint')"
      >
        <div v-if="allowedWorkspaceConfiguredModels.length" class="space-y-3">
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
        <UiEmptyState
          v-else
          :title="$t('projectSettings.models.emptyTitle')"
          :description="$t('projectSettings.models.emptyDescription')"
        />
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

      <div class="grid gap-4 md:grid-cols-2">
        <UiField
          :label="$t('projectSettings.models.totalTokensLabel')"
          :hint="$t('projectSettings.models.totalTokensHint')"
        >
          <UiInput
            v-model="modelsForm.totalTokens"
            data-testid="project-settings-total-tokens-input"
            type="number"
            :placeholder="$t('projectSettings.models.totalTokensPlaceholder')"
          />
        </UiField>

        <UiField
          :label="$t('projectSettings.models.usedTokensLabel')"
          :hint="$t('projectSettings.models.usedTokensHint')"
        >
          <div
            data-testid="project-settings-used-tokens-value"
            class="flex min-h-8 items-center rounded-[var(--radius-s)] border border-border bg-surface-muted px-3 text-sm text-text-primary"
          >
            {{ projectUsedTokens.toLocaleString() }}
          </div>
        </UiField>
      </div>

      <UiStatusCallout v-if="modelsError" tone="error" :description="modelsError" />
    </div>

    <template #actions>
      <UiButton data-testid="project-settings-models-reset-button" variant="ghost" :disabled="savingModels" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton data-testid="project-settings-models-save-button" :disabled="savingModels" @click="emit('save')">
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
