<script setup lang="ts">
import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiField, UiInput, UiInspectorPanel, UiListDetailShell, UiPageHeader, UiPageShell, UiStatusCallout, UiSwitch } from '@octopus/ui'

import McpDetailPanel from './McpDetailPanel.vue'
import SkillActionDialog from './SkillActionDialog.vue'
import SkillDetailPanel from './SkillDetailPanel.vue'
import ToolsCatalogListPanel from './ToolsCatalogListPanel.vue'
import { useToolsView } from './useToolsView'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const {
  t,
  catalogStore,
  activeTab,
  searchQuery,
  selectedExternalSkillIds,
  draftMode,
  loadingDetail,
  loadingSkillFile,
  submitting,
  deleting,
  toggling,
  panelError,
  currentSkillFile,
  selectedSkillFilePath,
  skillFileDraft,
  skillSlugDraft,
  newSkillContentDraft,
  mcpServerNameDraft,
  mcpConfigDraft,
  skillActionDialogOpen,
  pendingSkillAction,
  pendingSkillImportSource,
  pendingSkillCopies,
  tabs,
  activeTabCount,
  filteredEntries,
  pagedEntries,
  listPage,
  listPageCount,
  listPagination,
  selectedEntry,
  selectedSkillEntry,
  selectedMcpEntry,
  selectedExternalSkillEntries,
  selectedSkillTreeRows,
  canSaveSkillFile,
  canCopySkillToManaged,
  canCopyMcpToManaged,
  canCopySelectedSkillsToManaged,
  pendingSkillActionTitle,
  pendingSkillActionDescription,
  pendingSkillSelectionLabel,
  pendingSkillImportTargets,
  pendingSkillActionReady,
  availabilityTone,
  kindLabel,
  availabilityLabel,
  permissionLabel,
  ownerScopeLabel,
  skillStateLabel,
  sourceOriginLabel,
  sourceKindLabel,
  executionKindLabel,
  fileTypeLabel,
  isExternalSkillEntry,
  skillDisplayPath,
  beginNewSkill,
  beginNewMcp,
  saveCurrent,
  deleteCurrent,
  toggleDisabled,
  openImportSkillDialog,
  importArchiveSkill,
  importFolderSkill,
  copySelectedSkillToManaged,
  copySelectedMcpToManaged,
  copySelectedSkillsToManaged,
  selectEntry,
  selectSkillFile,
  submitPendingSkillAction,
  suggestSlug,
} = useToolsView()
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    :width="props.embedded ? undefined : 'wide'"
    :test-id="props.embedded ? undefined : 'workspace-tools-view'"
    :data-testid="props.embedded ? 'workspace-tools-embedded' : undefined"
    class="space-y-6"
  >
    <UiPageHeader
      v-if="!props.embedded"
      :eyebrow="t('tools.header.eyebrow')"
      :title="t('sidebar.navigation.tools')"
      :description="t('tools.header.subtitle')"
    />

    <UiStatusCallout
      v-if="catalogStore.error"
      tone="error"
      :description="catalogStore.error"
    />

    <UiListDetailShell class="xl:grid-cols-[minmax(0,1fr)_520px]" list-class="p-3" detail-class="p-3">
      <template #list>
        <ToolsCatalogListPanel
          :active-tab="activeTab"
          :tabs="tabs"
          :search-query="searchQuery"
          :paged-entries="pagedEntries"
          :filtered-entries="filteredEntries"
          :active-tab-count="activeTabCount"
          :page="listPage"
          :page-count="listPageCount"
          :selected-entry-id="draftMode === 'none' ? (selectedEntry?.id ?? '') : ''"
          :selected-external-skill-ids="selectedExternalSkillIds"
          :can-copy-selected-skills-to-managed="canCopySelectedSkillsToManaged"
          :selected-external-skill-count="selectedExternalSkillEntries.length"
          :kind-label="kindLabel"
          :availability-label="availabilityLabel"
          :availability-tone="availabilityTone"
          :permission-label="permissionLabel"
          :owner-scope-label="ownerScopeLabel"
          :skill-state-label="skillStateLabel"
          :source-origin-label="sourceOriginLabel"
          :source-kind-label="sourceKindLabel"
          :execution-kind-label="executionKindLabel"
          :is-external-skill-entry="isExternalSkillEntry"
          @update:active-tab="activeTab = $event"
          @update:search-query="searchQuery = $event"
          @update:page="listPagination.setPage"
          @update:selected-external-skill-ids="selectedExternalSkillIds = $event"
          @select-entry="selectEntry"
          @begin-new-skill="beginNewSkill"
          @open-import-skill-dialog="openImportSkillDialog"
          @copy-selected-skills-to-managed="copySelectedSkillsToManaged"
          @begin-new-mcp="beginNewMcp"
        />
      </template>

      <UiInspectorPanel
        :title="draftMode === 'new-skill'
          ? t('tools.actions.newSkill')
          : draftMode === 'new-mcp'
            ? t('tools.actions.newMcp')
            : (selectedEntry?.name ?? t('tools.detail.title'))"
        :subtitle="draftMode === 'new-skill'
          ? t('tools.editor.skillCreateDescription')
          : draftMode === 'new-mcp'
            ? t('tools.editor.mcpCreateDescription')
            : (selectedEntry?.description ?? t('tools.empty.selectionDescription'))"
      >
        <div
          v-if="draftMode === 'new-skill'"
          class="space-y-5"
        >
          <div class="space-y-1 border-b border-border pb-4">
            <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
              {{ t('tools.detail.title') }}
            </div>
            <p class="text-[13px] leading-6 text-text-secondary">
              {{ t('tools.editor.skillCreateDescription') }}
            </p>
          </div>

          <div class="space-y-4">
            <UiField :label="t('tools.editor.skillSlug')">
              <UiInput v-model="skillSlugDraft" />
            </UiField>

            <UiField :label="t('tools.editor.skillContent')">
              <UiCodeEditor
                language="markdown"
                theme="octopus"
                :model-value="newSkillContentDraft"
                @update:model-value="newSkillContentDraft = $event"
              />
            </UiField>

            <UiStatusCallout v-if="panelError" tone="error" :description="panelError" />

            <div class="flex gap-2">
              <UiButton :loading="submitting" @click="saveCurrent">
                {{ t('common.save') }}
              </UiButton>
            </div>
          </div>
        </div>

        <div
          v-else-if="draftMode === 'new-mcp'"
          class="space-y-5"
        >
          <div class="space-y-1 border-b border-border pb-4">
            <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
              {{ t('tools.detail.title') }}
            </div>
            <p class="text-[13px] leading-6 text-text-secondary">
              {{ t('tools.editor.mcpCreateDescription') }}
            </p>
          </div>

          <div class="space-y-4">
            <UiField :label="t('tools.editor.mcpServerName')">
              <UiInput v-model="mcpServerNameDraft" />
            </UiField>

            <UiField :label="t('tools.editor.mcpConfig')">
              <UiCodeEditor
                language="json"
                theme="octopus"
                :model-value="mcpConfigDraft"
                @update:model-value="mcpConfigDraft = $event"
              />
            </UiField>

            <UiStatusCallout v-if="panelError" tone="error" :description="panelError" />

            <div class="flex gap-2">
              <UiButton :loading="submitting" @click="saveCurrent">
                {{ t('common.save') }}
              </UiButton>
            </div>
          </div>
        </div>

        <div
          v-else-if="selectedEntry"
          data-testid="workspace-tools-detail-document"
          class="space-y-5"
        >

          <SkillDetailPanel
            v-if="selectedSkillEntry"
            :entry="selectedSkillEntry"
            :loading-detail="loadingDetail"
            :loading-skill-file="loadingSkillFile"
            :selected-skill-tree-rows="selectedSkillTreeRows"
            :selected-skill-file-path="selectedSkillFilePath"
            :current-skill-file="currentSkillFile"
            :can-save-skill-file="canSaveSkillFile"
            :can-copy-skill-to-managed="canCopySkillToManaged"
            :skill-file-draft="skillFileDraft"
            :panel-error="panelError"
            :submitting="submitting"
            :deleting="deleting"
            :toggling="toggling"
            :availability-label="availabilityLabel"
            :availability-tone="availabilityTone"
            :owner-scope-label="ownerScopeLabel"
            :skill-state-label="skillStateLabel"
            :source-origin-label="sourceOriginLabel"
            :file-type-label="fileTypeLabel"
            @update:skill-file-draft="skillFileDraft = $event"
            @select-skill-file="selectSkillFile"
            @toggle-disabled="toggleDisabled(selectedSkillEntry, $event)"
            @save="saveCurrent"
            @delete="deleteCurrent"
            @copy-to-managed="copySelectedSkillToManaged"
          />

          <McpDetailPanel
            v-else-if="selectedMcpEntry"
            :entry="selectedMcpEntry"
            :loading-detail="loadingDetail"
            :mcp-server-name-draft="mcpServerNameDraft"
            :mcp-config-draft="mcpConfigDraft"
            :panel-error="panelError"
            :submitting="submitting"
            :deleting="deleting"
            :toggling="toggling"
            :can-copy-mcp-to-managed="canCopyMcpToManaged"
            :availability-label="availabilityLabel"
            :availability-tone="availabilityTone"
            :owner-scope-label="ownerScopeLabel"
            :source-kind-label="sourceKindLabel"
            :execution-kind-label="executionKindLabel"
            @update:mcp-config-draft="mcpConfigDraft = $event"
            @toggle-disabled="toggleDisabled(selectedMcpEntry, $event)"
            @save="saveCurrent"
            @delete="deleteCurrent"
            @copy-to-managed="copySelectedMcpToManaged"
          />

          <div v-else class="space-y-5">
            <div
              data-testid="workspace-tools-detail-meta"
              class="grid gap-3 border-b border-border pb-4 sm:grid-cols-[minmax(0,1fr)_auto]"
            >
              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ kindLabel(selectedEntry.kind) }}
                </div>
                <div class="text-[12px] text-text-secondary">
                  {{ selectedEntry.description }}
                </div>
              </div>

              <div class="flex min-h-10 min-w-[196px] flex-wrap content-start justify-end gap-1.5">
                <UiBadge :label="availabilityLabel(selectedEntry.availability)" :tone="availabilityTone(selectedEntry.availability)" />
                <UiBadge v-if="selectedEntry.disabled" :label="t('tools.states.disabled')" tone="warning" />
                <UiBadge v-if="selectedEntry.requiredPermission" :label="permissionLabel(selectedEntry.requiredPermission)" subtle />
              </div>
            </div>

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

              <div v-if="selectedEntry.ownerScope || selectedEntry.ownerLabel" class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.source') }}
                </div>
                <div class="flex flex-wrap gap-1.5">
                  <UiBadge
                    v-if="selectedEntry.ownerScope"
                    :label="ownerScopeLabel(selectedEntry.ownerScope)"
                    subtle
                  />
                  <UiBadge
                    v-if="selectedEntry.ownerLabel"
                    :label="selectedEntry.ownerLabel"
                    subtle
                  />
                </div>
              </div>

              <div v-if="selectedEntry.consumers?.length" class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.consumers') }}
                </div>
                <div class="flex flex-wrap gap-1.5">
                  <UiBadge
                    v-for="consumer in selectedEntry.consumers"
                    :key="`${selectedEntry.id}-${consumer.kind}-${consumer.id}`"
                    :label="consumer.name"
                    subtle
                  />
                </div>
              </div>

              <div v-if="selectedEntry.requiredPermission" class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.requiredPermission') }}
                </div>
                <div class="text-[13px] text-text-primary">
                  {{ permissionLabel(selectedEntry.requiredPermission) }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.disabled') }}
                </div>
                <UiSwitch
                  :model-value="selectedEntry.disabled"
                  :disabled="toggling || !selectedEntry.management.canDisable"
                  :label="t('tools.actions.disable')"
                  @update:model-value="toggleDisabled(selectedEntry, $event)"
                />
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.builtinKey') }}
                </div>
                <div class="font-mono text-[12px] text-text-secondary">
                  {{ selectedEntry.kind === 'builtin' ? selectedEntry.builtinKey : '' }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <UiEmptyState
          v-else
          :title="t('tools.empty.selectionTitle')"
        :description="t('tools.empty.selectionDescription')"
      />
    </UiInspectorPanel>
    </UiListDetailShell>
  </component>

  <SkillActionDialog
    :open="skillActionDialogOpen"
    :title="pendingSkillActionTitle"
    :description="pendingSkillActionDescription"
    :pending-skill-action="pendingSkillAction"
    :pending-skill-import-source="pendingSkillImportSource"
    :pending-skill-selection-label="pendingSkillSelectionLabel"
    :pending-skill-import-targets="pendingSkillImportTargets"
    :pending-skill-copies="pendingSkillCopies"
    :panel-error="panelError"
    :submitting="submitting"
    :pending-skill-action-ready="pendingSkillActionReady"
    :skill-display-path="skillDisplayPath"
    :suggest-slug="suggestSlug"
    @update:open="skillActionDialogOpen = $event"
    @import-archive-skill="importArchiveSkill"
    @import-folder-skill="importFolderSkill"
    @submit="submitPendingSkillAction"
  />
</template>
