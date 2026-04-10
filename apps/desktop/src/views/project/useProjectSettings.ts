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

import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

export type ProjectSettingsTab = 'basics' | 'models' | 'tools' | 'agents' | 'users'
export type ToolPermissionSelection = 'inherit' | WorkspaceToolPermissionMode

export interface ToolSection {
  kind: WorkspaceToolKind
  entries: WorkspaceToolCatalogEntry[]
}

const TOOL_TAB_ORDER: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']
const TOOL_PERMISSION_VALUES: ToolPermissionSelection[] = ['inherit', 'allow', 'ask', 'readonly', 'deny']

export function useProjectSettings() {
  const { t } = useI18n()
  const route = useRoute()
  const workspaceStore = useWorkspaceStore()
  const agentStore = useAgentStore()
  const catalogStore = useCatalogStore()
  const teamStore = useTeamStore()
  const workspaceAccessStore = useWorkspaceAccessStore()

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
    [...workspaceAccessStore.users].sort((left, right) =>
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
          workspaceAccessStore.load(nextConnectionId),
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
      await workspaceAccessStore.setProjectMembers(project.value.id, selectedMemberUserIds.value)
    } catch (cause) {
      usersError.value = cause instanceof Error ? cause.message : t('projectSettings.users.saveError')
    } finally {
      savingUsers.value = false
    }
  }

  return {
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
    workspaceAssignedAgents,
    workspaceAssignedTeams,
    workspaceUsers,
    modelTabReady,
    viewReady,
    toolSections,
    summaryAllowedModels,
    summaryOverrideCount,
    summaryActorCount,
    summaryMemberCount,
    toolPermissionOptions,
    loadingDependencies,
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
  }
}
