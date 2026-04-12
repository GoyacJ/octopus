<script setup lang="ts">
import type { WorkspaceResourceRecord } from '@octopus/schema'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiEmptyState,
  UiInput,
  UiListRow,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiStatusCallout,
} from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useResourceStore } from '@/stores/resource'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const { t } = useI18n()
const resourceStore = useResourceStore()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const searchQuery = ref('')

interface ResourceSection {
  id: string
  title: string
  subtitle?: string
  resources: WorkspaceResourceRecord[]
}

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void resourceStore.loadWorkspaceResources(connectionId)
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

function resourceBadgeLabel(group: string, value?: string | null) {
  return enumLabel(group, value)
}

function resourceSubtitle(resource: WorkspaceResourceRecord) {
  return resource.location || resourceBadgeLabel('resourceOrigin', resource.origin)
}

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
      resource.scope,
      resource.visibility,
      resource.status,
      projectLabel(resource.projectId),
      ...resource.tags,
    ].join(' ').toLowerCase().includes(query)
  })
})

const workspaceSection = computed<ResourceSection>(() => ({
  id: 'workspace',
  title: t('resources.workspaceSections.workspace'),
  subtitle: t('resources.workspaceSections.workspaceDescription'),
  resources: filteredResources.value.filter(resource =>
    resource.scope === 'workspace' && !resource.projectId,
  ),
}))

const personalSection = computed<ResourceSection>(() => ({
  id: 'personal',
  title: t('resources.workspaceSections.personal'),
  subtitle: t('resources.workspaceSections.personalDescription'),
  resources: filteredResources.value.filter(resource =>
    resource.scope === 'personal' && resource.ownerUserId === currentUserId.value,
  ),
}))

const projectSections = computed<ResourceSection[]>(() =>
  workspaceStore.projects
    .map(project => ({
      id: project.id,
      title: project.name,
      resources: filteredResources.value.filter(resource =>
        resource.projectId === project.id && resource.scope !== 'personal',
      ),
    }))
    .filter(section => section.resources.length > 0),
)

const hasVisibleResources = computed(() =>
  workspaceSection.value.resources.length > 0
  || personalSection.value.resources.length > 0
  || projectSections.value.length > 0,
)
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    :width="props.embedded ? undefined : 'standard'"
    :test-id="props.embedded ? undefined : 'workspace-resources-view'"
    :data-testid="props.embedded ? 'workspace-resources-embedded' : undefined"
    class="space-y-6"
  >
    <UiPageHeader
      v-if="!props.embedded"
      :eyebrow="t('resources.header.eyebrow')"
      :title="t('sidebar.navigation.resources')"
      :description="t('resources.header.subtitle')"
    >
      <template #actions>
        <UiInput
          v-model="searchQuery"
          :placeholder="t('resources.filters.searchPlaceholder')"
          class="w-full md:w-[320px]"
        />
      </template>
    </UiPageHeader>

    <div v-else class="flex justify-end">
      <UiInput
        v-model="searchQuery"
        :placeholder="t('resources.filters.searchPlaceholder')"
        class="w-full md:w-[320px]"
      />
    </div>

    <UiStatusCallout
      v-if="resourceStore.error"
      tone="error"
      :description="resourceStore.error"
    />

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('sidebar.navigation.resources')"
      :subtitle="t('resources.header.subtitle')"
    >
      <div v-if="hasVisibleResources" class="space-y-8">
        <section
          v-if="workspaceSection.resources.length"
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
          <div class="space-y-2">
            <UiListRow
              v-for="resource in workspaceSection.resources"
              :key="resource.id"
              :title="resource.name"
              :subtitle="resourceSubtitle(resource)"
            >
              <template #meta>
                <UiBadge :label="resourceBadgeLabel('resourceKind', resource.kind)" subtle />
                <UiBadge :label="resourceBadgeLabel('resourceScope', resource.scope)" subtle />
                <UiBadge :label="resourceBadgeLabel('resourceVisibility', resource.visibility)" subtle />
                <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
              </template>
            </UiListRow>
          </div>
        </section>

        <section
          v-if="personalSection.resources.length"
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
          <div class="space-y-2">
            <UiListRow
              v-for="resource in personalSection.resources"
              :key="resource.id"
              :title="resource.name"
              :subtitle="resourceSubtitle(resource)"
            >
              <template #meta>
                <UiBadge :label="resourceBadgeLabel('resourceKind', resource.kind)" subtle />
                <UiBadge :label="resourceBadgeLabel('resourceScope', resource.scope)" subtle />
                <UiBadge :label="resourceBadgeLabel('resourceVisibility', resource.visibility)" subtle />
                <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
              </template>
            </UiListRow>
          </div>
        </section>

        <section
          v-if="projectSections.length"
          class="space-y-4 border-t border-border-subtle pt-6"
        >
          <header class="space-y-1">
            <h2 class="text-sm font-semibold text-text-primary">
              {{ t('resources.workspaceSections.projectGroups') }}
            </h2>
            <p class="text-xs text-text-secondary">
              {{ t('resources.workspaceSections.projectGroupsDescription') }}
            </p>
          </header>
          <div class="space-y-6">
            <div
              v-for="section in projectSections"
              :key="section.id"
              class="space-y-2"
            >
              <h3 class="text-sm font-semibold text-text-primary">
                {{ section.title }}
              </h3>
              <div class="space-y-2">
                <UiListRow
                  v-for="resource in section.resources"
                  :key="resource.id"
                  :title="resource.name"
                  :subtitle="resourceSubtitle(resource)"
                >
                  <template #meta>
                    <UiBadge :label="resourceBadgeLabel('resourceKind', resource.kind)" subtle />
                    <UiBadge :label="resourceBadgeLabel('resourceScope', resource.scope)" subtle />
                    <UiBadge :label="resourceBadgeLabel('resourceVisibility', resource.visibility)" subtle />
                    <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
                  </template>
                </UiListRow>
              </div>
            </div>
          </div>
        </section>
      </div>
      <UiEmptyState
        v-else
        :title="t('resources.empty.title')"
        :description="t('resources.empty.description')"
      />
    </UiPanelFrame>
  </component>
</template>
