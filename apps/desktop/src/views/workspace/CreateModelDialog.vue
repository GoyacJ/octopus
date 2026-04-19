<script setup lang="ts">
import { UiButton, UiCheckbox, UiDialog, UiInput, UiSelect, UiStatusCallout } from '@octopus/ui'

import type { CatalogFilterOption } from '@/stores/catalog'

defineProps<{
  open: boolean
  createName: string
  createProviderType: string
  createProviderOptions: CatalogFilterOption[]
  createRequiresCustomProviderName: boolean
  createCustomProviderLabel: string
  createUsesFreeformModel: boolean
  createModelId: string
  createUpstreamModelOptions: CatalogFilterOption[]
  createApiKey: string
  createBaseUrl: string
  createBudgetTotal: string
  createBudgetAccountingMode: string
  createBudgetTrafficClasses: string
  createBudgetWarningThresholds: string
  createBudgetReservationStrategy: string
  budgetAccountingModeOptions: CatalogFilterOption[]
  budgetReservationStrategyOptions: CatalogFilterOption[]
  createEnabled: boolean
  createFormError: string
  runtimeConfigSaving: boolean
  runtimeConfigValidating: boolean
  t: (key: string) => string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'update:create-name': [value: string]
  'update:create-provider-type': [value: string]
  'update:create-custom-provider-label': [value: string]
  'update:create-model-id': [value: string]
  'update:create-api-key': [value: string]
  'update:create-base-url': [value: string]
  'update:create-budget-total': [value: string]
  'update:create-budget-accounting-mode': [value: string]
  'update:create-budget-traffic-classes': [value: string]
  'update:create-budget-warning-thresholds': [value: string]
  'update:create-budget-reservation-strategy': [value: string]
  'update:create-enabled': [value: boolean]
  submit: []
}>()
</script>

<template>
  <UiDialog
    :open="open"
    :title="t('models.create.title')"
    :description="t('models.create.description')"
    content-test-id="models-create-dialog"
    @update:open="emit('update:open', Boolean($event))"
  >
    <div class="grid gap-3">
      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.name') }}</p>
        <UiInput
          :model-value="createName"
          data-testid="models-create-name-input"
          @update:model-value="emit('update:create-name', String($event))"
        />
      </div>

      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.provider') }}</p>
        <UiSelect
          :model-value="createProviderType"
          data-testid="models-create-provider-select"
          :options="createProviderOptions"
          @update:model-value="emit('update:create-provider-type', String($event))"
        />
      </div>

      <div v-if="createRequiresCustomProviderName" class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.customProviderName') }}</p>
        <UiInput
          :model-value="createCustomProviderLabel"
          data-testid="models-create-custom-provider-name-input"
          :placeholder="t('models.create.placeholders.customProviderName')"
          @update:model-value="emit('update:create-custom-provider-label', String($event))"
        />
      </div>

      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.model') }}</p>
        <UiSelect
          v-if="!createUsesFreeformModel"
          :model-value="createModelId"
          data-testid="models-create-upstream-model-select"
          :options="createUpstreamModelOptions"
          @update:model-value="emit('update:create-model-id', String($event))"
        />
        <UiInput
          v-else
          :model-value="createModelId"
          data-testid="models-create-upstream-model-input"
          :placeholder="t('models.create.placeholders.modelId')"
          @update:model-value="emit('update:create-model-id', String($event))"
        />
      </div>

      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.credentialRef') }}</p>
        <UiInput
          :model-value="createApiKey"
          data-testid="models-create-credential-ref-input"
          type="password"
          :placeholder="t('models.detail.credentialRefPlaceholder')"
          @update:model-value="emit('update:create-api-key', String($event))"
        />
        <UiStatusCallout
          tone="info"
          :description="t('models.security.createHint')"
        />
      </div>

      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.baseUrl') }}</p>
        <UiInput
          :model-value="createBaseUrl"
          data-testid="models-create-base-url-input"
          :placeholder="t('models.detail.baseUrlPlaceholder')"
          @update:model-value="emit('update:create-base-url', String($event))"
        />
      </div>

      <div class="space-y-1">
        <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.budgetTotal') }}</p>
        <UiInput
          :model-value="createBudgetTotal"
          data-testid="models-create-total-tokens-input"
          type="number"
          :placeholder="t('models.create.placeholders.budgetTotal')"
          @update:model-value="emit('update:create-budget-total', String($event))"
        />
      </div>

      <div class="grid gap-3 md:grid-cols-2">
        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.budgetAccountingMode') }}</p>
          <UiSelect
            :model-value="createBudgetAccountingMode"
            data-testid="models-create-budget-accounting-mode-select"
            :options="budgetAccountingModeOptions"
            @update:model-value="emit('update:create-budget-accounting-mode', String($event))"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.budgetReservationStrategy') }}</p>
          <UiSelect
            :model-value="createBudgetReservationStrategy"
            data-testid="models-create-budget-reservation-strategy-select"
            :options="budgetReservationStrategyOptions"
            @update:model-value="emit('update:create-budget-reservation-strategy', String($event))"
          />
        </div>
      </div>

      <div class="grid gap-3 md:grid-cols-2">
        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.budgetTrafficClasses') }}</p>
          <UiInput
            :model-value="createBudgetTrafficClasses"
            data-testid="models-create-budget-traffic-classes-input"
            :placeholder="t('models.create.placeholders.budgetTrafficClasses')"
            @update:model-value="emit('update:create-budget-traffic-classes', String($event))"
          />
        </div>

        <div class="space-y-1">
          <p class="text-xs font-medium text-text-secondary">{{ t('models.create.fields.budgetWarningThresholds') }}</p>
          <UiInput
            :model-value="createBudgetWarningThresholds"
            data-testid="models-create-budget-warning-thresholds-input"
            :placeholder="t('models.create.placeholders.budgetWarningThresholds')"
            @update:model-value="emit('update:create-budget-warning-thresholds', String($event))"
          />
        </div>
      </div>

      <UiCheckbox
        :model-value="createEnabled"
        :label="t('models.create.fields.enabled')"
        data-testid="models-create-enabled"
        @update:model-value="emit('update:create-enabled', Boolean($event))"
      />

      <UiStatusCallout v-if="createFormError" tone="error" :description="createFormError" />
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="runtimeConfigSaving || runtimeConfigValidating" @click="emit('update:open', false)">
        {{ t('models.actions.cancel') }}
      </UiButton>
      <UiButton :disabled="runtimeConfigSaving || runtimeConfigValidating" @click="emit('submit')">
        {{ t('models.actions.confirmCreate') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
