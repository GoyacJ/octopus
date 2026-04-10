<script setup lang="ts">
import { Trash2 } from 'lucide-vue-next'

import { UiButton, UiDialog, UiPageHeader, UiPageShell, UiSurface, UiTabs } from '@octopus/ui'

import AgentBundleImportDialog from './AgentBundleImportDialog.vue'
import AgentEditorDialog from './AgentEditorDialog.vue'
import AgentListPanel from './AgentListPanel.vue'
import AgentsStatsStrip from './AgentsStatsStrip.vue'
import TeamEditorDialog from './TeamEditorDialog.vue'
import TeamListPanel from './TeamListPanel.vue'
import { useAgentCenter } from './useAgentCenter'

const props = defineProps<{
  scope: 'workspace' | 'project'
}>()

const {
  t,
  workspaceStore,
  activeTab,
  agentViewMode,
  teamViewMode,
  agentQuery,
  teamQuery,
  agentDialogOpen,
  teamDialogOpen,
  deleteConfirmOpen,
  itemToDelete,
  agentImportDialogOpen,
  agentImportPreview,
  agentImportResult,
  agentImportError,
  agentImportLoading,
  agentForm,
  teamForm,
  isProjectScope,
  currentAgents,
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
  tabs,
  pagedAgents,
  pagedTeams,
  agentTotal,
  teamTotal,
  agentPage,
  teamPage,
  agentPageCount,
  teamPageCount,
  agentPagination,
  teamPagination,
  centerStats,
  setTab,
  openCreateAgent,
  openAgentImportDialog,
  confirmAgentImport,
  handleAgentImportDialogOpen,
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
  removeAgent,
  removeTeam,
  confirmDelete,
} = useAgentCenter(props.scope)
</script>

<template>
  <UiPageShell width="wide" test-id="agent-center-view">
    <UiPageHeader
      eyebrow="Agent Center"
      :title="pageTitle"
      :description="pageDescription"
    />

    <AgentsStatsStrip :stats="centerStats" />

    <UiSurface data-testid="agent-center-tabs-shell" variant="subtle" padding="sm">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        @update:model-value="setTab"
      />
    </UiSurface>

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
      @update:query="agentQuery = $event"
      @update:view-mode="agentViewMode = $event"
      @update:page="agentPagination.setPage"
      @create-agent="openCreateAgent"
      @open-import-dialog="openAgentImportDialog"
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
      :current-agents="currentAgents"
      @update:query="teamQuery = $event"
      @update:view-mode="teamViewMode = $event"
      @update:page="teamPagination.setPage"
      @create-team="openCreateTeam"
      @open-team="openEditTeam"
      @remove-team="removeTeam"
    />

    <AgentEditorDialog
      :open="agentDialogOpen"
      :form="agentForm"
      :status-options="statusOptions"
      :builtin-options="builtinOptions"
      :skill-options="skillOptions"
      :mcp-options="mcpOptions"
      :avatar-preview="agentAvatarPreview(currentEditingAgent())"
      :scope="props.scope"
      @update:open="agentDialogOpen = $event"
      @pick-avatar="pickAgentAvatar"
      @remove-avatar="clearAgentAvatar"
      @save="saveAgent"
    />

    <TeamEditorDialog
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
      @update:open="teamDialogOpen = $event"
      @pick-avatar="pickTeamAvatar"
      @remove-avatar="clearTeamAvatar"
      @save="saveTeam"
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
  </UiPageShell>
</template>
