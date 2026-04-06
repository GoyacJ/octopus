<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'

import { UiEmptyState, UiMetricCard, UiRecordCard, UiSectionHeading, UiTimelineList } from '@octopus/ui'

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
  <div class="flex w-full flex-col gap-8 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('projectDashboard.header.eyebrow')"
        :title="project?.name ?? t('projectDashboard.header.titleFallback')"
        :subtitle="project?.description ?? t('projectDashboard.header.subtitleFallback')"
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
        <h3 class="text-lg font-semibold text-text-primary">{{ t('projectDashboard.conversations.title') }}</h3>
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
        <UiEmptyState v-else :title="t('projectDashboard.empty.conversationsTitle')" :description="t('projectDashboard.empty.conversationsDescription')" />
      </section>

      <section class="space-y-4">
        <h3 class="text-lg font-semibold text-text-primary">{{ t('projectDashboard.activity.title') }}</h3>
        <UiTimelineList v-if="activities.length" :items="activities" />
        <UiEmptyState v-else :title="t('projectDashboard.empty.activityTitle')" :description="t('projectDashboard.empty.activityDescription')" />
      </section>
    </div>
  </div>
</template>
