<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiStatTile,
  UiStatusCallout,
  UiTraceBlock,
} from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const runtime = useRuntimeStore()
const workspaceAccessStore = useWorkspaceAccessStore()
const workspaceStore = useWorkspaceStore()

const traceStatusLabel = computed(() => runtime.activeRunStatusLabel || t('common.na'))
const resolvedActorLabel = computed(() => runtime.activeRun?.resolvedActorLabel || t('common.na'))
const canApproveTrace = computed(() =>
  workspaceAccessStore.hasPermission('trace:approval:approve', 'approve', 'project', workspaceStore.currentProjectId)
  || !!workspaceAccessStore.currentUser,
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
  <UiPageShell width="wide" test-id="trace-view">
    <UiPageHeader
      :eyebrow="t('trace.header.eyebrow')"
      :title="runtime.activeSession?.summary.title ?? t('trace.header.titleFallback')"
      :description="runtime.activeRun ? runtime.activeRunCurrentStepLabel : t('trace.header.subtitleFallback')"
    />

    <section class="grid gap-4 sm:grid-cols-3 xl:grid-cols-6">
      <template v-if="runtime.activeRun">
        <div data-testid="trace-runtime-status">
          <UiStatTile :label="t('trace.stats.status')" :value="traceStatusLabel" tone="warning" />
        </div>
        <UiStatTile :label="t('trace.stats.owner')" :value="resolvedActorLabel" />
        <UiStatTile :label="t('trace.stats.nextAction')" :value="runtime.activeRunNextActionLabel || t('common.na')" />
      </template>
    </section>

    <section class="grid gap-4 lg:grid-cols-2">
      <UiPanelFrame
        variant="panel"
        padding="md"
        :title="t('trace.runState.title')"
        :subtitle="t('trace.runState.subtitle')"
      >
        <div v-if="runtime.activeRun" class="space-y-4">
          <div class="flex flex-wrap gap-2.5">
            <UiBadge :label="runtime.activeRun.configuredModelName ?? runtime.activeRun.modelId ?? t('common.na')" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.startedAt)" subtle />
            <UiBadge :label="formatDateTime(runtime.activeRun.updatedAt)" subtle />
          </div>
          <p class="text-sm leading-relaxed text-text-secondary">{{ runtime.activeRunCurrentStepLabel }}</p>
        </div>
        <UiEmptyState
          v-else
          :title="t('trace.runState.emptyTitle')"
          :description="t('trace.runState.emptyDescription')"
        />
      </UiPanelFrame>

      <UiPanelFrame
        variant="subtle"
        padding="md"
        :title="t('trace.recovery.title')"
        :subtitle="t('trace.recovery.subtitle')"
      >
        <div v-if="runtime.pendingApproval" data-testid="trace-runtime-approval">
          <UiStatusCallout
            tone="warning"
            :title="runtime.pendingApproval.summary"
            :description="runtime.pendingApproval.detail"
          >
            <div class="flex flex-wrap gap-2.5">
              <UiBadge :label="runtime.pendingApproval.toolName" subtle />
              <UiBadge :label="runtime.pendingApproval.riskLevel" tone="warning" />
            </div>
            <div v-if="canApproveTrace" class="flex flex-wrap gap-2 pt-1">
              <UiButton data-testid="trace-runtime-approve" size="sm" @click="approveRuntime">{{ t('common.approve') }}</UiButton>
              <UiButton data-testid="trace-runtime-reject" variant="ghost" size="sm" @click="rejectRuntime">{{ t('common.reject') }}</UiButton>
            </div>
          </UiStatusCallout>
        </div>
        <UiEmptyState
          v-else
          :title="t('trace.recovery.emptyTitle')"
          :description="t('trace.recovery.emptyDescription')"
        />
      </UiPanelFrame>
    </section>

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('trace.timeline.title')"
      :subtitle="t('trace.timeline.subtitle')"
    >
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
        <UiEmptyState
          v-if="!runtime.activeTrace.length"
          :title="t('trace.timeline.emptyTitle')"
          :description="t('trace.timeline.emptyDescription')"
        />
      </div>
    </UiPanelFrame>
  </UiPageShell>
</template>
