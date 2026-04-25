<script setup lang="ts">
import { Trash2 } from 'lucide-vue-next'

import { UiButton, UiDialog, UiPageHeader, UiPageShell, UiTabs } from '@octopus/ui'

import AgentBundleImportDialog from './AgentBundleImportDialog.vue'
import AgentEditorDialog from './AgentEditorDialog.vue'
import AgentListPanel from './AgentListPanel.vue'
import AgentResourceCatalogPanel from './AgentResourceCatalogPanel.vue'
import AgentsStatsStrip from './AgentsStatsStrip.vue'
import TeamEditorDialog from './TeamEditorDialog.vue'
import TeamListPanel from './TeamListPanel.vue'
import AgentInspectorPanel from './AgentInspectorPanel.vue'
import { useAgentCenter } from './useAgentCenter'

const props = defineProps<{
  scope: 'workspace' | 'project'
  embedded?: boolean
}>()

const {
  t,
  workspaceStore,
  activeTab,
  agentViewMode,
  teamViewMode,
  agentQuery,
  teamQuery,
  resourceQuery,
  agentDialogOpen,
  teamDialogOpen,
  deleteConfirmOpen,
  itemToDelete,
  agentImportDialogOpen,
  agentImportPreview,
  agentImportResult,
  agentImportError,
  agentImportLoading,
  agentExportLoading,
  teamExportLoading,
  promoteAgentLoading,
  promoteTeamLoading,
  agentForm,
  teamForm,
  isProjectScope,
  currentAgents,
  resolvedAgents,
  pageTitle,
  pageDescription,
  builtinOptions,
  skillOptions,
  mcpOptions,
  statusOptions,
  teamAgentOptions,
  dialogTeamLeader,
  dialogTeamMembers,
  leaderOptions,
  agentDialogContentReadonly,
  teamDialogContentReadonly,
  agentDialogStatusReadonly,
  teamDialogStatusReadonly,
  canSaveAgentDialog,
  canSaveTeamDialog,
  canCopyCurrentEditingAgent,
  canCopyCurrentEditingTeam,
  currentEditingAgentCopyLabel,
  currentEditingTeamCopyLabel,
  tabs,
  pagedAgents,
  pagedTeams,
  pagedResources,
  agentTotal,
  teamTotal,
  resourceTotal,
  agentPage,
  teamPage,
  resourcePage,
  agentPageCount,
  teamPageCount,
  resourcePageCount,
  agentPagination,
  teamPagination,
  resourcePagination,
  centerStats,
  selectedAgentIds,
  selectedTeamIds,
  allPagedAgentsSelected,
  allPagedTeamsSelected,
  setTab,
  openCreateAgent,
  openAgentImportDialog,
  confirmAgentImport,
  handleAgentImportDialogOpen,
  toggleAllPagedAgents,
  toggleAllPagedTeams,
  exportAgentRecord,
  exportSelectedAgents,
  exportTeamRecord,
  exportSelectedTeams,
  openEditAgent,
  openCreateTeam,
  openEditTeam,
  pickAgentAvatar,
  pickTeamAvatar,
  currentEditingAgent,
  currentEditingTeam,
  agentAvatarPreview,
  teamAvatarPreview,
  clearAgentAvatar,
  clearTeamAvatar,
  saveAgent,
  saveTeam,
  copyCurrentEditingAgent,
  copyCurrentEditingTeam,
  promoteAgentToWorkspace,
  promoteTeamToWorkspace,
  removeAgent,
  removeTeam,
  confirmDelete,
} = useAgentCenter(props.scope)
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    :width="props.embedded ? undefined : 'wide'"
    :test-id="props.embedded ? undefined : 'agent-center-view'"
    :data-testid="props.embedded ? 'agent-center-embedded' : undefined"
    class="h-full flex flex-col bg-transparent"
  >
    <div class="flex-1 flex min-h-0 overflow-hidden">
      <!-- Main Content Area -->
      <div class="flex-1 flex flex-col min-w-0 overflow-y-auto px-6 py-8 scroll-y">
        <UiPageHeader
          v-if="!props.embedded"
          eyebrow="Agent Center"
          :title="pageTitle"
          :description="pageDescription"
          class="mb-8"
        />

        <AgentsStatsStrip :stats="centerStats" class="mb-8" />

        <div
          data-testid="agent-center-tabs-shell"
          class="border-b border-border/50 pb-4 mb-6"
        >
          <UiTabs
            v-model="activeTab"
            :tabs="tabs"
            @update:model-value="setTab"
          />
        </div>

        <div class="flex-1">
          <AgentListPanel
            v-show="activeTab === 'agent'"
            :query="agentQuery"
            :view-mode="agentViewMode"
            :total="agentTotal"
            :page="agentPage"
            :page-count="agentPageCount"
            :paged-agents="pagedAgents"
            :is-project-scope="isProjectScope"
            :import-loading="agentImportLoading && !agentImportDialogOpen"
            :export-loading="agentExportLoading"
            :selected-agent-ids="selectedAgentIds"
            :all-paged-selected="allPagedAgentsSelected"
            @update:query="agentQuery = $event"
            @update:view-mode="agentViewMode = $event"
            @update:page="agentPagination.setPage"
            @update:selected-agent-ids="selectedAgentIds = $event"
            @create-agent="openCreateAgent"
            @open-import-dialog="openAgentImportDialog"
            @toggle-all-paged="toggleAllPagedAgents"
            @export-selected="exportSelectedAgents"
            @export-agent="exportAgentRecord"
            @open-agent="openEditAgent"
            @remove-agent="removeAgent"
          />

          <TeamListPanel
            v-show="activeTab === 'team'"
            :query="teamQuery"
            :view-mode="teamViewMode"
            :total="teamTotal"
            :page="teamPage"
            :page-count="teamPageCount"
            :paged-teams="pagedTeams"
            :resolved-agents="resolvedAgents"
            :is-project-scope="isProjectScope"
            :import-loading="agentImportLoading && !agentImportDialogOpen"
            :export-loading="teamExportLoading"
            :selected-team-ids="selectedTeamIds"
            :all-paged-selected="allPagedTeamsSelected"
            @update:query="teamQuery = $event"
            @update:view-mode="teamViewMode = $event"
            @update:page="teamPagination.setPage"
            @update:selected-team-ids="selectedTeamIds = $event"
            @create-team="openCreateTeam"
            @open-import-dialog="openAgentImportDialog"
            @toggle-all-paged="toggleAllPagedTeams"
            @export-selected="exportSelectedTeams"
            @export-team="exportTeamRecord"
            @open-team="openEditTeam"
            @remove-team="removeTeam"
          />

          <AgentResourceCatalogPanel
            v-show="activeTab === 'builtin' || activeTab === 'skill' || activeTab === 'mcp'"
            :query="resourceQuery"
            :total="resourceTotal"
            :page="resourcePage"
            :page-count="resourcePageCount"
            :paged-entries="pagedResources"
            @update:query="resourceQuery = $event"
            @update:page="resourcePagination.setPage"
          />
        </div>
      </div>

      <!-- Agent Split-View Inspector -->
      <AgentInspectorPanel
        v-if="agentDialogOpen"
        :form="agentForm"
        :status-options="statusOptions"
        :builtin-options="builtinOptions"
        :skill-options="skillOptions"
        :mcp-options="mcpOptions"
        :avatar-preview="agentAvatarPreview(currentEditingAgent())"
        :readonly="agentDialogContentReadonly"
        :loading="agentImportLoading"
        @close="agentDialogOpen = false"
        @save="saveAgent"
      />
    </div>

    <!-- Teams still use Dialog for now as they are more complex layouts -->
    <!-- Teams still use Dialog for now as they are more complex layouts -->
  :open="teamDialogOpen"
  :form="teamForm"
  :status-options="statusOptions"
  :builtin-options="builtinOptions"
  :skill-options="skillOptions"
  :mcp-options="mcpOptions"
  :leader-options="leaderOptions"
  :team-agent-options="teamAgentOptions"
  :avatar-preview="teamAvatarPreview(currentEditingTeam())"
  :dialog-team-leader="dialogTeamLeader"
  :dialog-team-members="dialogTeamMembers"
  :content-readonly="teamDialogContentReadonly"
  :status-readonly="teamDialogStatusReadonly"
  :can-save="canSaveTeamDialog"
  :can-copy="canCopyCurrentEditingTeam"
  :copy-label="currentEditingTeamCopyLabel"
  :can-promote="Boolean(isProjectScope && currentEditingTeam()?.projectId)"
  :promoting="promoteTeamLoading"
  @update:open="teamDialogOpen = $event"
  @pick-avatar="pickTeamAvatar"
  @remove-avatar="clearTeamAvatar"
  @save="saveTeam"
  @copy="copyCurrentEditingTeam"
  @promote="promoteTeamToWorkspace"
/>

    <UiDialog
      :open="deleteConfirmOpen"
      title="确认删除"
      :description="`您确定要删除「${itemToDelete?.name}」吗？此操作无法撤销。`"
      content-class="max-w-md"
      @update:open="deleteConfirmOpen = $event"
    >
      <div class="py-4 text-sm text-text-secondary">
        删除此项将永久移除相关配置及协作关系，且无法恢复。
      </div>
      <template #footer>
        <div class="flex w-full items-center justify-end gap-2">
          <UiButton variant="ghost" @click="deleteConfirmOpen = false">取消</UiButton>
          <UiButton variant="outline" class="border-error/20 text-error hover:bg-error/10 hover:border-error/40" @click="confirmDelete">
            <Trash2 :size="14" />
            确认删除
          </UiButton>
        </div>
      </template>
    </UiDialog>

    <AgentBundleImportDialog
      :open="agentImportDialogOpen"
      :preview="agentImportPreview"
      :result="agentImportResult"
      :loading="agentImportLoading"
      :error-message="agentImportError"
      @update:open="handleAgentImportDialogOpen"
      @confirm="confirmAgentImport"
    />
  </component>
</template>
