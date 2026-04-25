<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiContextMenu,
  UiEmptyState,
  UiInspectorPanel,
  UiPageHeader,
  UiPageShell,
  UiStatTile,
  UiStatusCallout,
  UiTraceBlock,
} from '@octopus/ui'

import { buildRoutePermalink, copyTextToClipboard } from '@/composables/clipboard'
import { formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
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
const pendingMemoryProposal = computed(() => runtime.pendingMemoryProposal)
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

function resolveTraceTone(tone: string | undefined): 'default' | 'success' | 'warning' | 'error' | 'info' {
  if (tone === 'success' || tone === 'warning' || tone === 'error' || tone === 'info') {
    return tone
  }
  return 'default'
}

function formatTraceMeta(value: string | undefined): string {
  if (!value) {
    return ''
  }

  return value
    .split(/[_-]/)
    .filter(Boolean)
    .map(segment => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(' ')
}

function buildTraceMetaItems(trace: { kind: string, relatedToolName?: string }): string[] {
  return [
    formatTraceMeta(trace.kind),
    trace.relatedToolName ?? '',
  ].filter(Boolean)
}

function buildTraceContextMenuItems(trace: { detail?: string | null }) {
  return [
    {
      key: 'copy-detail',
      label: t('trace.context.copyDetail'),
      disabled: !trace.detail,
    },
    {
      key: 'copy-link',
      label: t('common.copyLink'),
    },
  ]
}

async function handleTraceContextSelect(
  trace: { detail?: string | null },
  key: string,
) {
  if (key === 'copy-detail' && trace.detail) {
    await copyTextToClipboard(trace.detail)
    return
  }

  if (key === 'copy-link') {
    await copyTextToClipboard(buildRoutePermalink(route.fullPath))
  }
}

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
  <UiPageShell width="wide" test-id="trace-view" class="bg-transparent">
    <UiPageHeader
      :eyebrow="t('trace.header.eyebrow')"
      :title="runtime.activeSession?.summary.title ?? t('trace.header.titleFallback')"
      :description="runtime.activeRun ? runtime.activeRunCurrentStepLabel : t('trace.header.subtitleFallback')"
      class="px-6 py-8"
    />

    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 px-6 pb-8">
      <!-- Left Column: Trace Timeline (Main Content) -->
      <div class="lg:col-span-8 space-y-6">
        <UiPanelFrame
          variant="glass"
          padding="lg"
          :title="t('trace.timeline.title')"
          :subtitle="t('trace.timeline.subtitle')"
          class="min-h-[600px]"
        >
          <div class="space-y-6 relative">
            <!-- Vertical Timeline Line -->
            <div class="absolute left-0 top-2 bottom-2 w-px bg-gradient-to-b from-primary/30 via-primary/5 to-transparent ml-[19px] hidden sm:block" />

            <div
              v-for="trace in runtime.activeTrace"
              :key="trace.id"
              class="relative pl-0 sm:pl-10"
            >
              <!-- Timeline Bullet -->
              <div class="absolute left-0 top-3 size-2.5 rounded-full bg-primary border-2 border-surface shadow-[0_0_8px_var(--color-primary)] ml-[14.5px] hidden sm:block z-10" />

              <UiContextMenu
                :items="buildTraceContextMenuItems(trace)"
                @select="handleTraceContextSelect(trace, $event)"
              >
                <div data-testid="trace-runtime-item" :data-trace-id="trace.id">
                  <UiTraceBlock
                    :title="trace.title"
                    :detail="trace.detail"
                    :actor="trace.actor"
                    :timestamp-label="formatDateTime(trace.timestamp)"
                    :tone="resolveTraceTone(trace.tone)"
                    :meta-items="buildTraceMetaItems(trace)"
                    class="w-full"
                  />
                </div>
              </UiContextMenu>
            </div>

            <UiEmptyState
              v-if="!runtime.activeTrace.length"
              :title="t('trace.timeline.emptyTitle')"
              :description="t('trace.timeline.emptyDescription')"
              class="bg-black/5"
            />
          </div>
        </UiPanelFrame>
      </div>

      <!-- Right Column: Info & Actions (Sidebar) -->
      <div class="lg:col-span-4 space-y-6">
        <!-- Quick Stats -->
        <div class="grid grid-cols-1 gap-4">
          <UiStatTile 
            v-if="runtime.activeRun"
            :label="t('trace.stats.status')" 
            :value="traceStatusLabel" 
            tone="warning" 
          />
          <UiStatTile 
            v-if="runtime.activeRun"
            :label="t('trace.stats.owner')" 
            :value="resolvedActorLabel" 
            tone="info"
          />
        </div>

        <!-- Run State -->
        <UiPanelFrame
          variant="glass-strong"
          padding="md"
          :title="t('trace.runState.title')"
        >
          <div v-if="runtime.activeRun" class="space-y-4">
            <div class="flex flex-wrap gap-2">
              <UiBadge :label="runtime.activeRun.configuredModelName ?? runtime.activeRun.modelId ?? t('common.na')" class="bg-black/20" />
              <UiBadge :label="formatDateTime(runtime.activeRun.startedAt)" class="bg-black/20 text-[10px]" />
            </div>
            <p class="text-[13px] leading-relaxed text-text-secondary border-l-2 border-primary/30 pl-3 py-1">
              {{ runtime.activeRunCurrentStepLabel }}
            </p>
          </div>
          <UiEmptyState
            v-else
            :title="t('trace.runState.emptyTitle')"
            compact
          />
        </UiPanelFrame>

        <!-- Recovery / Actions -->
        <UiPanelFrame
          variant="glass"
          padding="md"
          :title="t('trace.recovery.title')"
          highlight
        >
          <div v-if="activeMediationKind" data-testid="trace-runtime-approval" class="space-y-4">
            <UiStatusCallout
              tone="warning"
              :title="activeMediationTitle"
              :description="activeMediationDetail"
              class="border-status-warning/30 bg-status-warning/5"
            >
              <div class="flex flex-wrap gap-1.5 mt-2">
                <UiBadge v-if="runtime.pendingApproval?.toolName" :label="runtime.pendingApproval.toolName" class="bg-status-warning/10" />
                <UiBadge v-if="runtime.pendingApproval?.riskLevel" :label="runtime.pendingApproval.riskLevel" tone="warning" />
              </div>
              <div class="flex flex-wrap gap-2 pt-3">
                <template v-if="activeMediationKind === 'approval' && runtime.pendingApproval && canResolveApprovalTrace">
                  <UiButton data-testid="trace-runtime-approve" size="sm" class="flex-1">{{ t('common.approve') }}</UiButton>
                  <UiButton data-testid="trace-runtime-reject" variant="ghost" size="sm" class="flex-1">{{ t('common.reject') }}</UiButton>
                </template>
                <template v-else-if="activeMediationKind === 'auth' && runtime.authTarget && canResolveAuthTrace">
                  <UiButton data-testid="trace-runtime-auth-resolve" size="sm" class="w-full">{{ t('common.resolveAuth') }}</UiButton>
                </template>
              </div>
            </UiStatusCallout>
          </div>
          <UiEmptyState
            v-else
            :title="t('trace.recovery.emptyTitle')"
            compact
          />
        </UiPanelFrame>
      </div>
    </div>
  </UiPageShell>
</template>
