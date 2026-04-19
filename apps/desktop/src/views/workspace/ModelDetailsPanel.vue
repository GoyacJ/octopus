<script setup lang="ts">
import { computed } from 'vue'
import type {
  ConfiguredModelRecord,
  ModelRegistryRecord,
  ProviderRegistryRecord,
} from '@octopus/schema'
import { Trash2 } from 'lucide-vue-next'

import { enumLabel } from '@/i18n/copy'
import type { CatalogConfiguredModelRow } from '@/stores/catalog'
import { summarizeModelExecution } from '@/stores/catalog_normalizers'
import type { CatalogFilterOption } from '@/stores/catalog'
import { UiBadge, UiButton, UiCheckbox, UiEmptyState, UiField, UiInput, UiSelect, UiStatusCallout, UiSurface } from '@octopus/ui'

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
  selectedBudgetAccountingMode: string
  selectedBudgetTrafficClasses: string
  selectedBudgetWarningThresholds: string
  selectedBudgetReservationStrategy: string
  budgetAccountingModeOptions: CatalogFilterOption[]
  budgetReservationStrategyOptions: CatalogFilterOption[]
  t: (key: string, params?: Record<string, unknown>) => string
}>()

const t = props.t

const emit = defineEmits<{
  'update:name': [value: string]
  'update:custom-provider-label': [value: string]
  'update:api-key': [value: string]
  'update:base-url': [value: string]
  'update:budget-total': [value: string]
  'update:budget-accounting-mode': [value: string]
  'update:budget-traffic-classes': [value: string]
  'update:budget-warning-thresholds': [value: string]
  'update:budget-reservation-strategy': [value: string]
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

const executionSummary = computed(() => summarizeModelExecution(props.selectedModel))

function executionClassLabel() {
  return enumLabel('modelExecutionClass', executionSummary.value.executionClass)
}

function executionClassTone() {
  switch (executionSummary.value.executionClass) {
    case 'agent_conversation':
      return 'success' as const
    case 'single_shot_generation':
      return 'warning' as const
    default:
      return 'error' as const
  }
}

function capabilityStateLabel(value: boolean) {
  return value
    ? props.t('models.execution.supported')
    : props.t('models.execution.notSupported')
}

function runtimeProfileTone() {
  switch (executionSummary.value.executionClass) {
    case 'agent_conversation':
      return 'success' as const
    case 'single_shot_generation':
      return 'warning' as const
    default:
      return 'error' as const
  }
}

function runtimeProfileTitle() {
  switch (executionSummary.value.executionClass) {
    case 'agent_conversation':
      return props.t('models.execution.profileReadyTitle')
    case 'single_shot_generation':
      return props.t('models.execution.profileGenerationTitle')
    default:
      return props.t('models.execution.profileUnsupportedTitle')
  }
}

function runtimeProfileDescription() {
  switch (executionSummary.value.executionClass) {
    case 'agent_conversation':
      return props.t('models.execution.profileReadyDescription')
    case 'single_shot_generation':
      return props.t('models.execution.profileGenerationDescription')
    default:
      return props.t('models.execution.profileUnsupportedDescription')
  }
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
        :title="t('models.detail.sections.executionProfile')"
        :subtitle="t('models.detail.sections.executionProfileDescription')"
      >
        <div class="space-y-4">
          <UiStatusCallout
            :tone="runtimeProfileTone()"
            :title="runtimeProfileTitle()"
            :description="runtimeProfileDescription()"
          />

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('models.detail.baseUrl')">
              <UiInput
                :model-value="selectedConfiguredModel.baseUrl ?? selectedProvider.surfaces[0]?.baseUrl ?? ''"
                data-testid="models-detail-base-url"
                :placeholder="t('models.detail.baseUrlPlaceholder')"
                @update:model-value="emit('update:base-url', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.executionClass')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge :label="executionClassLabel()" :tone="executionClassTone()" />
              </div>
            </UiField>

            <UiField :label="t('models.detail.agentSessionEligibility')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  :label="capabilityStateLabel(executionSummary.supportsConversationExecution)"
                  :tone="executionSummary.supportsConversationExecution ? 'success' : 'warning'"
                />
              </div>
            </UiField>

            <UiField :label="t('models.detail.upstreamStreaming')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  :label="capabilityStateLabel(executionSummary.upstreamStreaming)"
                  :tone="executionSummary.upstreamStreaming ? 'success' : 'warning'"
                />
              </div>
            </UiField>

            <UiField :label="t('models.detail.toolLoop')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  :label="capabilityStateLabel(executionSummary.toolLoop)"
                  :tone="executionSummary.toolLoop ? 'success' : 'warning'"
                />
              </div>
            </UiField>

            <UiField :label="t('models.detail.enabledSurfaces')">
              <div class="flex min-h-9 flex-wrap items-center gap-2 rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  v-for="surface in executionSummary.enabledSurfaces"
                  :key="surface"
                  :label="enumLabel('modelSurface', surface)"
                  subtle
                />
                <span
                  v-if="!executionSummary.enabledSurfaces.length"
                  class="text-sm text-text-secondary"
                >
                  {{ t('models.execution.noEnabledSurfaces') }}
                </span>
              </div>
            </UiField>

            <UiField :label="t('models.detail.agentSurfaces')">
              <div class="flex min-h-9 flex-wrap items-center gap-2 rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  v-for="surface in executionSummary.conversationSurfaces"
                  :key="surface"
                  :label="enumLabel('modelSurface', surface)"
                  subtle
                />
                <span
                  v-if="!executionSummary.conversationSurfaces.length"
                  class="text-sm text-text-secondary"
                >
                  {{ t('models.execution.noAgentSurfaces') }}
                </span>
              </div>
            </UiField>
          </div>
        </div>
      </UiSurface>

      <UiSurface
        variant="subtle"
        padding="sm"
        :title="t('models.detail.sections.budgetPolicy')"
        :subtitle="t('models.detail.sections.budgetPolicyDescription')"
      >
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('models.detail.budgetTotal')">
              <UiInput
                :model-value="selectedConfiguredModel.budgetPolicy?.totalBudgetTokens ? String(selectedConfiguredModel.budgetPolicy.totalBudgetTokens) : ''"
                data-testid="models-detail-total-tokens"
                type="number"
                :placeholder="t('models.detail.budgetTotalPlaceholder')"
                @update:model-value="emit('update:budget-total', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.budgetAccountingMode')">
              <UiSelect
                :model-value="selectedBudgetAccountingMode"
                data-testid="models-detail-budget-accounting-mode"
                :options="budgetAccountingModeOptions"
                @update:model-value="emit('update:budget-accounting-mode', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.budgetTrafficClasses')">
              <UiInput
                :model-value="selectedBudgetTrafficClasses"
                data-testid="models-detail-budget-traffic-classes"
                :placeholder="t('models.detail.budgetTrafficClassesPlaceholder')"
                @update:model-value="emit('update:budget-traffic-classes', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.budgetReservationStrategy')">
              <UiSelect
                :model-value="selectedBudgetReservationStrategy"
                data-testid="models-detail-budget-reservation-strategy"
                :options="budgetReservationStrategyOptions"
                @update:model-value="emit('update:budget-reservation-strategy', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.budgetWarningThresholds')">
              <UiInput
                :model-value="selectedBudgetWarningThresholds"
                data-testid="models-detail-budget-warning-thresholds"
                :placeholder="t('models.detail.budgetWarningThresholdsPlaceholder')"
                @update:model-value="emit('update:budget-warning-thresholds', String($event))"
              />
            </UiField>

            <UiField :label="t('models.detail.usedTokens')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedRow.usedTokens.toLocaleString() }}
              </div>
            </UiField>

            <UiField :label="t('models.detail.remainingTokens')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2 text-sm text-text-primary">
                {{ selectedRow.remainingTokens?.toLocaleString() ?? t('models.budget.unlimited') }}
              </div>
            </UiField>

            <UiField :label="t('models.detail.budgetStatus')">
              <div class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-2">
                <UiBadge
                  :label="selectedRow.totalTokens
                    ? (selectedRow.budgetExhausted ? t('models.budget.exhausted') : t('models.budget.available'))
                    : t('models.budget.unlimited')"
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
