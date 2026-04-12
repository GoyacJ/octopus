<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ProjectRecord, WorkspaceToolCatalogEntry, WorkspaceToolKind } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiInspectorPanel,
  UiListDetailShell,
  UiMetricCard,
  UiPageHeader,
  UiPageShell,
  UiRecordCard,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import ProjectResourceDirectoryField from '@/components/projects/ProjectResourceDirectoryField.vue'
import { formatDateTime } from '@/i18n/copy'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'

const props = withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

const { t } = useI18n()
const agentStore = useAgentStore()
const catalogStore = useCatalogStore()
const shell = useShellStore()
const teamStore = useTeamStore()
const workspaceStore = useWorkspaceStore()

const selectedProjectId = ref('')
const form = reactive({
  name: '',
  description: '',
  resourceDirectory: '',
  configuredModelIds: [] as string[],
  defaultConfiguredModelId: '',
  toolSourceKeys: [] as string[],
  agentIds: [] as string[],
  teamIds: [] as string[],
})

const projects = computed(() => workspaceStore.projects)
const workspaceConfiguredModels = computed(() => catalogStore.workspaceConfiguredModelOptions)
const workspaceToolEntries = computed(() => catalogStore.toolCatalogEntries.filter(entry => !entry.disabled))
const workspaceAgents = computed(() => agentStore.workspaceAgents)
const workspaceTeams = computed(() => teamStore.workspaceTeams)
const viewReady = computed(() =>
  Boolean(shell.activeWorkspaceConnectionId)
  && (!workspaceStore.loading || projects.value.length > 0 || Boolean(workspaceStore.error)),
)
const selectedProject = computed(() =>
  projects.value.find(project => project.id === selectedProjectId.value) ?? null,
)
const TOOL_GROUP_ORDER: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']
const workspaceToolSections = computed<{ kind: WorkspaceToolKind, entries: WorkspaceToolCatalogEntry[] }[]>(() =>
  TOOL_GROUP_ORDER
    .map(kind => ({
      kind,
      entries: workspaceToolEntries.value.filter(entry => entry.kind === kind),
    }))
    .filter(section => section.entries.length > 0),
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
const isCreateMode = computed(() => !selectedProject.value)

watch(
  () => shell.activeWorkspaceConnectionId,
  async (connectionId) => {
    if (!connectionId) {
      return
    }
    await workspaceStore.bootstrap(connectionId)
    await Promise.all([
      catalogStore.load(connectionId),
      agentStore.load(connectionId),
      teamStore.load(connectionId),
    ])
  },
  { immediate: true },
)

watch(
  () => projects.value.map(project => `${project.id}:${project.status}:${project.name}:${project.description}:${JSON.stringify(project.assignments ?? {})}`).join('|'),
  () => {
    if (!selectedProjectId.value) {
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
  () => [...form.configuredModelIds].join(','),
  (value) => {
    const selectedIds = value ? value.split(',').filter(Boolean) : []
    if (!selectedIds.length) {
      form.defaultConfiguredModelId = ''
      return
    }
    if (!selectedIds.includes(form.defaultConfiguredModelId)) {
      form.defaultConfiguredModelId = selectedIds[0] ?? ''
    }
  },
)

function applyProject(projectId?: string) {
  const project = projects.value.find(item => item.id === projectId)
  selectedProjectId.value = project?.id ?? ''
  workspaceStore.syncRouteScope(undefined, project?.id ?? '')
  form.name = project?.name ?? ''
  form.description = project?.description ?? ''
  form.resourceDirectory = project?.resourceDirectory ?? ''
  form.configuredModelIds = [...(project?.assignments?.models?.configuredModelIds ?? [])]
  form.defaultConfiguredModelId = project?.assignments?.models?.defaultConfiguredModelId ?? ''
  form.toolSourceKeys = [...(project?.assignments?.tools?.sourceKeys ?? [])]
  form.agentIds = [...(project?.assignments?.agents?.agentIds ?? [])]
  form.teamIds = [...(project?.assignments?.agents?.teamIds ?? [])]
}

function buildAssignments() {
  const configuredModelIds = [...new Set(form.configuredModelIds)]
  const toolSourceKeys = [...new Set(form.toolSourceKeys)]
  const agentIds = [...new Set(form.agentIds)]
  const teamIds = [...new Set(form.teamIds)]

  return {
    models: configuredModelIds.length
      ? {
          configuredModelIds,
          defaultConfiguredModelId: configuredModelIds.includes(form.defaultConfiguredModelId)
            ? form.defaultConfiguredModelId
            : configuredModelIds[0] ?? '',
        }
      : undefined,
    tools: toolSourceKeys.length
      ? { sourceKeys: toolSourceKeys }
      : undefined,
    agents: agentIds.length || teamIds.length
      ? { agentIds, teamIds }
      : undefined,
  }
}

async function submitProject() {
  if (!form.name.trim() || !form.resourceDirectory.trim()) {
    return
  }

  const assignments = buildAssignments()

  if (selectedProject.value) {
    const updated = await workspaceStore.updateProject(selectedProject.value.id, {
      name: form.name,
      description: form.description,
      resourceDirectory: form.resourceDirectory,
      status: selectedProject.value.status,
      assignments,
    })
    if (updated) {
      applyProject(updated.id)
    }
    return
  }

  const created = await workspaceStore.createProject({
    name: form.name,
    description: form.description,
    resourceDirectory: form.resourceDirectory,
    assignments,
  })
  if (created) {
    applyProject(created.id)
  }
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

    <UiListDetailShell>
      <template #list>
        <section class="space-y-3">
        <UiRecordCard
          v-for="project in projects"
          :key="project.id"
          :data-testid="`projects-select-${project.id}`"
          :title="project.name"
          :description="project.description"
          interactive
          class="cursor-pointer"
          :active="selectedProjectId === project.id"
          @click="applyProject(project.id)"
        >
          <template #badges>
            <UiBadge :label="statusLabel(project.status)" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ formatDateTime(Date.now()) }}</span>
          </template>
        </UiRecordCard>
        <UiEmptyState
          v-if="!projects.length"
          :title="t('projects.empty.title')"
          :description="t('projects.empty.description')"
        />
        </section>
      </template>

      <UiInspectorPanel :title="isCreateMode ? t('projects.actions.create') : t('projects.actions.edit')">
        <div class="space-y-4">
        <h3 class="sr-only">
          {{ isCreateMode ? t('projects.actions.create') : t('projects.actions.edit') }}
        </h3>
        <UiField :label="t('projects.fields.name')">
          <UiInput v-model="form.name" data-testid="projects-name-input" />
        </UiField>
        <UiField :label="t('projects.fields.description')">
          <UiTextarea v-model="form.description" data-testid="projects-description-input" :rows="8" />
        </UiField>
        <ProjectResourceDirectoryField
          v-model="form.resourceDirectory"
          path-test-id="projects-resource-directory-path"
          pick-test-id="projects-resource-directory-pick"
        />

        <UiField
          :label="t('projects.fields.models')"
          :hint="t('projects.hints.models')"
        >
          <div v-if="workspaceConfiguredModels.length" class="space-y-3">
            <label
              v-for="modelOption in workspaceConfiguredModels"
              :key="modelOption.value"
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
                v-model="form.configuredModelIds"
                :value="modelOption.value"
                :aria-label="modelOption.label"
              />
            </label>
          </div>
          <div v-else class="text-sm text-text-secondary">
            {{ t('projects.emptyAssignments.models') }}
          </div>
        </UiField>

        <UiField
          :label="t('projects.fields.defaultModel')"
          :hint="t('projects.hints.defaultModel')"
        >
          <select
            v-model="form.defaultConfiguredModelId"
            data-testid="projects-default-model-select"
            class="w-full rounded-[var(--radius-xs)] border border-input bg-transparent px-3 py-2 text-sm text-text-primary outline-none"
            :disabled="!form.configuredModelIds.length"
          >
            <option value="">
              {{ t('common.na') }}
            </option>
            <option
              v-for="modelOption in workspaceConfiguredModels.filter(option => form.configuredModelIds.includes(option.value))"
              :key="modelOption.value"
              :value="modelOption.value"
            >
              {{ `${modelOption.label} · ${modelOption.providerLabel}` }}
            </option>
          </select>
        </UiField>

        <UiField
          :label="t('projects.fields.tools')"
          :hint="t('projects.hints.tools')"
        >
          <div v-if="workspaceToolSections.length" class="space-y-5">
            <section
              v-for="section in workspaceToolSections"
              :key="section.kind"
              class="space-y-3"
            >
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t(`projectSettings.tools.groups.${section.kind}`) }}
              </div>
              <label
                v-for="entry in section.entries"
                :key="entry.sourceKey"
                class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
              >
                <div class="min-w-0 space-y-1">
                  <div class="text-sm font-semibold text-text-primary">
                    {{ entry.name }}
                  </div>
                  <div class="text-xs text-text-secondary">
                    {{ entry.sourceKey }}
                  </div>
                </div>
                <UiCheckbox
                  v-model="form.toolSourceKeys"
                  :value="entry.sourceKey"
                  :aria-label="entry.name"
                />
              </label>
            </section>
          </div>
          <div v-else class="text-sm text-text-secondary">
            {{ t('projects.emptyAssignments.tools') }}
          </div>
        </UiField>

        <UiField
          :label="t('projects.fields.agents')"
          :hint="t('projects.hints.agents')"
        >
          <div v-if="workspaceAgents.length" class="space-y-3">
            <label
              v-for="agent in workspaceAgents"
              :key="agent.id"
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
                v-model="form.agentIds"
                :value="agent.id"
                :aria-label="agent.name"
              />
            </label>
          </div>
          <div v-else class="text-sm text-text-secondary">
            {{ t('projects.emptyAssignments.agents') }}
          </div>
        </UiField>

        <UiField
          :label="t('projects.fields.teams')"
          :hint="t('projects.hints.teams')"
        >
          <div v-if="workspaceTeams.length" class="space-y-3">
            <label
              v-for="team in workspaceTeams"
              :key="team.id"
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
                v-model="form.teamIds"
                :value="team.id"
                :aria-label="team.name"
              />
            </label>
          </div>
          <div v-else class="text-sm text-text-secondary">
            {{ t('projects.emptyAssignments.teams') }}
          </div>
        </UiField>

        <div class="flex flex-wrap gap-3">
          <UiButton
            :data-testid="isCreateMode ? 'projects-create-button' : 'projects-save-button'"
            :disabled="!form.name.trim() || !form.resourceDirectory.trim()"
            @click="submitProject"
          >
            {{ isCreateMode ? t('projects.actions.create') : t('common.save') }}
          </UiButton>
          <UiButton variant="ghost" @click="applyProject()">
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
        </div>
      </UiInspectorPanel>
    </UiListDetailShell>
  </component>
</template>
