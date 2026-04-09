<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import {
  UiBadge,
  UiEmptyState,
  UiInput,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiRecordCard,
  UiStatusCallout,
} from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const knowledgeStore = useKnowledgeStore()
const searchQuery = ref('')

async function loadKnowledge() {
  const projectId = typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId
  if (!projectId) {
    return
  }
  await knowledgeStore.loadProjectKnowledge(projectId)
}

watch(
  () => [shell.activeWorkspaceConnectionId, route.params.projectId],
  ([connectionId]) => {
    if (typeof connectionId === 'string' && connectionId) {
      void loadKnowledge()
    }
  },
  { immediate: true },
)

const entries = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return knowledgeStore.activeProjectKnowledge.filter((entry) => {
    if (!query) {
      return true
    }
    return [
      entry.title,
      entry.summary,
      entry.kind,
      entry.sourceType,
      entry.sourceRef,
    ].join(' ').toLowerCase().includes(query)
  })
})
</script>

<template>
  <UiPageShell width="standard" test-id="project-knowledge-view">
    <UiPageHeader
      :eyebrow="t('knowledge.header.projectEyebrow')"
      :title="workspaceStore.activeProject?.name ?? t('knowledge.header.titleFallback')"
      :description="workspaceStore.activeProject?.description || t('knowledge.header.projectSubtitle')"
    >
      <template #actions>
        <UiInput
          v-model="searchQuery"
          :placeholder="t('knowledge.filters.searchPlaceholder')"
          class="w-full md:w-[320px]"
        />
      </template>
    </UiPageHeader>

    <UiStatusCallout
      v-if="knowledgeStore.error"
      tone="error"
      :description="knowledgeStore.error"
    />

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('sidebar.navigation.knowledge')"
      :subtitle="workspaceStore.activeProject?.description || t('knowledge.header.projectSubtitle')"
    >
      <div class="grid gap-3 lg:grid-cols-2">
        <UiRecordCard
          v-for="entry in entries"
          :key="entry.id"
          :title="entry.title"
          :description="entry.summary"
        >
          <template #badges>
            <UiBadge :label="entry.kind" subtle />
            <UiBadge :label="entry.status" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ formatDateTime(entry.updatedAt) }}</span>
          </template>
        </UiRecordCard>
      </div>
      <UiEmptyState
        v-if="!entries.length"
        :title="t('knowledge.empty.projectTitle')"
        :description="t('knowledge.empty.projectDescription')"
      />
    </UiPanelFrame>
  </UiPageShell>
</template>
