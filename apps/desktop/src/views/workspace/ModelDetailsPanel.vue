<script setup lang="ts">
import type {
  ConfiguredModelRecord,
  ModelRegistryRecord,
  ProviderRegistryRecord,
} from '@octopus/schema'
import { Trash2 } from 'lucide-vue-next'

import { enumLabel } from '@/i18n/copy'
import type { CatalogConfiguredModelRow } from '@/stores/catalog'
import { hasRuntimeExecutionSupport } from './models-runtime-helpers'
import { UiBadge, UiButton, UiCheckbox, UiEmptyState, UiField, UiInput, UiStatusCallout, UiSurface } from '@octopus/ui'

const props = defineProps<{
  selectedRow: CatalogConfiguredModelRow | null
  selectedConfiguredModel: ConfiguredModelRecord | null
  selectedModel: ModelRegistryRecord | null
  selectedProvider: ProviderRegistryRecord | null
  selectedApiKey: string
  selectedCredentialSourceLabel: string
  selectedCredentialSourceDescription: string
  selectedCredentialStatusLabel: string
  selectedCredentialStatusDescription: string
  selectedCredentialStatusTone: 'info' | 'success' | 'warning' | 'error'
  selectedCredentialBlocked: boolean
  selectedCanClearCredentialOverride: boolean
  selectedIsCustomManaged: boolean
  selectedProbeResult: {
    valid?: boolean
    reachable?: boolean
    configuredModelName?: string
    consumedTokens?: number
    requestId?: string
    errors?: string[]
    warnings?: string[]
  } | null
  runtimeConfigValidating: boolean
  runtimeConfiguredModelProbing: boolean
  runtimeConfigSaving: boolean
  validationErrors: string[]
  validationWarnings: string[]
  t: (key: string, params?: Record<string, unknown>) => string
}>()

const t = props.t

const emit = defineEmits<{
  'update:name': [value: string]
  'update:custom-provider-label': [value: string]
  'update:api-key': [value: string]
  'update:base-url': [value: string]
  'update:total-tokens': [value: string]
  'update:enabled': [value: boolean]
  clearCredentialOverride: []
  validate: []
  save: []
  delete: []
}>()

function validationTone() {
  if (props.selectedProbeResult?.valid && props.selectedProbeResult?.reachable) {
    return 'success' as const
  }

  if (props.validationErrors.length || props.selectedProbeResult?.errors?.length) {
    return 'error' as const
  }

  if (props.validationWarnings.length || props.selectedProbeResult?.warnings?.length) {
    return 'warning' as const
  }

  return 'info' as const
}

function validationTitle() {
  if (props.selectedProbeResult?.valid && props.selectedProbeResult?.reachable) {
    return props.t('models.validation.lastSuccessTitle')
  }

  if (props.validationErrors.length || props.selectedProbeResult?.errors?.length) {
    return props.t('models.validation.lastFailureTitle')
  }

  if (props.validationWarnings.length || props.selectedProbeResult?.warnings?.length) {
    return props.t('models.validation.lastWarningTitle')
  }

  return props.t('models.validation.idleTitle')
}

function validationDescription() {
  if (props.selectedProbeResult?.valid && props.selectedProbeResult?.reachable) {
    const description = props.t('models.validation.success', {
      name: props.selectedProbeResult.configuredModelName ?? props.selectedConfiguredModel?.name ?? '',
      tokens: props.selectedProbeResult.consumedTokens ?? 0,
    })

    return props.selectedProbeResult.requestId
      ? `${description} ${props.t('models.validation.requestId', { requestId: props.selectedProbeResult.requestId })}`
      : description
  }

  return props.validationErrors[0]
    ?? props.selectedProbeResult?.errors?.[0]
    ?? props.validationWarnings[0]
    ?? props.selectedProbeResult?.warnings?.[0]
    ?? props.t('models.validation.idleDescription')
}
</script>

<template>
  <section data-testid="workspace-models-detail-pane" class="space-y-4">
    <div
      v-if="selectedRow && selectedConfiguredModel && selectedModel && selectedProvider"
      data-testid="models-detail-panel"
      class="space-y-4"
    >
      <UiSurface variant="subtle" padding="sm">
        <div class="space-y-3">
          <div class="flex flex-wrap items-center gap-2">
            <UiBadge :label="selectedProvider.label" subtle />
            <UiBadge :label="selectedModel.label" subtle />
            <UiBadge
              :label="selectedConfiguredModel.enabled ? t('models.states.enabled') : t('models.states.disabled')"
              :tone="selectedConfiguredModel.enabled ? 'success' : 'warning'"
            />
          </div>
          <p class="text-sm leading-6 text-text-secondary">
            {{ selectedModel.description || t('models.detail.noDescription') }}
          </p>
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.overview')"
        :subtitle="t('models.detail.sections.overviewDescription')"
      >
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('models.detail.name')">
              <UiInput
                :model-value="selectedConfiguredModel.name"
                data-testid="models-detail-name-input"
                @update:model-value="emit('update:name', String($event))"
              />
            </UiField>

            <UiField v-if="selectedIsCustomManaged" :label="t('models.detail.customProviderName')">
              <UiInput
                :model-value="selectedProvider.label"
                data-testid="models-detail-provider-label-input"
                @update:model-value="emit('update:custom-provider-label', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.provider')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedProvider.label }}
              </div>
            </UiField>

            <UiField :label="t('models.detail.upstreamModel')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedModel.label }}
              </div>
            </UiField>
          </div>
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.authentication')"
        :subtitle="t('models.detail.sections.authenticationDescription')"
      >
        <div class="space-y-4">
          <div class="grid gap-3 xl:grid-cols-2">
            <UiStatusCallout
              tone="info"
              :title="selectedCredentialSourceLabel"
              :description="selectedCredentialSourceDescription"
            />
            <UiStatusCallout
              :tone="selectedCredentialStatusTone"
              :title="selectedCredentialStatusLabel"
              :description="selectedCredentialStatusDescription"
            />
          </div>

          <UiField
            :label="t('models.detail.credentialRef')"
            :hint="t('models.security.inputHint')"
          >
            <UiInput
              :model-value="selectedApiKey"
              data-testid="models-detail-credential-ref"
              type="password"
              :placeholder="t('models.detail.credentialRefPlaceholder')"
              @update:model-value="emit('update:api-key', String($event))"
            />
          </UiField>

          <div class="flex flex-wrap gap-2">
            <UiButton
              v-if="selectedCanClearCredentialOverride"
              variant="ghost"
              size="sm"
              data-testid="models-clear-credential-override"
              @click="emit('clearCredentialOverride')"
            >
              {{ t('models.actions.clearOverride') }}
            </UiButton>
          </div>
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.routing')"
        :subtitle="t('models.detail.sections.routingDescription')"
      >
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('models.detail.baseUrl')">
              <UiInput
                :model-value="selectedConfiguredModel.baseUrl ?? selectedProvider.surfaces[0]?.baseUrl ?? ''"
                data-testid="models-detail-base-url"
                :placeholder="t('models.detail.baseUrlPlaceholder')"
                @update:model-value="emit('update:base-url', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.surfaces')">
              <div class="flex min-h-9 flex-wrap items-center gap-2 rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  v-for="binding in selectedModel.surfaceBindings.filter(item => item.enabled && hasRuntimeExecutionSupport(item.runtimeSupport))"
                  :key="binding.surface"
                  :label="enumLabel('modelSurface', binding.surface)"
                  subtle
                />
                <span
                  v-if="!selectedModel.surfaceBindings.some(item => item.enabled && hasRuntimeExecutionSupport(item.runtimeSupport))"
                  class="text-sm text-text-secondary"
                >
                  {{ t('models.detail.noSurfaces') }}
                </span>
              </div>
            </UiField>
          </div>
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.quota')"
        :subtitle="t('models.detail.sections.quotaDescription')"
      >
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('models.detail.totalTokens')">
              <UiInput
                :model-value="selectedConfiguredModel.tokenQuota?.totalTokens ? String(selectedConfiguredModel.tokenQuota.totalTokens) : ''"
                data-testid="models-detail-total-tokens"
                type="number"
                :placeholder="t('models.detail.totalTokensPlaceholder')"
                @update:model-value="emit('update:total-tokens', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.usedTokens')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedRow.usedTokens.toLocaleString() }}
              </div>
            </UiField>

            <UiField :label="t('models.detail.remainingTokens')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedRow.remainingTokens?.toLocaleString() ?? t('models.quota.unlimited') }}
              </div>
            </UiField>

            <UiField :label="t('models.detail.quotaStatus')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  :label="selectedRow.totalTokens
                    ? (selectedRow.quotaExhausted ? t('models.quota.exhausted') : t('models.quota.available'))
                    : t('models.quota.unlimited')"
                  subtle
                />
              </div>
            </UiField>
          </div>

          <UiCheckbox
            :model-value="selectedConfiguredModel.enabled"
            :label="t('models.detail.enabled')"
            data-testid="models-detail-enabled"
            @update:model-value="emit('update:enabled', Boolean($event))"
          />
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.validation')"
        :subtitle="t('models.detail.sections.validationDescription')"
      >
        <div class="space-y-4">
          <div class="flex flex-wrap items-center gap-2">
            <UiButton
              data-testid="models-validate-button"
              variant="outline"
              size="sm"
              :loading="runtimeConfigValidating || runtimeConfiguredModelProbing"
              :disabled="selectedCredentialBlocked"
              @click="emit('validate')"
            >
              {{ t('models.actions.validate') }}
            </UiButton>
            <UiButton
              data-testid="models-save-button"
              size="sm"
              :loading="runtimeConfigSaving"
              :disabled="selectedCredentialBlocked"
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
            :tone="validationTone()"
            :title="validationTitle()"
            :description="validationDescription()"
          />
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.capabilities')"
        :subtitle="t('models.detail.sections.capabilitiesDescription')"
      >
        <div class="space-y-3">
          <div class="flex flex-wrap gap-2">
            <UiBadge
              v-for="capability in selectedModel.capabilities"
              :key="capability.capabilityId"
              :label="enumLabel('modelCapability', capability.capabilityId) || capability.label || capability.capabilityId"
              subtle
            />
            <p v-if="!selectedModel.capabilities.length" class="text-sm text-text-secondary">
              {{ t('models.detail.noCapabilities') }}
            </p>
          </div>
        </div>
      </UiSurface>
    </div>

    <UiEmptyState
      v-else
      :title="t('models.empty.selectionTitle')"
      :description="t('models.empty.selectionDescription')"
    />
  </section>
</template>
