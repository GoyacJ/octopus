<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { KnowledgeEntry } from '@octopus/schema'
import {
  UiBadge,
  UiEmptyState,
  UiFilterChipGroup,
  UiInput,
  UiMetricCard,
  UiNavCardList,
  UiPageHero,
  UiSectionHeading,
  UiSelect,
} from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

type KnowledgeKindFilter = 'all' | KnowledgeEntry['kind']
type KnowledgeSortKey = 'recent' | 'trust' | 'title'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const searchQuery = ref('')
const selectedKnowledgeId = ref('')
const kindFilter = ref<KnowledgeKindFilter>('all')
const sortKey = ref<KnowledgeSortKey>('recent')
const kindOptions: KnowledgeKindFilter[] = ['all', 'private', 'shared']
const kindChipItems = computed(() =>
  kindOptions.map((kind) => ({
    value: kind,
    label: t(`knowledge.kinds.${kind}`),
  })),
)

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const workspaceName = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayName(workbench.activeWorkspace.id)
    : t('knowledge.header.titleFallback'),
)

// Workspace Level Knowledge
const knowledgeEntries = computed(() => [...workbench.workspaceKnowledge])

const knowledgeStats = computed(() => ({
  total: knowledgeEntries.value.length,
  private: knowledgeEntries.value.filter((entry) => entry.kind === 'private').length,
  shared: knowledgeEntries.value.filter((entry) => entry.kind === 'shared').length,
}))

const sortOptions = computed(() => [
  { value: 'recent', label: t('knowledge.filters.sortRecent') },
  { value: 'trust', label: t('knowledge.filters.sortTrust') },
  { value: 'title', label: t('knowledge.filters.sortTitle') },
])

const filteredEntries = computed(() => {
  const filtered = knowledgeEntries.value.filter((entry) => {
    if (kindFilter.value !== 'all' && entry.kind !== kindFilter.value) return false
    if (!normalizedSearch.value) return true

    const haystack = [
      entryTitle(entry),
      entrySummary(entry),
      entry.sourceId,
      entry.ownerId ?? '',
    ].join(' ').toLowerCase()

    return haystack.includes(normalizedSearch.value)
  })

  return filtered.sort((left, right) => {
    if (sortKey.value === 'title') return entryTitle(left).localeCompare(entryTitle(right))
    return right.lastUsedAt - left.lastUsedAt
  })
})

const selectedEntry = computed(() =>
  filteredEntries.value.find((entry) => entry.id === selectedKnowledgeId.value),
)

const knowledgeNavItems = computed(() =>
  filteredEntries.value.map((entry) => ({
    id: entry.id,
    label: entryTitle(entry),
    helper: entrySummary(entry),
    badge: enumLabel('riskLevel', entry.trustLevel),
    active: selectedKnowledgeId.value === entry.id,
  })),
)

watch(filteredEntries, (entries) => {
  if (!entries.length) { selectedKnowledgeId.value = ''; return }
  if (!entries.some((entry) => entry.id === selectedKnowledgeId.value)) selectedKnowledgeId.value = entries[0].id
}, { immediate: true })

function entryTitle(entry: KnowledgeEntry): string { return workbench.knowledgeEntryDisplayTitle(entry.id) }
function entrySummary(entry: KnowledgeEntry): string { return workbench.knowledgeEntryDisplaySummary(entry.id) }
function sourceDescription(entry: KnowledgeEntry): string { return t(`knowledge.sourceDescriptions.${entry.sourceType}`) }
</script>

<template>
  <div class="w-full flex flex-col gap-6 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('knowledge.header.workspaceEyebrow')"
        :title="workspaceName"
        :subtitle="t('knowledge.header.workspaceSubtitle')"
      />
    </header>

    <UiPageHero class="px-2 shrink-0">
      <p class="text-[15px] leading-relaxed text-text-secondary max-w-4xl">{{ t('knowledge.hero.workspaceSummary') }}</p>
      <template #aside>
        <div class="grid gap-3 sm:grid-cols-2">
          <UiMetricCard :label="t('knowledge.hero.cards.total')" :value="knowledgeStats.total" tone="accent" />
          <UiMetricCard :label="t('knowledge.hero.cards.shared')" :value="knowledgeStats.shared" />
        </div>
      </template>
    </UiPageHero>

    <div class="flex flex-1 min-h-0 gap-10 px-2">
      <!-- Left Sidebar -->
      <aside class="flex flex-col w-96 shrink-0 border-r border-border-subtle pr-10 gap-6">
        <div class="space-y-4">
          <UiInput v-model="searchQuery" class="w-full bg-subtle/30 h-10" :placeholder="t('knowledge.filters.searchPlaceholder')" />
          <div class="flex gap-2">
            <UiSelect v-model="sortKey" :options="sortOptions" class="flex-1" />
          </div>
          <UiFilterChipGroup v-model="kindFilter" :items="kindChipItems" :allow-empty="false" />
        </div>
        <div class="flex-1 overflow-y-auto min-h-0 pt-2 pb-4 pr-2">
          <UiNavCardList v-if="knowledgeNavItems.length" :items="knowledgeNavItems" @select="selectedKnowledgeId = $event" />
          <UiEmptyState v-else :title="t('knowledge.empty.workspaceTitle')" :description="t('knowledge.empty.workspaceDescription')" />
        </div>
      </aside>

      <!-- Right Content -->
      <main class="flex-1 overflow-y-auto min-h-0 pl-2 pr-6 pb-12 space-y-16">
        <template v-if="selectedEntry">
          <section class="space-y-8 max-w-5xl">
            <header>
              <h2 class="text-3xl font-bold text-text-primary mb-3">{{ entryTitle(selectedEntry) }}</h2>
              <p class="text-base leading-relaxed text-text-secondary mb-6 max-w-4xl">{{ entrySummary(selectedEntry) }}</p>
              <div class="flex flex-wrap gap-2.5">
                <UiBadge :label="t(`knowledge.kinds.${selectedEntry.kind}`)" subtle />
                <UiBadge :label="enumLabel('knowledgeStatus', selectedEntry.status)" subtle />
              </div>
            </header>
            <div class="grid gap-x-12 gap-y-8 sm:grid-cols-2 md:grid-cols-3 text-[13px] border-t border-border-subtle pt-8">
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.owner') }}</span><span class="text-text-primary font-medium text-sm">{{ selectedEntry.ownerId ?? t('common.workspace') }}</span></div>
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.lastUsedAt') }}</span><span class="text-text-primary text-sm">{{ formatDateTime(selectedEntry.lastUsedAt) }}</span></div>
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.trustLevel') }}</span><UiBadge :label="enumLabel('riskLevel', selectedEntry.trustLevel)" subtle /></div>
            </div>
          </section>

          <section class="space-y-6 border-t border-border-subtle pt-12 max-w-5xl">
            <h3 class="text-xl font-bold text-text-primary">{{ t('knowledge.detail.sourceTitle') }}</h3>
            <div class="bg-subtle/30 rounded-lg border border-border-subtle p-6 space-y-4">
              <strong class="block text-lg font-bold text-text-primary">{{ selectedEntry.sourceId }}</strong>
              <p class="text-[14px] leading-relaxed text-text-secondary max-w-3xl">{{ sourceDescription(selectedEntry) }}</p>
            </div>
          </section>
        </template>
        <div v-else class="flex h-full items-center justify-center">
          <UiEmptyState :title="t('knowledge.empty.selectionTitle')" :description="t('knowledge.empty.selectionDescription')" />
        </div>
      </main>
    </div>
  </div>
</template>
