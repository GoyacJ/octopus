<script setup lang="ts">
import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiField, UiInput, UiInspectorPanel, UiListDetailShell, UiPageHeader, UiPageShell, UiRecordCard, UiStatusCallout, UiSwitch } from '@octopus/ui'

import McpDetailPanel from './McpDetailPanel.vue'
import SkillActionDialog from './SkillActionDialog.vue'
import SkillDetailPanel from './SkillDetailPanel.vue'
import ToolsCatalogListPanel from './ToolsCatalogListPanel.vue'
import { useToolsView } from './useToolsView'

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
  selectedEntry,
  selectedSkillEntry,
  selectedMcpEntry,
  selectedExternalSkillEntries,
  selectedSkillTreeRows,
  canSaveSkillFile,
  canCopySkillToManaged,
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
  skillStateLabel,
  sourceOriginLabel,
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
  copySelectedSkillsToManaged,
  selectEntry,
  selectSkillFile,
  submitPendingSkillAction,
  suggestSlug,
} = useToolsView()
</script>

<template>
  <UiPageShell width="wide" test-id="workspace-tools-view">
    <UiPageHeader
      :eyebrow="t('tools.header.eyebrow')"
      :title="t('sidebar.navigation.tools')"
      :description="t('tools.header.subtitle')"
    />

    <UiStatusCallout
      v-if="catalogStore.error"
      tone="error"
      :description="catalogStore.error"
    />

    <UiListDetailShell class="xl:grid-cols-[minmax(0,1fr)_520px]">
      <template #list>
        <ToolsCatalogListPanel
          :active-tab="activeTab"
          :tabs="tabs"
          :search-query="searchQuery"
          :filtered-entries="filteredEntries"
          :active-tab-count="activeTabCount"
          :selected-entry-id="draftMode === 'none' ? (selectedEntry?.id ?? '') : ''"
          :selected-external-skill-ids="selectedExternalSkillIds"
          :can-copy-selected-skills-to-managed="canCopySelectedSkillsToManaged"
          :selected-external-skill-count="selectedExternalSkillEntries.length"
          :kind-label="kindLabel"
          :availability-label="availabilityLabel"
          :availability-tone="availabilityTone"
          :permission-label="permissionLabel"
          :skill-state-label="skillStateLabel"
          :source-origin-label="sourceOriginLabel"
          :is-external-skill-entry="isExternalSkillEntry"
          @update:active-tab="activeTab = $event"
          @update:search-query="searchQuery = $event"
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
        <UiRecordCard
          v-if="draftMode === 'new-skill'"
          :title="t('tools.actions.newSkill')"
          :description="t('tools.editor.skillCreateDescription')"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

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
        </UiRecordCard>

        <UiRecordCard
          v-else-if="draftMode === 'new-mcp'"
          :title="t('tools.actions.newMcp')"
          :description="t('tools.editor.mcpCreateDescription')"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

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
        </UiRecordCard>

        <UiRecordCard
          v-else-if="selectedEntry"
          :title="selectedEntry.name"
          :description="selectedEntry.description"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

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
            :availability-label="availabilityLabel"
            :availability-tone="availabilityTone"
            @update:mcp-config-draft="mcpConfigDraft = $event"
            @toggle-disabled="toggleDisabled(selectedMcpEntry, $event)"
            @save="saveCurrent"
            @delete="deleteCurrent"
          />

          <div v-else class="space-y-4">
            <div class="grid gap-3 border-b border-border/40 pb-4 sm:grid-cols-[minmax(0,1fr)_auto]">
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

            <div class="space-y-3">
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
        </UiRecordCard>

        <UiEmptyState
          v-else
          :title="t('tools.empty.selectionTitle')"
          :description="t('tools.empty.selectionDescription')"
        />
      </UiInspectorPanel>
    </UiListDetailShell>
  </UiPageShell>

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
