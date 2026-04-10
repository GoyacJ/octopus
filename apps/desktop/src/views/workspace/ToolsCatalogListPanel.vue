<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type {
  WorkspaceToolCatalogEntry,
  WorkspaceToolKind,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiInput,
  UiPagination,
  UiRecordCard,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

defineProps<{
  activeTab: WorkspaceToolKind
  tabs: Array<{ value: WorkspaceToolKind, label: string }>
  searchQuery: string
  pagedEntries: WorkspaceToolCatalogEntry[]
  filteredEntries: WorkspaceToolCatalogEntry[]
  activeTabCount: number
  page: number
  pageCount: number
  selectedEntryId: string
  selectedExternalSkillIds: string[]
  canCopySelectedSkillsToManaged: boolean
  selectedExternalSkillCount: number
  kindLabel: (kind: WorkspaceToolKind) => string
  availabilityLabel: (availability: WorkspaceToolCatalogEntry['availability']) => string
  availabilityTone: (availability: WorkspaceToolCatalogEntry['availability']) => 'default' | 'success' | 'warning'
  permissionLabel: (permission: WorkspaceToolCatalogEntry['requiredPermission']) => string
  ownerScopeLabel: (ownerScope: WorkspaceToolCatalogEntry['ownerScope']) => string
  skillStateLabel: (entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) => string
  sourceOriginLabel: (entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) => string
  isExternalSkillEntry: (entry: WorkspaceToolCatalogEntry) => boolean
}>()

const emit = defineEmits<{
  'update:activeTab': [value: WorkspaceToolKind]
  'update:searchQuery': [value: string]
  'update:page': [value: number]
  'update:selectedExternalSkillIds': [value: string[]]
  selectEntry: [entryId: string]
  beginNewSkill: []
  openImportSkillDialog: []
  copySelectedSkillsToManaged: []
  beginNewMcp: []
}>()

const { t } = useI18n()
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow test-id="workspace-tools-toolbar">
      <template #search>
        <UiInput
          :model-value="searchQuery"
          :placeholder="t('tools.search.placeholder')"
          @update:model-value="emit('update:searchQuery', String($event))"
        />
      </template>

      <template #tabs>
        <UiTabs
          :model-value="activeTab"
          :tabs="tabs"
          @update:model-value="emit('update:activeTab', $event as WorkspaceToolKind)"
        />
      </template>

      <template #actions>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <span class="text-[12px] text-text-tertiary">
            {{ t('tools.summary.results', { count: filteredEntries.length, total: activeTabCount }) }}
          </span>
          <UiButton
            v-if="activeTab === 'skill'"
            variant="ghost"
            size="sm"
            @click="emit('beginNewSkill')"
          >
            {{ t('tools.actions.newSkill') }}
          </UiButton>
          <UiButton
            v-if="activeTab === 'skill'"
            variant="ghost"
            size="sm"
            @click="emit('openImportSkillDialog')"
          >
            {{ t('tools.actions.importSkill') }}
          </UiButton>
          <UiButton
            v-if="activeTab === 'skill' && canCopySelectedSkillsToManaged"
            variant="ghost"
            size="sm"
            data-testid="tools-copy-selected-skills-button"
            @click="emit('copySelectedSkillsToManaged')"
          >
            {{ `${t('tools.actions.copyToManaged')} (${selectedExternalSkillCount})` }}
          </UiButton>
          <UiButton
            v-if="activeTab === 'mcp'"
            variant="ghost"
            size="sm"
            @click="emit('beginNewMcp')"
          >
            {{ t('tools.actions.newMcp') }}
          </UiButton>
        </div>
      </template>
    </UiToolbarRow>

    <section class="space-y-3">
      <UiRecordCard
        v-for="entry in pagedEntries"
        :key="entry.id"
        :title="entry.name"
        :description="entry.description"
        :active="selectedEntryId === entry.id"
        :test-id="`tool-entry-${entry.id}`"
        interactive
        @click="emit('selectEntry', entry.id)"
      >
        <template #eyebrow>
          {{ kindLabel(entry.kind) }}
        </template>

        <template #badges>
          <UiBadge :label="availabilityLabel(entry.availability)" :tone="availabilityTone(entry.availability)" />
          <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
          <UiBadge v-if="entry.kind === 'builtin' && entry.requiredPermission" :label="permissionLabel(entry.requiredPermission)" subtle />
          <UiBadge v-if="entry.kind === 'skill'" :label="skillStateLabel(entry)" subtle />
          <UiBadge v-if="entry.kind === 'skill' && entry.workspaceOwned" :label="t('tools.states.managed')" subtle />
          <UiBadge v-if="entry.kind === 'skill' && !entry.workspaceOwned" :label="t('tools.states.readonly')" subtle />
          <UiBadge v-if="entry.kind === 'skill' && !entry.workspaceOwned" :label="t('tools.states.external')" subtle />
          <UiBadge v-if="entry.kind === 'mcp' && entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
          <UiBadge v-if="entry.ownerScope" :label="ownerScopeLabel(entry.ownerScope)" subtle />
          <UiBadge v-if="entry.ownerLabel" :label="entry.ownerLabel" subtle />
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
          <p
            v-if="entry.ownerScope || entry.ownerLabel"
            class="text-[11px] text-text-tertiary"
          >
            {{ t('tools.detail.source') }}:
            {{ entry.ownerScope ? ownerScopeLabel(entry.ownerScope) : t('common.na') }}
            <template v-if="entry.ownerLabel">
              · {{ entry.ownerLabel }}
            </template>
          </p>
          <div
            v-if="entry.consumers?.length"
            class="flex flex-wrap gap-1.5 pt-1"
          >
            <UiBadge
              v-for="consumer in entry.consumers"
              :key="`${entry.id}-${consumer.kind}-${consumer.id}`"
              :label="consumer.name"
              subtle
            />
          </div>
        </div>

        <template #meta>
          <span
            v-if="entry.kind === 'mcp' && entry.statusDetail"
            class="text-[11px] text-status-warning"
          >
            {{ entry.statusDetail }}
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

        <template #actions>
          <div
            v-if="isExternalSkillEntry(entry)"
            class="flex items-center"
            @click.stop
            @keydown.stop
          >
            <UiCheckbox
              :model-value="selectedExternalSkillIds"
              :value="entry.id"
              :label="t('tools.actions.selectForCopy')"
              :class="'text-[12px] text-text-secondary'"
              :data-testid="`tool-entry-select-${entry.id}`"
              @update:model-value="emit('update:selectedExternalSkillIds', $event as string[])"
            />
          </div>
        </template>
      </UiRecordCard>

      <UiEmptyState
        v-if="!filteredEntries.length"
        :title="searchQuery ? t('tools.empty.filteredTitle') : t('tools.empty.title')"
        :description="searchQuery ? t('tools.empty.filteredDescription') : t('tools.empty.description')"
      />

      <UiPagination
        v-else-if="pageCount > 1"
        :page="page"
        :page-count="pageCount"
        :meta-label="`共 ${filteredEntries.length} 项`"
        @update:page="emit('update:page', $event)"
      />
    </section>
  </section>
</template>
