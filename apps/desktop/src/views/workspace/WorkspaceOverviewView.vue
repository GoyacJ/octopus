<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'

import { UiEmptyState, UiMetricCard, UiRecordCard, UiSectionHeading, UiTimelineList } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectDashboardTarget } from '@/i18n/navigation'
import { useWorkspaceStore } from '@/stores/workspace'

type MetricTone = 'default' | 'accent' | 'muted' | 'success' | 'warning'

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()

async function loadOverview() {
  await workspaceStore.bootstrap()
  const projectId = typeof route.query.project === 'string' ? route.query.project : workspaceStore.currentProjectId
  if (projectId) {
    await workspaceStore.loadProjectDashboard(projectId)
  }
}

onMounted(() => {
  void loadOverview()
})

watch(() => route.query.project, () => {
  void loadOverview()
})

const snapshot = computed(() => workspaceStore.activeOverview)
const metrics = computed(() => snapshot.value?.metrics ?? [])
const projects = computed(() => snapshot.value?.projects ?? [])
const conversations = computed(() => snapshot.value?.recentConversations ?? [])
const activities = computed(() =>
  (snapshot.value?.recentActivity ?? []).map(item => ({
    id: item.id,
    title: item.title,
    description: item.description,
    timestamp: formatDateTime(item.timestamp),
  })),
)

function resolveMetricTone(tone?: string): MetricTone {
  switch (tone) {
    case 'success':
    case 'warning':
    case 'accent':
      return tone
    case 'error':
      return 'warning'
    case 'info':
      return 'muted'
    default:
      return 'default'
  }
}
</script>

<template>
  <div class="flex w-full flex-col gap-8 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('overview.header.eyebrow')"
        :title="snapshot?.workspace.name ?? t('overview.header.titleFallback')"
        :subtitle="snapshot?.workspace.listenAddress ?? t('overview.header.subtitleFallback')"
      />
    </header>

    <section class="grid gap-3 px-2 sm:grid-cols-2 xl:grid-cols-4">
      <UiMetricCard
        v-for="metric in metrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="resolveMetricTone(metric.tone)"
      />
    </section>

    <div class="grid gap-8 px-2 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section class="space-y-4">
        <h3 class="text-lg font-semibold text-text-primary">{{ t('overview.projects.title') }}</h3>
        <div v-if="projects.length" class="grid gap-3 lg:grid-cols-2">
          <UiRecordCard
            v-for="project in projects"
            :key="project.id"
            :title="project.name"
            :description="project.description"
          >
            <template #meta>
              <RouterLink
                class="text-sm font-medium text-primary hover:underline"
                :to="createProjectDashboardTarget(project.workspaceId, project.id)"
              >
                {{ t('overview.projects.openDashboard') }}
              </RouterLink>
            </template>
          </UiRecordCard>
        </div>
        <UiEmptyState v-else :title="t('overview.empty.projectsTitle')" :description="t('overview.empty.projectsDescription')" />
      </section>

      <section class="space-y-4">
        <h3 class="text-lg font-semibold text-text-primary">{{ t('overview.activity.title') }}</h3>
        <UiTimelineList
          v-if="activities.length"
          :items="activities"
        />
        <UiEmptyState v-else :title="t('overview.empty.activityTitle')" :description="t('overview.empty.activityDescription')" />
      </section>
    </div>

    <section class="space-y-4 px-2">
      <h3 class="text-lg font-semibold text-text-primary">{{ t('overview.conversations.title') }}</h3>
      <div v-if="conversations.length" class="grid gap-3 lg:grid-cols-2">
        <UiRecordCard
          v-for="conversation in conversations"
          :key="conversation.id"
          :title="conversation.title"
          :description="conversation.lastMessagePreview ?? conversation.status"
        >
          <template #meta>
            <RouterLink
              class="text-sm font-medium text-primary hover:underline"
              :to="createProjectConversationTarget(conversation.workspaceId, conversation.projectId, conversation.id)"
            >
              {{ formatDateTime(conversation.updatedAt) }}
            </RouterLink>
          </template>
        </UiRecordCard>
      </div>
      <UiEmptyState v-else :title="t('overview.empty.conversationsTitle')" :description="t('overview.empty.conversationsDescription')" />
    </section>
  </div>
</template>
