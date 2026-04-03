<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import { Activity, ArrowRight, Library, MessageSquare } from 'lucide-vue-next'

import {
  UiActionCard,
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInfoCard,
  UiInput,
  UiMetricCard,
  UiPageHero,
  UiPanelFrame,
  UiRankingList,
  UiSectionHeading,
  UiTimelineList,
  UiTextarea,
} from '@octopus/ui'

import { enumLabel, resolveCopy } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const snapshot = computed(() => workbench.projectDashboard)
const project = computed(() => snapshot.value.project)
const editing = ref(false)
const draft = reactive({
  name: '',
  goal: '',
  phase: '',
  summary: '',
})

const conversationTarget = computed(() =>
  createProjectConversationTarget(
    workbench.currentWorkspaceId,
    workbench.currentProjectId,
    workbench.currentConversationId,
  ),
)

watch(
  project,
  (value) => {
    if (!value || editing.value) {
      return
    }

    draft.name = value.name
    draft.goal = value.goal
    draft.phase = value.phase
    draft.summary = value.summary
  },
  { immediate: true },
)

function startEdit() {
  draft.name = project.value.name
  draft.goal = project.value.goal
  draft.phase = project.value.phase
  draft.summary = project.value.summary
  editing.value = true
}

function cancelEdit() {
  editing.value = false
  draft.name = project.value.name
  draft.goal = project.value.goal
  draft.phase = project.value.phase
  draft.summary = project.value.summary
}

function saveProject() {
  workbench.updateProjectDetails(project.value.id, draft)
  editing.value = false
}

const progressTone = computed(() => {
  if (snapshot.value.progress.blockerCount > 0) {
    return 'warning'
  }
  if (snapshot.value.progress.runStatus === 'completed') {
    return 'success'
  }
  return 'info'
})

function metricNumber(value: string | number) {
  if (typeof value === 'number') {
    return value
  }

  const normalized = String(value).replace(/,/g, '').replace(/[^0-9.]/g, '')
  return Number(normalized) || 0
}

const progressPercent = computed(() => Math.min(100, Math.max(0, Number(snapshot.value.progress.progress) || 0)))

const resourceVisuals = computed(() => {
  const metrics = snapshot.value.resourceMetrics.map((metric) => ({
    ...metric,
    numeric: metricNumber(metric.value),
  }))
  const max = Math.max(1, ...metrics.map((metric) => metric.numeric))

  return metrics.map((metric) => ({
    ...metric,
    ratio: Math.max(0.12, metric.numeric / max),
  }))
})

const dataVisuals = computed(() => {
  const metrics = snapshot.value.dataMetrics.map((metric) => ({
    ...metric,
    numeric: metricNumber(metric.value),
  }))
  const max = Math.max(1, ...metrics.map((metric) => metric.numeric))

  return metrics.map((metric) => ({
    ...metric,
    ratio: Math.max(0.12, metric.numeric / max),
  }))
})

const rankingVisuals = computed(() => {
  const metrics = snapshot.value.conversationTokenTop.map((item) => ({
    ...item,
    numeric: metricNumber(item.value),
  }))
  const max = Math.max(1, ...metrics.map((metric) => metric.numeric))

  return metrics.map((metric) => ({
    ...metric,
    ratio: Math.max(0.18, metric.numeric / max),
  }))
})

const tokenValue = computed(() => snapshot.value.dataMetrics.find((metric) => metric.label === 'projectDashboard.data.tokens')?.value ?? t('common.na'))
const activityTimelineItems = computed(() =>
  snapshot.value.activity.map((activity) => ({
    id: activity.id,
    title: activity.title,
    description: activity.description,
    timestamp: new Date(activity.timestamp).toLocaleString(),
  })),
)
const rankingItems = computed(() =>
  rankingVisuals.value.map((item) => ({
    id: item.id,
    label: item.label,
    helper: item.secondary,
    value: item.value,
    ratio: item.ratio,
  })),
)
</script>

<template>
  <div class="w-full space-y-12 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading
        :eyebrow="t('projectDashboard.header.eyebrow')"
        :title="project.name"
        :subtitle="project.goal"
      />
    </header>

    <UiPageHero data-testid="project-dashboard-hero" class="px-2">
      <template #meta>
        <UiBadge :label="enumLabel('projectStatus', project.status)" :tone="project.status === 'active' ? 'success' : 'default'" />
        <UiBadge :label="project.phase" subtle />
        <UiBadge
          :label="snapshot.progress.runStatus ? enumLabel('runStatus', snapshot.progress.runStatus) : t('common.na')"
          subtle
        />
      </template>

      <p class="text-[15px] leading-relaxed text-text-secondary max-w-4xl">
        {{ project.summary }}
      </p>

      <template #actions>
        <RouterLink :to="conversationTarget" class="block min-w-0 no-underline">
          <UiActionCard
            :title="t('projectDashboard.actions.openConversation')"
            :description="t('projectDashboard.actions.openConversationHint')"
            class="h-full"
          >
            <template #icon><MessageSquare :size="16" /></template>
          </UiActionCard>
        </RouterLink>

        <RouterLink
          :to="{ name: 'knowledge', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          class="block min-w-0 no-underline"
        >
          <UiActionCard
            :title="t('projectDashboard.actions.openKnowledge')"
            :description="t('projectDashboard.actions.openKnowledgeHint')"
            class="h-full"
          >
            <template #icon><Library :size="16" /></template>
          </UiActionCard>
        </RouterLink>

        <RouterLink
          :to="{ name: 'trace', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          class="block min-w-0 no-underline"
        >
          <UiActionCard
            :title="t('projectDashboard.actions.openTrace')"
            :description="t('projectDashboard.actions.openTraceHint')"
            class="h-full"
          >
            <template #icon><Activity :size="16" /></template>
          </UiActionCard>
        </RouterLink>
      </template>

      <template #aside>
        <UiMetricCard
          :label="t('projectDashboard.progress.progress')"
          :value="`${progressPercent}%`"
          :helper="snapshot.progress.runStatus ? enumLabel('runStatus', snapshot.progress.runStatus) : t('common.na')"
          :progress="progressPercent"
          tone="accent"
        />
      </template>
    </UiPageHero>

    <!-- Sub Sections (Automatically expanded grid) -->
    <div class="grid gap-10 lg:grid-cols-2 px-2">
      <!-- Progress -->
      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.progress.title') }}</h3>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-2 2xl:grid-cols-4">
          <UiMetricCard :label="t('projectDashboard.progress.phase')" :value="snapshot.progress.phase" />
          <UiMetricCard :label="t('projectDashboard.progress.blockers')" :value="snapshot.progress.blockerCount" tone="warning" />
          <UiMetricCard :label="t('projectDashboard.progress.pendingInbox')" :value="snapshot.progress.pendingInboxCount" />
          <UiMetricCard
            :label="t('projectDashboard.progress.currentStep')"
            :value="snapshot.progress.currentStep ? resolveCopy(snapshot.progress.currentStep) : t('common.na')"
            tone="accent"
          />
        </div>
      </section>

      <!-- Resources -->
      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.resources.title') }}</h3>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-2 2xl:grid-cols-4">
          <UiMetricCard
            v-for="metric in resourceVisuals"
            :key="metric.label"
            :label="t(metric.label)"
            :value="metric.value"
            :progress="metric.ratio * 100"
          />
        </div>
      </section>
    </div>

    <!-- Data Overview -->
    <section class="space-y-4 px-2 pt-8 border-t border-border-subtle">
      <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.data.title') }}</h3>
      <div class="grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 2xl:grid-cols-8">
        <UiMetricCard
          v-for="metric in dataVisuals"
          :key="metric.label"
          :label="t(metric.label)"
          :value="metric.value"
          :progress="metric.ratio * 100"
          tone="muted"
        />
      </div>
    </section>

    <div class="grid gap-12 lg:grid-cols-2 px-2 pt-8 border-t border-border-subtle">
      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.activity.title') }}</h3>
        <div class="bg-subtle/20 rounded-lg p-5 border border-border-subtle h-[400px] overflow-y-auto">
          <UiTimelineList v-if="activityTimelineItems.length" :items="activityTimelineItems" />
          <UiEmptyState v-else :title="t('projectDashboard.empty.activityTitle')" :description="t('projectDashboard.empty.activityDescription')" />
        </div>
      </section>

      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.data.conversationTop') }}</h3>
        <div class="bg-subtle/20 rounded-lg p-5 border border-border-subtle h-[400px] overflow-y-auto">
          <UiRankingList v-if="rankingItems.length" :items="rankingItems" />
          <UiEmptyState v-else :title="t('projectDashboard.empty.rankingTitle')" :description="t('projectDashboard.empty.rankingDescription')" />
        </div>
      </section>
    </div>

    <!-- Project Settings -->
    <section class="space-y-8 px-2 pt-8 border-t border-border-subtle">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-bold text-text-primary">{{ t('projectDashboard.info.title') }}</h3>
        <div class="flex gap-2">
          <UiButton v-if="!editing" variant="ghost" size="sm" @click="startEdit">{{ t('common.edit') }}</UiButton>
          <template v-else>
            <UiButton variant="ghost" size="sm" @click="cancelEdit">{{ t('common.cancel') }}</UiButton>
            <UiButton variant="primary" size="sm" @click="saveProject">{{ t('common.save') }}</UiButton>
          </template>
        </div>
      </div>

      <div v-if="editing" class="grid gap-8 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 max-w-full">
        <UiField :label="t('projectDashboard.fields.name')"><UiInput v-model="draft.name" /></UiField>
        <UiField :label="t('projectDashboard.fields.phase')"><UiInput v-model="draft.phase" /></UiField>
        <UiField :label="t('projectDashboard.fields.goal')" class="md:col-span-2 lg:col-span-1"><UiTextarea v-model="draft.goal" :rows="2" /></UiField>
        <UiField :label="t('projectDashboard.fields.summary')" class="md:col-span-2 lg:col-span-2"><UiTextarea v-model="draft.summary" :rows="3" /></UiField>
      </div>

      <div v-else class="grid gap-y-8 gap-x-16 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 max-w-full text-[13px]">
        <div>
          <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('projectDashboard.fields.name') }}</strong>
          <span class="text-text-primary font-semibold text-base">{{ project.name }}</span>
        </div>
        <div>
          <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('projectDashboard.fields.status') }}</strong>
          <UiBadge :label="enumLabel('projectStatus', project.status)" :tone="project.status === 'active' ? 'success' : 'default'" />
        </div>
        <div class="lg:col-span-2">
          <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('projectDashboard.fields.goal') }}</strong>
          <span class="text-text-primary text-sm leading-relaxed">{{ project.goal }}</span>
        </div>
        <div class="md:col-span-2 lg:col-span-4">
          <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('projectDashboard.fields.summary') }}</strong>
          <span class="text-text-secondary text-sm leading-relaxed max-w-5xl block">{{ project.summary }}</span>
        </div>
      </div>
    </section>
  </div>
</template>
