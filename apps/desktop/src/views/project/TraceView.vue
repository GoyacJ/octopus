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
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const runtime = useRuntimeStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

const traceStatusLabel = computed(() => runtime.activeRunStatusLabel || t('common.na'))
const resolvedActorLabel = computed(() => runtime.activeRun?.resolvedActorLabel || t('common.na'))
const canResolveApprovalTrace = computed(() =>
  workspaceAccessControlStore.currentResourceActionGrants.some(grant =>
    (grant.resourceType === 'runtime.approval' && grant.actions.includes('resolve'))
    || (grant.resourceType === 'runtime' && grant.actions.includes('approval.resolve'))),
)
const canResolveAuthTrace = computed(() =>
  workspaceAccessControlStore.currentResourceActionGrants.some(grant =>
    (grant.resourceType === 'runtime.auth' && grant.actions.includes('resolve'))
    || (grant.resourceType === 'runtime' && grant.actions.includes('auth.resolve'))),
)
const pendingMemoryProposal = computed(() => runtime.activeRun?.pendingMemoryProposal ?? null)
const activeMediationKind = computed(() => {
  const mediationKind = runtime.pendingMediation?.mediationKind
  if (mediationKind && mediationKind !== 'none') {
    return mediationKind
  }
  if (runtime.pendingApproval) {
    return 'approval'
  }
  if (runtime.authTarget) {
    return 'auth'
  }
  return pendingMemoryProposal.value ? 'memory' : ''
})
const activeMediationTitle = computed(() =>
  runtime.pendingMediation?.summary
  ?? runtime.pendingApproval?.summary
  ?? runtime.authTarget?.summary
  ?? pendingMemoryProposal.value?.summary
  ?? '',
)
const activeMediationDetail = computed(() =>
  runtime.pendingMediation?.detail
  ?? runtime.pendingApproval?.detail
  ?? runtime.authTarget?.detail
  ?? pendingMemoryProposal.value?.proposalReason
  ?? '',
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

async function resolveRuntimeAuthChallenge() {
  await runtime.resolveAuthChallenge('resolved')
}

async function cancelRuntimeAuthChallenge() {
  await runtime.resolveAuthChallenge('cancelled')
}

async function approveMemoryProposal() {
  await runtime.resolveMemoryProposal('approve')
}

async function rejectMemoryProposal() {
  await runtime.resolveMemoryProposal('reject')
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
        <div v-if="activeMediationKind" data-testid="trace-runtime-approval">
          <UiStatusCallout
            tone="warning"
            :title="activeMediationTitle"
            :description="activeMediationDetail"
          >
            <div class="flex flex-wrap gap-2.5">
              <UiBadge v-if="runtime.pendingApproval?.toolName" :label="runtime.pendingApproval.toolName" subtle />
              <UiBadge v-if="runtime.pendingApproval?.riskLevel" :label="runtime.pendingApproval.riskLevel" tone="warning" />
              <UiBadge v-if="runtime.authTarget?.providerKey" :label="runtime.authTarget.providerKey" subtle />
              <UiBadge v-if="runtime.pendingMediation?.targetKind" :label="runtime.pendingMediation.targetKind" subtle />
            </div>
            <div class="flex flex-wrap gap-2 pt-1">
              <template v-if="activeMediationKind === 'approval' && runtime.pendingApproval && canResolveApprovalTrace">
                <UiButton data-testid="trace-runtime-approve" size="sm" @click="approveRuntime">{{ t('common.approve') }}</UiButton>
                <UiButton data-testid="trace-runtime-reject" variant="ghost" size="sm" @click="rejectRuntime">{{ t('common.reject') }}</UiButton>
              </template>
              <template v-else-if="activeMediationKind === 'auth' && runtime.authTarget && canResolveAuthTrace">
                <UiButton data-testid="trace-runtime-auth-resolve" size="sm" @click="resolveRuntimeAuthChallenge">{{ t('common.resolveAuth') }}</UiButton>
                <UiButton data-testid="trace-runtime-auth-cancel" variant="ghost" size="sm" @click="cancelRuntimeAuthChallenge">{{ t('common.cancel') }}</UiButton>
              </template>
              <template v-else-if="activeMediationKind === 'memory' && pendingMemoryProposal">
                <UiButton data-testid="trace-runtime-memory-approve" size="sm" @click="approveMemoryProposal">{{ t('common.approve') }}</UiButton>
                <UiButton data-testid="trace-runtime-memory-reject" variant="ghost" size="sm" @click="rejectMemoryProposal">{{ t('common.reject') }}</UiButton>
              </template>
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
