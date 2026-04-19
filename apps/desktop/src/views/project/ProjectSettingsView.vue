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
  UiInput,
  UiInspectorPanel,
  UiPageHeader,
  UiPageShell,
  UiSelect,
  UiStatusCallout,
  UiTabs,
} from '@octopus/ui'

import { createWorkspaceConsoleSurfaceTarget } from '@/i18n/navigation'

import ProjectBasicsPanel from './ProjectBasicsPanel.vue'
import ProjectCapabilitiesPanel from './ProjectCapabilitiesPanel.vue'
import ProjectLifecyclePanel from './ProjectLifecyclePanel.vue'
import { useProjectSettings } from './useProjectSettings'

const {
  t,
  workspaceStore,
  project,
  basicsForm,
  basicsError,
  savingBasics,
  leaderDraft,
  leaderOptions,
  currentLeaderLabel,
  managerOptions,
  presetOptions,
  toolTabs,
  capabilityScopeTabs,
  actorTabs,
  modelDialogScope,
  toolDialogScope,
  actorDialogScope,
  grantToolTab,
  runtimeToolTab,
  grantActorTab,
  runtimeActorTab,
  activeToolTab,
  activeActorTab,
  grantToolSearchQuery,
  runtimeToolSearchQuery,
  grantActorSearchQuery,
  runtimeActorSearchQuery,
  activeToolSearchQuery,
  activeActorSearchQuery,
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
  activeToolEntries,
  activeActorEntries,
  activeTeamEntries,
  workspaceActiveAgents,
  workspaceActiveTeams,
  grantedAgents,
  grantedTeams,
  projectOwnedAgents,
  projectOwnedTeams,
  grantedProjectOwnedTools,
  workspaceUsers,
  toolPermissionOptions,
  capabilityCards,
  memberSummary,
  accessSummary,
  deletionRequestsReady,
  latestDeletionRequest,
  canReviewDeletion,
  lifecycleReviewCallout,
  lifecycleError,
  creatingDeletionRequest,
  reviewingDeletionRequest,
  deletingProject,
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
  resetBasics,
  openModelsDialog,
  openToolsDialog,
  openActorsDialog,
  selectAllGrantModels,
  clearGrantModels,
  selectAllGrantTools,
  clearGrantTools,
  selectAllGrantActors,
  clearGrantActors,
  selectAllRuntimeTools,
  clearAllRuntimeTools,
  selectAllRuntimeActors,
  clearAllRuntimeActors,
  openMembersDialog,
  resolveRuntimeToolSelection,
  runtimeToolPermissionSummaryLabel,
  updateRuntimeToolPermission,
  deletionRequestStatusLabel,
  archiveProject,
  restoreProject,
  createDeletionRequest,
  reviewDeletionRequest,
  deleteProject,
  saveBasics,
  saveLeader,
  saveGrantModels,
  saveGrantTools,
  saveGrantActors,
  saveRuntimeModels,
  saveRuntimeTools,
  saveRuntimeActors,
  saveModelsDialog,
  saveToolsDialog,
  saveActorsDialog,
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
            class="space-y-4"
          >
            <ProjectBasicsPanel
              :basics-form="basicsForm"
              :manager-options="managerOptions"
              :preset-options="presetOptions"
              :status-label="statusLabel"
              :badge-tone="badgeTone(project.status)"
              :basics-error="basicsError"
              :saving-basics="savingBasics"
              @reset="resetBasics"
              @save="saveBasics"
            />

            <div class="flex flex-wrap items-center justify-between gap-3 rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5">
              <div class="max-w-[40rem]">
                <div class="text-sm font-semibold text-text-primary">
                  {{ t('projects.fields.leader') }}
                </div>
                <div class="mt-1 text-sm leading-6 text-text-secondary">
                  {{ t('projectSettings.sections.overview.editHint', { leader: currentLeaderLabel }) }}
                </div>
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
            data-testid="project-settings-capabilities-section"
            class="space-y-4"
          >
            <ProjectCapabilitiesPanel
              :capability-cards="capabilityCards"
              @edit-models="openModelsDialog()"
              @edit-tools="openToolsDialog()"
              @edit-agents="openActorsDialog('agents')"
              @edit-teams="openActorsDialog('teams')"
            />
          </section>

          <ProjectLifecyclePanel
            :title="t('projectSettings.sections.lifecycle.title')"
            :description="t('projectSettings.sections.lifecycle.description')"
            :review-callout="lifecycleReviewCallout"
            :status-label="statusLabel"
            :badge-tone="badgeTone(project.status)"
            :project-status="project.status"
            :deletion-request-status-label="deletionRequestStatusLabel(latestDeletionRequest?.status)"
            :latest-deletion-request="latestDeletionRequest"
            :deletion-requests-ready="deletionRequestsReady"
            :can-review-deletion="canReviewDeletion"
            :creating-deletion-request="creatingDeletionRequest"
            :reviewing-deletion-request="reviewingDeletionRequest"
            :deleting-project="deletingProject"
            :lifecycle-error="lifecycleError"
            @archive="archiveProject"
            @restore="restoreProject"
            @request-delete="createDeletionRequest"
            @approve-delete-request="reviewDeletionRequest(true)"
            @reject-delete-request="reviewDeletionRequest(false)"
            @delete-project="deleteProject"
          />

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
        v-model:open="dialogOpen.models"
        :title="t('projectSettings.dialogs.models.title')"
        :description="t('projectSettings.dialogs.models.description')"
        content-test-id="project-settings-models-dialog"
      >
        <div class="space-y-4">
          <UiTabs
            v-model="modelDialogScope"
            :tabs="capabilityScopeTabs"
            data-testid="project-settings-models-scope-tabs"
          />

          <template v-if="modelDialogScope === 'workspace'">
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
          </template>

          <template v-else>
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
          </template>
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.models = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            :data-testid="modelDialogScope === 'workspace' ? 'project-settings-grants-models-save-button' : 'project-settings-runtime-models-save-button'"
            :disabled="modelDialogScope === 'workspace' ? saving.grantModels : saving.runtimeModels"
            @click="saveModelsDialog"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.tools"
        :title="t('projectSettings.dialogs.tools.title')"
        :description="t('projectSettings.dialogs.tools.description')"
        content-test-id="project-settings-tools-dialog"
      >
        <div class="space-y-4">
          <UiTabs
            v-model="toolDialogScope"
            :tabs="capabilityScopeTabs"
            data-testid="project-settings-tools-scope-tabs"
          />

          <UiEmptyState
            v-if="toolDialogScope === 'workspace' ? !workspaceToolEntries.length : !grantedToolEntries.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <div v-else class="space-y-3">
            <UiTabs
              v-model="activeToolTab"
              :tabs="toolTabs"
              :data-testid="toolDialogScope === 'workspace' ? 'project-settings-grants-tools-tabs' : 'project-settings-tools-scope-tabs-inner'"
            />

            <div class="flex flex-wrap items-center gap-3">
              <div class="min-w-[16rem] flex-1">
                <UiInput
                  v-model="activeToolSearchQuery"
                  :data-testid="toolDialogScope === 'workspace' ? 'project-settings-grants-tools-search' : 'project-settings-runtime-tools-search'"
                  :placeholder="t('projectSettings.search.tools')"
                />
              </div>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                :data-testid="toolDialogScope === 'workspace' ? 'project-settings-grants-tools-select-all' : 'project-settings-runtime-tools-select-all'"
                @click="toolDialogScope === 'workspace' ? selectAllGrantTools() : selectAllRuntimeTools()"
              >
                {{ t('common.selectAll') }}
              </UiButton>
              <UiButton
                type="button"
                variant="ghost"
                size="sm"
                :data-testid="toolDialogScope === 'workspace' ? 'project-settings-grants-tools-clear-all' : 'project-settings-runtime-tools-clear-all'"
                @click="toolDialogScope === 'workspace' ? clearGrantTools() : clearAllRuntimeTools()"
              >
                {{ t('common.clearAll') }}
              </UiButton>
            </div>

            <div class="space-y-3">
              <template v-if="toolDialogScope === 'workspace'">
                <div
                  v-for="entry in activeToolEntries"
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
              </template>

              <template v-else>
                <div
                  v-for="entry in activeToolEntries"
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
              </template>

              <UiEmptyState
                v-if="!activeToolEntries.length"
                :title="t('projectSettings.search.emptyTitle')"
                :description="t('projectSettings.search.emptyDescription')"
              />
            </div>
          </div>

          <UiStatusCallout
            v-if="toolDialogScope === 'workspace' ? dialogErrors.grantTools : dialogErrors.runtimeTools"
            tone="error"
            :description="toolDialogScope === 'workspace' ? dialogErrors.grantTools : dialogErrors.runtimeTools"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.tools = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            :data-testid="toolDialogScope === 'workspace' ? 'project-settings-grants-tools-save-button' : 'project-settings-runtime-tools-save-button'"
            :disabled="toolDialogScope === 'workspace' ? saving.grantTools : saving.runtimeTools"
            @click="saveToolsDialog"
          >
            {{ t('common.save') }}
          </UiButton>
        </template>
      </UiDialog>

      <UiDialog
        v-model:open="dialogOpen.actors"
        :title="t('projectSettings.dialogs.actors.title')"
        :description="t('projectSettings.dialogs.actors.description')"
        content-test-id="project-settings-actors-dialog"
      >
        <div class="space-y-4">
          <UiTabs
            v-model="actorDialogScope"
            :tabs="capabilityScopeTabs"
            data-testid="project-settings-actors-scope-tabs"
          />

          <div
            v-if="actorDialogScope === 'project' && (projectOwnedAgents.length || projectOwnedTeams.length)"
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
            v-model="activeActorTab"
            :tabs="actorTabs"
            data-testid="project-settings-actors-actor-tabs"
          />

          <div class="flex flex-wrap items-center gap-3">
            <div class="min-w-[16rem] flex-1">
              <UiInput
                v-model="activeActorSearchQuery"
                :data-testid="actorDialogScope === 'workspace' ? 'project-settings-grants-actors-search' : 'project-settings-runtime-actors-search'"
                :placeholder="t('projectSettings.search.actors')"
              />
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="actorDialogScope === 'workspace'
                ? (activeActorTab === 'agents' ? 'project-settings-grants-agents-select-all' : 'project-settings-grants-teams-select-all')
                : (activeActorTab === 'agents' ? 'project-settings-runtime-agents-select-all' : 'project-settings-runtime-teams-select-all')"
              @click="actorDialogScope === 'workspace' ? selectAllGrantActors() : selectAllRuntimeActors()"
            >
              {{ t('common.selectAll') }}
            </UiButton>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              :data-testid="actorDialogScope === 'workspace'
                ? (activeActorTab === 'agents' ? 'project-settings-grants-agents-clear-all' : 'project-settings-grants-teams-clear-all')
                : (activeActorTab === 'agents' ? 'project-settings-runtime-agents-clear-all' : 'project-settings-runtime-teams-clear-all')"
              @click="actorDialogScope === 'workspace' ? clearGrantActors() : clearAllRuntimeActors()"
            >
              {{ t('common.clearAll') }}
            </UiButton>
          </div>

          <section v-if="activeActorTab === 'agents'" class="space-y-3">
            <div
              v-for="agent in activeActorEntries"
              :key="agent.id"
              :data-testid="`${actorDialogScope === 'workspace' ? 'project-grant-agent-option' : 'project-runtime-agent-option'}-${agent.id}`"
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
                :model-value="actorDialogScope === 'workspace' ? isGrantAgentEnabled(agent.id) : isRuntimeAgentEnabled(agent.id)"
                :aria-label="agent.name"
                @update:model-value="actorDialogScope === 'workspace'
                  ? setGrantAgentEnabled(agent.id, Boolean($event))
                  : setRuntimeAgentEnabled(agent.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!activeActorEntries.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <section v-else class="space-y-3">
            <div
              v-for="team in activeTeamEntries"
              :key="team.id"
              :data-testid="`${actorDialogScope === 'workspace' ? 'project-grant-team-option' : 'project-runtime-team-option'}-${team.id}`"
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
                :model-value="actorDialogScope === 'workspace' ? isGrantTeamEnabled(team.id) : isRuntimeTeamEnabled(team.id)"
                :aria-label="team.name"
                @update:model-value="actorDialogScope === 'workspace'
                  ? setGrantTeamEnabled(team.id, Boolean($event))
                  : setRuntimeTeamEnabled(team.id, Boolean($event))"
              />
            </div>

            <UiEmptyState
              v-if="!activeTeamEntries.length"
              :title="t('projectSettings.search.emptyTitle')"
              :description="t('projectSettings.search.emptyDescription')"
            />
          </section>

          <UiStatusCallout
            v-if="actorDialogScope === 'workspace' ? dialogErrors.grantActors : dialogErrors.runtimeActors"
            tone="error"
            :description="actorDialogScope === 'workspace' ? dialogErrors.grantActors : dialogErrors.runtimeActors"
          />
        </div>

        <template #footer>
          <UiButton variant="ghost" @click="dialogOpen.actors = false">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            :data-testid="actorDialogScope === 'workspace' ? 'project-settings-grants-actors-save-button' : 'project-settings-runtime-actors-save-button'"
            :disabled="actorDialogScope === 'workspace' ? saving.grantActors : saving.runtimeActors"
            @click="saveActorsDialog"
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
