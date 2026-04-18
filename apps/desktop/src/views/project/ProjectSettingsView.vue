<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInfoCard,
  UiInput,
  UiInspectorPanel,
  UiPageHeader,
  UiPageShell,
  UiSelect,
  UiStatusCallout,
  UiTabs,
} from '@octopus/ui'

import { createWorkspaceConsoleSurfaceTarget } from '@/i18n/navigation'

import { useProjectSettings } from './useProjectSettings'

const {
  t,
  workspaceStore,
  project,
  leaderDraft,
  leaderOptions,
  currentLeaderLabel,
  toolTabs,
  actorTabs,
  grantToolTab,
  runtimeToolTab,
  grantActorTab,
  runtimeActorTab,
  grantToolSearchQuery,
  runtimeToolSearchQuery,
  grantActorSearchQuery,
  runtimeActorSearchQuery,
  dialogOpen,
  dialogErrors,
  saving,
  grantForm,
  runtimeForm,
  memberDraft,
  workspaceConfiguredModels,
  workspaceToolEntries,
  grantedConfiguredModels,
  grantedToolEntries,
  filteredGrantToolEntries,
  filteredRuntimeToolEntries,
  filteredGrantAgents,
  filteredGrantTeams,
  filteredRuntimeAgents,
  filteredRuntimeTeams,
  workspaceActiveAgents,
  workspaceActiveTeams,
  grantedAgents,
  grantedTeams,
  projectOwnedAgents,
  projectOwnedTeams,
  grantedProjectOwnedTools,
  workspaceUsers,
  toolPermissionOptions,
  grantSummary,
  runtimeSummary,
  memberSummary,
  accessSummary,
  completionItems,
  completionProgress,
  projectUsedTokens,
  viewReady,
  statusLabel,
  badgeTone,
  toolOriginBadge,
  actorOriginBadge,
  isLeaderAgent,
  isProjectOwnedAgentRecord,
  isProjectOwnedTeamRecord,
  isGrantToolEnabled,
  setGrantToolEnabled,
  isRuntimeToolEnabled,
  setRuntimeToolEnabled,
  isGrantAgentEnabled,
  isGrantTeamEnabled,
  isRuntimeAgentEnabled,
  isRuntimeTeamEnabled,
  setGrantAgentEnabled,
  setGrantTeamEnabled,
  setRuntimeAgentEnabled,
  setRuntimeTeamEnabled,
  openLeaderDialog,
  openGrantModelsDialog,
  openGrantToolsDialog,
  openGrantActorsDialog,
  selectAllGrantModels,
  clearGrantModels,
  selectAllGrantTools,
  clearGrantTools,
  selectAllGrantActors,
  clearGrantActors,
  openRuntimeModelsDialog,
  openRuntimeToolsDialog,
  openRuntimeActorsDialog,
  selectAllRuntimeTools,
  clearAllRuntimeTools,
  selectAllRuntimeActors,
  clearAllRuntimeActors,
  openMembersDialog,
  resolveRuntimeToolSelection,
  runtimeToolPermissionSummaryLabel,
  updateRuntimeToolPermission,
  saveLeader,
  saveGrantModels,
  saveGrantTools,
  saveGrantActors,
  saveRuntimeModels,
  saveRuntimeTools,
  saveRuntimeActors,
  saveMembers,
} = useProjectSettings()

const projectManagementTarget = computed(() =>
  workspaceStore.currentWorkspaceId
    ? createWorkspaceConsoleSurfaceTarget('workspace-console-projects', workspaceStore.currentWorkspaceId)
    : null,
)

const summaryRowButtonClass = 'h-auto w-full justify-between whitespace-normal rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left text-text-primary transition-colors hover:border-border-strong hover:bg-surface-muted'
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
        <RouterLink v-if="projectManagementTarget" :to="projectManagementTarget">
          <UiButton variant="ghost">
            {{ t('projectSettings.actions.openProjects') }}
          </UiButton>
        </RouterLink>
      </template>
    </UiPageHeader>

    <UiEmptyState
      v-if="!project"
      :title="t('projectSettings.emptyTitle')"
      :description="workspaceStore.error || t('projectSettings.emptyDescription')"
    />

    <template v-else>
      <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <div class="space-y-4">
          <section
            data-testid="project-settings-overview-section"
            class="rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5"
          >
            <div class="space-y-1">
              <div class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
                {{ t('projectSettings.sections.overview.title') }}
              </div>
              <div class="text-sm leading-6 text-text-secondary">
                {{ t('projectSettings.sections.overview.description') }}
              </div>
            </div>

            <div class="mt-4 grid gap-3 md:grid-cols-2">
              <UiInfoCard :label="t('projects.fields.name')" :title="project.name" />
              <UiInfoCard :label="t('projects.fields.resourceDirectory')" :title="project.resourceDirectory" />
              <UiInfoCard :label="t('projects.fields.description')" :title="project.description || t('common.na')" />
              <UiInfoCard :label="t('projectSettings.summary.status')" :title="statusLabel" />
              <UiInfoCard
                :label="t('projects.fields.leader')"
                :title="currentLeaderLabel"
              />
            </div>

            <div class="mt-4 flex flex-wrap items-center justify-between gap-3 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-sm leading-6 text-text-secondary">
              <div class="max-w-[40rem]">
                {{ t('projectSettings.sections.overview.editHint') }}
              </div>
              <UiButton
                variant="ghost"
                data-testid="project-settings-open-leader-dialog"
                @click="openLeaderDialog"
              >
                {{ t('projectSettings.leader.editAction') }}
              </UiButton>
            </div>
          </section>

          <section
            data-testid="project-settings-grants-section"
            class="rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5"
          >
            <div class="space-y-1">
              <div class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
                {{ t('projectSettings.sections.grants.title') }}
              </div>
              <div class="text-sm leading-6 text-text-secondary">
                {{ t('projectSettings.sections.grants.description') }}
              </div>
            </div>

            <div class="mt-4 space-y-3">
              <UiButton
                variant="outline"
                data-testid="project-settings-open-grants-models"
                :class="summaryRowButtonClass"
                @click="openGrantModelsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.grants.modelsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.workspaceGrant') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ grantSummary.models }}
                </div>
              </UiButton>

              <UiButton
                variant="outline"
                data-testid="project-settings-open-grants-tools"
                :class="summaryRowButtonClass"
                @click="openGrantToolsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.grants.toolsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.workspaceGrant') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ grantSummary.tools }}
                </div>
              </UiButton>

              <UiButton
                variant="outline"
                data-testid="project-settings-open-grants-actors"
                :class="summaryRowButtonClass"
                @click="openGrantActorsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.grants.actorsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.workspaceGrant') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ grantSummary.actors }}
                </div>
              </UiButton>
            </div>
          </section>

          <section
            data-testid="project-settings-runtime-section"
            class="rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5"
          >
            <div class="space-y-1">
              <div class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
                {{ t('projectSettings.sections.runtime.title') }}
              </div>
              <div class="text-sm leading-6 text-text-secondary">
                {{ t('projectSettings.sections.runtime.description') }}
              </div>
            </div>

            <div class="mt-4 space-y-3">
              <UiButton
                variant="outline"
                data-testid="project-settings-open-runtime-models"
                :class="summaryRowButtonClass"
                @click="openRuntimeModelsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.runtime.modelsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.projectEnablement') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ runtimeSummary.models }}
                </div>
              </UiButton>

              <UiButton
                variant="outline"
                data-testid="project-settings-open-runtime-tools"
                :class="summaryRowButtonClass"
                @click="openRuntimeToolsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.runtime.toolsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.projectOverride') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ runtimeSummary.tools }}
                </div>
              </UiButton>

              <UiButton
                variant="outline"
                data-testid="project-settings-open-runtime-actors"
                :class="summaryRowButtonClass"
                @click="openRuntimeActorsDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.runtime.actorsTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.projectEnablement') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ runtimeSummary.actors }}
                </div>
              </UiButton>
            </div>
          </section>

          <section
            data-testid="project-settings-members-section"
            class="rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5"
          >
            <div class="space-y-1">
              <div class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
                {{ t('projectSettings.sections.members.title') }}
              </div>
              <div class="text-sm leading-6 text-text-secondary">
                {{ t('projectSettings.sections.members.description') }}
              </div>
            </div>

            <div class="mt-4 space-y-3">
              <UiButton
                variant="outline"
                data-testid="project-settings-open-members"
                :class="summaryRowButtonClass"
                @click="openMembersDialog"
              >
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ t('projectSettings.sections.members.membersTitle') }}
                  </div>
                  <div class="text-xs text-text-tertiary">
                    {{ t('projectSettings.labels.members') }}
                  </div>
                </div>
                <div class="max-w-[28rem] text-sm leading-6 text-text-secondary">
                  {{ memberSummary }}
                </div>
              </UiButton>

              <div class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3">
                <div class="flex items-start justify-between gap-4">
                  <div class="space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ t('projectSettings.sections.members.accessTitle') }}
                    </div>
                    <div class="text-xs text-text-tertiary">
                      {{ t('projectSettings.sections.members.accessHint') }}
                    </div>
                  </div>
                  <div class="max-w-[24rem] text-right text-sm leading-6 text-text-secondary">
                    {{ accessSummary }}
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>

        <div class="xl:sticky xl:top-4 xl:self-start">
          <UiInspectorPanel
            :title="t('projectSettings.summary.title')"
            :subtitle="t('projectSettings.summary.description')"
          >
            <div class="space-y-4">
              <div class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3">
                <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
                  {{ t('projectSettings.summary.completion') }}
                </div>
                <div class="mt-1 text-2xl font-bold tracking-[-0.02em] text-text-primary">
                  {{ completionProgress.percent }}%
                </div>
                <div class="mt-1 text-sm text-text-secondary">
                  {{ t('projectSettings.summary.completionValue', { completed: completionProgress.completed, total: completionProgress.total }) }}
                </div>
              </div>

              <div class="space-y-2">
                <div
                  v-for="item in completionItems"
                  :key="item.id"
                  class="flex items-center justify-between gap-3 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-2"
                >
                  <span class="text-sm text-text-primary">{{ item.label }}</span>
                  <UiBadge :label="item.done ? t('common.done') : t('common.pending')" :tone="item.done ? 'success' : 'default'" />
                </div>
              </div>

              <div class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 text-sm leading-6 text-text-secondary">
                <div class="font-semibold text-text-primary">
                  {{ t('projectSettings.summary.nextTitle') }}
                </div>
                <div class="mt-1">
                  {{ completionItems.find(item => !item.done)?.label || t('projectSettings.summary.nextDone') }}
                </div>
              </div>
            </div>
          </UiInspectorPanel>
        </div>
      </div>

      <UiDialog
        v-model:open="dialogOpen.leader"
        :title="t('projectSettings.leader.dialogTitle')"
        :description="t('projectSettings.leader.dialogDescription')"
        content-test-id="project-settings-leader-dialog"
      >
        <div class="space-y-4">
          <UiField
            :label="t('projects.fields.leader')"
            :hint="t('projectSettings.leader.hint')"
          >
            <UiSelect
              v-model="leaderDraft"
              data-testid="project-settings-leader-select"
              :options="leaderOptions"
              :placeholder="t('projectSettings.leader.selectPlaceholder')"
            />
          </UiField>

          <UiStatusCallout
            v-if="dialogErrors.leader"
            tone="error"
            :description="dialogErrors.leader"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.leader = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-leader-save-button"
            :disabled="saving.leader || !leaderDraft"
            @click="saveLeader"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.grantModels"
        :title="t('projectSettings.dialogs.grantModels.title')"
        :description="t('projectSettings.dialogs.grantModels.description')"
        content-test-id="project-settings-grants-models-dialog"
      >
        <div class="space-y-4">
          <UiField
            :label="t('projectSettings.sections.grants.modelsTitle')"
            :hint="t('projectSettings.dialogs.grantModels.hint')"
          >
            <div v-if="workspaceConfiguredModels.length" class="space-y-3">
              <div class="flex items-center justify-end gap-2">
                <UiButton
                  type="button"
                  variant="ghost"
                  size="sm"
                  data-testid="project-settings-grants-models-select-all"
                  @click="selectAllGrantModels"
                >
                  {{ t('common.selectAll') }}
                </UiButton>
                <UiButton
                  type="button"
                  variant="ghost"
                  size="sm"
                  data-testid="project-settings-grants-models-clear-all"
                  @click="clearGrantModels"
                >
                  {{ t('common.clearAll') }}
                </UiButton>
              </div>
              <label
                v-for="modelOption in workspaceConfiguredModels"
                :key="modelOption.value"
                :data-testid="`project-grant-model-option-${modelOption.value}`"
                class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="min-w-0 space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ modelOption.label }}
                  </div>
                  <div class="text-xs text-text-secondary">
                    {{ modelOption.providerLabel }} · {{ modelOption.modelLabel }}
                  </div>
                </div>
                <UiCheckbox
                  v-model="grantForm.assignedConfiguredModelIds"
                  :value="modelOption.value"
                  :aria-label="modelOption.label"
                />
              </label>
            </div>
            <UiEmptyState
              v-else
              :title="t('projectSettings.models.emptyTitle')"
              :description="t('projectSettings.models.emptyDescription')"
            />
          </UiField>

          <UiField
            :label="t('projects.fields.defaultModel')"
            :hint="t('projectSettings.dialogs.grantModels.defaultHint')"
          >
            <UiSelect
              v-model="grantForm.defaultConfiguredModelId"
              data-testid="project-grant-default-model-select"
              :disabled="!grantForm.assignedConfiguredModelIds.length"
              :options="workspaceConfiguredModels
                .filter(option => grantForm.assignedConfiguredModelIds.includes(option.value))
                .map(option => ({
                  value: option.value,
                  label: `${option.label} · ${option.providerLabel}`,
                }))"
            />
          </UiField>

          <UiStatusCallout
            v-if="dialogErrors.grantModels"
            tone="error"
            :description="dialogErrors.grantModels"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.grantModels = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-grants-models-save-button"
            :disabled="saving.grantModels"
            @click="saveGrantModels"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.grantTools"
        :title="t('projectSettings.dialogs.grantTools.title')"
        :description="t('projectSettings.dialogs.grantTools.description')"
        content-test-id="project-settings-grants-tools-dialog"
      >
        <div class="space-y-4">
          <UiEmptyState
            v-if="!workspaceToolEntries.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <div v-else class="space-y-3">
            <UiTabs
              v-model="grantToolTab"
              :tabs="toolTabs"
              data-testid="project-settings-grants-tools-tabs"
            />

            <div class="flex flex-wrap items-center gap-3">
              <div class="min-w-[16rem] flex-1">
                <UiInput
                  v-model="grantToolSearchQuery"
                  data-testid="project-settings-grants-tools-search"
                  :placeholder="t('projectSettings.search.tools')"
                />
              </div>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                data-testid="project-settings-grants-tools-select-all"
                @click="selectAllGrantTools"
              >
                {{ t('common.selectAll') }}
              </UiButton>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                data-testid="project-settings-grants-tools-clear-all"
                @click="clearGrantTools"
              >
                {{ t('common.clearAll') }}
              </UiButton>
            </div>

            <div class="space-y-3">
              <div
                v-for="entry in filteredGrantToolEntries"
                :key="entry.sourceKey"
                :data-testid="`project-grant-tool-option-${entry.sourceKey}`"
                class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="min-w-0 space-y-1">
                  <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                    <span>{{ entry.name }}</span>
                    <UiBadge :label="toolOriginBadge(entry)" subtle />
                  </div>
                  <div class="text-xs text-text-secondary">
                    {{ entry.description || entry.sourceKey }}
                  </div>
                </div>
                <UiCheckbox
                  :model-value="isGrantToolEnabled(entry.sourceKey)"
                  :aria-label="entry.name"
                  @update:model-value="setGrantToolEnabled(entry.sourceKey, Boolean($event))"
                />
              </div>

              <UiEmptyState
                v-if="!filteredGrantToolEntries.length"
                :title="t('projectSettings.search.emptyTitle')"
                :description="t('projectSettings.search.emptyDescription')"
              />

              <div
                v-if="grantedProjectOwnedTools.length"
                class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-sm leading-6 text-text-secondary"
              >
                <div class="font-semibold text-text-primary">
                  {{ t('projectSettings.labels.projectOwned') }}
                </div>
                <div class="mt-1">
                  {{ t('projectSettings.tools.projectOwnedHint') }}
                </div>
              </div>
            </div>
          </div>

          <UiStatusCallout
            v-if="dialogErrors.grantTools"
            tone="error"
            :description="dialogErrors.grantTools"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.grantTools = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-grants-tools-save-button"
            :disabled="saving.grantTools"
            @click="saveGrantTools"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.grantActors"
        :title="t('projectSettings.dialogs.grantActors.title')"
        :description="t('projectSettings.dialogs.grantActors.description')"
        content-test-id="project-settings-grants-actors-dialog"
      >
        <div class="space-y-4">
          <UiTabs
            v-model="grantActorTab"
            :tabs="actorTabs"
            data-testid="project-settings-grants-actors-tabs"
          />

          <div class="flex flex-wrap items-center gap-3">
            <div class="min-w-[16rem] flex-1">
              <UiInput
                v-model="grantActorSearchQuery"
                data-testid="project-settings-grants-actors-search"
                :placeholder="t('projectSettings.search.actors')"
              />
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="grantActorTab === 'agents' ? 'project-settings-grants-agents-select-all' : 'project-settings-grants-teams-select-all'"
              @click="selectAllGrantActors"
            >
              {{ t('common.selectAll') }}
            </UiButton>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="grantActorTab === 'agents' ? 'project-settings-grants-agents-clear-all' : 'project-settings-grants-teams-clear-all'"
              @click="clearGrantActors"
            >
              {{ t('common.clearAll') }}
            </UiButton>
          </div>

          <section v-if="grantActorTab === 'agents'" class="space-y-3">
            <div
              v-for="agent in filteredGrantAgents"
              :key="agent.id"
              :data-testid="`project-grant-agent-option-${agent.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                  <span>{{ agent.name }}</span>
                  <UiBadge :label="actorOriginBadge(agent)" subtle />
                  <UiBadge v-if="isLeaderAgent(agent.id)" :label="t('projects.fields.leader')" tone="info" />
                </div>
                <div class="text-xs text-text-secondary">
                  {{ agent.description || t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                :model-value="isGrantAgentEnabled(agent.id)"
                :aria-label="agent.name"
                @update:model-value="setGrantAgentEnabled(agent.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!filteredGrantAgents.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <section v-else class="space-y-3">
            <div
              v-for="team in filteredGrantTeams"
              :key="team.id"
              :data-testid="`project-grant-team-option-${team.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                  <span>{{ team.name }}</span>
                  <UiBadge :label="actorOriginBadge(team)" subtle />
                </div>
                <div class="text-xs text-text-secondary">
                  {{ team.description || t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                :model-value="isGrantTeamEnabled(team.id)"
                :aria-label="team.name"
                @update:model-value="setGrantTeamEnabled(team.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!filteredGrantTeams.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <UiStatusCallout
            v-if="dialogErrors.grantActors"
            tone="error"
            :description="dialogErrors.grantActors"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.grantActors = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-grants-actors-save-button"
            :disabled="saving.grantActors"
            @click="saveGrantActors"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.runtimeModels"
        :title="t('projectSettings.dialogs.runtimeModels.title')"
        :description="t('projectSettings.dialogs.runtimeModels.description')"
        content-test-id="project-settings-runtime-models-dialog"
      >
        <div class="space-y-4">
          <UiField
            :label="t('projectSettings.models.allowedLabel')"
            :hint="t('projectSettings.dialogs.runtimeModels.hint')"
          >
            <div v-if="grantedConfiguredModels.length" class="space-y-3">
              <label
                v-for="modelOption in grantedConfiguredModels"
                :key="modelOption.value"
                :data-testid="`project-runtime-model-option-${modelOption.value}`"
                class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="min-w-0 space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ modelOption.label }}
                  </div>
                  <div class="text-xs text-text-secondary">
                    {{ modelOption.providerLabel }} · {{ modelOption.modelLabel }}
                  </div>
                </div>
                <UiCheckbox
                  v-model="runtimeForm.allowedConfiguredModelIds"
                  :value="modelOption.value"
                  :aria-label="modelOption.label"
                />
              </label>
            </div>
            <UiEmptyState
              v-else
              :title="t('projectSettings.models.emptyTitle')"
              :description="t('projectSettings.models.emptyDescription')"
            />
          </UiField>

          <UiField
            :label="t('projectSettings.models.defaultLabel')"
            :hint="t('projectSettings.models.defaultHint')"
          >
            <UiSelect
              v-model="runtimeForm.defaultConfiguredModelId"
              data-testid="project-runtime-default-model-select"
              :disabled="!runtimeForm.allowedConfiguredModelIds.length"
              :options="grantedConfiguredModels
                .filter(option => runtimeForm.allowedConfiguredModelIds.includes(option.value))
                .map(option => ({
                  value: option.value,
                  label: `${option.label} · ${option.providerLabel}`,
                }))"
            />
          </UiField>

          <div class="grid gap-4 md:grid-cols-2">
            <UiField
              :label="t('projectSettings.models.totalTokensLabel')"
              :hint="t('projectSettings.models.totalTokensHint')"
            >
              <UiInput
                v-model="runtimeForm.totalTokens"
                data-testid="project-runtime-total-tokens-input"
                type="number"
                :placeholder="t('projectSettings.models.totalTokensPlaceholder')"
              />
            </UiField>

            <UiField
              :label="t('projectSettings.models.usedTokensLabel')"
              :hint="t('projectSettings.models.usedTokensHint')"
            >
              <div class="flex min-h-8 items-center rounded-[var(--radius-s)] border border-border bg-surface-muted px-3 text-sm text-text-primary">
                {{ projectUsedTokens.toLocaleString() }}
              </div>
            </UiField>
          </div>

          <UiStatusCallout
            v-if="dialogErrors.runtimeModels"
            tone="error"
            :description="dialogErrors.runtimeModels"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.runtimeModels = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-runtime-models-save-button"
            :disabled="saving.runtimeModels"
            @click="saveRuntimeModels"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.runtimeTools"
        :title="t('projectSettings.dialogs.runtimeTools.title')"
        :description="t('projectSettings.dialogs.runtimeTools.description')"
        content-test-id="project-settings-runtime-tools-dialog"
      >
        <div class="space-y-4">
          <UiEmptyState
            v-if="!grantedToolEntries.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <div v-else class="space-y-3">
            <UiTabs
              v-model="runtimeToolTab"
              :tabs="toolTabs"
              data-testid="project-settings-runtime-tools-tabs"
            />

            <div class="flex flex-wrap items-center gap-3">
              <div class="min-w-[16rem] flex-1">
                <UiInput
                  v-model="runtimeToolSearchQuery"
                  data-testid="project-settings-runtime-tools-search"
                  :placeholder="t('projectSettings.search.tools')"
                />
              </div>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                data-testid="project-settings-runtime-tools-select-all"
                @click="selectAllRuntimeTools"
              >
                {{ t('common.selectAll') }}
              </UiButton>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                data-testid="project-settings-runtime-tools-clear-all"
                @click="clearAllRuntimeTools"
              >
                {{ t('common.clearAll') }}
              </UiButton>
            </div>

            <div class="space-y-3">
              <div
                v-for="entry in filteredRuntimeToolEntries"
                :key="entry.sourceKey"
                :data-testid="`project-runtime-tool-option-${entry.sourceKey}`"
                class="space-y-3 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="flex items-start justify-between gap-4">
                  <div class="min-w-0 space-y-1">
                    <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                      <span>{{ entry.name }}</span>
                      <UiBadge :label="toolOriginBadge(entry)" subtle />
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ entry.description || entry.sourceKey }}
                    </div>
                  </div>
                  <UiCheckbox
                    :model-value="isRuntimeToolEnabled(entry.sourceKey)"
                    :aria-label="entry.name"
                    @update:model-value="setRuntimeToolEnabled(entry.sourceKey, Boolean($event))"
                  />
                </div>

                <div class="grid gap-2 md:grid-cols-[minmax(0,16rem)_1fr] md:items-center">
                  <UiSelect
                    :model-value="resolveRuntimeToolSelection(entry.sourceKey)"
                    :options="toolPermissionOptions"
                    @update:model-value="updateRuntimeToolPermission(entry.sourceKey, $event)"
                  />
                  <div class="text-xs text-text-tertiary">
                    {{ runtimeToolPermissionSummaryLabel(entry) }}
                  </div>
                </div>
              </div>

              <UiEmptyState
                v-if="!filteredRuntimeToolEntries.length"
                :title="t('projectSettings.search.emptyTitle')"
                :description="t('projectSettings.search.emptyDescription')"
              />
            </div>
          </div>

          <UiStatusCallout
            v-if="dialogErrors.runtimeTools"
            tone="error"
            :description="dialogErrors.runtimeTools"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.runtimeTools = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-runtime-tools-save-button"
            :disabled="saving.runtimeTools"
            @click="saveRuntimeTools"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.runtimeActors"
        :title="t('projectSettings.dialogs.runtimeActors.title')"
        :description="t('projectSettings.dialogs.runtimeActors.description')"
        content-test-id="project-settings-runtime-actors-dialog"
      >
        <div class="space-y-4">
          <div
            v-if="projectOwnedAgents.length || projectOwnedTeams.length"
            class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-sm leading-6 text-text-secondary"
          >
            <div class="font-semibold text-text-primary">
              {{ t('projectSettings.dialogs.runtimeActors.projectOwnedTitle') }}
            </div>
            <div class="mt-1">
              {{ t('projectSettings.dialogs.runtimeActors.projectOwnedDescription') }}
            </div>
          </div>

          <UiTabs
            v-model="runtimeActorTab"
            :tabs="actorTabs"
            data-testid="project-settings-runtime-actors-tabs"
          />

          <div class="flex flex-wrap items-center gap-3">
            <div class="min-w-[16rem] flex-1">
              <UiInput
                v-model="runtimeActorSearchQuery"
                data-testid="project-settings-runtime-actors-search"
                :placeholder="t('projectSettings.search.actors')"
              />
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="runtimeActorTab === 'agents' ? 'project-settings-runtime-agents-select-all' : 'project-settings-runtime-teams-select-all'"
              @click="selectAllRuntimeActors"
            >
              {{ t('common.selectAll') }}
            </UiButton>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="runtimeActorTab === 'agents' ? 'project-settings-runtime-agents-clear-all' : 'project-settings-runtime-teams-clear-all'"
              @click="clearAllRuntimeActors"
            >
              {{ t('common.clearAll') }}
            </UiButton>
          </div>

          <section v-if="runtimeActorTab === 'agents'" class="space-y-3">
            <div
              v-for="agent in filteredRuntimeAgents"
              :key="agent.id"
              :data-testid="`project-runtime-agent-option-${agent.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                  <span>{{ agent.name }}</span>
                  <UiBadge :label="actorOriginBadge(agent)" subtle />
                  <UiBadge v-if="isLeaderAgent(agent.id)" :label="t('projects.fields.leader')" tone="info" />
                </div>
                <div class="text-xs text-text-secondary">
                  {{ agent.description || t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                :model-value="isRuntimeAgentEnabled(agent.id)"
                :aria-label="agent.name"
                @update:model-value="setRuntimeAgentEnabled(agent.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!filteredRuntimeAgents.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <section v-else class="space-y-3">
            <div
              v-for="team in filteredRuntimeTeams"
              :key="team.id"
              :data-testid="`project-runtime-team-option-${team.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex flex-wrap items-center gap-2 text-sm font-semibold text-text-primary">
                  <span>{{ team.name }}</span>
                  <UiBadge :label="actorOriginBadge(team)" subtle />
                </div>
                <div class="text-xs text-text-secondary">
                  {{ team.description || t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                :model-value="isRuntimeTeamEnabled(team.id)"
                :aria-label="team.name"
                @update:model-value="setRuntimeTeamEnabled(team.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!filteredRuntimeTeams.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <UiStatusCallout
            v-if="dialogErrors.runtimeActors"
            tone="error"
            :description="dialogErrors.runtimeActors"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.runtimeActors = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-runtime-actors-save-button"
            :disabled="saving.runtimeActors"
            @click="saveRuntimeActors"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.members"
        :title="t('projectSettings.dialogs.members.title')"
        :description="t('projectSettings.dialogs.members.description')"
        content-test-id="project-settings-members-dialog"
      >
        <div class="space-y-4">
          <UiField
            :label="t('projectSettings.users.title')"
            :hint="t('projectSettings.dialogs.members.hint')"
          >
            <div v-if="workspaceUsers.length" class="space-y-3">
              <label
                v-for="user in workspaceUsers"
                :key="user.id"
                :data-testid="`project-member-option-${user.id}`"
                class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="min-w-0 space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ user.displayName || user.username }}
                  </div>
                  <div class="text-xs text-text-secondary">
                    @{{ user.username }}
                  </div>
                </div>
                <UiCheckbox
                  v-model="memberDraft"
                  :value="user.id"
                  :aria-label="user.displayName || user.username"
                />
              </label>
            </div>
            <UiEmptyState
              v-else
              :title="t('projectSettings.users.emptyTitle')"
              :description="t('projectSettings.users.emptyDescription')"
            />
          </UiField>

          <UiStatusCallout
            v-if="dialogErrors.members"
            tone="error"
            :description="dialogErrors.members"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.members = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-settings-members-save-button"
            :disabled="saving.members"
            @click="saveMembers"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>
    </template>
  </UiPageShell>
</template>
