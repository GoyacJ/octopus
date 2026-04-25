<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import {
  UiAreaChart,
  UiBadge,
  UiDonutChart,
  UiEmptyState,
  UiMetricCard,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiRankingList,
  UiRecordCard,
  UiTimelineList,
} from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectSurfaceTarget } from '@/i18n/navigation'
import { useArtifactStore } from '@/stores/artifact'
import { useWorkspaceStore } from '@/stores/workspace'

type MetricTone = 'default' | 'accent' | 'muted' | 'success' | 'warning'

const EMPTY_SUMMARY = {
  memberCount: 0,
  activeUserCount: 0,
  agentCount: 0,
  teamCount: 0,
  conversationCount: 0,
  messageCount: 0,
  toolCallCount: 0,
  approvalCount: 0,
  resourceCount: 0,
  knowledgeCount: 0,
  toolCount: 0,
  tokenRecordCount: 0,
  totalTokens: 0,
  activityCount: 0,
}

const DONUT_COLORS = [
  'var(--color-primary)',
  'var(--color-status-success)',
  'var(--color-status-warning)',
  'var(--color-text-secondary)',
  'var(--color-text-tertiary)',
  'var(--color-border-strong)',
]

const { t, te, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const artifactStore = useArtifactStore()

async function loadDashboard() {
  const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId
  if (!projectId) {
    return
  }
  await Promise.all([
    workspaceStore.loadProjectDashboard(projectId),
    artifactStore.ensureProjectDeliverables(projectId),
  ])
}

watch(() => [route.params.projectId, workspaceStore.activeConnectionId], () => {
  void loadDashboard()
}, { immediate: true })

const snapshot = computed(() => workspaceStore.activeDashboard)
const project = computed(() => snapshot.value?.project ?? null)
const overview = computed(() => snapshot.value?.overview ?? EMPTY_SUMMARY)
const trend = computed(() => snapshot.value?.trend ?? [])
const userStats = computed(() => snapshot.value?.userStats ?? [])
const toolRanking = computed(() => snapshot.value?.toolRanking ?? [])
const resourceBreakdown = computed(() => snapshot.value?.resourceBreakdown ?? [])
const modelBreakdown = computed(() => snapshot.value?.modelBreakdown ?? [])
const conversationInsights = computed(() => snapshot.value?.conversationInsights ?? [])
const conversations = computed(() => snapshot.value?.recentConversations ?? [])
const recentActivity = computed(() => snapshot.value?.recentActivity ?? [])
const recentDeliverables = computed(() => artifactStore.activeProjectDeliverables.slice(0, 3))

const numberFormatter = computed(() => new Intl.NumberFormat(locale.value))
const compactFormatter = computed(() => new Intl.NumberFormat(locale.value, {
  notation: 'compact',
  maximumFractionDigits: 1,
}))

function formatNumber(value: number) {
  return numberFormatter.value.format(Math.max(0, Math.round(value)))
}

function formatCompact(value: number) {
  return compactFormatter.value.format(Math.max(0, Math.round(value)))
}

function formatShortDate(timestamp: number) {
  if (!timestamp) {
    return '--'
  }
  return new Intl.DateTimeFormat(locale.value, {
    month: 'short',
    day: 'numeric',
  }).format(timestamp)
}

function resolveMetricTone(value: number, preferred: MetricTone = 'default'): MetricTone {
  if (value === 0 && preferred === 'default') {
    return 'muted'
  }
  return preferred
}

function breakdownLabel(id: string, fallback: string) {
  const key = `projectDashboard.breakdown.items.${id}`
  return te(key) ? t(key) : fallback
}

function deliverablesTarget(deliverableId: string) {
  return {
    ...createProjectSurfaceTarget(
      'project-deliverables',
      typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workspaceStore.currentWorkspaceId,
      typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
    ),
    query: { deliverable: deliverableId },
  }
}

function deliverablesHref(deliverableId: string) {
  return router.resolve(deliverablesTarget(deliverableId)).href
}

const trendTokens = computed(() => trend.value.map(item => item.tokenCount))
const trendMessages = computed(() => trend.value.map(item => item.messageCount))
const trendLabels = computed(() => trend.value.map(item => formatShortDate(item.timestamp)))

const headlineMetrics = computed(() => [
  {
    id: 'tokens',
    label: t('projectDashboard.metrics.totalTokens'),
    value: formatNumber(overview.value.totalTokens),
    helper: t('projectDashboard.metrics.totalTokensHint', { records: formatNumber(overview.value.tokenRecordCount) }),
    tone: 'accent' as const,
    trend: trendTokens.value,
  },
  {
    id: 'active-users',
    label: t('projectDashboard.metrics.activeUsers'),
    value: formatNumber(overview.value.activeUserCount),
    helper: t('projectDashboard.metrics.activeUsersHint', { total: formatNumber(overview.value.memberCount) }),
    tone: resolveMetricTone(overview.value.activeUserCount, 'success'),
    trend: userStats.value.slice(0, 2).flatMap(item => item.activityTrend),
  },
  {
    id: 'agents-teams',
    label: t('projectDashboard.metrics.agentTeamCapacity'),
    value: formatNumber(overview.value.agentCount + overview.value.teamCount),
    helper: t('projectDashboard.metrics.agentTeamCapacityHint', {
      agents: formatNumber(overview.value.agentCount),
      teams: formatNumber(overview.value.teamCount),
    }),
    tone: resolveMetricTone(overview.value.agentCount + overview.value.teamCount),
    trend: trend.value.map(item => item.toolCallCount),
  },
  {
    id: 'sessions',
    label: t('projectDashboard.metrics.sessions'),
    value: formatNumber(overview.value.conversationCount),
    helper: t('projectDashboard.metrics.sessionsHint', { messages: formatNumber(overview.value.messageCount) }),
    tone: resolveMetricTone(overview.value.conversationCount),
    trend: trend.value.map(item => item.conversationCount),
  },
  {
    id: 'tool-calls',
    label: t('projectDashboard.metrics.toolCalls'),
    value: formatNumber(overview.value.toolCallCount),
    helper: t('projectDashboard.metrics.toolCallsHint', { tools: formatNumber(overview.value.toolCount) }),
    tone: resolveMetricTone(overview.value.toolCallCount),
    trend: trend.value.map(item => item.toolCallCount),
  },
  {
    id: 'approvals',
    label: t('projectDashboard.metrics.approvals'),
    value: formatNumber(overview.value.approvalCount),
    helper: t('projectDashboard.metrics.approvalsHint', { activity: formatNumber(overview.value.activityCount) }),
    tone: resolveMetricTone(overview.value.approvalCount, 'warning'),
    trend: trend.value.map(item => item.approvalCount),
  },
  {
    id: 'resources',
    label: t('projectDashboard.metrics.resources'),
    value: formatNumber(overview.value.resourceCount),
    helper: t('projectDashboard.metrics.resourcesHint', { knowledge: formatNumber(overview.value.knowledgeCount) }),
    tone: resolveMetricTone(overview.value.resourceCount),
    trend: trend.value.map(item => item.conversationCount + item.messageCount),
  },
  {
    id: 'messages',
    label: t('projectDashboard.metrics.messages'),
    value: formatNumber(overview.value.messageCount),
    helper: t('projectDashboard.metrics.messagesHint', { average: formatCompact(overview.value.messageCount / Math.max(overview.value.conversationCount, 1)) }),
    tone: resolveMetricTone(overview.value.messageCount),
    trend: trendMessages.value,
  },
])

const contributorRanking = computed(() => {
  const max = Math.max(...userStats.value.map(item => item.tokenCount), 1)
  return userStats.value.slice(0, 6).map(item => ({
    id: item.userId,
    label: item.displayName,
    value: formatNumber(item.tokenCount),
    helper: t('projectDashboard.userSummary.helper', {
      activity: formatNumber(item.activityCount),
      tools: formatNumber(item.toolCallCount),
    }),
    ratio: item.tokenCount / max,
  }))
})

const topUserCards = computed(() =>
  userStats.value.slice(0, 4).map(item => ({
    ...item,
    helper: t('projectDashboard.userSummary.cardHelper', {
      activity: formatNumber(item.activityCount),
      messages: formatNumber(item.messageCount),
    }),
  })),
)

const toolRankingItems = computed(() => {
  const max = Math.max(...toolRanking.value.map(item => item.value), 1)
  return toolRanking.value.slice(0, 6).map(item => ({
    id: item.id,
    label: item.label,
    value: formatNumber(item.value),
    helper: item.helper,
    ratio: item.value / max,
  }))
})

const conversationRankingItems = computed(() => {
  const max = Math.max(...conversationInsights.value.map(item => item.tokenCount), 1)
  return conversationInsights.value.slice(0, 6).map(item => ({
    id: item.id,
    label: item.title,
    value: formatNumber(item.tokenCount),
    helper: t('projectDashboard.sessions.rankingHelper', {
      messages: formatNumber(item.messageCount),
      tools: formatNumber(item.toolCallCount),
    }),
    ratio: item.tokenCount / max,
  }))
})

const resourceChartItems = computed(() =>
  resourceBreakdown.value.map((item, index) => ({
    ...item,
    label: breakdownLabel(item.id, item.label),
    color: DONUT_COLORS[index % DONUT_COLORS.length],
  })),
)

const modelChartItems = computed(() =>
  modelBreakdown.value.map((item, index) => ({
    ...item,
    color: DONUT_COLORS[index % DONUT_COLORS.length],
  })),
)

const activityItems = computed(() =>
  recentActivity.value.map(item => ({
    id: item.id,
    title: item.title,
    description: item.description,
    helper: item.actorId
      ? t('projectDashboard.activity.actorHelper', {
          actor: item.actorId,
          outcome: item.outcome ?? t('projectDashboard.common.unknown'),
        })
      : t('projectDashboard.common.system'),
    timestamp: formatDateTime(item.timestamp),
  })),
)
</script>

<template>
  <UiPageShell width="wide" test-id="project-dashboard-view" class="bg-transparent">
    <UiPageHeader
      :eyebrow="t('projectDashboard.header.eyebrow')"
      :title="project?.name ?? t('projectDashboard.header.titleFallback')"
      :description="project?.description ?? t('projectDashboard.header.subtitleFallback')"
      class="px-6 py-8"
    >
      <template #meta>
        <UiBadge
          :label="t('projectDashboard.header.meta.members', { count: formatNumber(overview.memberCount) })"
          class="bg-primary/10 text-primary border-primary/20"
        />
        <UiBadge
          tone="warning"
          :label="t('projectDashboard.header.meta.health', { approvals: formatNumber(overview.approvalCount) })"
          class="bg-status-warning/10 text-status-warning border-status-warning/20"
        />
      </template>
    </UiPageHeader>

    <template v-if="snapshot">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 px-8 pb-12">
        <!-- Headline Metrics (4 Small Cards) -->
        <UiMetricCard
          v-for="metric in headlineMetrics.slice(0, 4)"
          :key="metric.id"
          :label="metric.label"
          :value="metric.value"
          :helper="metric.helper"
          :tone="metric.tone"
          :trend="metric.trend"
        />

        <!-- Usage Trend (Large 2x2 Area) -->
        <UiPanelFrame
          variant="glass-strong"
          padding="lg"
          :title="t('projectDashboard.sections.usageTrend.title')"
          :subtitle="t('projectDashboard.sections.usageTrend.subtitle')"
          class="md:col-span-2 lg:col-span-2 lg:row-span-2"
          inner-class="h-full flex flex-col"
        >
          <div class="flex-1 flex flex-col gap-6">
            <div class="flex-1 min-h-[280px] rounded-[var(--radius-xl)] bg-black/5 p-6 shadow-inner ring-1 ring-white/5">
              <UiAreaChart :data="trendTokens" :labels="trendLabels" stroke-color="var(--color-primary)" />
            </div>
            <div class="grid grid-cols-2 gap-8 px-2">
               <div class="space-y-1">
                 <div class="text-[11px] font-bold uppercase tracking-[0.1em] text-text-tertiary">{{ t('projectDashboard.sections.usageTrend.cards.tokens') }}</div>
                 <div class="text-3xl font-bold text-text-primary tracking-tight">{{ formatNumber(overview.totalTokens) }}</div>
               </div>
               <div class="space-y-1">
                 <div class="text-[11px] font-bold uppercase tracking-[0.1em] text-text-tertiary">{{ t('projectDashboard.sections.usageTrend.cards.messages') }}</div>
                 <div class="text-3xl font-bold text-text-primary tracking-tight">{{ formatNumber(overview.messageCount) }}</div>
               </div>
            </div>
          </div>
        </UiPanelFrame>

        <!-- Resource Mix (Tall Card) -->
        <UiPanelFrame
          variant="panel"
          padding="lg"
          :title="t('projectDashboard.sections.resourceMix.title')"
          class="lg:row-span-2"
          inner-class="h-full"
        >
          <div class="h-full flex flex-col">
            <div class="flex-1 flex items-center justify-center py-8">
              <UiDonutChart
                :items="resourceChartItems"
                :size="160"
                :stroke-width="16"
                :total-label="t('projectDashboard.sections.resourceMix.total')"
              />
            </div>
            <div class="space-y-3 mt-6">
              <div
                v-for="item in resourceChartItems"
                :key="item.id"
                class="flex items-center justify-between text-xs font-medium"
              >
                <div class="flex items-center gap-3">
                  <span class="size-2.5 rounded-full" :style="{ backgroundColor: item.color }" />
                  <span class="text-text-secondary">{{ item.label }}</span>
                </div>
                <span class="text-text-primary tabular-nums">{{ formatNumber(item.value) }}</span>
              </div>
            </div>
          </div>
        </UiPanelFrame>

        <!-- Tool Calls & Approvals (Stacking) -->
        <UiMetricCard
           :label="t('projectDashboard.metrics.toolCalls')"
           :value="formatNumber(overview.toolCallCount)"
           :helper="t('projectDashboard.metrics.toolCallsHint', { tools: formatNumber(overview.toolCount) })"
           tone="default"
           :trend="trend.map(item => item.toolCallCount)"
        />
        <UiMetricCard
           :label="t('projectDashboard.metrics.approvals')"
           :value="formatNumber(overview.approvalCount)"
           :helper="t('projectDashboard.metrics.approvalsHint', { activity: formatNumber(overview.activityCount) })"
           tone="warning"
           :trend="trend.map(item => item.approvalCount)"
        />

        <!-- Top Contributors (2x1) -->
        <UiPanelFrame
          variant="panel"
          padding="lg"
          :title="t('projectDashboard.sections.topContributors.title')"
          class="md:col-span-2"
        >
           <UiRankingList v-if="contributorRanking.length" :items="contributorRanking.slice(0, 5)" class="gap-4" />
        </UiPanelFrame>

        <!-- Recent Activity (Vertical List) -->
        <UiPanelFrame
          variant="subtle"
          padding="lg"
          :title="t('projectDashboard.sections.activity.title')"
          class="md:col-span-2 lg:col-span-2 lg:row-span-2"
        >
          <UiTimelineList v-if="activityItems.length" :items="activityItems" density="compact" />
        </UiPanelFrame>

        <!-- Tool Usage & Session Ranking -->
        <UiPanelFrame
          variant="panel"
          padding="lg"
          :title="t('projectDashboard.sections.toolUsage.title')"
        >
          <UiRankingList v-if="toolRankingItems.length" :items="toolRankingItems.slice(0, 4)" class="gap-4" />
        </UiPanelFrame>
        
        <UiPanelFrame
          variant="panel"
          padding="lg"
          :title="t('projectDashboard.sections.sessionRanking.title')"
        >
          <UiRankingList v-if="conversationRankingItems.length" :items="conversationRankingItems.slice(0, 4)" class="gap-4" />
        </UiPanelFrame>
      </div>
    </template>

    <UiEmptyState
      v-else
      :title="t('projectDashboard.empty.snapshotTitle')"
      :description="t('projectDashboard.empty.snapshotDescription')"
    />
  </UiPageShell>
</template>
