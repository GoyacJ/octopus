<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import { ArrowRight, LayoutDashboard, MessageSquare } from 'lucide-vue-next'

import {
  UiActionCard,
  UiBadge,
  UiEmptyState,
  UiInfoCard,
  UiMetricCard,
  UiPageHero,
  UiPanelFrame,
  UiRankingList,
  UiSectionHeading,
  UiTimelineList,
} from '@octopus/ui'

import { createProjectConversationTarget, createProjectDashboardTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const snapshot = computed(() => workbench.workspaceOverview)
const currentProject = computed(() =>
  snapshot.value.projectId
    ? workbench.projects.find((project) => project.id === snapshot.value.projectId)
    : undefined,
)
const currentConversationId = computed(() => currentProject.value?.conversationIds[0])

function metricNumber(value: string | number) {
  if (typeof value === 'number') {
    return value
  }

  const normalized = String(value).replace(/,/g, '').replace(/[^0-9.]/g, '')
  return Number(normalized) || 0
}

function withRatio<T extends { value: string | number }>(items: T[], minRatio = 0.12) {
  const entries = items.map((item) => ({
    ...item,
    numeric: metricNumber(item.value),
  }))
  const max = Math.max(1, ...entries.map((item) => item.numeric))

  return entries.map((item) => ({
    ...item,
    ratio: Math.max(minRatio, item.numeric / max),
  }))
}

function toTimelineItems(items: Array<{ id: string, title: string, description: string, timestamp: number }>) {
  return items.map((item) => ({
    id: item.id,
    title: item.title,
    description: item.description,
    timestamp: new Date(item.timestamp).toLocaleString(),
  }))
}

function toRankingItems(items: Array<{ id: string, label: string, value: string | number, secondary?: string, ratio: number }>) {
  return items.map((item) => ({
    id: item.id,
    label: item.label,
    helper: item.secondary,
    value: item.value,
    ratio: item.ratio,
  }))
}

const userVisuals = computed(() => withRatio(snapshot.value.userMetrics, 0.18))
const projectVisuals = computed(() => withRatio(snapshot.value.projectSummary.metrics))
const workspaceVisuals = computed(() => withRatio(snapshot.value.workspaceMetrics))
const projectTokenVisuals = computed(() => withRatio(snapshot.value.projectTokenTop, 0.18))
const agentUsageVisuals = computed(() => withRatio(snapshot.value.agentUsage, 0.18))
const teamUsageVisuals = computed(() => withRatio(snapshot.value.teamUsage, 0.18))
const toolUsageVisuals = computed(() => withRatio(snapshot.value.toolUsage, 0.18))
const modelUsageVisuals = computed(() => withRatio(snapshot.value.modelUsage, 0.18))
const conversationTopVisuals = computed(() => withRatio(snapshot.value.projectSummary.conversationTokenTop, 0.18))

const userTimelineItems = computed(() => toTimelineItems(snapshot.value.userActivity))
const projectTimelineItems = computed(() => toTimelineItems(snapshot.value.projectSummary.activity))
const conversationTopItems = computed(() => toRankingItems(conversationTopVisuals.value))
const workspaceProjectRankingItems = computed(() => toRankingItems(projectTokenVisuals.value))
const workspaceAgentRankingItems = computed(() => toRankingItems(agentUsageVisuals.value))
const workspaceTeamRankingItems = computed(() => toRankingItems(teamUsageVisuals.value))
const workspaceToolRankingItems = computed(() => toRankingItems(toolUsageVisuals.value))
const workspaceModelRankingItems = computed(() => toRankingItems(modelUsageVisuals.value))
</script>

<template>
  <div class="w-full space-y-16 pb-20">
    <header class="space-y-2 px-2">
      <UiSectionHeading
        :eyebrow="t('overview.header.eyebrow')"
        :title="workbench.activeWorkspace?.name ?? t('overview.header.titleFallback')"
        :subtitle="workbench.activeWorkspace?.description ?? t('overview.header.subtitleFallback')"
      />
    </header>

    <!-- Active Project Spotlight -->
    <section v-if="currentProject" class="px-2 space-y-4" data-testid="workspace-overview-hero">
      <h3 class="text-lg font-bold text-text-primary">{{ t('overview.sections.project.title') }}</h3>
      
      <div class="bg-subtle/30 border border-border-subtle p-6 rounded-lg space-y-6">
        <div class="flex items-start justify-between">
          <div class="space-y-1">
            <h4 class="text-xl font-bold text-text-primary">{{ currentProject.name }}</h4>
            <p class="text-sm text-text-secondary">{{ currentProject.summary }}</p>
          </div>
          <UiBadge :label="currentProject.phase" subtle />
        </div>

        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <UiMetricCard
            v-for="metric in projectVisuals"
            :key="metric.label"
            :label="t(metric.label)"
            :value="metric.value"
            :progress="metric.ratio * 100"
            tone="accent"
          />
        </div>

        <div class="flex gap-3">
          <RouterLink class="min-w-[160px] block" :to="createProjectDashboardTarget(currentProject.workspaceId, currentProject.id)" data-testid="workspace-overview-action-dashboard">
            <UiActionCard :title="t('overview.sections.project.openDashboard')">
              <template #icon><LayoutDashboard :size="16" /></template>
            </UiActionCard>
          </RouterLink>
          <RouterLink class="min-w-[160px] block" :to="createProjectConversationTarget(currentProject.workspaceId, currentProject.id, currentConversationId)" data-testid="workspace-overview-action-conversation">
            <UiActionCard :title="t('overview.sections.project.openConversation')">
              <template #icon><MessageSquare :size="16" /></template>
            </UiActionCard>
          </RouterLink>
        </div>
      </div>
    </section>

    <!-- User Section -->
    <section class="px-2 space-y-6 border-t border-border-subtle pt-8">
      <h3 class="text-lg font-bold text-text-primary">{{ t('overview.sections.user.title') }}</h3>
      <div class="grid gap-6 md:grid-cols-3">
        <div class="space-y-3">
          <UiMetricCard
            v-for="metric in userVisuals"
            :key="metric.label"
            :label="t(metric.label)"
            :value="metric.value"
            :progress="metric.ratio * 100"
          />
        </div>
        <div class="md:col-span-2 bg-subtle/20 border border-border-subtle rounded-lg p-4 h-64 overflow-y-auto">
          <UiTimelineList v-if="userTimelineItems.length" :items="userTimelineItems" />
          <UiEmptyState v-else :title="t('overview.empty.activityTitle')" :description="t('overview.empty.activityDescription')" />
        </div>
      </div>
    </section>

    <!-- Workspace Overview Section -->
    <section class="px-2 space-y-6 border-t border-border-subtle pt-8">
      <h3 class="text-lg font-bold text-text-primary">{{ t('overview.sections.workspace.title') }}</h3>
      
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <UiMetricCard
          v-for="metric in workspaceVisuals"
          :key="metric.label"
          :label="t(metric.label)"
          :value="metric.value"
          :progress="metric.ratio * 100"
          tone="muted"
        />
      </div>

      <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3 pt-4">
        <div class="space-y-3">
          <h4 class="text-[12px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('overview.sections.workspace.projectRanking') }}</h4>
          <UiRankingList :items="workspaceProjectRankingItems" />
        </div>
        <div class="space-y-3">
          <h4 class="text-[12px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('overview.sections.workspace.agentRanking') }}</h4>
          <UiRankingList :items="workspaceAgentRankingItems" />
        </div>
        <div class="space-y-3">
          <h4 class="text-[12px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('overview.sections.workspace.toolRanking') }}</h4>
          <UiRankingList :items="workspaceToolRankingItems" />
        </div>
      </div>
    </section>
  </div>
</template>
