<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiMetricCard, UiPanelFrame } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useCatalogStore } from '@/stores/catalog'
import { usePetStore } from '@/stores/pet'

const { t } = useI18n()
const catalogStore = useCatalogStore()
const petStore = usePetStore()

const selectedModelSummary = computed(() => {
  const configuredModelId = petStore.resolvedConfiguredModelId || petStore.preferredConfiguredModelId
  const configuredModel = catalogStore.getConfiguredModelRowById(configuredModelId)
  return configuredModel?.name ?? t('common.na')
})

const permissionSummary = computed(() => {
  switch (petStore.preferredPermissionMode) {
    case 'workspace-write':
      return t('personalCenter.pet.permissionModes.workspaceWrite')
    case 'danger-full-access':
      return t('personalCenter.pet.permissionModes.dangerFullAccess')
    default:
      return t('personalCenter.pet.permissionModes.readOnly')
  }
})

const lastInteractionLabel = computed(() =>
  petStore.dashboard.lastInteractionAt
    ? formatDateTime(petStore.dashboard.lastInteractionAt)
    : t('common.na'),
)

const metrics = computed(() => [
  {
    id: 'memory',
    label: t('personalCenter.pet.stats.metrics.memory'),
    value: String(petStore.dashboard.memoryCount),
    helper: t('personalCenter.pet.stats.helpers.memory'),
  },
  {
    id: 'knowledge',
    label: t('personalCenter.pet.stats.metrics.knowledge'),
    value: String(petStore.dashboard.knowledgeCount),
    helper: t('personalCenter.pet.stats.helpers.knowledge'),
  },
  {
    id: 'resource',
    label: t('personalCenter.pet.stats.metrics.resource'),
    value: String(petStore.dashboard.resourceCount),
    helper: t('personalCenter.pet.stats.helpers.resource'),
  },
  {
    id: 'reminder',
    label: t('personalCenter.pet.stats.metrics.reminder'),
    value: String(petStore.dashboard.reminderCount),
    helper: t('personalCenter.pet.stats.helpers.reminder'),
  },
  {
    id: 'activity',
    label: t('personalCenter.pet.stats.metrics.activity'),
    value: String(petStore.dashboard.activeConversationCount),
    helper: t('personalCenter.pet.stats.helpers.activity'),
  },
])
</script>

<template>
  <section data-testid="personal-center-pet-stats-panel">
    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('personalCenter.pet.stats.title')"
      :subtitle="t('personalCenter.pet.stats.description')"
    >
      <template #actions>
        <div class="flex flex-wrap gap-2">
          <UiBadge
            :label="t('personalCenter.pet.stats.badges.mood', { mood: petStore.dashboard.mood || petStore.profile.mood })"
            subtle
          />
          <UiBadge
            :label="t('personalCenter.pet.stats.badges.lastInteraction', { time: lastInteractionLabel })"
            subtle
          />
        </div>
      </template>

      <div class="grid gap-3 lg:grid-cols-3">
        <section
          data-testid="personal-center-pet-species-summary"
          class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-3"
        >
          <p class="text-[11px] font-semibold uppercase text-text-tertiary">
            {{ t('personalCenter.pet.stats.summary.species') }}
          </p>
          <p class="mt-2 text-sm font-medium text-text-primary">
            {{ petStore.dashboard.species || petStore.profile.species }}
          </p>
        </section>

        <section
          data-testid="personal-center-pet-model-summary"
          class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-3"
        >
          <p class="text-[11px] font-semibold uppercase text-text-tertiary">
            {{ t('personalCenter.pet.stats.summary.model') }}
          </p>
          <p class="mt-2 text-sm font-medium text-text-primary">
            {{ selectedModelSummary }}
          </p>
        </section>

        <section
          data-testid="personal-center-pet-permission-summary"
          class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-3"
        >
          <p class="text-[11px] font-semibold uppercase text-text-tertiary">
            {{ t('personalCenter.pet.stats.summary.permission') }}
          </p>
          <p class="mt-2 text-sm font-medium text-text-primary">
            {{ permissionSummary }}
          </p>
        </section>
      </div>

      <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-5">
        <UiMetricCard
          v-for="metric in metrics"
          :key="metric.id"
          :label="metric.label"
          :value="metric.value"
          :helper="metric.helper"
          :data-testid="`personal-center-pet-metric-${metric.id}`"
        />
      </div>
    </UiPanelFrame>
  </section>
</template>
