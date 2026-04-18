<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import type { ProjectRecord } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiInspectorPanel,
  UiListDetailShell,
  UiListRow,
  UiMetricCard,
  UiPageHeader,
  UiPageShell,
  UiSelect,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import ProjectResourceDirectoryField from '@/components/projects/ProjectResourceDirectoryField.vue'
import { createProjectSurfaceTarget } from '@/i18n/navigation'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import {
  buildProjectCapabilitySummary,
  buildProjectSetupPresetSeed,
  type ProjectCapabilitySummary,
  type ProjectSetupPreset,
} from '@/stores/project_setup'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const { t } = useI18n()
const router = useRouter()
const agentStore = useAgentStore()
const catalogStore = useCatalogStore()
const shell = useShellStore()
const teamStore = useTeamStore()
const workspaceStore = useWorkspaceStore()

const selectedProjectId = ref('')
const detailsError = ref('')

const form = reactive({
  name: '',
  description: '',
  resourceDirectory: '',
  preset: 'general' as ProjectSetupPreset,
})

const projects = computed(() => workspaceStore.projects)
const workspaceConfiguredModels = computed(() => catalogStore.configuredModelOptions)
const workspaceToolEntries = computed(() => catalogStore.managementProjection.assets.filter(entry => entry.enabled))
const workspaceAgents = computed(() => agentStore.workspaceAgents)
const workspaceTeams = computed(() => teamStore.workspaceTeams)
const viewReady = computed(() =>
  Boolean(shell.activeWorkspaceConnectionId)
  && (!workspaceStore.loading || projects.value.length > 0 || Boolean(workspaceStore.error)),
)
const selectedProject = computed(() =>
  projects.value.find(project => project.id === selectedProjectId.value) ?? null,
)
const selectedProjectSettings = computed(() =>
  selectedProjectId.value ? workspaceStore.getProjectSettings(selectedProjectId.value) : {},
)
const selectedProjectDashboard = computed(() =>
  selectedProjectId.value ? workspaceStore.getProjectDashboard(selectedProjectId.value) : null,
)
const usedTokens = computed(() => selectedProjectDashboard.value?.usedTokens ?? 0)
const isCreateMode = computed(() => !selectedProject.value)

const presetOptions = computed(() => ([
  { value: 'general', label: String(t('projects.presets.options.general.label')) },
  { value: 'engineering', label: String(t('projects.presets.options.engineering.label')) },
  { value: 'documentation', label: String(t('projects.presets.options.documentation.label')) },
  { value: 'advanced', label: String(t('projects.presets.options.advanced.label')) },
]))

const presetSeed = computed(() => buildProjectSetupPresetSeed(form.preset, {
  models: workspaceConfiguredModels.value,
  tools: workspaceToolEntries.value,
  agents: workspaceAgents.value,
  teams: workspaceTeams.value,
}))

const selectedAssignedConfiguredModels = computed(() => {
  const configuredIds = selectedProject.value?.assignments?.models?.configuredModelIds ?? []
  return workspaceConfiguredModels.value.filter(item => configuredIds.includes(item.value))
})

const selectedAssignedToolEntries = computed(() => {
  const sourceKeys = selectedProject.value?.assignments?.tools?.sourceKeys ?? []
  return workspaceToolEntries.value.filter(entry => sourceKeys.includes(entry.sourceKey))
})

const selectedSummary = computed(() =>
  buildProjectCapabilitySummary({
    project: selectedProject.value,
    projectSettings: selectedProjectSettings.value,
    assignedConfiguredModels: selectedAssignedConfiguredModels.value,
    assignedToolEntries: selectedAssignedToolEntries.value,
    workspaceTools: catalogStore.tools,
  }),
)

const draftSummary = computed<ProjectCapabilitySummary>(() => {
  const grantedModelIds = presetSeed.value.assignments?.models?.configuredModelIds ?? []
  const enabledModelIds = presetSeed.value.modelSettings?.allowedConfiguredModelIds ?? grantedModelIds
  const defaultConfiguredModelId = presetSeed.value.modelSettings?.defaultConfiguredModelId
    || presetSeed.value.assignments?.models?.defaultConfiguredModelId
    || ''
  const defaultModelLabel = workspaceConfiguredModels.value.find(item => item.value === defaultConfiguredModelId)?.label ?? ''
  const grantedToolCount = presetSeed.value.assignments?.tools?.sourceKeys?.length ?? 0
  const enabledToolCount = presetSeed.value.toolSettings?.enabledSourceKeys?.length ?? grantedToolCount
  const grantedActorCount = (presetSeed.value.assignments?.agents?.agentIds?.length ?? 0)
    + (presetSeed.value.assignments?.agents?.teamIds?.length ?? 0)
  const enabledActorCount = (presetSeed.value.agentSettings?.enabledAgentIds?.length
    ?? presetSeed.value.assignments?.agents?.agentIds?.length
    ?? 0)
    + (presetSeed.value.agentSettings?.enabledTeamIds?.length
      ?? presetSeed.value.assignments?.agents?.teamIds?.length
      ?? 0)

  return {
    grantedModels: grantedModelIds.length,
    enabledModels: enabledModelIds.length,
    defaultModelLabel,
    grantedTools: grantedToolCount,
    enabledTools: enabledToolCount,
    toolOverrideCount: Object.keys(presetSeed.value.toolSettings?.overrides ?? {}).length,
    grantedActors: grantedActorCount,
    enabledActors: enabledActorCount,
    memberCount: 0,
    editableMemberCount: 0,
  }
})

const capabilitySummary = computed(() =>
  selectedProject.value ? selectedSummary.value : draftSummary.value,
)

const metrics = computed(() => [
  { id: 'total', label: t('projects.metrics.total'), value: String(projects.value.length) },
  { id: 'active', label: t('projects.metrics.active'), value: String(projects.value.filter(project => project.status === 'active').length) },
  { id: 'archived', label: t('projects.metrics.archived'), value: String(projects.value.filter(project => project.status === 'archived').length) },
])

const errorMessage = computed(() => {
  const message = workspaceStore.error
  if (!message) {
    return ''
  }

  if (message.includes('last active project')) {
    return t('projects.errors.lastActiveProject')
  }

  return message
})

watch(
  () => shell.activeWorkspaceConnectionId,
  async (connectionId) => {
    if (!connectionId) {
      return
    }
    await workspaceStore.ensureWorkspaceBootstrap(connectionId)
    await Promise.all([
      catalogStore.load(connectionId),
      agentStore.load(connectionId),
      teamStore.load(connectionId),
    ])
  },
  { immediate: true },
)

watch(
  () => projects.value.map(project => `${project.id}:${project.status}:${project.name}:${project.description}:${JSON.stringify(project.assignments ?? {})}:${(project.memberUserIds ?? []).join(',')}`).join('|'),
  () => {
    if (!selectedProjectId.value) {
      const fallbackProjectId = projects.value.find(project => project.id === workspaceStore.currentProjectId)?.id
        ?? projects.value[0]?.id
      if (fallbackProjectId) {
        applyProject(fallbackProjectId)
      }
      return
    }

    const current = projects.value.find(project => project.id === selectedProjectId.value)
    if (!current) {
      applyProject()
      return
    }
    applyProject(current.id)
  },
  { immediate: true },
)

watch(
  () => [shell.activeWorkspaceConnectionId, selectedProjectId.value] as const,
  async ([connectionId, projectId]) => {
    if (!connectionId || !projectId) {
      return
    }
    await Promise.all([
      workspaceStore.loadProjectDashboard(projectId, connectionId),
      workspaceStore.loadProjectRuntimeConfig(projectId, false, connectionId),
    ])
  },
  { immediate: true },
)

function applyProject(projectId?: string) {
  const project = projects.value.find(item => item.id === projectId)
  selectedProjectId.value = project?.id ?? ''
  workspaceStore.syncRouteScope(undefined, project?.id ?? '')
  form.name = project?.name ?? ''
  form.description = project?.description ?? ''
  form.resourceDirectory = project?.resourceDirectory ?? ''
  form.preset = 'general'
  detailsError.value = ''
}

async function submitProject() {
  if (!form.name.trim() || !form.resourceDirectory.trim()) {
    return
  }

  detailsError.value = ''

  if (selectedProject.value) {
    const updated = await workspaceStore.updateProject(selectedProject.value.id, {
      name: form.name,
      description: form.description,
      resourceDirectory: form.resourceDirectory,
      status: selectedProject.value.status,
      assignments: selectedProject.value.assignments,
      ownerUserId: selectedProject.value.ownerUserId,
      memberUserIds: selectedProject.value.memberUserIds,
      permissionOverrides: selectedProject.value.permissionOverrides,
      linkedWorkspaceAssets: selectedProject.value.linkedWorkspaceAssets,
    })
    if (!updated) {
      detailsError.value = workspaceStore.error || String(t('projects.errors.saveMetadata'))
      return
    }
    applyProject(updated.id)
    return
  }

  const created = await workspaceStore.createProject({
    name: form.name,
    description: form.description,
    resourceDirectory: form.resourceDirectory,
    assignments: presetSeed.value.assignments,
  })
  if (!created) {
    detailsError.value = workspaceStore.error || String(t('projects.errors.create'))
    return
  }

  const [savedModels, savedTools, savedActors] = await Promise.all([
    presetSeed.value.modelSettings
      ? workspaceStore.saveProjectModelSettings(created.id, presetSeed.value.modelSettings)
      : Promise.resolve(null),
    presetSeed.value.toolSettings
      ? workspaceStore.saveProjectToolSettings(created.id, presetSeed.value.toolSettings)
      : Promise.resolve(null),
    presetSeed.value.agentSettings
      ? workspaceStore.saveProjectAgentSettings(created.id, presetSeed.value.agentSettings)
      : Promise.resolve(null),
  ])

  if (
    (presetSeed.value.modelSettings && !savedModels)
    || (presetSeed.value.toolSettings && !savedTools)
    || (presetSeed.value.agentSettings && !savedActors)
  ) {
    detailsError.value = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
      || workspaceStore.error
      || String(t('projects.errors.seedRuntime'))
  }

  applyProject(created.id)
}

async function openSelectedProjectSettings() {
  const workspaceId = workspaceStore.currentWorkspaceId
  if (!workspaceId || !selectedProject.value) {
    return
  }

  await router.push(createProjectSurfaceTarget('project-settings', workspaceId, selectedProject.value.id))
}

async function archiveSelectedProject() {
  if (!selectedProject.value) {
    return
  }

  const updated = await workspaceStore.archiveProject(selectedProject.value.id)
  if (updated) {
    applyProject(workspaceStore.currentProjectId || updated.id)
  }
}

async function restoreSelectedProject() {
  if (!selectedProject.value) {
    return
  }

  const updated = await workspaceStore.restoreProject(selectedProject.value.id)
  if (updated) {
    applyProject(workspaceStore.currentProjectId || updated.id)
  }
}

function statusLabel(status: ProjectRecord['status']) {
  return status === 'archived'
    ? t('projects.status.archived')
    : t('projects.status.active')
}

function modelSummaryLabel(summary: ProjectCapabilitySummary) {
  return t('projects.summary.modelsValue', {
    granted: summary.grantedModels,
    enabled: summary.enabledModels,
    defaultModel: summary.defaultModelLabel || t('common.na'),
  })
}

function toolSummaryLabel(summary: ProjectCapabilitySummary) {
  return t('projects.summary.toolsValue', {
    granted: summary.grantedTools,
    enabled: summary.enabledTools,
    overrides: summary.toolOverrideCount,
  })
}

function actorSummaryLabel(summary: ProjectCapabilitySummary) {
  return t('projects.summary.actorsValue', {
    granted: summary.grantedActors,
    enabled: summary.enabledActors,
  })
}

function memberSummaryLabel(summary: ProjectCapabilitySummary) {
  return t('projects.summary.membersValue', {
    members: summary.memberCount,
    editors: summary.editableMemberCount,
  })
}
</script>

<template>
  <component
    :is="props.embedded ? 'div' : UiPageShell"
    v-if="viewReady"
    :width="props.embedded ? undefined : 'standard'"
    :test-id="props.embedded ? undefined : 'workspace-projects-view'"
    :data-testid="props.embedded ? 'workspace-projects-embedded' : undefined"
    class="space-y-6"
  >
    <UiPageHeader
      v-if="!props.embedded"
      :eyebrow="t('projects.header.eyebrow')"
      :title="t('sidebar.navigation.projects')"
      :description="errorMessage || t('projects.header.subtitle')"
    >
      <template #actions>
        <UiButton data-testid="projects-create-header-button" @click="applyProject()">
          {{ t('projects.actions.create') }}
        </UiButton>
      </template>
    </UiPageHeader>

    <div v-else class="flex justify-end">
      <UiButton data-testid="projects-create-header-button" @click="applyProject()">
        {{ t('projects.actions.create') }}
      </UiButton>
    </div>

    <section class="space-y-4">
      <div class="grid gap-3 md:grid-cols-3">
        <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
      </div>
    </section>

    <UiStatusCallout
      v-if="errorMessage"
      data-testid="projects-error"
      tone="error"
      :description="errorMessage"
    />

    <UiListDetailShell list-class="p-3" detail-class="p-3">
      <template #list>
        <section class="space-y-3">
          <UiListRow
            v-for="project in projects"
            :key="project.id"
            :data-testid="`projects-select-${project.id}`"
            :title="project.name"
            :subtitle="project.description || project.resourceDirectory || t('common.na')"
            interactive
            class="cursor-pointer"
            :active="selectedProjectId === project.id"
            @click="applyProject(project.id)"
          >
            <div class="flex flex-wrap gap-1.5 pt-1">
              <UiBadge :label="statusLabel(project.status)" subtle />
            </div>
            <template #meta>
              <span class="line-clamp-1 text-xs text-text-tertiary">{{ project.resourceDirectory }}</span>
            </template>
          </UiListRow>
          <UiEmptyState
            v-if="!projects.length"
            :title="t('projects.empty.title')"
            :description="t('projects.empty.description')"
          />
        </section>
      </template>

      <UiInspectorPanel :title="isCreateMode ? t('projects.actions.create') : t('projects.actions.edit')">
        <div class="space-y-6">
          <section class="space-y-4">
            <div class="grid gap-4 md:grid-cols-2">
              <UiField :label="t('projects.fields.name')">
                <UiInput v-model="form.name" data-testid="projects-name-input" />
              </UiField>

              <UiField :label="t('projects.fields.resourceDirectory')">
                <ProjectResourceDirectoryField
                  v-model="form.resourceDirectory"
                  path-test-id="projects-resource-directory-path"
                  pick-test-id="projects-resource-directory-pick"
                />
              </UiField>
            </div>

            <UiField :label="t('projects.fields.description')">
              <UiTextarea v-model="form.description" data-testid="projects-description-input" :rows="6" />
            </UiField>

            <UiField
              :label="t('projects.fields.preset')"
              :hint="isCreateMode ? t('projects.presets.hint') : t('projects.presets.editHint')"
            >
              <UiSelect
                v-model="form.preset"
                data-testid="projects-preset-select"
                :disabled="!isCreateMode"
                :options="presetOptions"
              />
            </UiField>

            <div class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3 text-sm text-text-secondary">
              <div class="font-medium text-text-primary">
                {{ t(`projects.presets.options.${form.preset}.label`) }}
              </div>
              <div class="mt-1 leading-6">
                {{ t(`projects.presets.options.${form.preset}.description`) }}
              </div>
            </div>
          </section>

          <section class="space-y-3 border-t border-border pt-4">
            <div class="space-y-1">
              <div class="text-sm font-semibold text-text-primary">
                {{ t('projects.summary.title') }}
              </div>
              <div class="text-sm leading-6 text-text-secondary">
                {{ selectedProject ? t('projects.summary.currentDescription') : t('projects.summary.createDescription') }}
              </div>
            </div>

            <div
              data-testid="projects-summary-models"
              class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
                {{ t('projects.summary.models') }}
              </div>
              <div class="mt-1 text-sm leading-6 text-text-primary">
                {{ modelSummaryLabel(capabilitySummary) }}
              </div>
            </div>

            <div
              data-testid="projects-summary-tools"
              class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
                {{ t('projects.summary.tools') }}
              </div>
              <div class="mt-1 text-sm leading-6 text-text-primary">
                {{ toolSummaryLabel(capabilitySummary) }}
              </div>
            </div>

            <div
              data-testid="projects-summary-actors"
              class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
                {{ t('projects.summary.actors') }}
              </div>
              <div class="mt-1 text-sm leading-6 text-text-primary">
                {{ actorSummaryLabel(capabilitySummary) }}
              </div>
            </div>

            <div
              data-testid="projects-summary-members"
              class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
                {{ t('projects.summary.members') }}
              </div>
              <div class="mt-1 text-sm leading-6 text-text-primary">
                {{ memberSummaryLabel(capabilitySummary) }}
              </div>
            </div>
          </section>

          <section v-if="selectedProject" class="space-y-3 border-t border-border pt-4">
            <div class="flex items-center justify-between gap-3">
              <div class="space-y-1">
                <div class="text-sm font-semibold text-text-primary">
                  {{ t('projects.summary.advancedTitle') }}
                </div>
                <div class="text-sm leading-6 text-text-secondary">
                  {{ t('projects.summary.advancedDescription') }}
                </div>
              </div>

              <UiBadge :label="statusLabel(selectedProject.status)" subtle />
            </div>

            <UiButton
              data-testid="projects-open-settings-button"
              variant="ghost"
              @click="openSelectedProjectSettings"
            >
              {{ t('projects.actions.openSettings') }}
            </UiButton>
          </section>

          <div class="flex flex-wrap gap-3">
            <UiButton
              :data-testid="isCreateMode ? 'projects-create-button' : 'projects-save-button'"
              :disabled="!form.name.trim() || !form.resourceDirectory.trim()"
              @click="submitProject"
            >
              {{ isCreateMode ? t('projects.actions.create') : t('common.save') }}
            </UiButton>
            <UiButton variant="ghost" @click="applyProject(selectedProject?.id)">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton
              v-if="selectedProject && selectedProject.status === 'active'"
              data-testid="projects-archive-button"
              variant="ghost"
              @click="archiveSelectedProject"
            >
              {{ t('projects.actions.archive') }}
            </UiButton>
            <UiButton
              v-if="selectedProject && selectedProject.status === 'archived'"
              data-testid="projects-restore-button"
              variant="ghost"
              @click="restoreSelectedProject"
            >
              {{ t('projects.actions.restore') }}
            </UiButton>
          </div>

          <UiStatusCallout
            v-if="detailsError"
            data-testid="projects-detail-error"
            tone="error"
            :description="detailsError"
          />
        </div>
      </UiInspectorPanel>
    </UiListDetailShell>
  </component>
</template>
