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
  UiToolbarRow,
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
    if (kindFilter.value !== 'all' && entry.kind !== kindFilter.value) {
      return false
    }

    if (sourceFilter.value !== 'all' && entry.sourceType !== sourceFilter.value) {
      return false
    }

    if (!normalizedSearch.value) {
      return true
    }

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
    if (sortKey.value === 'title') {
      return entryTitle(left).localeCompare(entryTitle(right))
    }

    if (sortKey.value === 'trust') {
      return trustRank(right.trustLevel) - trustRank(left.trustLevel) || right.lastUsedAt - left.lastUsedAt
    }

    return right.lastUsedAt - left.lastUsedAt
  })
})

const selectedEntry = computed(() =>
  filteredEntries.value.find((entry) => entry.id === selectedKnowledgeId.value),
)

const selectedConversation = computed<Conversation | undefined>(() => {
  const entry = selectedEntry.value
  if (!entry) {
    return undefined
  }

  return workbench.projectConversations.find((conversation) =>
    conversation.id === entry.conversationId
    || (entry.sourceType === 'conversation' && conversation.id === entry.sourceId),
  )
})

const sourceArtifact = computed<Artifact | undefined>(() => {
  const entry = selectedEntry.value
  if (!entry || entry.sourceType !== 'artifact') {
    return undefined
  }

  return workbench.artifacts.find((artifact) => artifact.id === entry.sourceId)
})

const sourceRun = computed<RunSummary | undefined>(() => {
  const entry = selectedEntry.value
  if (!entry || entry.sourceType !== 'run') {
    return undefined
  }

  return workbench.runs.find((run) => run.id === entry.sourceId)
})

const relatedArtifacts = computed(() => {
  const entry = selectedEntry.value
  if (!entry) {
    return []
  }

  const artifactIds = new Set<string>()

  if (sourceArtifact.value) {
    artifactIds.add(sourceArtifact.value.id)
  }

  selectedConversation.value?.artifactIds.forEach((artifactId) => artifactIds.add(artifactId))
  entry.lineage.forEach((lineageId) => {
    if (workbench.artifacts.some((artifact) => artifact.id === lineageId)) {
      artifactIds.add(lineageId)
    }
  })

  return workbench.artifacts.filter((artifact) => artifactIds.has(artifact.id)).slice(0, 3)
})

const relatedResources = computed(() => {
  const entry = selectedEntry.value
  if (!entry) {
    return []
  }

  return workbench.projectResources
    .filter((resource) =>
      (entry.conversationId ? resource.linkedConversationIds.includes(entry.conversationId) : false)
      || resource.linkedConversationIds.includes(entry.sourceId)
      || resource.id === entry.sourceId
      || resource.sourceArtifactId === entry.sourceId
      || entry.lineage.includes(resource.id)
      || (resource.sourceArtifactId ? entry.lineage.includes(resource.sourceArtifactId) : false),
    )
    .sort((left, right) => right.createdAt - left.createdAt)
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

watch(
  filteredEntries,
  (entries) => {
    if (!entries.length) {
      selectedKnowledgeId.value = ''
      return
    }

    if (!entries.some((entry) => entry.id === selectedKnowledgeId.value)) {
      selectedKnowledgeId.value = entries[0].id
    }
  },
  { immediate: true },
)

function entryTitle(entry: KnowledgeEntry): string {
  return workbench.knowledgeEntryDisplayTitle(entry.id)
}

function entrySummary(entry: KnowledgeEntry): string {
  return workbench.knowledgeEntryDisplaySummary(entry.id)
}

function conversationTitle(conversation: Conversation): string {
  return workbench.conversationDisplayTitle(conversation.id)
}

function conversationSummary(conversation: Conversation): string {
  return workbench.conversationDisplaySummary(conversation.id)
}

function artifactTitle(artifact: Artifact): string {
  return workbench.artifactDisplayTitle(artifact.id)
}

function resourceTitle(resource: ProjectResource): string {
  return workbench.projectResourceDisplayName(resource.id)
}

function traceRankLabel(level: KnowledgeEntry['trustLevel']): string {
  return enumLabel('riskLevel', level)
}

function trustRank(level: KnowledgeEntry['trustLevel']): number {
  if (level === 'high') {
    return 3
  }

  if (level === 'medium') {
    return 2
  }

  return 1
}

function kindLabel(kind: KnowledgeEntry['kind'] | 'all'): string {
  return t(`knowledge.kinds.${kind}`)
}

function kindTone(kind: KnowledgeEntry['kind']) {
  if (kind === 'shared') {
    return 'success'
  }

  return kind === 'candidate' ? 'warning' : 'info'
}

function trustTone(level: KnowledgeEntry['trustLevel']) {
  if (level === 'high') {
    return 'success'
  }

  return level === 'medium' ? 'info' : 'warning'
}

function sourceDescription(entry: KnowledgeEntry): string {
  return t(`knowledge.sourceDescriptions.${entry.sourceType}`)
}

function sourceConversationForEntry(entry: KnowledgeEntry): Conversation | undefined {
  return workbench.projectConversations.find((conversation) =>
    conversation.id === entry.conversationId
    || (entry.sourceType === 'conversation' && conversation.id === entry.sourceId),
  )
}

function sourceArtifactForEntry(entry: KnowledgeEntry): Artifact | undefined {
  if (entry.sourceType !== 'artifact') {
    return undefined
  }

  return workbench.artifacts.find((artifact) => artifact.id === entry.sourceId)
}

function sourceRunForEntry(entry: KnowledgeEntry): RunSummary | undefined {
  if (entry.sourceType !== 'run') {
    return undefined
  }

  return workbench.runs.find((run) => run.id === entry.sourceId)
}

function sourceLabel(entry: KnowledgeEntry): string {
  const sourceConversation = sourceConversationForEntry(entry)
  if (entry.sourceType === 'conversation' && sourceConversation) {
    return conversationTitle(sourceConversation)
  }

  const sourceArtifactEntry = sourceArtifactForEntry(entry)
  if (entry.sourceType === 'artifact' && sourceArtifactEntry) {
    return artifactTitle(sourceArtifactEntry)
  }

  const sourceRunEntry = sourceRunForEntry(entry)
  if (entry.sourceType === 'run' && sourceRunEntry) {
    return sourceRunEntry.title
  }

  return entry.sourceId
}

function conversationTarget(conversationId?: string) {
  if (!conversationId) {
    return createProjectSurfaceTarget('knowledge', workbench.currentWorkspaceId, workbench.currentProjectId)
  }

  return createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conversationId)
}

function traceTarget() {
  return createProjectSurfaceTarget('trace', workbench.currentWorkspaceId, workbench.currentProjectId)
}

function resourcesTarget() {
  return createProjectSurfaceTarget('resources', workbench.currentWorkspaceId, workbench.currentProjectId)
}

function resolveLineageItem(itemId: string): { id: string, type: LineageItemType, label: string } {
  const conversation = workbench.projectConversations.find((entry) => entry.id === itemId)
  if (conversation) {
    return { id: itemId, type: 'conversation', label: conversationTitle(conversation) }
  }

  const artifact = workbench.artifacts.find((entry) => entry.id === itemId)
  if (artifact) {
    return { id: itemId, type: 'artifact', label: artifactTitle(artifact) }
  }

  const run = workbench.runs.find((entry) => entry.id === itemId)
  if (run) {
    return { id: itemId, type: 'run', label: run.title }
  }

  const resource = workbench.projectResources.find((entry) => entry.id === itemId)
  if (resource) {
    return { id: itemId, type: 'resource', label: resourceTitle(resource) }
  }

  const trace = workbench.traces.find((entry) => entry.id === itemId)
  if (trace) {
    return { id: itemId, type: 'trace', label: workbench.traceDisplayTitle(trace.id) }
  }

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
        <UiBadge :label="t('knowledge.hero.badges.context')" subtle />
      </template>

      <p class="text-[15px] leading-relaxed text-text-secondary max-w-4xl">
        {{ t('knowledge.hero.summary') }}
      </p>

      <template #aside>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-2">
          <UiMetricCard :label="t('knowledge.hero.cards.total')" :value="knowledgeStats.total" tone="accent" />
          <UiMetricCard :label="t('knowledge.hero.cards.shared')" :value="knowledgeStats.shared" />
        </div>
      </template>
    </UiPageHero>

    <div class="flex flex-1 min-h-0 gap-10 px-2">
      <!-- Left Sidebar: Filters & List (Slightly wider for full width layout) -->
      <aside class="flex flex-col w-96 shrink-0 border-r border-border-subtle dark:border-white/[0.05] pr-10 gap-6">
        <div class="space-y-4">
          <UiInput
            v-model="searchQuery"
            class="w-full bg-subtle/30 h-10"
            :placeholder="t('knowledge.filters.searchPlaceholder')"
          />
          <div class="flex gap-2">
            <UiSelect v-model="sourceFilter" :options="sourceOptions" class="flex-1" />
            <UiSelect v-model="sortKey" :options="sortOptions" class="flex-1" />
          </div>
          <UiFilterChipGroup
            v-model="kindFilter"
            :items="kindChipItems"
            :allow-empty="false"
            class="pt-1"
          />
        </div>

        <div class="flex-1 overflow-y-auto min-h-0 pt-2 pb-4 pr-2">
          <UiNavCardList
            v-if="knowledgeNavItems.length"
            :items="knowledgeNavItems"
            @select="selectedKnowledgeId = $event"
          />
          <UiEmptyState
            v-else
            :title="knowledgeEntries.length ? t('knowledge.empty.filteredTitle') : t('knowledge.empty.projectTitle')"
            :description="knowledgeEntries.length ? t('knowledge.empty.filteredDescription') : t('knowledge.empty.projectDescription')"
          />
        </div>
      </aside>

      <!-- Right Content: Details (Expanded freely) -->
      <main class="flex-1 overflow-y-auto min-h-0 pl-2 pr-6 pb-12 space-y-16">
        <template v-if="selectedEntry">
          <section class="space-y-8 max-w-5xl">
            <header>
              <h2 class="text-3xl font-bold text-text-primary mb-3">{{ entryTitle(selectedEntry) }}</h2>
              <p class="text-base leading-relaxed text-text-secondary mb-6 max-w-4xl">{{ entrySummary(selectedEntry) }}</p>
              <div class="flex flex-wrap gap-2.5">
                <UiBadge :label="kindLabel(selectedEntry.kind)" :tone="kindTone(selectedEntry.kind)" subtle />
                <UiBadge :label="enumLabel('knowledgeStatus', selectedEntry.status)" subtle />
                <UiBadge :label="enumLabel('knowledgeSourceType', selectedEntry.sourceType)" subtle />
              </div>
            </header>

            <div class="grid gap-x-12 gap-y-8 sm:grid-cols-2 md:grid-cols-4 text-[13px] border-t border-border-subtle dark:border-white/[0.05] pt-8">
              <div>
                <span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.owner') }}</span>
                <span class="text-text-primary font-medium text-sm">{{ selectedEntry.ownerId ?? t('common.workspace') }}</span>
              </div>
              <div>
                <span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.source') }}</span>
                <span class="text-text-primary font-medium text-sm">{{ sourceLabel(selectedEntry) }}</span>
              </div>
              <div>
                <span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.lastUsedAt') }}</span>
                <span class="text-text-primary text-sm">{{ formatDateTime(selectedEntry.lastUsedAt) }}</span>
              </div>
              <div>
                <span class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1.5">{{ t('knowledge.detail.fields.trustLevel') }}</span>
                <UiBadge :label="traceRankLabel(selectedEntry.trustLevel)" :tone="trustTone(selectedEntry.trustLevel)" />
              </div>
            </div>
          </section>

          <section class="space-y-6 border-t border-border-subtle dark:border-white/[0.05] pt-12 max-w-5xl">
            <h3 class="text-xl font-bold text-text-primary">{{ t('knowledge.detail.sourceTitle') }}</h3>
            
            <div class="bg-subtle/30 rounded-lg border border-border-subtle dark:border-white/[0.08] p-6 space-y-4">
              <div class="flex flex-wrap gap-2.5">
                <UiBadge :label="enumLabel('knowledgeSourceType', selectedEntry.sourceType)" subtle />
                <UiBadge :label="selectedEntry.sourceId" subtle />
              </div>
              <strong class="block text-lg font-bold text-text-primary">{{ sourceLabel(selectedEntry) }}</strong>
              <p class="text-[14px] leading-relaxed text-text-secondary max-w-3xl">{{ sourceDescription(selectedEntry) }}</p>
            </div>

            <div class="pt-6">
              <h4 class="text-[14px] font-bold text-text-primary mb-4">{{ t('knowledge.detail.lineageTitle') }}</h4>
              <div v-if="lineageItems.length" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <UiInfoCard
                  v-for="item in lineageItems"
                  :key="item.id"
                  :label="t(`knowledge.lineageTypes.${item.type}`)"
                  :title="item.label"
                />
              </div>
              <p v-else class="text-[14px] text-text-tertiary italic">{{ t('knowledge.empty.lineageDescription') }}</p>
            </div>
          </section>

          <section class="space-y-6 border-t border-border-subtle dark:border-white/[0.05] pt-12">
            <h3 class="text-xl font-bold text-text-primary">{{ t('knowledge.detail.contextTitle') }}</h3>
            
            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
              <RouterLink
                v-if="selectedConversation"
                class="block min-w-0 no-underline"
                :to="conversationTarget(selectedConversation.id)"
              >
                <UiActionCard
                  :eyebrow="t('knowledge.detail.actions.openConversation')"
                  :title="conversationTitle(selectedConversation)"
                  :description="conversationSummary(selectedConversation)"
                  class="h-full"
                />
              </RouterLink>

              <RouterLink class="block min-w-0 no-underline" :to="traceTarget()">
                <UiActionCard
                  :eyebrow="t('knowledge.detail.actions.openTrace')"
                  :title="t('knowledge.detail.actions.traceLabel')"
                  :description="sourceRun ? sourceRun.currentStep : t('knowledge.detail.actions.traceHint')"
                  class="h-full"
                />
              </RouterLink>

              <RouterLink class="block min-w-0 no-underline" :to="resourcesTarget()">
                <UiActionCard
                  :eyebrow="t('knowledge.detail.actions.openResources')"
                  :title="t('knowledge.detail.actions.resourcesLabel')"
                  :description="t('knowledge.detail.actions.resourcesHint', { count: relatedResources.length })"
                  class="h-full"
                />
              </RouterLink>
            </div>
          </section>
        </template>

        <div v-else class="flex h-full items-center justify-center">
          <UiEmptyState
            :title="t('knowledge.empty.selectionTitle')"
            :description="t('knowledge.empty.selectionDescription')"
          />
        </div>
      </main>
    </div>
  </div>
</template>
