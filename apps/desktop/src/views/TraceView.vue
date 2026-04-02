<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiSectionHeading, UiStatTile, UiSurface, UiTraceBlock } from '@octopus/ui'

import { enumLabel, formatDateTime, resolveCopy, resolveMockField } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const runtime = useRuntimeStore()
const workbench = useWorkbenchStore()

const traceStatusLabel = computed(() => {
  const status = runtime.activeRun?.status
  if (!status) {
    return t('common.na')
  }

  try {
    return enumLabel('runStatus', status)
  } catch {
    return status
  }
})

const runtimeTraceTone = computed<'default' | 'success' | 'warning' | 'error' | 'info'>(() => {
  const tone = runtime.activeTrace[0]?.tone
  if (tone === 'success' || tone === 'warning' || tone === 'error' || tone === 'info') {
    return tone
  }
  return 'default'
})
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('trace.header.eyebrow')"
      :title="runtime.activeSession ? runtime.activeSession.summary.title : (workbench.activeRun ? resolveMockField('run', workbench.activeRun.id, 'title', workbench.activeRun.title) : t('trace.header.titleFallback'))"
      :subtitle="runtime.activeRun ? resolveCopy(runtime.activeRun.currentStep) : (workbench.activeRun ? resolveMockField('run', workbench.activeRun.id, 'currentStep', resolveCopy(workbench.activeRun.currentStep)) : t('trace.header.subtitleFallback'))"
    />

    <div v-if="runtime.activeRun" class="surface-grid three">
      <UiStatTile data-testid="trace-runtime-status" :label="t('trace.stats.status')" :value="traceStatusLabel" tone="warning" />
      <UiStatTile :label="t('trace.stats.owner')" :value="runtime.activeSession?.summary.id ?? t('common.na')" />
      <UiStatTile :label="t('trace.stats.nextAction')" :value="runtime.activeRun.nextAction ?? t('common.na')" />
    </div>
    <div v-else-if="workbench.activeRun" class="surface-grid three">
      <UiStatTile :label="t('trace.stats.status')" :value="enumLabel('runStatus', workbench.activeRun.status)" tone="warning" />
      <UiStatTile :label="t('trace.stats.owner')" :value="`${workbench.activeRun.ownerType}:${workbench.activeRun.ownerId}`" />
      <UiStatTile :label="t('trace.stats.nextAction')" :value="resolveMockField('run', workbench.activeRun.id, 'nextAction', resolveCopy(workbench.activeRun.nextAction))" />
    </div>

    <div class="surface-grid two">
      <UiSurface :title="t('trace.runState.title')" :subtitle="t('trace.runState.subtitle')">
        <div v-if="runtime.activeRun" class="section-stack">
          <div class="meta-row">
            <UiBadge :label="runtime.activeRun.modelId ?? t('common.na')" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.updatedAt)" subtle />
          </div>
          <p class="trace-copy">{{ resolveCopy(runtime.activeRun.currentStep) }}</p>
        </div>
        <div v-else-if="workbench.activeRun" class="section-stack">
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
        <div v-if="runtime.pendingApproval" class="section-stack">
          <div class="meta-row">
            <UiBadge :label="runtime.pendingApproval.toolName" subtle />
            <UiBadge :label="runtime.pendingApproval.riskLevel" tone="warning" />
          </div>
          <p class="trace-copy">{{ runtime.pendingApproval.summary }}</p>
          <p class="trace-copy">{{ runtime.pendingApproval.detail }}</p>
        </div>
        <ul v-else-if="workbench.activeConversation?.resumePoints.length">
          <li v-for="resumePoint in workbench.activeConversation?.resumePoints" :key="resumePoint.id">
            {{ resolveMockField('conversation', workbench.activeConversation.id, `resumePoints.${resumePoint.id}.label`, resumePoint.label) }}
          </li>
        </ul>
        <UiEmptyState v-else :title="t('trace.recovery.emptyTitle')" :description="t('trace.recovery.emptyDescription')" />
      </UiSurface>
    </div>

    <UiSurface :title="t('trace.timeline.title')" :subtitle="t('trace.timeline.subtitle')">
      <div v-if="runtime.activeTrace.length" class="panel-list">
        <UiTraceBlock
          v-for="trace in runtime.activeTrace"
          :key="trace.id"
          data-testid="trace-runtime-item"
          :title="trace.title"
          :detail="trace.detail"
          :actor="trace.actor"
          :timestamp-label="formatDateTime(trace.timestamp)"
          :tone="runtimeTraceTone"
        />
      </div>
      <div v-else-if="workbench.activeTrace.length" class="panel-list">
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
