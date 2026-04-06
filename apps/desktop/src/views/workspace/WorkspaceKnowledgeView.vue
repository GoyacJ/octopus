<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiInput, UiRecordCard, UiSectionHeading } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const knowledgeStore = useKnowledgeStore()
const shell = useShellStore()
const searchQuery = ref('')

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void knowledgeStore.loadWorkspaceKnowledge(connectionId)
    }
  },
  { immediate: true },
)

const entries = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return knowledgeStore.workspaceKnowledge.filter((entry) => {
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
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading
        :eyebrow="t('knowledge.header.workspaceEyebrow')"
        :title="t('sidebar.navigation.knowledge')"
        :subtitle="knowledgeStore.error || t('knowledge.header.workspaceSubtitle')"
      />
      <UiInput v-model="searchQuery" :placeholder="t('knowledge.filters.searchPlaceholder')" class="max-w-md" />
    </header>

    <main class="grid gap-3 px-2 lg:grid-cols-2">
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
      <UiEmptyState v-if="!entries.length" :title="t('knowledge.empty.workspaceTitle')" :description="t('knowledge.empty.workspaceDescription')" />
    </main>
  </div>
</template>
