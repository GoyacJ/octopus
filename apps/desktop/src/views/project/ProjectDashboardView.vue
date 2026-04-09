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
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkspaceStore } from '@/stores/workspace'

type MetricTone = 'default' | 'accent' | 'muted' | 'success' | 'warning'

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()

async function loadDashboard() {
  const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId
  if (!projectId) {
    return
  }
  await workspaceStore.loadProjectDashboard(projectId)
}

onMounted(() => {
  void loadDashboard()
})

watch(() => route.params.projectId, () => {
  void loadDashboard()
})

const snapshot = computed(() => workspaceStore.activeDashboard)
const project = computed(() => snapshot.value?.project ?? null)
const metrics = computed(() => snapshot.value?.metrics ?? [])
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
  <UiPageShell width="standard" test-id="project-dashboard-view">
    <UiPageHeader
      :eyebrow="t('projectDashboard.header.eyebrow')"
      :title="project?.name ?? t('projectDashboard.header.titleFallback')"
      :description="project?.description ?? t('projectDashboard.header.subtitleFallback')"
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
        :title="t('projectDashboard.conversations.title')"
      >
        <div v-if="conversations.length" class="grid gap-3">
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
          :title="t('projectDashboard.empty.conversationsTitle')"
          :description="t('projectDashboard.empty.conversationsDescription')"
        />
      </UiPanelFrame>

      <UiPanelFrame
        variant="subtle"
        padding="md"
        :title="t('projectDashboard.activity.title')"
      >
        <UiTimelineList v-if="activities.length" :items="activities" density="compact" />
        <UiEmptyState
          v-else
          :title="t('projectDashboard.empty.activityTitle')"
          :description="t('projectDashboard.empty.activityDescription')"
        />
      </UiPanelFrame>
    </section>
  </UiPageShell>
</template>
