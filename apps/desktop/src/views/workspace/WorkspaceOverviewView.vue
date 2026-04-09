<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'

import {
  UiEmptyState,
  UiMetricCard,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiRecordCard,
  UiTimelineList,
} from '@octopus/ui'

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
  <UiPageShell width="standard" test-id="workspace-overview-view">
    <UiPageHeader
      :eyebrow="t('overview.header.eyebrow')"
      :title="snapshot?.workspace.name ?? t('overview.header.titleFallback')"
      :description="snapshot?.workspace.listenAddress ?? t('overview.header.subtitleFallback')"
    />

    <section class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <UiMetricCard
        v-for="metric in metrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="resolveMetricTone(metric.tone)"
      />
    </section>

    <section class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
      <UiPanelFrame
        variant="panel"
        padding="md"
        :title="t('overview.projects.title')"
        :subtitle="t('overview.projects.openDashboard')"
      >
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
        <UiEmptyState
          v-else
          :title="t('overview.empty.projectsTitle')"
          :description="t('overview.empty.projectsDescription')"
        />
      </UiPanelFrame>

      <UiPanelFrame
        variant="subtle"
        padding="md"
        :title="t('overview.activity.title')"
      >
        <UiTimelineList
          v-if="activities.length"
          :items="activities"
          density="compact"
        />
        <UiEmptyState
          v-else
          :title="t('overview.empty.activityTitle')"
          :description="t('overview.empty.activityDescription')"
        />
      </UiPanelFrame>
    </section>

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('overview.conversations.title')"
    >
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
      <UiEmptyState
        v-else
        :title="t('overview.empty.conversationsTitle')"
        :description="t('overview.empty.conversationsDescription')"
      />
    </UiPanelFrame>
  </UiPageShell>
</template>
