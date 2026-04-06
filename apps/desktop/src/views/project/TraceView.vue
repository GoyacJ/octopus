<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiEmptyState, UiSectionHeading, UiStatTile, UiTraceBlock } from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const runtime = useRuntimeStore()
const workbench = useWorkbenchStore()

const traceStatusLabel = computed(() => runtime.activeRunStatusLabel || t('common.na'))
const canApproveTrace = computed(() =>
  workbench.hasPermission('trace:approval:approve', 'approve', 'project', workbench.currentProjectId),
)

const runtimeTraceTone = computed<'default' | 'success' | 'warning' | 'error' | 'info'>(() => {
  const tone = runtime.activeTrace[0]?.tone
  if (tone === 'success' || tone === 'warning' || tone === 'error' || tone === 'info') {
    return tone
  }
  return 'default'
})

async function approveRuntime() {
  await runtime.resolveApproval('approve')
}

async function rejectRuntime() {
  await runtime.resolveApproval('reject')
}
</script>

<template>
  <div class="w-full flex flex-col gap-10 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('trace.header.eyebrow')"
        :title="runtime.activeSession?.summary.title ?? (workbench.activeRun ? workbench.runDisplayTitle(workbench.activeRun.id) : t('trace.header.titleFallback'))"
        :subtitle="runtime.activeRun ? runtime.activeRunCurrentStepLabel : (workbench.activeRun ? workbench.runDisplayCurrentStep(workbench.activeRun.id) : t('trace.header.subtitleFallback'))"
      />
    </header>

    <!-- Run Stats (Full Width Grid) -->
    <div class="grid gap-4 sm:grid-cols-3 lg:grid-cols-3 xl:grid-cols-6 px-2">
      <template v-if="runtime.activeRun">
        <UiStatTile data-testid="trace-runtime-status" :label="t('trace.stats.status')" :value="traceStatusLabel" tone="warning" />
        <UiStatTile :label="t('trace.stats.owner')" :value="runtime.activeSession?.summary.id ?? t('common.na')" />
        <UiStatTile :label="t('trace.stats.nextAction')" :value="runtime.activeRunNextActionLabel || t('common.na')" />
      </template>
      <template v-else-if="workbench.activeRun">
        <UiStatTile :label="t('trace.stats.status')" :value="enumLabel('runStatus', workbench.activeRun.status)" tone="warning" />
        <UiStatTile :label="t('trace.stats.owner')" :value="`${workbench.activeRun.ownerType}:${workbench.activeRun.ownerId}`" />
        <UiStatTile :label="t('trace.stats.nextAction')" :value="workbench.runDisplayNextAction(workbench.activeRun.id)" />
      </template>
    </div>

    <!-- Run State & Recovery (Split View) -->
    <div class="grid gap-12 lg:grid-cols-2 px-2 border-t border-border-subtle dark:border-white/5 pt-10">
      <section class="space-y-6">
        <div class="space-y-1">
          <h3 class="text-xl font-bold text-text-primary">{{ t('trace.runState.title') }}</h3>
          <p class="text-[14px] text-text-secondary">{{ t('trace.runState.subtitle') }}</p>
        </div>

        <div v-if="runtime.activeRun" class="space-y-4 bg-card rounded-lg border border-border-subtle dark:border-white/5 p-6">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="runtime.activeRun.modelId ?? t('common.na')" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.updatedAt)" subtle />
          </div>
          <p class="text-sm leading-relaxed text-text-secondary">{{ runtime.activeRunCurrentStepLabel }}</p>
        </div>
        <div v-else-if="workbench.activeRun" class="space-y-4 bg-card rounded-lg border border-border-subtle dark:border-white/5 p-6">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="enumLabel('runType', workbench.activeRun.type)" subtle />
            <UiBadge :label="formatDateTime(workbench.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(workbench.activeRun.updatedAt)" subtle />
          </div>
          <p class="text-sm leading-relaxed text-text-secondary font-medium">{{ workbench.runDisplayCurrentStep(workbench.activeRun.id) }}</p>
          <ul class="list-disc pl-5 space-y-2 text-sm text-text-secondary">
            <li v-for="(blocker, index) in workbench.runDisplayBlockers(workbench.activeRun.id)" :key="`${workbench.activeRun.id}-${index}`">
              {{ blocker }}
            </li>
          </ul>
        </div>
        <UiEmptyState v-else :title="t('trace.runState.emptyTitle')" :description="t('trace.runState.emptyDescription')" />
      </section>

      <section class="space-y-6">
        <div class="space-y-1">
          <h3 class="text-xl font-bold text-text-primary">{{ t('trace.recovery.title') }}</h3>
          <p class="text-[14px] text-text-secondary">{{ t('trace.recovery.subtitle') }}</p>
        </div>

        <div v-if="runtime.pendingApproval" class="space-y-4 bg-status-warning/5 rounded-lg border border-status-warning/10 dark:border-status-warning/20 p-6" data-testid="trace-runtime-approval">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="runtime.pendingApproval.toolName" subtle />
            <UiBadge :label="runtime.pendingApproval.riskLevel" tone="warning" />
          </div>
          <p class="text-sm leading-relaxed text-text-primary font-bold">{{ runtime.pendingApproval.summary }}</p>
          <p class="text-sm leading-relaxed text-text-secondary">{{ runtime.pendingApproval.detail }}</p>
          <div v-if="canApproveTrace" class="flex flex-wrap gap-2">
            <UiButton data-testid="trace-runtime-approve" size="sm" @click="approveRuntime">{{ t('common.approve') }}</UiButton>
            <UiButton data-testid="trace-runtime-reject" variant="ghost" size="sm" @click="rejectRuntime">{{ t('common.reject') }}</UiButton>
          </div>
        </div>
        <div v-else-if="workbench.activeConversation?.resumePoints.length" class="bg-card rounded-lg border border-border-subtle dark:border-white/5 p-6">
          <ul class="list-disc pl-5 space-y-2 text-sm text-text-secondary">
            <li v-for="(resumePoint, index) in workbench.conversationResumePointLabels(workbench.activeConversation.id)" :key="`${workbench.activeConversation.id}-${index}`">
              {{ resumePoint }}
            </li>
          </ul>
        </div>
        <UiEmptyState v-else :title="t('trace.recovery.emptyTitle')" :description="t('trace.recovery.emptyDescription')" />
      </section>
    </div>

    <!-- Trace Timeline (Extended Full Width) -->
    <section class="space-y-8 px-2 border-t border-border-subtle dark:border-white/5 pt-10">
      <div class="space-y-1">
        <h3 class="text-xl font-bold text-text-primary">{{ t('trace.timeline.title') }}</h3>
        <p class="text-[14px] text-text-secondary">{{ t('trace.timeline.subtitle') }}</p>
      </div>

      <div class="space-y-4">
        <template v-if="runtime.activeTrace.length">
          <UiTraceBlock
            v-for="trace in runtime.activeTrace"
            :key="trace.id"
            data-testid="trace-runtime-item"
            :title="trace.title"
            :detail="trace.detail"
            :actor="trace.actor"
            :timestamp-label="formatDateTime(trace.timestamp)"
            :tone="runtimeTraceTone"
            class="max-w-5xl"
          />
        </template>
        <template v-else-if="workbench.activeTrace.length">
          <UiTraceBlock
            v-for="trace in workbench.activeTrace"
            :key="trace.id"
            :title="workbench.traceDisplayTitle(trace.id)"
            :detail="workbench.traceDisplayDetail(trace.id)"
            :actor="trace.actor"
            :timestamp-label="formatDateTime(trace.timestamp)"
            :tone="trace.status"
            class="max-w-5xl"
          />
        </template>
        <UiEmptyState v-else :title="t('trace.timeline.emptyTitle')" :description="t('trace.timeline.emptyDescription')" />
      </div>
    </section>
  </div>
</template>
