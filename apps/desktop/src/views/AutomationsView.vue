<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('automations.placeholder.eyebrow')"
      :title="t('automations.placeholder.title')"
      :subtitle="t('automations.placeholder.subtitle')"
    />

    <UiSurface :title="t('automations.list.title')" :subtitle="t('automations.list.subtitle')">
      <div v-if="workbench.workspaceAutomations.length" class="panel-list">
        <article v-for="automation in workbench.workspaceAutomations" :key="automation.id" class="automation-card">
          <div class="meta-row">
            <UiBadge :label="enumLabel('automationStatus', automation.status)" :tone="automation.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="resolveMockField('automation', automation.id, 'cadence', automation.cadence)" subtle />
          </div>
          <strong>{{ resolveMockField('automation', automation.id, 'title', automation.title) }}</strong>
          <p>{{ resolveMockField('automation', automation.id, 'description', automation.description) }}</p>
          <small>{{ resolveMockField('automation', automation.id, 'output', automation.output) }}</small>
        </article>
      </div>
      <UiEmptyState v-else :title="t('automations.list.emptyTitle')" :description="t('automations.list.emptyDescription')" />
    </UiSurface>
  </section>
</template>

<style scoped>
.automation-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.automation-card p,
.automation-card small {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}
</style>
