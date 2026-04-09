<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

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
  <UiPageShell width="standard" test-id="workspace-knowledge-view">
    <UiPageHeader
      :eyebrow="t('knowledge.header.workspaceEyebrow')"
      :title="t('sidebar.navigation.knowledge')"
      :description="t('knowledge.header.workspaceSubtitle')"
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
      :subtitle="t('knowledge.header.workspaceSubtitle')"
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
        :title="t('knowledge.empty.workspaceTitle')"
        :description="t('knowledge.empty.workspaceDescription')"
      />
    </UiPanelFrame>
  </UiPageShell>
</template>
