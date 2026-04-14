<script setup lang="ts">
import { UiBadge, UiEmptyState, UiInspectorPanel, UiListDetailShell, UiPageHeader, UiPageShell, UiTabs } from '@octopus/ui'

import ProjectActorsPanel from './ProjectActorsPanel.vue'
import ProjectBasicsPanel from './ProjectBasicsPanel.vue'
import ProjectMembersPanel from './ProjectMembersPanel.vue'
import ProjectModelsPanel from './ProjectModelsPanel.vue'
import ProjectToolsPanel from './ProjectToolsPanel.vue'
import { useProjectSettings } from './useProjectSettings'

const {
  t,
  workspaceStore,
  activeTab,
  basicsForm,
  modelsForm,
  enabledAgentIds,
  enabledTeamIds,
  selectedMemberUserIds,
  tabs,
  project,
  allowedWorkspaceConfiguredModels,
  actorCandidateAgents,
  actorCandidateTeams,
  projectOwnedAgents,
  projectOwnedTeams,
  workspaceAssignedAgents,
  workspaceAssignedTeams,
  workspaceUsers,
  modelTabReady,
  viewReady,
  toolSections,
  summaryAllowedModels,
  projectUsedTokens,
  summaryOverrideCount,
  summaryActorCount,
  summaryMemberCount,
  toolPermissionOptions,
  savingBasics,
  savingModels,
  savingTools,
  savingAgents,
  savingUsers,
  basicsError,
  modelsError,
  toolsError,
  agentsError,
  usersError,
  statusLabel,
  badgeTone,
  resolveToolSelection,
  toolPermissionSummaryLabel,
  updateToolPermission,
  resetBasics,
  resetModels,
  resetTools,
  resetAgents,
  resetUsers,
  submitBasics,
  saveModels,
  saveTools,
  saveAgents,
  saveUsers,
} = useProjectSettings()
</script>

<template>
  <UiPageShell
    v-if="viewReady"
    width="wide"
    test-id="project-settings-view"
  >
    <UiPageHeader
      :eyebrow="t('projectSettings.header.eyebrow')"
      :title="project?.name ?? t('projectSettings.header.title')"
      :description="project?.description || t('projectSettings.header.subtitle')"
    >
      <template #meta>
        <UiBadge v-if="project" :label="statusLabel" :tone="badgeTone(project.status)" />
      </template>
      <template #actions>
        <div class="w-full max-w-2xl md:w-auto">
          <UiTabs v-model="activeTab" :tabs="tabs" />
        </div>
      </template>
    </UiPageHeader>

    <UiEmptyState
      v-if="!project"
      :title="t('projectSettings.emptyTitle')"
      :description="workspaceStore.error || t('projectSettings.emptyDescription')"
    />

    <template v-else>
      <UiListDetailShell class="xl:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
        <template #list>
          <ProjectBasicsPanel
            v-if="activeTab === 'basics'"
            :basics-form="basicsForm"
            :status-label="statusLabel"
            :badge-tone="badgeTone(project.status)"
            :basics-error="basicsError"
            :saving-basics="savingBasics"
            @reset="resetBasics"
            @save="submitBasics"
          />

          <ProjectModelsPanel
            v-else-if="activeTab === 'models'"
            :model-tab-ready="modelTabReady"
            :allowed-workspace-configured-models="allowedWorkspaceConfiguredModels"
            :models-form="modelsForm"
            :project-used-tokens="projectUsedTokens"
            :models-error="modelsError"
            :saving-models="savingModels"
            @reset="resetModels"
            @save="saveModels"
          />

          <ProjectToolsPanel
            v-else-if="activeTab === 'tools'"
            :tool-sections="toolSections"
            :tool-permission-options="toolPermissionOptions"
            :tools-error="toolsError"
            :saving-tools="savingTools"
            :resolve-tool-selection="resolveToolSelection"
            :tool-permission-summary-label="toolPermissionSummaryLabel"
            @reset="resetTools"
            @save="saveTools"
            @update-tool-permission="updateToolPermission"
          />

          <ProjectActorsPanel
            v-else-if="activeTab === 'agents'"
            :candidate-agents="actorCandidateAgents"
            :candidate-teams="actorCandidateTeams"
            :project-owned-agents="projectOwnedAgents"
            :project-owned-teams="projectOwnedTeams"
            :workspace-assigned-agents="workspaceAssignedAgents"
            :workspace-assigned-teams="workspaceAssignedTeams"
            :enabled-agent-ids="enabledAgentIds"
            :enabled-team-ids="enabledTeamIds"
            :agents-error="agentsError"
            :saving-agents="savingAgents"
            @reset="resetAgents"
            @save="saveAgents"
            @update:enabled-agent-ids="enabledAgentIds = $event"
            @update:enabled-team-ids="enabledTeamIds = $event"
          />

          <ProjectMembersPanel
            v-else
            :workspace-users="workspaceUsers"
            :selected-member-user-ids="selectedMemberUserIds"
            :users-error="usersError"
            :saving-users="savingUsers"
            @reset="resetUsers"
            @save="saveUsers"
            @update:selected-member-user-ids="selectedMemberUserIds = $event"
          />
        </template>

        <UiInspectorPanel
          :title="t('projectSettings.summary.title')"
          :subtitle="t('projectSettings.summary.description')"
        >
          <div class="space-y-4 text-sm text-text-secondary">
            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projects.fields.name') }}
              </div>
              <div class="text-text-primary">
                {{ project.name }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projects.fields.description') }}
              </div>
              <div class="whitespace-pre-wrap leading-6 text-text-primary">
                {{ project.description || t('common.na') }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projectSettings.summary.status') }}
              </div>
              <UiBadge :label="statusLabel" :tone="badgeTone(project.status)" />
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projectSettings.summary.models') }}
              </div>
              <div class="text-text-primary">
                {{ summaryAllowedModels.length }}
              </div>
              <div class="text-xs text-text-secondary">
                {{ summaryAllowedModels.map(model => model.label).join(' / ') || t('common.na') }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projectSettings.summary.toolOverrides') }}
              </div>
              <div class="text-text-primary">
                {{ summaryOverrideCount }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projectSettings.summary.actors') }}
              </div>
              <div class="text-text-primary">
                {{ summaryActorCount }}
              </div>
            </div>

            <div class="space-y-1">
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t('projectSettings.summary.members') }}
              </div>
              <div class="text-text-primary">
                {{ summaryMemberCount }}
              </div>
            </div>
          </div>
        </UiInspectorPanel>
      </UiListDetailShell>
    </template>
  </UiPageShell>
</template>
