<script setup lang="ts">
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

import { formatDateTime } from '@/i18n/copy'
import { useResourceStore } from '@/stores/resource'
import { useShellStore } from '@/stores/shell'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

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
      <UiEmptyState
        v-else
        :title="t('resources.empty.title')"
        :description="t('resources.empty.description')"
      />
    </UiPanelFrame>
  </component>
</template>
