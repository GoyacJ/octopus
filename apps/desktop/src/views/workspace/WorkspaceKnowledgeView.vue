<script setup lang="ts">
import type { KnowledgeRecord } from '@octopus/schema'
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

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const { t } = useI18n()
const knowledgeStore = useKnowledgeStore()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const searchQuery = ref('')

interface KnowledgeSection {
  id: string
  title: string
  subtitle?: string
  entries: KnowledgeRecord[]
}

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void knowledgeStore.loadWorkspaceKnowledge(connectionId)
    }
  },
  { immediate: true },
)

const currentUserId = computed(() => shell.activeWorkspaceSession?.session.userId ?? '')
const projectNameById = computed(() => new Map(workspaceStore.projects.map(project => [project.id, project.name])))

function projectLabel(projectId?: string | null) {
  if (!projectId) {
    return ''
  }

  return projectNameById.value.get(projectId) ?? projectId
}

function knowledgeBadgeLabel(group: 'resourceScope' | 'resourceVisibility', value?: string | null) {
  return enumLabel(group, value)
}

const filteredKnowledge = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return knowledgeStore.workspaceKnowledge.filter((entry) => {
    if (!query) {
      return true
    }
    return [
      entry.title,
      entry.summary,
      entry.kind,
      entry.scope ?? '',
      entry.visibility ?? '',
      entry.sourceType,
      entry.sourceRef,
      projectLabel(entry.projectId),
    ].join(' ').toLowerCase().includes(query)
  })
})

const workspaceSection = computed<KnowledgeSection>(() => ({
  id: 'workspace',
  title: t('knowledge.workspaceSections.workspace'),
  subtitle: t('knowledge.workspaceSections.workspaceDescription'),
  entries: filteredKnowledge.value.filter(entry =>
    entry.scope === 'workspace' && !entry.projectId,
  ),
}))

const personalSection = computed<KnowledgeSection>(() => ({
  id: 'personal',
  title: t('knowledge.workspaceSections.personal'),
  subtitle: t('knowledge.workspaceSections.personalDescription'),
  entries: filteredKnowledge.value.filter(entry =>
    entry.scope === 'personal' && entry.ownerUserId === currentUserId.value,
  ),
}))

const projectSections = computed<KnowledgeSection[]>(() =>
  workspaceStore.projects
    .map(project => ({
      id: project.id,
      title: project.name,
      entries: filteredKnowledge.value.filter(entry =>
        entry.projectId === project.id && entry.scope !== 'personal',
      ),
    }))
    .filter(section => section.entries.length > 0),
)

const hasVisibleKnowledge = computed(() =>
  workspaceSection.value.entries.length > 0
  || personalSection.value.entries.length > 0
  || projectSections.value.length > 0,
)
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    :width="props.embedded ? undefined : 'standard'"
    :test-id="props.embedded ? undefined : 'workspace-knowledge-view'"
    :data-testid="props.embedded ? 'workspace-knowledge-embedded' : undefined"
    class="space-y-6"
  >
    <UiPageHeader
      v-if="!props.embedded"
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

    <div v-else class="flex justify-end">
      <UiInput
        v-model="searchQuery"
        :placeholder="t('knowledge.filters.searchPlaceholder')"
        class="w-full md:w-[320px]"
      />
    </div>

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
      <div v-if="hasVisibleKnowledge" class="space-y-8">
        <section
          v-if="workspaceSection.entries.length"
          class="space-y-3"
        >
          <header class="space-y-1">
            <h2 class="text-sm font-semibold text-text-primary">
              {{ workspaceSection.title }}
            </h2>
            <p v-if="workspaceSection.subtitle" class="text-xs text-text-secondary">
              {{ workspaceSection.subtitle }}
            </p>
          </header>
          <div class="grid gap-3 lg:grid-cols-2">
            <UiRecordCard
              v-for="entry in workspaceSection.entries"
              :key="entry.id"
              :title="entry.title"
              :description="entry.summary"
            >
              <template #badges>
                <UiBadge :label="entry.kind" subtle />
                <UiBadge :label="knowledgeBadgeLabel('resourceScope', entry.scope)" subtle />
                <UiBadge :label="knowledgeBadgeLabel('resourceVisibility', entry.visibility)" subtle />
              </template>
              <template #meta>
                <span class="text-xs text-text-tertiary">{{ formatDateTime(entry.updatedAt) }}</span>
              </template>
            </UiRecordCard>
          </div>
        </section>

        <section
          v-if="personalSection.entries.length"
          class="space-y-3 border-t border-border-subtle pt-6"
        >
          <header class="space-y-1">
            <h2 class="text-sm font-semibold text-text-primary">
              {{ personalSection.title }}
            </h2>
            <p v-if="personalSection.subtitle" class="text-xs text-text-secondary">
              {{ personalSection.subtitle }}
            </p>
          </header>
          <div class="grid gap-3 lg:grid-cols-2">
            <UiRecordCard
              v-for="entry in personalSection.entries"
              :key="entry.id"
              :title="entry.title"
              :description="entry.summary"
            >
              <template #badges>
                <UiBadge :label="entry.kind" subtle />
                <UiBadge :label="knowledgeBadgeLabel('resourceScope', entry.scope)" subtle />
                <UiBadge :label="knowledgeBadgeLabel('resourceVisibility', entry.visibility)" subtle />
              </template>
              <template #meta>
                <span class="text-xs text-text-tertiary">{{ formatDateTime(entry.updatedAt) }}</span>
              </template>
            </UiRecordCard>
          </div>
        </section>

        <section
          v-if="projectSections.length"
          class="space-y-4 border-t border-border-subtle pt-6"
        >
          <header class="space-y-1">
            <h2 class="text-sm font-semibold text-text-primary">
              {{ t('knowledge.workspaceSections.projectGroups') }}
            </h2>
            <p class="text-xs text-text-secondary">
              {{ t('knowledge.workspaceSections.projectGroupsDescription') }}
            </p>
          </header>
          <div class="space-y-6">
            <div
              v-for="section in projectSections"
              :key="section.id"
              class="space-y-3"
            >
              <h3 class="text-sm font-semibold text-text-primary">
                {{ section.title }}
              </h3>
              <div class="grid gap-3 lg:grid-cols-2">
                <UiRecordCard
                  v-for="entry in section.entries"
                  :key="entry.id"
                  :title="entry.title"
                  :description="entry.summary"
                >
                  <template #badges>
                    <UiBadge :label="entry.kind" subtle />
                    <UiBadge :label="knowledgeBadgeLabel('resourceScope', entry.scope)" subtle />
                    <UiBadge :label="knowledgeBadgeLabel('resourceVisibility', entry.visibility)" subtle />
                  </template>
                  <template #meta>
                    <span class="text-xs text-text-tertiary">{{ formatDateTime(entry.updatedAt) }}</span>
                  </template>
                </UiRecordCard>
              </div>
            </div>
          </div>
        </section>
      </div>
      <UiEmptyState
        v-else
        :title="t('knowledge.empty.workspaceTitle')"
        :description="t('knowledge.empty.workspaceDescription')"
      />
    </UiPanelFrame>
  </component>
</template>
