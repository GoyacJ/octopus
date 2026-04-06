<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search, Plus, Filter } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiInput,
  UiListRow,
  UiMetricCard,
  UiPagination,
  UiSectionHeading,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

const PAGE_SIZE = 12
const { t } = useI18n()
const workbench = useWorkbenchStore()

const searchQuery = ref('')

// Simplified for workspace - showing all resources in the workspace
const resources = computed(() => workbench.resources.filter(r => r.workspaceId === workbench.currentWorkspaceId))
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())

const filteredResources = computed(() => {
  return resources.value.filter((resource) => {
    if (!normalizedSearchQuery.value) return true
    return workbench.projectResourceDisplayName(resource.id).toLowerCase().includes(normalizedSearchQuery.value)
  })
})

const { currentPage, pageCount, pagedItems, setPage } = usePagination(filteredResources, { pageSize: PAGE_SIZE })

function resourceLabel(resource: any): string {
  return workbench.projectResourceDisplayName(resource.id)
}
</script>

<template>
  <div class="w-full flex flex-col gap-6 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0 space-y-6">
      <UiSectionHeading eyebrow="WORKSPACE" title="Resource Library" subtitle="Global intelligence assets and shared files." />
      <div class="grid gap-3 sm:grid-cols-2 md:grid-cols-4">
        <UiMetricCard label="Total Resources" :value="resources.length" tone="accent" />
        <UiMetricCard label="Shared Files" :value="resources.filter(r => r.origin === 'source').length" />
      </div>
    </header>

    <div class="px-2 flex flex-wrap items-center justify-between gap-4 border-b border-border-subtle pb-4">
      <div class="flex items-center gap-2">
        <UiButton variant="ghost" size="sm" class="bg-accent font-medium">All Assets</UiButton>
      </div>
      <div class="flex items-center gap-2">
        <div class="relative w-64">
          <Search :size="14" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-tertiary" />
          <UiInput v-model="searchQuery" class="pl-8 bg-subtle/30 h-8" placeholder="Search library..." />
        </div>
        <UiButton variant="primary" class="h-8"><Plus :size="16" /> Upload to Library</UiButton>
      </div>
    </div>

    <main class="flex-1 overflow-y-auto min-h-0 px-2">
      <div class="flex flex-col gap-1">
        <UiListRow
          v-for="resource in pagedItems"
          :key="resource.id"
          :title="resourceLabel(resource)"
          :subtitle="resource.location"
          interactive
        >
          <template #meta>
            <UiBadge :label="resource.kind" subtle />
            <span class="text-[11px] text-text-tertiary">Shared</span>
          </template>
        </UiListRow>
      </div>
      
      <UiEmptyState v-if="!pagedItems.length" title="No resources found" description="Try adjusting your search or upload new files." />

      <div v-if="pageCount > 1" class="mt-8 flex justify-center border-t border-border-subtle pt-4">
        <UiPagination :page="currentPage" :page-count="pageCount" @update:page="setPage" />
      </div>
    </main>
  </div>
</template>
