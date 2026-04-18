<script setup lang="ts">
import { computed, ref } from 'vue'
import { RouterLink } from 'vue-router'

import {
  UiAccordion,
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
} from '@octopus/ui'

import { createWorkspaceConsoleSurfaceTarget } from '@/i18n/navigation'

import { useProjectSettings } from './useProjectSettings'

const {
  t,
  workspaceStore,
  project,
  dialogOpen,
  dialogErrors,
  saving,
  grantForm,
  runtimeForm,
  memberDraft,
  workspaceConfiguredModels,
  workspaceToolSections,
  grantedConfiguredModels,
  grantedToolSections,
  actorCandidateAgents,
  actorCandidateTeams,
  grantedAgents,
  grantedTeams,
  projectOwnedAgents,
  projectOwnedTeams,
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
  openGrantModelsDialog,
  openGrantToolsDialog,
  openGrantActorsDialog,
  openRuntimeModelsDialog,
  openRuntimeToolsDialog,
  openRuntimeActorsDialog,
  openMembersDialog,
  resolveRuntimeToolSelection,
  runtimeToolPermissionSummaryLabel,
  updateRuntimeToolPermission,
  saveGrantModels,
  saveGrantTools,
  saveGrantActors,
  saveRuntimeModels,
  saveRuntimeTools,
  saveRuntimeActors,
  saveMembers,
} = useProjectSettings()

const grantToolsAccordion = ref<string[]>([])
const runtimeToolsAccordion = ref<string[]>([])

const projectManagementTarget = computed(() =>
  workspaceStore.currentWorkspaceId
    ? createWorkspaceConsoleSurfaceTarget('workspace-console-projects', workspaceStore.currentWorkspaceId)
    : null,
)

const grantToolAccordionItems = computed(() =>
  workspaceToolSections.value.map(section => ({
    value: section.kind,
    title: `${t(`projectSettings.tools.groups.${section.kind}`)} · ${section.entries.length}`,
  })),
)

const runtimeToolAccordionItems = computed(() =>
  grantedToolSections.value.map(section => ({
    value: section.kind,
    title: `${t(`projectSettings.tools.groups.${section.kind}`)} · ${section.entries.length}`,
  })),
)

function entriesForGrantSection(kind: string) {
  return workspaceToolSections.value.find(section => section.kind === kind)?.entries ?? []
}

function entriesForRuntimeSection(kind: string) {
  return grantedToolSections.value.find(section => section.kind === kind)?.entries ?? []
}
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
            </div>

            <div class="mt-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-sm leading-6 text-text-secondary">
              {{ t('projectSettings.sections.overview.editHint') }}
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
              <button
                type="button"
                data-testid="project-settings-open-grants-models"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>

              <button
                type="button"
                data-testid="project-settings-open-grants-tools"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>

              <button
                type="button"
                data-testid="project-settings-open-grants-actors"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>
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
              <button
                type="button"
                data-testid="project-settings-open-runtime-models"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>

              <button
                type="button"
                data-testid="project-settings-open-runtime-tools"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>

              <button
                type="button"
                data-testid="project-settings-open-runtime-actors"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>
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
              <button
                type="button"
                data-testid="project-settings-open-members"
                class="ui-focus-ring flex w-full items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-left transition-colors hover:border-border-strong"
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
              </button>

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
            v-if="!workspaceToolSections.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <UiAccordion
            v-else
            v-model="grantToolsAccordion"
            :items="grantToolAccordionItems"
          >
            <template #content="{ item }">
              <div class="space-y-3">
                <label
                  v-for="entry in entriesForGrantSection(item.value)"
                  :key="entry.sourceKey"
                  :data-testid="`project-grant-tool-option-${entry.sourceKey}`"
                  class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="min-w-0 space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ entry.name }}
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ entry.description || entry.sourceKey }}
                    </div>
                  </div>
                  <UiCheckbox
                    v-model="grantForm.assignedToolSourceKeys"
                    :value="entry.sourceKey"
                    :aria-label="entry.name"
                  />
                </label>
              </div>
            </template>
          </UiAccordion>

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
          <section class="space-y-3">
            <UiField
              :label="t('projectSettings.agents.agentsLabel')"
              :hint="t('projectSettings.dialogs.grantActors.agentsHint')"
            >
              <div v-if="actorCandidateAgents.length" class="space-y-3">
                <label
                  v-for="agent in actorCandidateAgents"
                  :key="agent.id"
                  :data-testid="`project-grant-agent-option-${agent.id}`"
                  class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="min-w-0 space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ agent.name }}
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ agent.description || t('common.na') }}
                    </div>
                  </div>
                  <UiCheckbox
                    v-model="grantForm.assignedAgentIds"
                    :value="agent.id"
                    :aria-label="agent.name"
                  />
                </label>
              </div>
              <UiEmptyState
                v-else
                :title="t('projectSettings.agents.emptyTitle')"
                :description="t('projectSettings.agents.emptyDescription')"
              />
            </UiField>
          </section>

          <section class="space-y-3">
            <UiField
              :label="t('projectSettings.agents.teamsLabel')"
              :hint="t('projectSettings.dialogs.grantActors.teamsHint')"
            >
              <div v-if="actorCandidateTeams.length" class="space-y-3">
                <label
                  v-for="team in actorCandidateTeams"
                  :key="team.id"
                  :data-testid="`project-grant-team-option-${team.id}`"
                  class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="min-w-0 space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ team.name }}
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ team.description || t('common.na') }}
                    </div>
                  </div>
                  <UiCheckbox
                    v-model="grantForm.assignedTeamIds"
                    :value="team.id"
                    :aria-label="team.name"
                  />
                </label>
              </div>
              <UiEmptyState
                v-else
                :title="t('projectSettings.agents.emptyTitle')"
                :description="t('projectSettings.agents.emptyDescription')"
              />
            </UiField>
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
            v-if="!grantedToolSections.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <UiAccordion
            v-else
            v-model="runtimeToolsAccordion"
            :items="runtimeToolAccordionItems"
          >
            <template #content="{ item }">
              <div class="space-y-3">
                <div
                  v-for="entry in entriesForRuntimeSection(item.value)"
                  :key="entry.sourceKey"
                  class="space-y-2 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="flex items-start justify-between gap-4">
                    <div class="min-w-0 space-y-1">
                      <div class="text-sm font-semibold text-text-primary">
                        {{ entry.name }}
                      </div>
                      <div class="text-xs text-text-secondary">
                        {{ entry.description || entry.sourceKey }}
                      </div>
                    </div>
                    <UiBadge :label="t('projectSettings.labels.workspaceGrant')" subtle />
                  </div>

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
          </UiAccordion>

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

          <section class="space-y-3">
            <UiField
              :label="t('projectSettings.agents.agentsLabel')"
              :hint="t('projectSettings.dialogs.runtimeActors.agentsHint')"
            >
              <div v-if="grantedAgents.length" class="space-y-3">
                <label
                  v-for="agent in grantedAgents"
                  :key="agent.id"
                  :data-testid="`project-runtime-agent-option-${agent.id}`"
                  class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="min-w-0 space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ agent.name }}
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ agent.description || t('common.na') }}
                    </div>
                  </div>
                  <UiCheckbox
                    v-model="runtimeForm.enabledAgentIds"
                    :value="agent.id"
                    :aria-label="agent.name"
                  />
                </label>
              </div>
              <UiEmptyState
                v-else
                :title="t('projectSettings.agents.emptyTitle')"
                :description="t('projectSettings.agents.emptyDescription')"
              />
            </UiField>
          </section>

          <section class="space-y-3">
            <UiField
              :label="t('projectSettings.agents.teamsLabel')"
              :hint="t('projectSettings.dialogs.runtimeActors.teamsHint')"
            >
              <div v-if="grantedTeams.length" class="space-y-3">
                <label
                  v-for="team in grantedTeams"
                  :key="team.id"
                  :data-testid="`project-runtime-team-option-${team.id}`"
                  class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
                >
                  <div class="min-w-0 space-y-1">
                    <div class="text-sm font-semibold text-text-primary">
                      {{ team.name }}
                    </div>
                    <div class="text-xs text-text-secondary">
                      {{ team.description || t('common.na') }}
                    </div>
                  </div>
                  <UiCheckbox
                    v-model="runtimeForm.enabledTeamIds"
                    :value="team.id"
                    :aria-label="team.name"
                  />
                </label>
              </div>
              <UiEmptyState
                v-else
                :title="t('projectSettings.agents.emptyTitle')"
                :description="t('projectSettings.agents.emptyDescription')"
              />
            </UiField>
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
