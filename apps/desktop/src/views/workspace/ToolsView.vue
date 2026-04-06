<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { WorkspaceToolCatalogEntry, WorkspaceToolKind } from '@octopus/schema'
import { UiBadge, UiEmptyState, UiInput, UiRecordCard, UiSectionHeading, UiTabs, UiToolbarRow } from '@octopus/ui'

import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const catalogStore = useCatalogStore()
const shell = useShellStore()

const activeTab = ref<WorkspaceToolKind>('builtin')
const searchQuery = ref('')
const selectedEntryId = ref('')

const tabOrder: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']

const tabs = computed(() => tabOrder.map(kind => ({
  value: kind,
  label: t(`tools.tabs.${kind}`),
})))

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void catalogStore.load(connectionId)
    }
  },
  { immediate: true },
)

const allEntries = computed(() => catalogStore.toolCatalogEntries)
const activeTabCount = computed(() => allEntries.value.filter(entry => entry.kind === activeTab.value).length)
const filteredEntries = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return allEntries.value.filter((entry) => {
    if (entry.kind !== activeTab.value) {
      return false
    }
    if (!query) {
      return true
    }

    const haystack = [
      entry.name,
      entry.description,
      entry.displayPath,
      entry.sourceKey,
      entry.kind,
      entry.availability,
      entry.requiredPermission ?? '',
      entry.kind === 'builtin' ? entry.builtinKey : '',
      entry.kind === 'skill' ? entry.shadowedBy ?? '' : '',
      entry.kind === 'skill' ? entry.sourceOrigin : '',
      entry.kind === 'mcp' ? entry.serverName : '',
      entry.kind === 'mcp' ? entry.endpoint : '',
      entry.kind === 'mcp' ? entry.toolNames.join(' ') : '',
      entry.kind === 'mcp' ? entry.statusDetail ?? '' : '',
      entry.kind === 'mcp' ? entry.scope : '',
    ]
      .join(' ')
      .toLowerCase()

    return haystack.includes(query)
  })
})

const selectedEntry = computed(() =>
  filteredEntries.value.find(entry => entry.id === selectedEntryId.value) ?? filteredEntries.value[0] ?? null,
)

watch(filteredEntries, (entries) => {
  if (!entries.length) {
    selectedEntryId.value = ''
    return
  }
  if (!entries.some(entry => entry.id === selectedEntryId.value)) {
    selectedEntryId.value = entries[0].id
  }
}, { immediate: true })

function availabilityTone(availability: WorkspaceToolCatalogEntry['availability']) {
  switch (availability) {
    case 'healthy':
      return 'success'
    case 'attention':
      return 'warning'
    default:
      return 'default'
  }
}

function kindLabel(kind: WorkspaceToolKind) {
  return t(`tools.tabs.${kind}`)
}

function availabilityLabel(availability: WorkspaceToolCatalogEntry['availability']) {
  return t(`tools.availability.${availability}`)
}

function permissionLabel(permission: WorkspaceToolCatalogEntry['requiredPermission']) {
  if (!permission) {
    return t('common.na')
  }
  return t(`tools.requiredPermissions.${permission}`)
}

function sourceOriginLabel(entry: WorkspaceToolCatalogEntry) {
  if (entry.kind !== 'skill') {
    return ''
  }
  return t(`tools.sourceOrigins.${entry.sourceOrigin}`)
}

function skillStateLabel(entry: WorkspaceToolCatalogEntry) {
  if (entry.kind !== 'skill') {
    return ''
  }
  return entry.active ? t('tools.states.active') : t('tools.states.shadowed')
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('tools.header.eyebrow')"
        :title="t('sidebar.navigation.tools')"
        :subtitle="catalogStore.error || t('tools.header.subtitle')"
      />
    </header>

    <section class="px-2">
      <UiToolbarRow test-id="workspace-tools-toolbar">
        <template #search>
          <UiInput
            v-model="searchQuery"
            :placeholder="t('tools.search.placeholder')"
          />
        </template>

        <template #tabs>
          <UiTabs v-model="activeTab" :tabs="tabs" />
        </template>

        <template #actions>
          <span class="text-[12px] text-text-tertiary">
            {{ t('tools.summary.results', { count: filteredEntries.length, total: activeTabCount }) }}
          </span>
        </template>
      </UiToolbarRow>
    </section>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_380px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="entry in filteredEntries"
          :key="entry.id"
          :title="entry.name"
          :description="entry.description"
          :active="selectedEntry?.id === entry.id"
          interactive
          @click="selectedEntryId = entry.id"
        >
          <template #eyebrow>
            {{ kindLabel(entry.kind) }}
          </template>

          <template #badges>
            <UiBadge :label="availabilityLabel(entry.availability)" :tone="availabilityTone(entry.availability)" />
            <UiBadge v-if="entry.requiredPermission" :label="permissionLabel(entry.requiredPermission)" subtle />
            <UiBadge v-if="entry.kind === 'skill'" :label="skillStateLabel(entry)" subtle />
            <UiBadge v-if="entry.kind === 'mcp' && entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
          </template>

          <div class="space-y-1">
            <p class="line-clamp-1 text-[12px] text-text-secondary">
              {{ entry.displayPath }}
            </p>
            <p
              v-if="entry.kind === 'mcp' && entry.endpoint"
              class="line-clamp-1 font-mono text-[11px] text-text-tertiary"
            >
              {{ entry.endpoint }}
            </p>
            <p
              v-else-if="entry.kind === 'skill' && entry.shadowedBy"
              class="line-clamp-1 text-[11px] text-text-tertiary"
            >
              {{ t('tools.detail.shadowedBy') }}: {{ entry.shadowedBy }}
            </p>
          </div>

          <template #meta>
            <span
              v-if="entry.kind === 'mcp' && entry.statusDetail"
              class="text-[11px] text-status-warning"
            >
              {{ entry.statusDetail }}
            </span>
            <span
              v-else-if="entry.kind === 'mcp' && entry.toolNames.length"
              class="line-clamp-1 font-mono text-[11px] text-text-tertiary"
            >
              {{ entry.toolNames.join(', ') }}
            </span>
            <span
              v-else-if="entry.kind === 'skill'"
              class="text-[11px] text-text-tertiary"
            >
              {{ sourceOriginLabel(entry) }}
            </span>
            <span
              v-else-if="entry.kind === 'builtin' && entry.builtinKey"
              class="font-mono text-[11px] text-text-tertiary"
            >
              {{ entry.builtinKey }}
            </span>
          </template>
        </UiRecordCard>

        <UiEmptyState
          v-if="!filteredEntries.length"
          :title="searchQuery ? t('tools.empty.filteredTitle') : t('tools.empty.title')"
          :description="searchQuery ? t('tools.empty.filteredDescription') : t('tools.empty.description')"
        />
      </section>

      <section>
        <UiRecordCard
          v-if="selectedEntry"
          :title="selectedEntry.name"
          :description="selectedEntry.description"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

          <template #badges>
            <UiBadge :label="kindLabel(selectedEntry.kind)" subtle />
            <UiBadge :label="availabilityLabel(selectedEntry.availability)" :tone="availabilityTone(selectedEntry.availability)" />
          </template>

          <div class="space-y-4">
            <div class="space-y-1">
              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('tools.detail.sourcePath') }}
              </div>
              <div class="break-all font-mono text-[12px] text-text-secondary">
                {{ selectedEntry.displayPath }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('tools.detail.sourceKey') }}
              </div>
              <div class="break-all font-mono text-[12px] text-text-secondary">
                {{ selectedEntry.sourceKey }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('tools.detail.requiredPermission') }}
              </div>
              <div class="text-[13px] text-text-primary">
                {{ permissionLabel(selectedEntry.requiredPermission) }}
              </div>
            </div>

            <template v-if="selectedEntry.kind === 'builtin'">
              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.builtinKey') }}
                </div>
                <div class="font-mono text-[12px] text-text-secondary">
                  {{ selectedEntry.builtinKey }}
                </div>
              </div>
            </template>

            <template v-else-if="selectedEntry.kind === 'skill'">
              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.activeState') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ skillStateLabel(selectedEntry) }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.sourceOrigin') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ sourceOriginLabel(selectedEntry) }}
                </div>
              </div>

              <div v-if="selectedEntry.shadowedBy" class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.shadowedBy') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ selectedEntry.shadowedBy }}
                </div>
              </div>
            </template>

            <template v-else>
              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.serverName') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ selectedEntry.serverName }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.endpoint') }}
                </div>
                <div class="break-all font-mono text-[12px] text-text-secondary">
                  {{ selectedEntry.endpoint }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.toolNames') }}
                </div>
                <div
                  v-if="selectedEntry.toolNames.length"
                  class="flex flex-wrap gap-1.5"
                >
                  <UiBadge
                    v-for="toolName in selectedEntry.toolNames"
                    :key="toolName"
                    :label="toolName"
                    subtle
                  />
                </div>
                <div v-else class="text-[13px] text-text-secondary">
                  {{ t('common.na') }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.scope') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ selectedEntry.scope }}
                </div>
              </div>

              <div v-if="selectedEntry.statusDetail" class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.statusDetail') }}
                </div>
                <div class="text-[13px] text-status-warning">
                  {{ selectedEntry.statusDetail }}
                </div>
              </div>

              <div class="rounded-md border border-border/40 bg-subtle/30 px-3 py-3 text-[12px] text-text-secondary">
                {{ t('tools.detail.settingsHint') }}
                <a class="ml-1 font-medium text-primary hover:underline" href="/settings">
                  {{ t('tools.detail.settingsLink') }}
                </a>
              </div>
            </template>
          </div>
        </UiRecordCard>

        <UiEmptyState
          v-else
          :title="t('tools.empty.selectionTitle')"
          :description="t('tools.empty.selectionDescription')"
        />
      </section>
    </div>
  </div>
</template>
