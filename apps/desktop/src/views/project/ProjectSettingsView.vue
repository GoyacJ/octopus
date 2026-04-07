<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import type {
  AgentRecord,
  ProjectRecord,
  TeamRecord,
  WorkspaceToolCatalogEntry,
  WorkspaceToolKind,
  WorkspaceToolPermissionMode,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiRecordCard,
  UiSectionHeading,
  UiSelect,
  UiTabs,
  UiTextarea,
} from '@octopus/ui'

import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useTeamStore } from '@/stores/team'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

type ProjectSettingsTab = 'basics' | 'models' | 'tools' | 'agents' | 'users'
type ToolPermissionSelection = 'inherit' | WorkspaceToolPermissionMode

interface ToolSection {
  kind: WorkspaceToolKind
  entries: WorkspaceToolCatalogEntry[]
}

const TOOL_TAB_ORDER: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']
const TOOL_PERMISSION_VALUES: ToolPermissionSelection[] = ['inherit', 'allow', 'ask', 'readonly', 'deny']

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()
const agentStore = useAgentStore()
const catalogStore = useCatalogStore()
const teamStore = useTeamStore()
const userCenterStore = useUserCenterStore()

const activeTab = ref<ProjectSettingsTab>('basics')
const loadingDependencies = ref(false)
const savingBasics = ref(false)
const savingModels = ref(false)
const savingTools = ref(false)
const savingAgents = ref(false)
const savingUsers = ref(false)
const basicsError = ref('')
const modelsError = ref('')
const toolsError = ref('')
const agentsError = ref('')
const usersError = ref('')

const basicsForm = reactive({
  name: '',
  description: '',
})

const modelsForm = reactive({
  allowedConfiguredModelIds: [] as string[],
  defaultConfiguredModelId: '',
})

const enabledAgentIds = ref<string[]>([])
const enabledTeamIds = ref<string[]>([])
const selectedMemberUserIds = ref<string[]>([])
const toolPermissionDraft = ref<Record<string, ToolPermissionSelection>>({})

const tabs = computed(() => [
  { value: 'basics', label: t('projectSettings.tabs.basics') },
  { value: 'models', label: t('projectSettings.tabs.models') },
  { value: 'tools', label: t('projectSettings.tabs.tools') },
  { value: 'agents', label: t('projectSettings.tabs.agents') },
  { value: 'users', label: t('projectSettings.tabs.users') },
])

const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const connectionId = computed(() => workspaceStore.activeConnectionId)
const project = computed(() =>
  workspaceStore.projects.find(item => item.id === projectId.value) ?? null,
)
const projectSettings = computed(() =>
  projectId.value ? workspaceStore.getProjectSettings(projectId.value) : {},
)
const workspaceAssignments = computed(() => project.value?.assignments)
const allowedWorkspaceConfiguredModels = computed(() => {
  const assignedIds = workspaceAssignments.value?.models?.configuredModelIds ?? []
  return catalogStore.workspaceConfiguredModelOptions.filter(item => assignedIds.includes(item.value))
})
const allowedToolSourceKeys = computed(() =>
  workspaceAssignments.value?.tools?.sourceKeys ?? [],
)
const allowedToolEntries = computed(() =>
  catalogStore.toolCatalogEntries.filter(entry => allowedToolSourceKeys.value.includes(entry.sourceKey) && !entry.disabled),
)
const workspaceAssignedAgents = computed<AgentRecord[]>(() => {
  const assignedIds = workspaceAssignments.value?.agents?.agentIds ?? []
  return agentStore.workspaceAgents.filter(agent => assignedIds.includes(agent.id))
})
const workspaceAssignedTeams = computed<TeamRecord[]>(() => {
  const assignedIds = workspaceAssignments.value?.agents?.teamIds ?? []
  return teamStore.workspaceTeams.filter(team => assignedIds.includes(team.id))
})
const workspaceUsers = computed(() =>
  [...userCenterStore.users].sort((left, right) =>
    (left.displayName || left.username).localeCompare(right.displayName || right.username),
  ),
)
const workspaceDefaultConfiguredModelId = computed(() =>
  workspaceAssignments.value?.models?.defaultConfiguredModelId
  ?? allowedWorkspaceConfiguredModels.value[0]?.value
  ?? '',
)
const modelTabReady = computed(() => !loadingDependencies.value && Boolean(connectionId.value))
const viewReady = computed(() =>
  Boolean(connectionId.value)
  && (!workspaceStore.loading || Boolean(project.value) || Boolean(workspaceStore.error)),
)

const resolvedModelSettings = computed(() => {
  const configuredIds = allowedWorkspaceConfiguredModels.value.map(item => item.value)
  const saved = projectSettings.value.models
  const savedAllowedIds = saved?.allowedConfiguredModelIds ?? []
  const allowedConfiguredModelIds = savedAllowedIds.length
    ? savedAllowedIds.filter(item => configuredIds.includes(item))
    : configuredIds
  const defaultConfiguredModelId = allowedConfiguredModelIds.includes(saved?.defaultConfiguredModelId ?? '')
    ? saved?.defaultConfiguredModelId ?? ''
    : allowedConfiguredModelIds.includes(workspaceDefaultConfiguredModelId.value)
      ? workspaceDefaultConfiguredModelId.value
      : allowedConfiguredModelIds[0] ?? ''

  return {
    allowedConfiguredModelIds,
    defaultConfiguredModelId,
  }
})

const resolvedToolSettings = computed(() => {
  const assignedSourceKeys = allowedToolEntries.value.map(entry => entry.sourceKey)
  const saved = projectSettings.value.tools
  const enabledSourceKeys = saved?.enabledSourceKeys?.length
    ? saved.enabledSourceKeys.filter(sourceKey => assignedSourceKeys.includes(sourceKey))
    : assignedSourceKeys

  return {
    enabledSourceKeys,
    overrides: saved?.overrides ?? {},
  }
})

const resolvedAgentSettings = computed(() => {
  const assignedAgentIds = workspaceAssignedAgents.value.map(agent => agent.id)
  const assignedTeamIds = workspaceAssignedTeams.value.map(team => team.id)
  const saved = projectSettings.value.agents

  return {
    enabledAgentIds: saved?.enabledAgentIds?.length
      ? saved.enabledAgentIds.filter(agentId => assignedAgentIds.includes(agentId))
      : assignedAgentIds,
    enabledTeamIds: saved?.enabledTeamIds?.length
      ? saved.enabledTeamIds.filter(teamId => assignedTeamIds.includes(teamId))
      : assignedTeamIds,
  }
})

const toolSections = computed<ToolSection[]>(() =>
  TOOL_TAB_ORDER
    .map(kind => ({
      kind,
      entries: allowedToolEntries.value.filter(entry => entry.kind === kind),
    }))
    .filter(section => section.entries.length > 0),
)

const currentMemberUserIds = computed(() =>
  workspaceUsers.value
    .filter(user => user.scopeProjectIds.includes(projectId.value))
    .map(user => user.id),
)

const summaryAllowedModels = computed(() =>
  allowedWorkspaceConfiguredModels.value.filter(item => modelsForm.allowedConfiguredModelIds.includes(item.value)),
)
const summaryOverrideCount = computed(() =>
  Object.values(toolPermissionDraft.value).filter(value => value !== 'inherit').length,
)
const summaryActorCount = computed(() => enabledAgentIds.value.length + enabledTeamIds.value.length)
const summaryMemberCount = computed(() => selectedMemberUserIds.value.length)

const toolPermissionOptions = computed(() =>
  TOOL_PERMISSION_VALUES.map(value => ({
    value,
    label: t(`projectSettings.tools.modes.${value}`),
  })),
)

watch(
  () => [
    connectionId.value,
    projectId.value,
  ],
  async ([nextConnectionId, nextProjectId]) => {
    if (!nextConnectionId || !nextProjectId) {
      return
    }

    loadingDependencies.value = true
    try {
      await Promise.all([
        workspaceStore.loadProjectRuntimeConfig(nextProjectId, false, nextConnectionId),
        agentStore.load(nextConnectionId),
        catalogStore.load(nextConnectionId),
        teamStore.load(nextConnectionId),
        userCenterStore.load(nextConnectionId),
      ])
    } finally {
      loadingDependencies.value = false
    }
  },
  { immediate: true },
)

watch(
  () => [project.value?.id, project.value?.name, project.value?.description].join('|'),
  () => {
    basicsForm.name = project.value?.name ?? ''
    basicsForm.description = project.value?.description ?? ''
    basicsError.value = ''
  },
  { immediate: true },
)

watch(
  () => `${projectId.value}|${resolvedModelSettings.value.allowedConfiguredModelIds.join(',')}|${resolvedModelSettings.value.defaultConfiguredModelId}`,
  () => {
    modelsForm.allowedConfiguredModelIds = [...resolvedModelSettings.value.allowedConfiguredModelIds]
    modelsForm.defaultConfiguredModelId = resolvedModelSettings.value.defaultConfiguredModelId
    modelsError.value = ''
  },
  { immediate: true },
)

watch(
  () => `${projectId.value}|${resolvedToolSettings.value.enabledSourceKeys.join(',')}|${JSON.stringify(resolvedToolSettings.value.overrides)}`,
  () => {
    const nextDraft = Object.fromEntries(
      allowedToolEntries.value.map(entry => {
        const override = resolvedToolSettings.value.overrides[entry.sourceKey]
        const disabled = !resolvedToolSettings.value.enabledSourceKeys.includes(entry.sourceKey)
        return [entry.sourceKey, disabled ? 'deny' : (override?.permissionMode ?? 'inherit')]
      }),
    ) as Record<string, ToolPermissionSelection>
    toolPermissionDraft.value = nextDraft
    toolsError.value = ''
  },
  { immediate: true },
)

watch(
  () => `${projectId.value}|${resolvedAgentSettings.value.enabledAgentIds.join(',')}|${resolvedAgentSettings.value.enabledTeamIds.join(',')}`,
  () => {
    enabledAgentIds.value = [...resolvedAgentSettings.value.enabledAgentIds]
    enabledTeamIds.value = [...resolvedAgentSettings.value.enabledTeamIds]
    agentsError.value = ''
  },
  { immediate: true },
)

watch(
  () => `${projectId.value}|${workspaceUsers.value.map(user => `${user.id}:${user.scopeProjectIds.join(',')}`).join('|')}`,
  () => {
    selectedMemberUserIds.value = [...currentMemberUserIds.value]
    usersError.value = ''
  },
  { immediate: true },
)

watch(
  () => [...modelsForm.allowedConfiguredModelIds].join(','),
  (value) => {
    const allowedIds = value ? value.split(',').filter(Boolean) : []
    if (!allowedIds.length) {
      modelsForm.defaultConfiguredModelId = ''
      return
    }
    if (!allowedIds.includes(modelsForm.defaultConfiguredModelId)) {
      modelsForm.defaultConfiguredModelId = allowedIds[0] ?? ''
    }
  },
)

const statusLabel = computed(() => {
  const status = project.value?.status
  return status === 'archived'
    ? t('projects.status.archived')
    : t('projects.status.active')
})

function badgeTone(status: ProjectRecord['status']) {
  return status === 'archived' ? 'warning' : 'success'
}

function inferWorkspaceToolPermission(entry: WorkspaceToolCatalogEntry): WorkspaceToolPermissionMode {
  const matchedTool = catalogStore.tools.find(tool =>
    tool.kind === entry.kind
    && tool.name.trim().toLowerCase() === entry.name.trim().toLowerCase(),
  )
  if (matchedTool) {
    return matchedTool.permissionMode
  }

  switch (entry.requiredPermission) {
    case 'readonly':
      return 'readonly'
    case 'workspace-write':
    case 'danger-full-access':
      return 'ask'
    default:
      return 'allow'
  }
}

function resolveToolSelection(sourceKey: string) {
  return toolPermissionDraft.value[sourceKey] ?? 'inherit'
}

function toolPermissionSummaryLabel(entry: WorkspaceToolCatalogEntry) {
  const selection = resolveToolSelection(entry.sourceKey)
  if (selection === 'inherit') {
    return `${t('projectSettings.tools.modes.inherit')} · ${t(`projectSettings.tools.modes.${inferWorkspaceToolPermission(entry)}`)}`
  }
  return t(`projectSettings.tools.modes.${selection}`)
}

function updateToolPermission(sourceKey: string, nextValue: string) {
  toolPermissionDraft.value = {
    ...toolPermissionDraft.value,
    [sourceKey]: TOOL_PERMISSION_VALUES.includes(nextValue as ToolPermissionSelection)
      ? nextValue as ToolPermissionSelection
      : 'inherit',
  }
}

function resetBasics() {
  basicsForm.name = project.value?.name ?? ''
  basicsForm.description = project.value?.description ?? ''
  basicsError.value = ''
}

function resetModels() {
  modelsForm.allowedConfiguredModelIds = [...resolvedModelSettings.value.allowedConfiguredModelIds]
  modelsForm.defaultConfiguredModelId = resolvedModelSettings.value.defaultConfiguredModelId
  modelsError.value = ''
}

function resetTools() {
  toolPermissionDraft.value = Object.fromEntries(
    allowedToolEntries.value.map(entry => {
      const override = resolvedToolSettings.value.overrides[entry.sourceKey]
      const disabled = !resolvedToolSettings.value.enabledSourceKeys.includes(entry.sourceKey)
      return [entry.sourceKey, disabled ? 'deny' : (override?.permissionMode ?? 'inherit')]
    }),
  ) as Record<string, ToolPermissionSelection>
  toolsError.value = ''
}

function resetAgents() {
  enabledAgentIds.value = [...resolvedAgentSettings.value.enabledAgentIds]
  enabledTeamIds.value = [...resolvedAgentSettings.value.enabledTeamIds]
  agentsError.value = ''
}

function resetUsers() {
  selectedMemberUserIds.value = [...currentMemberUserIds.value]
  usersError.value = ''
}

async function submitBasics() {
  if (!project.value || !basicsForm.name.trim() || savingBasics.value) {
    return
  }

  basicsError.value = ''
  savingBasics.value = true

  try {
    const updated = await workspaceStore.updateProject(project.value.id, {
      name: basicsForm.name,
      description: basicsForm.description,
      status: project.value.status,
      assignments: project.value.assignments,
    })
    if (!updated) {
      basicsError.value = workspaceStore.error || t('projectSettings.basics.saveError')
    }
  } finally {
    savingBasics.value = false
  }
}

async function saveModels() {
  if (!project.value || savingModels.value) {
    return
  }

  const allowedConfiguredModelIds = [...new Set(modelsForm.allowedConfiguredModelIds)]
  if (!allowedConfiguredModelIds.length) {
    modelsError.value = t('projectSettings.models.validation.required')
    return
  }
  if (!allowedConfiguredModelIds.includes(modelsForm.defaultConfiguredModelId)) {
    modelsError.value = t('projectSettings.models.validation.defaultMustBeAllowed')
    return
  }

  modelsError.value = ''
  savingModels.value = true

  try {
    const saved = await workspaceStore.saveProjectModelSettings(project.value.id, {
      allowedConfiguredModelIds,
      defaultConfiguredModelId: modelsForm.defaultConfiguredModelId,
    })
    if (!saved) {
      modelsError.value = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
        || workspaceStore.error
        || t('projectSettings.models.saveError')
    }
  } finally {
    savingModels.value = false
  }
}

async function saveTools() {
  if (!project.value || savingTools.value) {
    return
  }

  toolsError.value = ''
  savingTools.value = true

  try {
    const enabledSourceKeys = allowedToolEntries.value
      .map(entry => entry.sourceKey)
      .filter(sourceKey => resolveToolSelection(sourceKey) !== 'deny')
    const overrides = Object.fromEntries(
      allowedToolEntries.value.flatMap((entry) => {
        const selection = resolveToolSelection(entry.sourceKey)
        if (selection === 'inherit' || selection === 'deny' || selection === inferWorkspaceToolPermission(entry)) {
          return []
        }
        return [[entry.sourceKey, { permissionMode: selection }]]
      }),
    )
    const saved = await workspaceStore.saveProjectToolSettings(project.value.id, { enabledSourceKeys, overrides })
    if (!saved) {
      toolsError.value = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
        || workspaceStore.error
        || t('projectSettings.tools.saveError')
    }
  } finally {
    savingTools.value = false
  }
}

async function saveAgents() {
  if (!project.value || savingAgents.value) {
    return
  }

  agentsError.value = ''
  savingAgents.value = true

  try {
    const saved = await workspaceStore.saveProjectAgentSettings(project.value.id, {
      enabledAgentIds: [...new Set(enabledAgentIds.value)],
      enabledTeamIds: [...new Set(enabledTeamIds.value)],
    })
    if (!saved) {
      agentsError.value = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
        || workspaceStore.error
        || t('projectSettings.agents.saveError')
    }
  } finally {
    savingAgents.value = false
  }
}

async function saveUsers() {
  if (!project.value || savingUsers.value) {
    return
  }

  usersError.value = ''
  savingUsers.value = true

  try {
    await userCenterStore.setProjectMembers(project.value.id, selectedMemberUserIds.value)
  } catch (cause) {
    usersError.value = cause instanceof Error ? cause.message : t('projectSettings.users.saveError')
  } finally {
    savingUsers.value = false
  }
}
</script>

<template>
  <div
    v-if="viewReady"
    class="flex w-full flex-col gap-6 pb-20"
    data-testid="project-settings-view"
  >
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('projectSettings.header.eyebrow')"
        :title="project?.name ?? t('projectSettings.header.title')"
        :subtitle="project?.description || t('projectSettings.header.subtitle')"
      />
    </header>

    <UiEmptyState
      v-if="!project"
      class="px-2"
      :title="t('projectSettings.emptyTitle')"
      :description="workspaceStore.error || t('projectSettings.emptyDescription')"
    />

    <template v-else>
      <div class="px-2">
        <UiTabs v-model="activeTab" :tabs="tabs" />
      </div>

      <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
        <UiRecordCard
          v-if="activeTab === 'basics'"
          :title="t('projectSettings.basics.title')"
          :description="t('projectSettings.basics.description')"
        >
          <template #eyebrow>
            {{ t('projectSettings.tabs.basics') }}
          </template>
          <template #badges>
            <UiBadge :label="statusLabel" :tone="badgeTone(project.status)" />
          </template>

          <div class="space-y-4">
            <UiField :label="t('projects.fields.name')">
              <UiInput
                v-model="basicsForm.name"
                data-testid="project-settings-name-input"
                :placeholder="t('sidebar.projectTree.inputPlaceholder')"
              />
            </UiField>

            <UiField :label="t('projects.fields.description')">
              <UiTextarea
                v-model="basicsForm.description"
                data-testid="project-settings-description-input"
                :rows="8"
              />
            </UiField>

            <p v-if="basicsError" class="text-sm text-status-error">
              {{ basicsError }}
            </p>
          </div>

          <template #actions>
            <UiButton variant="ghost" :disabled="savingBasics" @click="resetBasics">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton
              data-testid="project-settings-save-button"
              :disabled="savingBasics || !basicsForm.name.trim()"
              @click="submitBasics"
            >
              {{ t('common.save') }}
            </UiButton>
          </template>
        </UiRecordCard>

        <UiRecordCard
          v-else-if="activeTab === 'models'"
          :title="t('projectSettings.models.title')"
          :description="t('projectSettings.models.description')"
        >
          <template #eyebrow>
            {{ t('projectSettings.tabs.models') }}
          </template>

          <div v-if="!modelTabReady" class="text-sm text-text-secondary">
            {{ t('projectSettings.loading') }}
          </div>

          <UiEmptyState
            v-else-if="!allowedWorkspaceConfiguredModels.length"
            :title="t('projectSettings.models.emptyTitle')"
            :description="t('projectSettings.models.emptyDescription')"
          />

          <div v-else class="space-y-5">
            <UiField
              :label="t('projectSettings.models.allowedLabel')"
              :hint="t('projectSettings.models.allowedHint')"
            >
              <div class="space-y-3">
                <label
                  v-for="modelOption in allowedWorkspaceConfiguredModels"
                  :key="modelOption.value"
                  class="flex items-start justify-between gap-4 rounded-2xl border border-border/40 bg-card/70 px-4 py-3 transition-colors dark:border-white/[0.08]"
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
                    v-model="modelsForm.allowedConfiguredModelIds"
                    :value="modelOption.value"
                    :aria-label="modelOption.label"
                  />
                </label>
              </div>
            </UiField>

            <UiField
              :label="t('projectSettings.models.defaultLabel')"
              :hint="t('projectSettings.models.defaultHint')"
            >
              <UiSelect
                v-model="modelsForm.defaultConfiguredModelId"
                :disabled="!modelsForm.allowedConfiguredModelIds.length"
                :options="allowedWorkspaceConfiguredModels
                  .filter(option => modelsForm.allowedConfiguredModelIds.includes(option.value))
                  .map(option => ({
                    value: option.value,
                    label: `${option.label} · ${option.providerLabel}`,
                  }))"
              />
            </UiField>

            <p v-if="modelsError" class="text-sm text-status-error">
              {{ modelsError }}
            </p>
          </div>

          <template #actions>
            <UiButton variant="ghost" :disabled="savingModels" @click="resetModels">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton :disabled="savingModels || !allowedWorkspaceConfiguredModels.length" @click="saveModels">
              {{ t('common.save') }}
            </UiButton>
          </template>
        </UiRecordCard>

        <UiRecordCard
          v-else-if="activeTab === 'tools'"
          :title="t('projectSettings.tools.title')"
          :description="t('projectSettings.tools.description')"
        >
          <template #eyebrow>
            {{ t('projectSettings.tabs.tools') }}
          </template>

          <UiEmptyState
            v-if="!toolSections.length"
            :title="t('projectSettings.tools.emptyTitle')"
            :description="t('projectSettings.tools.emptyDescription')"
          />

          <div v-else class="space-y-6">
            <section
              v-for="section in toolSections"
              :key="section.kind"
              class="space-y-3"
            >
              <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
                {{ t(`projectSettings.tools.groups.${section.kind}`) }}
              </div>

              <div class="space-y-3">
                <div
                  v-for="entry in section.entries"
                  :key="entry.sourceKey"
                  class="rounded-2xl border border-border/40 bg-card/70 px-4 py-3 dark:border-white/[0.08]"
                >
                  <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                    <div class="min-w-0 space-y-1">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="text-sm font-semibold text-text-primary">{{ entry.name }}</span>
                        <UiBadge
                          v-if="entry.requiredPermission"
                          :label="t(`tools.requiredPermissions.${entry.requiredPermission}`)"
                          subtle
                        />
                      </div>
                      <p class="text-xs leading-5 text-text-secondary">
                        {{ entry.description }}
                      </p>
                      <p class="text-[11px] text-text-tertiary">
                        {{ entry.sourceKey }}
                      </p>
                    </div>

                    <div class="w-full max-w-[15rem] space-y-1">
                      <UiSelect
                        :model-value="resolveToolSelection(entry.sourceKey)"
                        :options="toolPermissionOptions"
                        @update:model-value="updateToolPermission(entry.sourceKey, $event)"
                      />
                      <div class="text-[11px] text-text-tertiary">
                        {{ toolPermissionSummaryLabel(entry) }}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </section>

            <p v-if="toolsError" class="text-sm text-status-error">
              {{ toolsError }}
            </p>
          </div>

          <template #actions>
            <UiButton variant="ghost" :disabled="savingTools" @click="resetTools">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton :disabled="savingTools || !toolSections.length" @click="saveTools">
              {{ t('common.save') }}
            </UiButton>
          </template>
        </UiRecordCard>

        <UiRecordCard
          v-else-if="activeTab === 'agents'"
          :title="t('projectSettings.agents.title')"
          :description="t('projectSettings.agents.description')"
        >
          <template #eyebrow>
            {{ t('projectSettings.tabs.agents') }}
          </template>

          <UiEmptyState
            v-if="!workspaceAssignedAgents.length && !workspaceAssignedTeams.length"
            :title="t('projectSettings.agents.emptyTitle')"
            :description="t('projectSettings.agents.emptyDescription')"
          />

          <div v-else class="space-y-6">
            <section v-if="workspaceAssignedAgents.length" class="space-y-3">
              <UiField
                :label="t('projectSettings.agents.agentsLabel')"
                :hint="t('projectSettings.agents.agentsHint')"
              >
                <div class="space-y-3">
                  <label
                    v-for="agent in workspaceAssignedAgents"
                    :key="agent.id"
                    class="flex items-start justify-between gap-4 rounded-2xl border border-border/40 bg-card/70 px-4 py-3 dark:border-white/[0.08]"
                  >
                    <div class="min-w-0 space-y-1">
                      <div class="text-sm font-semibold text-text-primary">
                        {{ agent.name }}
                      </div>
                      <div class="text-xs text-text-secondary">
                        {{ agent.description || t('common.na') }}
                      </div>                    </div>
                    <UiCheckbox
                      v-model="enabledAgentIds"
                      :value="agent.id"
                      :aria-label="agent.name"
                    />
                  </label>
                </div>
              </UiField>
            </section>

            <section v-if="workspaceAssignedTeams.length" class="space-y-3">
              <UiField
                :label="t('projectSettings.agents.teamsLabel')"
                :hint="t('projectSettings.agents.teamsHint')"
              >
                <div class="space-y-3">
                  <label
                    v-for="team in workspaceAssignedTeams"
                    :key="team.id"
                    class="flex items-start justify-between gap-4 rounded-2xl border border-border/40 bg-card/70 px-4 py-3 dark:border-white/[0.08]"
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
                      v-model="enabledTeamIds"
                      :value="team.id"
                      :aria-label="team.name"
                    />
                  </label>
                </div>
              </UiField>
            </section>

            <p v-if="agentsError" class="text-sm text-status-error">
              {{ agentsError }}
            </p>
          </div>

          <template #actions>
            <UiButton variant="ghost" :disabled="savingAgents" @click="resetAgents">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton
              :disabled="savingAgents || (!workspaceAssignedAgents.length && !workspaceAssignedTeams.length)"
              @click="saveAgents"
            >
              {{ t('common.save') }}
            </UiButton>
          </template>
        </UiRecordCard>

        <UiRecordCard
          v-else
          :title="t('projectSettings.users.title')"
          :description="t('projectSettings.users.description')"
        >
          <template #eyebrow>
            {{ t('projectSettings.tabs.users') }}
          </template>

          <UiEmptyState
            v-if="!workspaceUsers.length"
            :title="t('projectSettings.users.emptyTitle')"
            :description="t('projectSettings.users.emptyDescription')"
          />

          <div v-else class="space-y-3">
            <label
              v-for="user in workspaceUsers"
              :key="user.id"
              class="flex items-start justify-between gap-4 rounded-2xl border border-border/40 bg-card/70 px-4 py-3 dark:border-white/[0.08]"
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
                v-model="selectedMemberUserIds"
                :value="user.id"
                :aria-label="user.displayName || user.username"
              />
            </label>

            <p v-if="usersError" class="text-sm text-status-error">
              {{ usersError }}
            </p>
          </div>

          <template #actions>
            <UiButton variant="ghost" :disabled="savingUsers" @click="resetUsers">
              {{ t('common.reset') }}
            </UiButton>
            <UiButton :disabled="savingUsers || !workspaceUsers.length" @click="saveUsers">
              {{ t('common.save') }}
            </UiButton>
          </template>
        </UiRecordCard>

        <UiRecordCard
          :title="t('projectSettings.summary.title')"
          :description="t('projectSettings.summary.description')"
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
        </UiRecordCard>
      </div>
    </template>
  </div>
</template>
