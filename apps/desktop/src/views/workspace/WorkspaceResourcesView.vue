<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiInput, UiListRow, UiSectionHeading } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useResourceStore } from '@/stores/resource'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const resourceStore = useResourceStore()
const shell = useShellStore()
const searchQuery = ref('')

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void resourceStore.loadWorkspaceResources(connectionId)
    }
  },
  { immediate: true },
)

const filteredResources = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return resourceStore.workspaceResources.filter((resource) => {
    if (!query) {
      return true
    }

    return [
      resource.name,
      resource.location ?? '',
      resource.kind,
      resource.origin,
      ...resource.tags,
    ].join(' ').toLowerCase().includes(query)
  })
})
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading
        :eyebrow="t('resources.header.eyebrow')"
        :title="t('sidebar.navigation.resources')"
        :subtitle="resourceStore.error || t('resources.header.subtitle')"
      />
      <UiInput v-model="searchQuery" :placeholder="t('resources.filters.searchPlaceholder')" class="max-w-md" />
    </header>

    <main class="px-2">
      <div v-if="filteredResources.length" class="space-y-2">
        <UiListRow
          v-for="resource in filteredResources"
          :key="resource.id"
          :title="resource.name"
          :subtitle="resource.location || resource.origin"
        >
          <template #meta>
            <UiBadge :label="resource.kind" subtle />
            <UiBadge :label="resource.origin" subtle />
            <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
          </template>
        </UiListRow>
      </div>
      <UiEmptyState v-else :title="t('resources.empty.title')" :description="t('resources.empty.description')" />
    </main>
  </div>
</template>
