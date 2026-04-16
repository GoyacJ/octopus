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
    artifactStore.loadProjectDeliverables(projectId),
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
  <UiPageShell width="wide" test-id="project-dashboard-view">
    <UiPageHeader
      :eyebrow="t('projectDashboard.header.eyebrow')"
      :title="project?.name ?? t('projectDashboard.header.titleFallback')"
      :description="project?.description ?? t('projectDashboard.header.subtitleFallback')"
    >
      <template #meta>
        <UiBadge
          :label="t('projectDashboard.header.meta.members', { count: formatNumber(overview.memberCount) })"
        />
        <UiBadge
          tone="warning"
          :label="t('projectDashboard.header.meta.health', { approvals: formatNumber(overview.approvalCount) })"
        />
      </template>
    </UiPageHeader>

    <template v-if="snapshot">
      <section class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <UiMetricCard
          v-for="metric in headlineMetrics"
          :key="metric.id"
          :label="metric.label"
          :value="metric.value"
          :helper="metric.helper"
          :tone="metric.tone"
          :trend="metric.trend"
        />
      </section>

      <section class="grid gap-4 xl:grid-cols-[minmax(0,1.7fr)_minmax(320px,0.9fr)]">
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('projectDashboard.sections.usageTrend.title')"
          :subtitle="t('projectDashboard.sections.usageTrend.subtitle')"
        >
          <div class="grid gap-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-4">
              <UiAreaChart :data="trendTokens" :labels="trendLabels" stroke-color="var(--color-primary)" />
            </div>
            <div class="grid gap-3 sm:grid-cols-3">
              <UiMetricCard
                :label="t('projectDashboard.sections.usageTrend.cards.tokens')"
                :value="formatNumber(overview.totalTokens)"
                :helper="t('projectDashboard.sections.usageTrend.cards.tokensHint')"
                tone="accent"
                :trend="trendTokens"
              />
              <UiMetricCard
                :label="t('projectDashboard.sections.usageTrend.cards.messages')"
                :value="formatNumber(overview.messageCount)"
                :helper="t('projectDashboard.sections.usageTrend.cards.messagesHint')"
                :trend="trendMessages"
              />
              <UiMetricCard
                :label="t('projectDashboard.sections.usageTrend.cards.tools')"
                :value="formatNumber(overview.toolCallCount)"
                :helper="t('projectDashboard.sections.usageTrend.cards.toolsHint')"
                tone="success"
                :trend="trend.map(item => item.toolCallCount)"
              />
            </div>
          </div>
        </UiPanelFrame>

        <UiPanelFrame
          variant="subtle"
          padding="md"
          :title="t('projectDashboard.sections.resourceMix.title')"
          :subtitle="t('projectDashboard.sections.resourceMix.subtitle')"
        >
          <div class="grid gap-4">
            <div class="flex justify-center">
              <UiDonutChart
                :items="resourceChartItems"
                :size="168"
                :stroke-width="16"
                :total-label="t('projectDashboard.sections.resourceMix.total')"
              />
            </div>
            <div class="grid gap-2">
              <div
                v-for="item in resourceChartItems"
                :key="item.id"
                class="flex items-center justify-between rounded-[var(--radius-m)] border border-border bg-surface px-3 py-2"
              >
                <div class="flex min-w-0 items-center gap-2">
                  <span class="size-2.5 shrink-0 rounded-full" :style="{ backgroundColor: item.color }" />
                  <span class="truncate text-sm text-text-secondary">{{ item.label }}</span>
                </div>
                <span class="text-sm font-semibold text-text-primary">{{ formatNumber(item.value) }}</span>
              </div>
            </div>
          </div>
        </UiPanelFrame>
      </section>

      <section class="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('projectDashboard.sections.topContributors.title')"
          :subtitle="t('projectDashboard.sections.topContributors.subtitle')"
        >
          <UiRankingList v-if="contributorRanking.length" :items="contributorRanking" />
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.usersTitle')"
            :description="t('projectDashboard.empty.usersDescription')"
          />
        </UiPanelFrame>

        <UiPanelFrame
          variant="subtle"
          padding="md"
          :title="t('projectDashboard.sections.approvalQueue.title')"
          :subtitle="t('projectDashboard.sections.approvalQueue.subtitle')"
        >
          <div class="grid gap-3">
            <UiMetricCard
              :label="t('projectDashboard.sections.approvalQueue.cards.approvals')"
              :value="formatNumber(overview.approvalCount)"
              :helper="t('projectDashboard.sections.approvalQueue.cards.approvalsHint')"
              tone="warning"
              :trend="trend.map(item => item.approvalCount)"
            />
            <UiMetricCard
              :label="t('projectDashboard.sections.approvalQueue.cards.activity')"
              :value="formatNumber(overview.activityCount)"
              :helper="t('projectDashboard.sections.approvalQueue.cards.activityHint')"
              :trend="trend.map(item => item.messageCount)"
            />
          </div>
        </UiPanelFrame>
      </section>

      <section class="grid gap-3 lg:grid-cols-2 xl:grid-cols-4">
        <UiMetricCard
          v-for="user in topUserCards"
          :key="user.userId"
          :label="user.displayName"
          :value="formatNumber(user.tokenCount)"
          :helper="user.helper"
          tone="accent"
          :trend="user.tokenTrend"
        />
      </section>

      <section class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('projectDashboard.sections.toolUsage.title')"
          :subtitle="t('projectDashboard.sections.toolUsage.subtitle')"
        >
          <UiRankingList v-if="toolRankingItems.length" :items="toolRankingItems" />
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.toolsTitle')"
            :description="t('projectDashboard.empty.toolsDescription')"
          />
        </UiPanelFrame>

        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('projectDashboard.sections.sessionRanking.title')"
          :subtitle="t('projectDashboard.sections.sessionRanking.subtitle')"
        >
          <UiRankingList v-if="conversationRankingItems.length" :items="conversationRankingItems" />
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.sessionsTitle')"
            :description="t('projectDashboard.empty.sessionsDescription')"
          />
        </UiPanelFrame>
      </section>

      <section class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('projectDashboard.sections.recentSessions.title')"
          :subtitle="t('projectDashboard.sections.recentSessions.subtitle')"
        >
          <div v-if="conversations.length" class="grid gap-3">
            <UiRecordCard
              v-for="conversation in conversations"
              :key="conversation.id"
              :title="conversation.title"
              :description="conversation.lastMessagePreview ?? conversation.status"
              layout="tile"
            >
              <template #meta>
                <RouterLink
                  class="text-sm font-medium text-primary hover:underline"
                  :to="createProjectConversationTarget(conversation.workspaceId, conversation.projectId, conversation.id)"
                >
                  {{ formatDateTime(conversation.updatedAt) }}
                </RouterLink>
                <span class="text-xs text-text-tertiary">
                  {{
                    t('projectDashboard.sections.recentSessions.meta', {
                      status: conversation.status,
                    })
                  }}
                </span>
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.conversationsTitle')"
            :description="t('projectDashboard.empty.conversationsDescription')"
          />
        </UiPanelFrame>

        <div class="grid gap-4">
          <UiPanelFrame
            variant="subtle"
            padding="md"
            :title="t('projectDashboard.sections.deliverables.title')"
            :subtitle="t('projectDashboard.sections.deliverables.subtitle')"
          >
            <div v-if="recentDeliverables.length" class="grid gap-3">
              <UiRecordCard
                v-for="deliverable in recentDeliverables"
                :key="deliverable.id"
                :title="deliverable.title"
                :description="t('projectDashboard.sections.deliverables.meta', {
                  version: deliverable.latestVersion,
                  state: deliverable.promotionState,
                })"
              >
                <template #meta>
                  <a
                    data-testid="project-dashboard-open-deliverables"
                    class="text-sm font-medium text-primary hover:underline"
                    :href="deliverablesHref(deliverable.id)"
                  >
                    {{ t('projectDashboard.sections.deliverables.open') }}
                  </a>
                  <span class="text-xs text-text-tertiary">{{ formatDateTime(deliverable.updatedAt) }}</span>
                </template>
              </UiRecordCard>
            </div>
            <UiEmptyState
              v-else
              :title="t('projectDashboard.empty.deliverablesTitle')"
              :description="t('projectDashboard.empty.deliverablesDescription')"
            />
          </UiPanelFrame>

          <UiPanelFrame
            variant="subtle"
            padding="md"
            :title="t('projectDashboard.sections.modelMix.title')"
            :subtitle="t('projectDashboard.sections.modelMix.subtitle')"
          >
            <div class="grid gap-4">
              <div class="flex justify-center">
                <UiDonutChart
                  :items="modelChartItems"
                  :size="152"
                  :stroke-width="16"
                  :total-label="t('projectDashboard.sections.modelMix.total')"
                />
              </div>
              <div class="grid gap-2">
                <div
                  v-for="item in modelChartItems"
                  :key="item.id"
                  class="flex items-center justify-between rounded-[var(--radius-m)] border border-border bg-surface px-3 py-2"
                >
                  <div class="flex min-w-0 items-center gap-2">
                    <span class="size-2.5 shrink-0 rounded-full" :style="{ backgroundColor: item.color }" />
                    <span class="truncate text-sm text-text-secondary">{{ item.label }}</span>
                  </div>
                  <span class="text-sm font-semibold text-text-primary">{{ formatNumber(item.value) }}</span>
                </div>
              </div>
            </div>
          </UiPanelFrame>

          <UiPanelFrame
            variant="subtle"
            padding="md"
            :title="t('projectDashboard.sections.activity.title')"
            :subtitle="t('projectDashboard.sections.activity.subtitle')"
          >
            <UiTimelineList v-if="activityItems.length" :items="activityItems" density="compact" />
            <UiEmptyState
              v-else
              :title="t('projectDashboard.empty.activityTitle')"
              :description="t('projectDashboard.empty.activityDescription')"
            />
          </UiPanelFrame>
        </div>
      </section>
    </template>

    <UiEmptyState
      v-else
      :title="t('projectDashboard.empty.snapshotTitle')"
      :description="t('projectDashboard.empty.snapshotDescription')"
    />
  </UiPageShell>
</template>
