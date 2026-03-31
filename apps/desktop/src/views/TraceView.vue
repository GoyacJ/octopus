<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiSectionHeading, UiStatTile, UiSurface, UiTraceBlock } from '@octopus/ui'

import { enumLabel, formatDateTime, resolveCopy, resolveMockField } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('trace.header.eyebrow')"
      :title="workbench.activeRun ? resolveMockField('run', workbench.activeRun.id, 'title', workbench.activeRun.title) : t('trace.header.titleFallback')"
      :subtitle="workbench.activeRun ? resolveMockField('run', workbench.activeRun.id, 'currentStep', resolveCopy(workbench.activeRun.currentStep)) : t('trace.header.subtitleFallback')"
    />

    <div v-if="workbench.activeRun" class="surface-grid three">
      <UiStatTile :label="t('trace.stats.status')" :value="enumLabel('runStatus', workbench.activeRun.status)" tone="warning" />
      <UiStatTile :label="t('trace.stats.owner')" :value="`${workbench.activeRun.ownerType}:${workbench.activeRun.ownerId}`" />
      <UiStatTile :label="t('trace.stats.nextAction')" :value="resolveMockField('run', workbench.activeRun.id, 'nextAction', resolveCopy(workbench.activeRun.nextAction))" />
    </div>

    <div class="surface-grid two">
      <UiSurface :title="t('trace.runState.title')" :subtitle="t('trace.runState.subtitle')">
        <div v-if="workbench.activeRun" class="section-stack">
          <div class="meta-row">
            <UiBadge :label="enumLabel('runType', workbench.activeRun.type)" subtle />
            <UiBadge :label="formatDateTime(workbench.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(workbench.activeRun.updatedAt)" subtle />
          </div>
          <p class="trace-copy">{{ resolveMockField('run', workbench.activeRun.id, 'currentStep', resolveCopy(workbench.activeRun.currentStep)) }}</p>
          <ul>
            <li v-for="(blocker, index) in workbench.activeRun.blockers" :key="`${workbench.activeRun.id}-${index}`">
              {{ resolveMockField('run', workbench.activeRun.id, `blockers.${index}`, resolveCopy(blocker)) }}
            </li>
          </ul>
        </div>
        <UiEmptyState v-else :title="t('trace.runState.emptyTitle')" :description="t('trace.runState.emptyDescription')" />
      </UiSurface>

      <UiSurface :title="t('trace.recovery.title')" :subtitle="t('trace.recovery.subtitle')">
        <ul v-if="workbench.activeConversation?.resumePoints.length">
          <li v-for="resumePoint in workbench.activeConversation?.resumePoints" :key="resumePoint.id">
            {{ resolveMockField('conversation', workbench.activeConversation.id, `resumePoints.${resumePoint.id}.label`, resumePoint.label) }}
          </li>
        </ul>
        <UiEmptyState v-else :title="t('trace.recovery.emptyTitle')" :description="t('trace.recovery.emptyDescription')" />
      </UiSurface>
    </div>

    <UiSurface :title="t('trace.timeline.title')" :subtitle="t('trace.timeline.subtitle')">
      <div v-if="workbench.activeTrace.length" class="panel-list">
        <UiTraceBlock
          v-for="trace in workbench.activeTrace"
          :key="trace.id"
          :title="resolveMockField('traceRecord', trace.id, 'title', trace.title)"
          :detail="resolveMockField('traceRecord', trace.id, 'detail', trace.detail)"
          :actor="trace.actor"
          :timestamp-label="formatDateTime(trace.timestamp)"
          :tone="trace.status"
        />
      </div>
      <UiEmptyState v-else :title="t('trace.timeline.emptyTitle')" :description="t('trace.timeline.emptyDescription')" />
    </UiSurface>
  </section>
</template>

<style scoped>
.trace-copy,
li {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding-left: 1rem;
}
</style>
