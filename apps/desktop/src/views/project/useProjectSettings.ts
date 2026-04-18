import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import type {
  AccessUserRecord,
  AgentRecord,
  CapabilityAssetManifest,
  ProjectRecord,
  TeamRecord,
  WorkspaceToolKind,
  WorkspaceToolPermissionMode,
} from '@octopus/schema'

import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import {
  buildProjectCapabilitySummary,
  buildProjectGrantState,
  buildProjectRuntimeRefinementState,
  inferWorkspaceToolPermission,
  type ProjectGrantState,
  type ProjectRuntimeRefinementState,
  type ToolPermissionSelection,
} from '@/stores/project_setup'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

export type ProjectSettingsTab = 'basics' | 'models' | 'tools' | 'agents' | 'users'

export interface ToolSection {
  kind: WorkspaceToolKind
  entries: CapabilityAssetManifest[]
}

type DialogKey =
  | 'grantModels'
  | 'grantTools'
  | 'grantActors'
  | 'runtimeModels'
  | 'runtimeTools'
  | 'runtimeActors'
  | 'members'

const TOOL_GROUP_ORDER: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']
const TOOL_PERMISSION_VALUES: ToolPermissionSelection[] = ['inherit', 'allow', 'ask', 'readonly', 'deny']

function unique(values: string[]) {
  return [...new Set(values.filter(Boolean))]
}

export function useProjectSettings() {
  const { t } = useI18n()
  const route = useRoute()
  const workspaceStore = useWorkspaceStore()
  const agentStore = useAgentStore()
  const catalogStore = useCatalogStore()
  const teamStore = useTeamStore()
  const workspaceAccessControlStore = useWorkspaceAccessControlStore()

  const loadingDependencies = ref(false)
  const dialogOpen = reactive<Record<DialogKey, boolean>>({
    grantModels: false,
    grantTools: false,
    grantActors: false,
    runtimeModels: false,
    runtimeTools: false,
    runtimeActors: false,
    members: false,
  })
  const saving = reactive<Record<DialogKey, boolean>>({
    grantModels: false,
    grantTools: false,
    grantActors: false,
    runtimeModels: false,
    runtimeTools: false,
    runtimeActors: false,
    members: false,
  })
  const dialogErrors = reactive<Record<DialogKey, string>>({
    grantModels: '',
    grantTools: '',
    grantActors: '',
    runtimeModels: '',
    runtimeTools: '',
    runtimeActors: '',
    members: '',
  })

  const grantForm = reactive<ProjectGrantState>({
    assignedConfiguredModelIds: [],
    defaultConfiguredModelId: '',
    assignedToolSourceKeys: [],
    assignedAgentIds: [],
    assignedTeamIds: [],
    memberUserIds: [],
  })
  const runtimeForm = reactive<ProjectRuntimeRefinementState>({
    allowedConfiguredModelIds: [],
    defaultConfiguredModelId: '',
    totalTokens: '',
    enabledToolSourceKeys: [],
    toolPermissionDraft: {},
    enabledAgentIds: [],
    enabledTeamIds: [],
  })
  const memberDraft = ref<string[]>([])

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
  const workspaceConfiguredModels = computed(() => catalogStore.configuredModelOptions)
  const workspaceToolEntries = computed(() => catalogStore.managementProjection.assets.filter(entry => entry.enabled))
  const workspaceToolSections = computed<ToolSection[]>(() =>
    TOOL_GROUP_ORDER
      .map(kind => ({
        kind,
        entries: workspaceToolEntries.value.filter(entry => entry.kind === kind),
      }))
      .filter(section => section.entries.length > 0),
  )
  const actorCandidateAgents = computed<AgentRecord[]>(() => [
    ...agentStore.workspaceOwnedAgents,
    ...agentStore.builtinTemplateAgents,
  ])
  const actorCandidateTeams = computed<TeamRecord[]>(() => [
    ...teamStore.workspaceOwnedTeams,
    ...teamStore.builtinTemplateTeams,
  ])
  const projectOwnedAgents = computed<AgentRecord[]>(() =>
    agentStore.agents.filter(agent => agent.projectId === projectId.value),
  )
  const projectOwnedTeams = computed<TeamRecord[]>(() =>
    teamStore.teams.filter(team => team.projectId === projectId.value),
  )
  const workspaceUsers = computed(() => {
    const records = new Map<string, AccessUserRecord>()
    for (const member of workspaceAccessControlStore.members) {
      records.set(member.user.id, member.user)
    }

    return [...records.values()].sort((left, right) =>
      (left.displayName || left.username).localeCompare(right.displayName || right.username),
    )
  })
  const grantState = computed(() => buildProjectGrantState(project.value))
  const grantedConfiguredModels = computed(() =>
    workspaceConfiguredModels.value.filter(item => grantState.value.assignedConfiguredModelIds.includes(item.value)),
  )
  const grantedToolEntries = computed(() =>
    workspaceToolEntries.value.filter(entry => grantState.value.assignedToolSourceKeys.includes(entry.sourceKey)),
  )
  const grantedToolSections = computed<ToolSection[]>(() =>
    TOOL_GROUP_ORDER
      .map(kind => ({
        kind,
        entries: grantedToolEntries.value.filter(entry => entry.kind === kind),
      }))
      .filter(section => section.entries.length > 0),
  )
  const grantedAgents = computed<AgentRecord[]>(() =>
    actorCandidateAgents.value.filter(agent => grantState.value.assignedAgentIds.includes(agent.id)),
  )
  const grantedTeams = computed<TeamRecord[]>(() =>
    actorCandidateTeams.value.filter(team => grantState.value.assignedTeamIds.includes(team.id)),
  )
  const runtimeState = computed(() => buildProjectRuntimeRefinementState({
    projectSettings: projectSettings.value,
    assignedConfiguredModels: grantedConfiguredModels.value,
    assignmentDefaultConfiguredModelId: grantState.value.defaultConfiguredModelId,
    assignedToolEntries: grantedToolEntries.value,
    workspaceTools: catalogStore.tools,
    assignedAgentIds: grantState.value.assignedAgentIds,
    assignedTeamIds: grantState.value.assignedTeamIds,
  }))
  const capabilitySummary = computed(() =>
    buildProjectCapabilitySummary({
      project: project.value,
      projectSettings: projectSettings.value,
      assignedConfiguredModels: grantedConfiguredModels.value,
      assignedToolEntries: grantedToolEntries.value,
      workspaceTools: catalogStore.tools,
    }),
  )
  const projectUsedTokens = computed(() => workspaceStore.getProjectDashboard(projectId.value)?.usedTokens ?? 0)
  const permissionOverrideCount = computed(() => Object.keys(project.value?.permissionOverrides ?? {}).length)
  const viewReady = computed(() =>
    Boolean(connectionId.value)
    && !loadingDependencies.value
    && (!workspaceStore.loading || Boolean(project.value) || Boolean(workspaceStore.error)),
  )

  const toolPermissionOptions = computed(() =>
    TOOL_PERMISSION_VALUES.map(value => ({
      value,
      label: t(`projectSettings.tools.modes.${value}`),
    })),
  )
  const grantDefaultModelLabel = computed(() =>
    grantedConfiguredModels.value.find(item => item.value === grantState.value.defaultConfiguredModelId)?.label
    || grantedConfiguredModels.value[0]?.label
    || t('common.na'),
  )

  const grantSummary = computed(() => ({
    models: t('projectSettings.sections.grants.modelsValue', {
      granted: capabilitySummary.value.grantedModels,
      defaultModel: grantDefaultModelLabel.value,
    }),
    tools: t('projectSettings.sections.grants.toolsValue', {
      granted: capabilitySummary.value.grantedTools,
    }),
    actors: t('projectSettings.sections.grants.actorsValue', {
      granted: capabilitySummary.value.grantedActors,
    }),
  }))

  const runtimeSummary = computed(() => ({
    models: t('projectSettings.sections.runtime.modelsValue', {
      granted: capabilitySummary.value.grantedModels,
      enabled: capabilitySummary.value.enabledModels,
      defaultModel: capabilitySummary.value.defaultModelLabel || t('common.na'),
      totalTokens: runtimeState.value.totalTokens || t('projectSettings.sections.runtime.unlimited'),
    }),
    tools: t('projectSettings.sections.runtime.toolsValue', {
      enabled: capabilitySummary.value.enabledTools,
      overrides: capabilitySummary.value.toolOverrideCount,
    }),
    actors: t('projectSettings.sections.runtime.actorsValue', {
      granted: capabilitySummary.value.grantedActors,
      enabled: capabilitySummary.value.enabledActors,
    }),
  }))

  const memberSummary = computed(() =>
    t('projectSettings.sections.members.membersValue', {
      members: capabilitySummary.value.memberCount,
      editors: capabilitySummary.value.editableMemberCount,
    }),
  )
  const accessSummary = computed(() =>
    permissionOverrideCount.value
      ? t('projectSettings.sections.members.accessValue', { count: permissionOverrideCount.value })
      : t('projectSettings.sections.members.accessEmpty'),
  )
  const completionItems = computed(() => ([
    {
      id: 'grant-models',
      label: t('projectSettings.nextSteps.grantModels'),
      done: capabilitySummary.value.grantedModels > 0,
    },
    {
      id: 'runtime-models',
      label: t('projectSettings.nextSteps.enableModel'),
      done: capabilitySummary.value.enabledModels > 0,
    },
    {
      id: 'runtime-actors',
      label: t('projectSettings.nextSteps.enableActor'),
      done: capabilitySummary.value.enabledActors > 0,
    },
    {
      id: 'members',
      label: t('projectSettings.nextSteps.addMembers'),
      done: capabilitySummary.value.memberCount > 0,
    },
  ]))
  const completionProgress = computed(() => {
    const completed = completionItems.value.filter(item => item.done).length
    return {
      completed,
      total: completionItems.value.length,
      percent: completionItems.value.length
        ? Math.round((completed / completionItems.value.length) * 100)
        : 0,
    }
  })

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
          workspaceStore.loadProjectDashboard(nextProjectId, nextConnectionId),
          workspaceStore.loadProjectRuntimeConfig(nextProjectId, false, nextConnectionId),
          agentStore.load(nextConnectionId),
          catalogStore.load(nextConnectionId),
          teamStore.load(nextConnectionId),
          workspaceAccessControlStore.loadMembersData(nextConnectionId),
        ])
      } finally {
        loadingDependencies.value = false
      }
    },
    { immediate: true },
  )

  function resetGrantModels() {
    grantForm.assignedConfiguredModelIds = [...grantState.value.assignedConfiguredModelIds]
    grantForm.defaultConfiguredModelId = grantState.value.defaultConfiguredModelId
    dialogErrors.grantModels = ''
  }

  function resetGrantTools() {
    grantForm.assignedToolSourceKeys = [...grantState.value.assignedToolSourceKeys]
    dialogErrors.grantTools = ''
  }

  function resetGrantActors() {
    grantForm.assignedAgentIds = [...grantState.value.assignedAgentIds]
    grantForm.assignedTeamIds = [...grantState.value.assignedTeamIds]
    dialogErrors.grantActors = ''
  }

  function resetRuntimeModels() {
    runtimeForm.allowedConfiguredModelIds = [...runtimeState.value.allowedConfiguredModelIds]
    runtimeForm.defaultConfiguredModelId = runtimeState.value.defaultConfiguredModelId
    runtimeForm.totalTokens = runtimeState.value.totalTokens
    dialogErrors.runtimeModels = ''
  }

  function resetRuntimeTools() {
    runtimeForm.enabledToolSourceKeys = [...runtimeState.value.enabledToolSourceKeys]
    runtimeForm.toolPermissionDraft = { ...runtimeState.value.toolPermissionDraft }
    dialogErrors.runtimeTools = ''
  }

  function resetRuntimeActors() {
    runtimeForm.enabledAgentIds = [...runtimeState.value.enabledAgentIds]
    runtimeForm.enabledTeamIds = [...runtimeState.value.enabledTeamIds]
    dialogErrors.runtimeActors = ''
  }

  function resetMembers() {
    memberDraft.value = [...(project.value?.memberUserIds ?? [])]
    dialogErrors.members = ''
  }

  watch(
    () => `${projectId.value}|${grantState.value.assignedConfiguredModelIds.join(',')}|${grantState.value.defaultConfiguredModelId}|${grantState.value.assignedToolSourceKeys.join(',')}|${grantState.value.assignedAgentIds.join(',')}|${grantState.value.assignedTeamIds.join(',')}`,
    () => {
      resetGrantModels()
      resetGrantTools()
      resetGrantActors()
    },
    { immediate: true },
  )

  watch(
    () => `${projectId.value}|${runtimeState.value.allowedConfiguredModelIds.join(',')}|${runtimeState.value.defaultConfiguredModelId}|${runtimeState.value.totalTokens}|${runtimeState.value.enabledToolSourceKeys.join(',')}|${JSON.stringify(runtimeState.value.toolPermissionDraft)}|${runtimeState.value.enabledAgentIds.join(',')}|${runtimeState.value.enabledTeamIds.join(',')}`,
    () => {
      resetRuntimeModels()
      resetRuntimeTools()
      resetRuntimeActors()
    },
    { immediate: true },
  )

  watch(
    () => `${projectId.value}|${(project.value?.memberUserIds ?? []).join(',')}`,
    () => {
      resetMembers()
    },
    { immediate: true },
  )

  watch(
    () => grantForm.assignedConfiguredModelIds.join(','),
    (value) => {
      const assignedIds = value ? value.split(',').filter(Boolean) : []
      if (!assignedIds.length) {
        grantForm.defaultConfiguredModelId = ''
        return
      }
      if (!assignedIds.includes(grantForm.defaultConfiguredModelId)) {
        grantForm.defaultConfiguredModelId = assignedIds[0] ?? ''
      }
    },
  )

  watch(
    () => runtimeForm.allowedConfiguredModelIds.join(','),
    (value) => {
      const allowedIds = value ? value.split(',').filter(Boolean) : []
      if (!allowedIds.length) {
        runtimeForm.defaultConfiguredModelId = ''
        return
      }
      if (!allowedIds.includes(runtimeForm.defaultConfiguredModelId)) {
        runtimeForm.defaultConfiguredModelId = allowedIds[0] ?? ''
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

  function buildProjectUpdateInput(overrides: Partial<ProjectRecord>) {
    if (!project.value) {
      return null
    }

    return {
      name: overrides.name ?? project.value.name,
      description: overrides.description ?? project.value.description,
      resourceDirectory: overrides.resourceDirectory ?? project.value.resourceDirectory,
      status: overrides.status ?? project.value.status,
      ownerUserId: overrides.ownerUserId ?? project.value.ownerUserId,
      memberUserIds: overrides.memberUserIds ?? project.value.memberUserIds,
      permissionOverrides: overrides.permissionOverrides ?? project.value.permissionOverrides,
      linkedWorkspaceAssets: overrides.linkedWorkspaceAssets ?? project.value.linkedWorkspaceAssets,
      assignments: overrides.assignments ?? project.value.assignments,
    }
  }

  function openGrantModelsDialog() {
    resetGrantModels()
    dialogOpen.grantModels = true
  }

  function openGrantToolsDialog() {
    resetGrantTools()
    dialogOpen.grantTools = true
  }

  function openGrantActorsDialog() {
    resetGrantActors()
    dialogOpen.grantActors = true
  }

  function openRuntimeModelsDialog() {
    resetRuntimeModels()
    dialogOpen.runtimeModels = true
  }

  function openRuntimeToolsDialog() {
    resetRuntimeTools()
    dialogOpen.runtimeTools = true
  }

  function openRuntimeActorsDialog() {
    resetRuntimeActors()
    dialogOpen.runtimeActors = true
  }

  function openMembersDialog() {
    resetMembers()
    dialogOpen.members = true
  }

  function selectAllGrantModels() {
    grantForm.assignedConfiguredModelIds = unique(workspaceConfiguredModels.value.map(item => item.value))
  }

  function clearGrantModels() {
    grantForm.assignedConfiguredModelIds = []
  }

  function selectAllGrantTools() {
    grantForm.assignedToolSourceKeys = unique(workspaceToolEntries.value.map(entry => entry.sourceKey))
  }

  function clearGrantTools() {
    grantForm.assignedToolSourceKeys = []
  }

  function selectAllGrantAgents() {
    grantForm.assignedAgentIds = unique(actorCandidateAgents.value.map(agent => agent.id))
  }

  function clearGrantAgents() {
    grantForm.assignedAgentIds = []
  }

  function selectAllGrantTeams() {
    grantForm.assignedTeamIds = unique(actorCandidateTeams.value.map(team => team.id))
  }

  function clearGrantTeams() {
    grantForm.assignedTeamIds = []
  }

  function resolveRuntimeToolSelection(sourceKey: string) {
    return runtimeForm.toolPermissionDraft[sourceKey] ?? 'inherit'
  }

  function runtimeToolPermissionSummaryLabel(entry: CapabilityAssetManifest) {
    const selection = resolveRuntimeToolSelection(entry.sourceKey)
    if (selection === 'inherit') {
      return `${t('projectSettings.tools.modes.inherit')} · ${t(`projectSettings.tools.modes.${inferWorkspaceToolPermission(entry, catalogStore.tools)}`)}`
    }
    return t(`projectSettings.tools.modes.${selection}`)
  }

  function updateRuntimeToolPermission(sourceKey: string, nextValue: string) {
    runtimeForm.toolPermissionDraft = {
      ...runtimeForm.toolPermissionDraft,
      [sourceKey]: TOOL_PERMISSION_VALUES.includes(nextValue as ToolPermissionSelection)
        ? nextValue as ToolPermissionSelection
        : 'inherit',
    }
  }

  async function saveGrantModels() {
    if (!project.value || saving.grantModels) {
      return
    }

    const assignedConfiguredModelIds = unique(grantForm.assignedConfiguredModelIds)
    if (assignedConfiguredModelIds.length && !assignedConfiguredModelIds.includes(grantForm.defaultConfiguredModelId)) {
      dialogErrors.grantModels = t('projectSettings.models.validation.defaultMustBeAllowed')
      return
    }

    dialogErrors.grantModels = ''
    saving.grantModels = true

    try {
      const assignments = {
        ...(project.value.assignments ?? {}),
        models: assignedConfiguredModelIds.length
          ? {
              configuredModelIds: assignedConfiguredModelIds,
              defaultConfiguredModelId: grantForm.defaultConfiguredModelId || assignedConfiguredModelIds[0] || '',
            }
          : undefined,
      }
      const updated = await workspaceStore.updateProject(
        project.value.id,
        buildProjectUpdateInput({ assignments })!,
      )
      if (!updated) {
        dialogErrors.grantModels = workspaceStore.error || t('projectSettings.sections.grants.saveError')
        return
      }
      dialogOpen.grantModels = false
    } finally {
      saving.grantModels = false
    }
  }

  async function saveGrantTools() {
    if (!project.value || saving.grantTools) {
      return
    }

    dialogErrors.grantTools = ''
    saving.grantTools = true

    try {
      const sourceKeys = unique(grantForm.assignedToolSourceKeys)
      const assignments = {
        ...(project.value.assignments ?? {}),
        tools: sourceKeys.length ? { sourceKeys } : undefined,
      }
      const updated = await workspaceStore.updateProject(
        project.value.id,
        buildProjectUpdateInput({ assignments })!,
      )
      if (!updated) {
        dialogErrors.grantTools = workspaceStore.error || t('projectSettings.sections.grants.saveError')
        return
      }
      dialogOpen.grantTools = false
    } finally {
      saving.grantTools = false
    }
  }

  async function saveGrantActors() {
    if (!project.value || saving.grantActors) {
      return
    }

    dialogErrors.grantActors = ''
    saving.grantActors = true

    try {
      const agentIds = unique(grantForm.assignedAgentIds)
      const teamIds = unique(grantForm.assignedTeamIds)
      const assignments = {
        ...(project.value.assignments ?? {}),
        agents: agentIds.length || teamIds.length
          ? { agentIds, teamIds }
          : undefined,
      }
      const updated = await workspaceStore.updateProject(
        project.value.id,
        buildProjectUpdateInput({ assignments })!,
      )
      if (!updated) {
        dialogErrors.grantActors = workspaceStore.error || t('projectSettings.sections.grants.saveError')
        return
      }
      dialogOpen.grantActors = false
    } finally {
      saving.grantActors = false
    }
  }

  async function saveRuntimeModels() {
    if (!project.value || saving.runtimeModels) {
      return
    }

    const allowedConfiguredModelIds = unique(runtimeForm.allowedConfiguredModelIds)
    if (allowedConfiguredModelIds.length && !allowedConfiguredModelIds.includes(runtimeForm.defaultConfiguredModelId)) {
      dialogErrors.runtimeModels = t('projectSettings.models.validation.defaultMustBeAllowed')
      return
    }

    dialogErrors.runtimeModels = ''
    saving.runtimeModels = true

    try {
      const saved = await workspaceStore.saveProjectModelSettings(project.value.id, {
        allowedConfiguredModelIds,
        defaultConfiguredModelId: allowedConfiguredModelIds.length
          ? runtimeForm.defaultConfiguredModelId
          : '',
        totalTokens: (() => {
          const trimmed = runtimeForm.totalTokens.trim()
          if (!trimmed) {
            return undefined
          }

          const parsed = Number(trimmed)
          return Number.isFinite(parsed) && parsed > 0 ? Math.trunc(parsed) : undefined
        })(),
      })
      if (!saved) {
        dialogErrors.runtimeModels = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
          || workspaceStore.error
          || t('projectSettings.models.saveError')
        return
      }
      dialogOpen.runtimeModels = false
    } finally {
      saving.runtimeModels = false
    }
  }

  async function saveRuntimeTools() {
    if (!project.value || saving.runtimeTools) {
      return
    }

    dialogErrors.runtimeTools = ''
    saving.runtimeTools = true

    try {
      const enabledSourceKeys = grantedToolEntries.value
        .map(entry => entry.sourceKey)
        .filter(sourceKey => resolveRuntimeToolSelection(sourceKey) !== 'deny')
      const overrides = Object.fromEntries(
        grantedToolEntries.value.flatMap((entry) => {
          const selection = resolveRuntimeToolSelection(entry.sourceKey)
          if (
            selection === 'inherit'
            || selection === 'deny'
            || selection === inferWorkspaceToolPermission(entry, catalogStore.tools)
          ) {
            return []
          }
          return [[entry.sourceKey, { permissionMode: selection }]]
        }),
      )
      const saved = await workspaceStore.saveProjectToolSettings(project.value.id, { enabledSourceKeys, overrides })
      if (!saved) {
        dialogErrors.runtimeTools = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
          || workspaceStore.error
          || t('projectSettings.tools.saveError')
        return
      }
      dialogOpen.runtimeTools = false
    } finally {
      saving.runtimeTools = false
    }
  }

  async function saveRuntimeActors() {
    if (!project.value || saving.runtimeActors) {
      return
    }

    dialogErrors.runtimeActors = ''
    saving.runtimeActors = true

    try {
      const saved = await workspaceStore.saveProjectAgentSettings(project.value.id, {
        enabledAgentIds: unique(runtimeForm.enabledAgentIds),
        enabledTeamIds: unique(runtimeForm.enabledTeamIds),
      })
      if (!saved) {
        dialogErrors.runtimeActors = workspaceStore.activeProjectRuntimeValidation?.errors.join(' ')
          || workspaceStore.error
          || t('projectSettings.agents.saveError')
        return
      }
      dialogOpen.runtimeActors = false
    } finally {
      saving.runtimeActors = false
    }
  }

  async function saveMembers() {
    if (!project.value || saving.members) {
      return
    }

    dialogErrors.members = ''
    saving.members = true

    try {
      const updated = await workspaceStore.updateProject(
        project.value.id,
        buildProjectUpdateInput({
          memberUserIds: unique(memberDraft.value),
        })!,
      )
      if (!updated) {
        dialogErrors.members = workspaceStore.error || t('projectSettings.users.saveError')
        return
      }
      dialogOpen.members = false
    } finally {
      saving.members = false
    }
  }

  return {
    t,
    workspaceStore,
    project,
    loadingDependencies,
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
    capabilitySummary,
    grantSummary,
    runtimeSummary,
    memberSummary,
    accessSummary,
    permissionOverrideCount,
    completionItems,
    completionProgress,
    projectUsedTokens,
    viewReady,
    statusLabel,
    badgeTone,
    openGrantModelsDialog,
    openGrantToolsDialog,
    openGrantActorsDialog,
    selectAllGrantModels,
    clearGrantModels,
    selectAllGrantTools,
    clearGrantTools,
    selectAllGrantAgents,
    clearGrantAgents,
    selectAllGrantTeams,
    clearGrantTeams,
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
  }
}
