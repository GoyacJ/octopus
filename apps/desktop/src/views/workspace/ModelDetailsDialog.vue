<script setup lang="ts">
import type {
  ConfiguredModelRecord,
  ModelRegistryRecord,
  ProviderRegistryRecord,
} from '@octopus/schema'
import { Trash2 } from 'lucide-vue-next'

import type { CatalogConfiguredModelRow } from '@/stores/catalog'
import { UiBadge, UiButton, UiCheckbox, UiDialog, UiEmptyState, UiInput, UiStatusCallout, UiSurface } from '@octopus/ui'

defineProps<{
  open: boolean
  selectedRow: CatalogConfiguredModelRow | null
  selectedConfiguredModel: ConfiguredModelRecord | null
  selectedModel: ModelRegistryRecord | null
  selectedProvider: ProviderRegistryRecord | null
  selectedIsCustomManaged: boolean
  selectedProbeResult: {
    reachable?: boolean
    configuredModelName?: string
    consumedTokens?: number
    requestId?: string
  } | null
  hasPendingPatch: boolean
  runtimeConfigValidating: boolean
  runtimeConfiguredModelProbing: boolean
  runtimeConfigSaving: boolean
  validationErrors: string[]
  validationWarnings: string[]
  t: (key: string, params?: Record<string, unknown>) => string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'update:name': [value: string]
  'update:custom-provider-label': [value: string]
  'update:credential-ref': [value: string]
  'update:base-url': [value: string]
  'update:total-tokens': [value: string]
  'update:enabled': [value: boolean]
  validate: []
  save: []
  delete: []
}>()
</script>

<template>
  <UiDialog
    :open="open"
    :title="selectedConfiguredModel?.name ?? t('models.empty.selectionTitle')"
    :description="selectedProvider && selectedModel ? `${selectedProvider.label} · ${selectedModel.label}` : t('models.empty.selectionDescription')"
    content-test-id="models-detail-dialog"
    @update:open="emit('update:open', Boolean($event))"
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
            @update:model-value="emit('update:name', String($event))"
          />
        </div>

        <div v-if="selectedIsCustomManaged" class="space-y-1">
          <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-text-tertiary">
            {{ t('models.detail.customProviderName') }}
          </p>
          <UiInput
            :model-value="selectedProvider.label"
            data-testid="models-detail-provider-label-input"
            @update:model-value="emit('update:custom-provider-label', String($event))"
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
            @update:model-value="emit('update:credential-ref', String($event))"
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
            @update:model-value="emit('update:base-url', String($event))"
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
            @update:model-value="emit('update:total-tokens', String($event))"
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
            @update:model-value="emit('update:enabled', Boolean($event))"
          />

          <div class="flex flex-wrap items-center gap-2">
            <UiButton
              data-testid="models-validate-button"
              variant="ghost"
              size="sm"
              :disabled="runtimeConfigValidating || runtimeConfiguredModelProbing"
              @click="emit('validate')"
            >
              {{ t('models.actions.validate') }}
            </UiButton>
            <UiButton
              data-testid="models-save-button"
              size="sm"
              :disabled="runtimeConfigSaving || !hasPendingPatch"
              @click="emit('save')"
            >
              {{ t('models.actions.save') }}
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              class="justify-start text-status-error"
              data-testid="models-delete-button"
              @click="emit('delete')"
            >
              <Trash2 :size="14" />
              {{ t('models.actions.delete') }}
            </UiButton>
          </div>

          <UiStatusCallout
            v-if="validationErrors.length"
            tone="error"
            :description="validationErrors.join(' ')"
          />
          <UiStatusCallout
            v-if="validationWarnings.length"
            tone="warning"
            :description="validationWarnings.join(' ')"
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
</template>
