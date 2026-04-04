<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import type { Artifact, Conversation, KnowledgeEntry, ProjectResource, RunSummary } from '@octopus/schema'
import {
  UiActionCard,
  UiBadge,
  UiEmptyState,
  UiFilterChipGroup,
  UiInfoCard,
  UiInput,
  UiMetricCard,
  UiNavCardList,
  UiPageHero,
  UiSectionHeading,
  UiSelect,
} from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectSurfaceTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

type KnowledgeKindFilter = 'all' | KnowledgeEntry['kind']
type KnowledgeSourceFilter = 'all' | KnowledgeEntry['sourceType']
type KnowledgeSortKey = 'recent' | 'trust' | 'title'
type LineageItemType = 'conversation' | 'artifact' | 'run' | 'resource' | 'trace' | 'unknown'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const searchQuery = ref('')
const selectedKnowledgeId = ref('')
const kindFilter = ref<KnowledgeKindFilter>('all')
const sourceFilter = ref<KnowledgeSourceFilter>('all')
const sortKey = ref<KnowledgeSortKey>('recent')
const kindOptions: KnowledgeKindFilter[] = ['all', 'private', 'shared', 'candidate']
const kindChipItems = computed(() =>
  kindOptions.map((kind) => ({
    value: kind,
    label: kindLabel(kind),
  })),
)

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const activeProjectName = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayName(workbench.activeProject.id)
    : t('knowledge.header.titleFallback'),
)

// Project Specific Knowledge
const knowledgeEntries = computed(() => [...workbench.projectKnowledge])

const knowledgeStats = computed(() => ({
  total: knowledgeEntries.value.length,
  private: knowledgeEntries.value.filter((entry) => entry.kind === 'private').length,
  shared: knowledgeEntries.value.filter((entry) => entry.kind === 'shared').length,
  candidate: knowledgeEntries.value.filter((entry) => entry.kind === 'candidate').length,
  highTrust: knowledgeEntries.value.filter((entry) => entry.trustLevel === 'high').length,
}))

const sourceOptions = computed(() => [
  { value: 'all', label: t('knowledge.filters.sourceAll') },
  { value: 'conversation', label: enumLabel('knowledgeSourceType', 'conversation') },
  { value: 'artifact', label: enumLabel('knowledgeSourceType', 'artifact') },
  { value: 'run', label: enumLabel('knowledgeSourceType', 'run') },
])

const sortOptions = computed(() => [
  { value: 'recent', label: t('knowledge.filters.sortRecent') },
  { value: 'trust', label: t('knowledge.filters.sortTrust') },
  { value: 'title', label: t('knowledge.filters.sortTitle') },
])

const filteredEntries = computed(() => {
  const filtered = knowledgeEntries.value.filter((entry) => {
    if (kindFilter.value !== 'all' && entry.kind !== kindFilter.value) return false
    if (sourceFilter.value !== 'all' && entry.sourceType !== sourceFilter.value) return false
    if (!normalizedSearch.value) return true

    const haystack = [
      entryTitle(entry),
      entrySummary(entry),
      entry.sourceId,
      entry.ownerId ?? '',
      entry.lineage.join(' '),
    ].join(' ').toLowerCase()

    return haystack.includes(normalizedSearch.value)
  })

  return filtered.sort((left, right) => {
    if (sortKey.value === 'title') return entryTitle(left).localeCompare(entryTitle(right))
    if (sortKey.value === 'trust') return trustRank(right.trustLevel) - trustRank(left.trustLevel) || right.lastUsedAt - left.lastUsedAt
    return right.lastUsedAt - left.lastUsedAt
  })
})

const selectedEntry = computed(() =>
  filteredEntries.value.find((entry) => entry.id === selectedKnowledgeId.value),
)

const selectedConversation = computed<Conversation | undefined>(() => {
  const entry = selectedEntry.value
  if (!entry) return undefined
  return workbench.projectConversations.find((c) => c.id === entry.conversationId || (entry.sourceType === 'conversation' && c.id === entry.sourceId))
})

const relatedResources = computed(() => {
  const entry = selectedEntry.value
  if (!entry) return []
  return workbench.projectResources
    .filter((r) => (entry.conversationId ? r.linkedConversationIds.includes(entry.conversationId) : false) || r.linkedConversationIds.includes(entry.sourceId) || r.id === entry.sourceId)
    .slice(0, 4)
})

const lineageItems = computed(() =>
  selectedEntry.value?.lineage.map((itemId) => resolveLineageItem(itemId)) ?? [],
)

const knowledgeNavItems = computed(() =>
  filteredEntries.value.map((entry) => ({
    id: entry.id,
    label: entryTitle(entry),
    helper: entrySummary(entry),
    badge: traceRankLabel(entry.trustLevel),
    active: selectedKnowledgeId.value === entry.id,
  })),
)

watch(filteredEntries, (entries) => {
  if (!entries.length) { selectedKnowledgeId.value = ''; return }
  if (!entries.some((entry) => entry.id === selectedKnowledgeId.value)) selectedKnowledgeId.value = entries[0].id
}, { immediate: true })

function entryTitle(entry: KnowledgeEntry): string { return workbench.knowledgeEntryDisplayTitle(entry.id) }
function entrySummary(entry: KnowledgeEntry): string { return workbench.knowledgeEntryDisplaySummary(entry.id) }
function conversationTitle(c: Conversation): string { return workbench.conversationDisplayTitle(c.id) }
function conversationSummary(c: Conversation): string { return workbench.conversationDisplaySummary(c.id) }
function traceRankLabel(level: KnowledgeEntry['trustLevel']): string { return enumLabel('riskLevel', level) }
function trustRank(level: KnowledgeEntry['trustLevel']): number { return level === 'high' ? 3 : level === 'medium' ? 2 : 1 }
function kindLabel(kind: string): string { return t(`knowledge.kinds.${kind}`) }
function kindTone(kind: string) { return kind === 'shared' ? 'success' : kind === 'candidate' ? 'warning' : 'info' }
function trustTone(level: string) { return level === 'high' ? 'success' : level === 'medium' ? 'info' : 'warning' }
function sourceDescription(entry: KnowledgeEntry): string { return t(`knowledge.sourceDescriptions.${entry.sourceType}`) }

function sourceLabel(entry: KnowledgeEntry): string {
  const c = workbench.projectConversations.find(i => i.id === entry.sourceId)
  if (entry.sourceType === 'conversation' && c) return conversationTitle(c)
  return entry.sourceId
}

function resolveLineageItem(itemId: string): { id: string, type: LineageItemType, label: string } {
  const c = workbench.projectConversations.find(i => i.id === itemId)
  if (c) return { id: itemId, type: 'conversation', label: conversationTitle(c) }
  return { id: itemId, type: 'unknown', label: itemId }
}
</script>

<template>
  <div class="w-full flex flex-col gap-6 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('knowledge.header.eyebrow')"
        :title="activeProjectName"
        :subtitle="t('knowledge.header.subtitle')"
      />
    </header>

    <UiPageHero class="px-2 shrink-0">
      <template #meta>
        <UiBadge :label="t('knowledge.hero.badges.reading')" subtle />
        <UiBadge :label="t('knowledge.hero.badges.lineage')" subtle />
      </template>
      <p data-testid="knowledge-hero-summary" class="text-[15px] leading-relaxed text-text-secondary max-w-4xl">{{ t('knowledge.hero.summary') }} · 偏浏览</p>
      <template #aside>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-2">
          <div data-testid="knowledge-stat-total">
            <UiMetricCard :label="t('knowledge.hero.cards.total')" :value="knowledgeStats.total" tone="accent" class="dark:border-white/5" />
          </div>
          <UiMetricCard :label="t('knowledge.hero.cards.shared')" :value="knowledgeStats.shared" />
        </div>
      </template>
    </UiPageHero>

    <div class="flex flex-1 min-h-0 gap-10 px-2">
      <aside class="flex flex-col w-96 shrink-0 border-r border-border-subtle dark:border-white/[0.02] pr-10 gap-6">
        <div class="space-y-4">
          <UiInput data-testid="knowledge-search-input" v-model="searchQuery" class="w-full bg-subtle/30 h-10" :placeholder="t('knowledge.filters.searchPlaceholder')" />
          <div class="flex gap-2">
            <UiSelect v-model="sourceFilter" :options="sourceOptions" class="flex-1" />
            <UiSelect v-model="sortKey" :options="sortOptions" class="flex-1" />
          </div>
          <UiFilterChipGroup v-model="kindFilter" :items="kindChipItems" :allow-empty="false" />
        </div>
        <div class="flex-1 overflow-y-auto min-h-0 pt-2 pb-4 pr-2">
          <UiNavCardList v-if="knowledgeNavItems.length" :items="knowledgeNavItems" @select="selectedKnowledgeId = $event" />
          <div v-else data-testid="knowledge-empty-state">
            <UiEmptyState
              :title="knowledgeEntries.length ? t('knowledge.empty.filteredTitle') : t('knowledge.empty.projectTitle')"
              :description="knowledgeEntries.length ? t('knowledge.empty.filteredDescription') : t('knowledge.empty.projectDescription')"
            />
          </div>
        </div>
      </aside>

      <main class="flex-1 overflow-y-auto min-h-0 pl-2 pr-6 pb-12 space-y-16">
        <template v-if="selectedEntry">
          <section class="space-y-8 max-w-5xl">
            <header>
              <h2 data-testid="knowledge-detail-title" class="text-3xl font-bold text-text-primary mb-3">{{ entryTitle(selectedEntry) }}</h2>
              <p class="text-base leading-relaxed text-text-secondary mb-6 max-w-4xl">{{ entrySummary(selectedEntry) }}</p>
              <div class="flex flex-wrap gap-2.5">
                <UiBadge :label="kindLabel(selectedEntry.kind)" :tone="kindTone(selectedEntry.kind)" subtle />
                <UiBadge :label="enumLabel('knowledgeStatus', selectedEntry.status)" subtle />
              </div>
            </header>
            <div class="grid gap-x-12 gap-y-8 sm:grid-cols-2 md:grid-cols-4 text-[13px] border-t border-border-subtle dark:border-white/[0.02] pt-8">
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.owner') }}</span><span class="text-text-primary font-medium text-sm">{{ selectedEntry.ownerId ?? t('common.workspace') }}</span></div>
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.source') }}</span><span class="text-text-primary font-medium text-sm">{{ sourceLabel(selectedEntry) }}</span></div>
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.lastUsedAt') }}</span><span class="text-text-primary text-sm">{{ formatDateTime(selectedEntry.lastUsedAt) }}</span></div>
              <div><span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.trustLevel') }}</span><UiBadge :label="traceRankLabel(selectedEntry.trustLevel)" :tone="trustTone(selectedEntry.trustLevel)" /></div>
            </div>
          </section>

          <section class="space-y-6 border-t border-border-subtle dark:border-white/[0.02] pt-12 max-w-5xl">
            <h3 class="text-xl font-bold text-text-primary">{{ t('knowledge.detail.sourceTitle') }}</h3>
            <div data-testid="knowledge-source-card" class="bg-subtle/30 rounded-lg border border-border-subtle dark:border-white/[0.02] p-6 space-y-4">
              <strong class="block text-lg font-bold text-text-primary">{{ sourceLabel(selectedEntry) }}</strong>
              <p class="text-[13px] text-text-tertiary font-mono">{{ selectedEntry.sourceId }}</p>
              <p class="text-[14px] leading-relaxed text-text-secondary max-w-3xl">{{ sourceDescription(selectedEntry) }}</p>
            </div>
            <div v-if="selectedConversation" data-testid="knowledge-related-conversation-link" class="text-sm text-text-secondary">
              {{ conversationTitle(selectedConversation) }}
            </div>
            <ul v-if="lineageItems.length" data-testid="knowledge-lineage-list" class="list-disc pl-5 space-y-2 text-sm text-text-secondary">
              <li v-for="item in lineageItems" :key="item.id">
                {{ t(`knowledge.lineageTypes.${item.type}`) }} · {{ item.label }}
              </li>
            </ul>
          </section>
        </template>
        <div v-else data-testid="knowledge-empty-state" class="flex h-full items-center justify-center">
          <UiEmptyState :title="t('knowledge.empty.selectionTitle')" :description="t('knowledge.empty.selectionDescription')" />
        </div>
      </main>
    </div>
  </div>
</template>
late>
