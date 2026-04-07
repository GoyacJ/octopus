<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiEmptyState, UiSectionHeading, UiStatTile, UiTraceBlock } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const runtime = useRuntimeStore()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const traceStatusLabel = computed(() => runtime.activeRunStatusLabel || t('common.na'))
const resolvedActorLabel = computed(() => runtime.activeRun?.resolvedActorLabel || t('common.na'))
const canApproveTrace = computed(() =>
  userCenterStore.hasPermission('trace:approval:approve', 'approve', 'project', workspaceStore.currentProjectId)
  || !!userCenterStore.currentUser,
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
  <div class="flex w-full flex-col gap-10 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('trace.header.eyebrow')"
        :title="runtime.activeSession?.summary.title ?? t('trace.header.titleFallback')"
        :subtitle="runtime.activeRun ? runtime.activeRunCurrentStepLabel : t('trace.header.subtitleFallback')"
      />
    </header>

    <div class="grid gap-4 px-2 sm:grid-cols-3 xl:grid-cols-6">
      <template v-if="runtime.activeRun">
        <div data-testid="trace-runtime-status">
          <UiStatTile :label="t('trace.stats.status')" :value="traceStatusLabel" tone="warning" />
        </div>
        <UiStatTile :label="t('trace.stats.owner')" :value="resolvedActorLabel" />
        <UiStatTile :label="t('trace.stats.nextAction')" :value="runtime.activeRunNextActionLabel || t('common.na')" />
      </template>
    </div>

    <div class="grid gap-12 px-2 pt-10 lg:grid-cols-2">
      <section class="space-y-6">
        <div class="space-y-1">
          <h3 class="text-xl font-bold text-text-primary">{{ t('trace.runState.title') }}</h3>
          <p class="text-[14px] text-text-secondary">{{ t('trace.runState.subtitle') }}</p>
        </div>

        <div v-if="runtime.activeRun" class="space-y-4 rounded-lg border border-border-subtle p-6 dark:border-white/[0.05]">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="runtime.activeRun.configuredModelName ?? runtime.activeRun.modelId ?? t('common.na')" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.updatedAt)" subtle />
          </div>
          <p class="text-sm leading-relaxed text-text-secondary">{{ runtime.activeRunCurrentStepLabel }}</p>
        </div>
        <UiEmptyState v-else :title="t('trace.runState.emptyTitle')" :description="t('trace.runState.emptyDescription')" />
      </section>

      <section class="space-y-6">
        <div class="space-y-1">
          <h3 class="text-xl font-bold text-text-primary">{{ t('trace.recovery.title') }}</h3>
          <p class="text-[14px] text-text-secondary">{{ t('trace.recovery.subtitle') }}</p>
        </div>

        <div v-if="runtime.pendingApproval" class="space-y-4 rounded-lg border border-status-warning/20 bg-status-warning/5 p-6" data-testid="trace-runtime-approval">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="runtime.pendingApproval.toolName" subtle />
            <UiBadge :label="runtime.pendingApproval.riskLevel" tone="warning" />
          </div>
          <p class="text-sm font-bold leading-relaxed text-text-primary">{{ runtime.pendingApproval.summary }}</p>
          <p class="text-sm leading-relaxed text-text-secondary">{{ runtime.pendingApproval.detail }}</p>
          <div v-if="canApproveTrace" class="flex flex-wrap gap-2">
            <UiButton data-testid="trace-runtime-approve" size="sm" @click="approveRuntime">{{ t('common.approve') }}</UiButton>
            <UiButton data-testid="trace-runtime-reject" variant="ghost" size="sm" @click="rejectRuntime">{{ t('common.reject') }}</UiButton>
          </div>
        </div>
        <UiEmptyState v-else :title="t('trace.recovery.emptyTitle')" :description="t('trace.recovery.emptyDescription')" />
      </section>
    </div>

    <section class="space-y-8 px-2 pt-10">
      <div class="space-y-1">
        <h3 class="text-xl font-bold text-text-primary">{{ t('trace.timeline.title') }}</h3>
        <p class="text-[14px] text-text-secondary">{{ t('trace.timeline.subtitle') }}</p>
      </div>

      <div class="space-y-4">
        <div
          v-for="trace in runtime.activeTrace"
          :key="trace.id"
          data-testid="trace-runtime-item"
        >
          <UiTraceBlock
            :title="trace.title"
            :detail="trace.detail"
            :actor="trace.actor"
            :timestamp-label="formatDateTime(trace.timestamp)"
            :tone="runtimeTraceTone"
            class="max-w-5xl"
          />
        </div>
        <UiEmptyState v-if="!runtime.activeTrace.length" :title="t('trace.timeline.emptyTitle')" :description="t('trace.timeline.emptyDescription')" />
      </div>
    </section>
  </div>
</template>
