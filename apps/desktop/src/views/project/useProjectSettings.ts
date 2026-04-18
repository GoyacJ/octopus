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
  resolveProjectGrantedAgents,
  resolveProjectGrantedTeams,
  resolveProjectGrantedToolEntries,
  type ProjectGrantState,
  type ProjectRuntimeRefinementState,
  type ToolPermissionSelection,
} from '@/stores/project_setup'
import { resolveProjectAgentSettings } from '@/stores/project_settings'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

export type ProjectSettingsTab = 'basics' | 'models' | 'tools' | 'agents' | 'users'
export type { ToolPermissionSelection } from '@/stores/project_setup'

export interface ToolSection {
  kind: WorkspaceToolKind
  entries: CapabilityAssetManifest[]
}

export type ActorDialogTab = 'agents' | 'teams'

type DialogKey =
  | 'leader'
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

function matchesQuery(values: Array<string | undefined>, query: string) {
  const normalizedQuery = query.trim().toLowerCase()
  if (!normalizedQuery) {
    return true
  }
  return values.some(value => value?.toLowerCase().includes(normalizedQuery))
}

function isProjectOwnedTool(entry: CapabilityAssetManifest, projectId: string) {
  return entry.ownerScope === 'project' && entry.ownerId === projectId
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
  const leaderDraft = ref('')
  const memberDraft = ref<string[]>([])

  const grantToolTab = ref<WorkspaceToolKind>('builtin')
  const runtimeToolTab = ref<WorkspaceToolKind>('builtin')
  const grantActorTab = ref<ActorDialogTab>('agents')
  const runtimeActorTab = ref<ActorDialogTab>('agents')
  const grantToolSearchQuery = ref('')
  const runtimeToolSearchQuery = ref('')
  const grantActorSearchQuery = ref('')
  const runtimeActorSearchQuery = ref('')

  const dialogOpen = reactive<Record<DialogKey, boolean>>({
    leader: false,
    grantModels: false,
    grantTools: false,
    grantActors: false,
    runtimeModels: false,
    runtimeTools: false,
    runtimeActors: false,
    members: false,
  })
  const saving = reactive<Record<DialogKey, boolean>>({
    leader: false,
    grantModels: false,
    grantTools: false,
    grantActors: false,
    runtimeModels: false,
    runtimeTools: false,
    runtimeActors: false,
    members: false,
  })
  const dialogErrors = reactive<Record<DialogKey, string>>({
    leader: '',
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
    disabledToolSourceKeys: [],
    toolPermissionDraft: {},
    disabledAgentIds: [],
    disabledTeamIds: [],
  })

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
  const workspaceEnabledToolEntries = computed<CapabilityAssetManifest[]>(() =>
    catalogStore.managementProjection.assets.filter(entry => entry.enabled),
  )
  const workspaceActiveAgents = computed<AgentRecord[]>(() =>
    agentStore.workspaceAgents.filter((agent: AgentRecord) => agent.status === 'active'),
  )
  const workspaceActiveTeams = computed<TeamRecord[]>(() =>
    teamStore.workspaceTeams.filter((team: TeamRecord) => team.status === 'active'),
  )
  const allProjectOwnedAgents = computed<AgentRecord[]>(() =>
    agentStore.agents.filter((agent: AgentRecord) => agent.projectId === projectId.value),
  )
  const allProjectOwnedTeams = computed<TeamRecord[]>(() =>
    teamStore.teams.filter((team: TeamRecord) => team.projectId === projectId.value),
  )
  const projectOwnedAgents = computed<AgentRecord[]>(() =>
    allProjectOwnedAgents.value.filter((agent: AgentRecord) => agent.status === 'active'),
  )
  const projectOwnedTeams = computed<TeamRecord[]>(() =>
    allProjectOwnedTeams.value.filter((team: TeamRecord) => team.status === 'active'),
  )
  const projectOwnedToolEntries = computed<CapabilityAssetManifest[]>(() =>
    workspaceEnabledToolEntries.value.filter(entry => isProjectOwnedTool(entry, projectId.value)),
  )
  const workspaceToolEntries = computed<CapabilityAssetManifest[]>(() =>
    workspaceEnabledToolEntries.value.filter(entry => !isProjectOwnedTool(entry, projectId.value)),
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
    resolveProjectGrantedToolEntries(project.value, workspaceEnabledToolEntries.value),
  )
  const grantedAgents = computed<AgentRecord[]>(() =>
    resolveProjectGrantedAgents(
      project.value,
      workspaceActiveAgents.value,
      projectOwnedAgents.value,
    ),
  )
  const grantedTeams = computed<TeamRecord[]>(() =>
    resolveProjectGrantedTeams(
      project.value,
      workspaceActiveTeams.value,
      projectOwnedTeams.value,
    ),
  )
  const grantedWorkspaceAgents = computed<AgentRecord[]>(() =>
    grantedAgents.value.filter((agent: AgentRecord) => !agent.projectId),
  )
  const grantedWorkspaceTeams = computed<TeamRecord[]>(() =>
    grantedTeams.value.filter((team: TeamRecord) => !team.projectId),
  )
  const grantedProjectOwnedTools = computed(() =>
    grantedToolEntries.value.filter(entry => isProjectOwnedTool(entry, projectId.value)),
  )
  const grantedWorkspaceTools = computed(() =>
    grantedToolEntries.value.filter(entry => !isProjectOwnedTool(entry, projectId.value)),
  )

  const runtimeState = computed(() => buildProjectRuntimeRefinementState({
    projectSettings: projectSettings.value,
    assignedConfiguredModels: grantedConfiguredModels.value,
    assignmentDefaultConfiguredModelId: grantState.value.defaultConfiguredModelId,
    grantedToolEntries: grantedToolEntries.value,
    workspaceTools: catalogStore.tools,
    grantedAgentIds: grantedAgents.value.map(agent => agent.id),
    grantedTeamIds: grantedTeams.value.map(team => team.id),
  }))
  const capabilitySummary = computed(() =>
    buildProjectCapabilitySummary({
      project: project.value,
      projectSettings: projectSettings.value,
      grantedConfiguredModels: grantedConfiguredModels.value,
      grantedToolEntries: grantedToolEntries.value,
      workspaceTools: catalogStore.tools,
      grantedAgentIds: grantedAgents.value.map(agent => agent.id),
      grantedTeamIds: grantedTeams.value.map(team => team.id),
    }),
  )

  const currentLeaderAgentId = computed(() => project.value?.leaderAgentId?.trim() ?? '')
  const currentLeader = computed(() =>
    workspaceActiveAgents.value.find((agent: AgentRecord) => agent.id === currentLeaderAgentId.value) ?? null,
  )
  const enabledWorkspaceLeaderAgents = computed<AgentRecord[]>(() =>
    grantedWorkspaceAgents.value.filter((agent: AgentRecord) =>
      !runtimeState.value.disabledAgentIds.includes(agent.id),
    ),
  )
  const leaderOptions = computed(() =>
    enabledWorkspaceLeaderAgents.value.map(agent => ({
      value: agent.id,
      label: agent.name,
    })),
  )
  const currentLeaderLabel = computed(() =>
    currentLeader.value?.name || t('projects.leader.none'),
  )

  const toolTabs = computed(() =>
    TOOL_GROUP_ORDER.map(kind => ({
      value: kind,
      label: t(`projectSettings.tools.groups.${kind}`),
    })),
  )
  const actorTabs = computed(() => ([
    {
      value: 'agents',
      label: t('projectSettings.agents.sections.agents'),
    },
    {
      value: 'teams',
      label: t('projectSettings.agents.sections.teams'),
    },
  ]))

  const filteredGrantToolEntries = computed(() =>
    workspaceToolEntries.value.filter(entry =>
      entry.kind === grantToolTab.value
      && matchesQuery([entry.name, entry.description, entry.sourceKey, entry.displayPath], grantToolSearchQuery.value),
    ),
  )
  const filteredRuntimeToolEntries = computed(() =>
    grantedToolEntries.value.filter(entry =>
      entry.kind === runtimeToolTab.value
      && matchesQuery([entry.name, entry.description, entry.sourceKey, entry.displayPath], runtimeToolSearchQuery.value),
    ),
  )
  const filteredGrantAgents = computed(() =>
    workspaceActiveAgents.value.filter(agent =>
      matchesQuery([agent.name, agent.description, agent.id], grantActorSearchQuery.value),
    ),
  )
  const filteredGrantTeams = computed(() =>
    workspaceActiveTeams.value.filter(team =>
      matchesQuery([team.name, team.description, team.id], grantActorSearchQuery.value),
    ),
  )
  const filteredRuntimeAgents = computed(() =>
    grantedAgents.value.filter(agent =>
      matchesQuery([agent.name, agent.description, agent.id], runtimeActorSearchQuery.value),
    ),
  )
  const filteredRuntimeTeams = computed(() =>
    grantedTeams.value.filter(team =>
      matchesQuery([team.name, team.description, team.id], runtimeActorSearchQuery.value),
    ),
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

  function buildAssignmentsOverride(next: {
    models?: NonNullable<ProjectRecord['assignments']>['models']
    tools?: NonNullable<ProjectRecord['assignments']>['tools']
    agents?: NonNullable<ProjectRecord['assignments']>['agents']
  }) {
    return {
      ...(project.value?.assignments ?? {}),
      ...(next.models !== undefined ? { models: next.models } : {}),
      ...(next.tools !== undefined ? { tools: next.tools } : {}),
      ...(next.agents !== undefined ? { agents: next.agents } : {}),
    }
  }

  function resetLeader() {
    leaderDraft.value = currentLeaderAgentId.value
    dialogErrors.leader = ''
  }

  function resetGrantModels() {
    grantForm.assignedConfiguredModelIds = [...grantState.value.assignedConfiguredModelIds]
    grantForm.defaultConfiguredModelId = grantState.value.defaultConfiguredModelId
    dialogErrors.grantModels = ''
  }

  function resetGrantTools() {
    const excluded = new Set(project.value?.assignments?.tools?.excludedSourceKeys ?? [])
    grantForm.assignedToolSourceKeys = workspaceToolEntries.value
      .filter(entry => !excluded.has(entry.sourceKey))
      .map(entry => entry.sourceKey)
    grantToolSearchQuery.value = ''
    grantToolTab.value = TOOL_GROUP_ORDER.find(kind =>
      workspaceToolEntries.value.some(entry => entry.kind === kind),
    ) ?? 'builtin'
    dialogErrors.grantTools = ''
  }

  function resetGrantActors() {
    const excludedAgentIds = new Set(project.value?.assignments?.agents?.excludedAgentIds ?? [])
    const excludedTeamIds = new Set(project.value?.assignments?.agents?.excludedTeamIds ?? [])
    grantForm.assignedAgentIds = workspaceActiveAgents.value
      .filter(agent => !excludedAgentIds.has(agent.id))
      .map(agent => agent.id)
    grantForm.assignedTeamIds = workspaceActiveTeams.value
      .filter(team => !excludedTeamIds.has(team.id))
      .map(team => team.id)
    grantActorSearchQuery.value = ''
    grantActorTab.value = 'agents'
    dialogErrors.grantActors = ''
  }

  function resetRuntimeModels() {
    runtimeForm.allowedConfiguredModelIds = [...runtimeState.value.allowedConfiguredModelIds]
    runtimeForm.defaultConfiguredModelId = runtimeState.value.defaultConfiguredModelId
    runtimeForm.totalTokens = runtimeState.value.totalTokens
    dialogErrors.runtimeModels = ''
  }

  function resetRuntimeTools() {
    runtimeForm.disabledToolSourceKeys = [...runtimeState.value.disabledToolSourceKeys]
    runtimeForm.toolPermissionDraft = { ...runtimeState.value.toolPermissionDraft }
    runtimeToolSearchQuery.value = ''
    runtimeToolTab.value = TOOL_GROUP_ORDER.find(kind =>
      grantedToolEntries.value.some(entry => entry.kind === kind),
    ) ?? 'builtin'
    dialogErrors.runtimeTools = ''
  }

  function resetRuntimeActors() {
    runtimeForm.disabledAgentIds = [...runtimeState.value.disabledAgentIds]
    runtimeForm.disabledTeamIds = [...runtimeState.value.disabledTeamIds]
    runtimeActorSearchQuery.value = ''
    runtimeActorTab.value = 'agents'
    dialogErrors.runtimeActors = ''
  }

  function resetMembers() {
    memberDraft.value = [...(project.value?.memberUserIds ?? [])]
    dialogErrors.members = ''
  }

  watch(
    () => [
      projectId.value,
      currentLeaderAgentId.value,
      leaderOptions.value.map(option => option.value).join(','),
      grantState.value.assignedConfiguredModelIds.join(','),
      grantState.value.defaultConfiguredModelId,
      workspaceToolEntries.value.map(entry => `${entry.kind}:${entry.sourceKey}`).join(','),
      workspaceActiveAgents.value.map(agent => agent.id).join(','),
      workspaceActiveTeams.value.map(team => team.id).join(','),
      JSON.stringify(project.value?.assignments ?? {}),
    ].join('|'),
    () => {
      resetLeader()
      resetGrantModels()
      resetGrantTools()
      resetGrantActors()
    },
    { immediate: true },
  )

  watch(
    () => [
      projectId.value,
      runtimeState.value.allowedConfiguredModelIds.join(','),
      runtimeState.value.defaultConfiguredModelId,
      runtimeState.value.totalTokens,
      runtimeState.value.disabledToolSourceKeys.join(','),
      JSON.stringify(runtimeState.value.toolPermissionDraft),
      runtimeState.value.disabledAgentIds.join(','),
      runtimeState.value.disabledTeamIds.join(','),
    ].join('|'),
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

  function isLeaderAgent(agentId: string) {
    return currentLeaderAgentId.value === agentId
  }

  function isProjectOwnedAgentRecord(agent: AgentRecord) {
    return agent.projectId === projectId.value
  }

  function isProjectOwnedTeamRecord(team: TeamRecord) {
    return team.projectId === projectId.value
  }

  function toolOriginBadge(entry: CapabilityAssetManifest) {
    return isProjectOwnedTool(entry, projectId.value)
      ? t('projectSettings.labels.projectOwned')
      : t('projectSettings.labels.inherited')
  }

  function actorOriginBadge(record: AgentRecord | TeamRecord) {
    return record.projectId === projectId.value
      ? t('projectSettings.labels.projectOwned')
      : t('projectSettings.labels.inherited')
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

  function isGrantToolEnabled(sourceKey: string) {
    return grantForm.assignedToolSourceKeys.includes(sourceKey)
  }

  function setGrantToolEnabled(sourceKey: string, enabled: boolean) {
    const current = new Set(grantForm.assignedToolSourceKeys)
    if (enabled) {
      current.add(sourceKey)
    } else {
      current.delete(sourceKey)
    }
    grantForm.assignedToolSourceKeys = [...current]
  }

  function isRuntimeToolEnabled(sourceKey: string) {
    return resolveRuntimeToolSelection(sourceKey) !== 'deny'
  }

  function setRuntimeToolEnabled(sourceKey: string, enabled: boolean) {
    updateRuntimeToolPermission(sourceKey, enabled ? 'inherit' : 'deny')
  }

  function isGrantAgentEnabled(agentId: string) {
    return grantForm.assignedAgentIds.includes(agentId)
  }

  function isGrantTeamEnabled(teamId: string) {
    return grantForm.assignedTeamIds.includes(teamId)
  }

  function isRuntimeAgentEnabled(agentId: string) {
    return !runtimeForm.disabledAgentIds.includes(agentId)
  }

  function isRuntimeTeamEnabled(teamId: string) {
    return !runtimeForm.disabledTeamIds.includes(teamId)
  }

  function setGrantAgentEnabled(agentId: string, enabled: boolean) {
    if (isLeaderAgent(agentId) && !enabled) {
      dialogErrors.grantActors = t('projectSettings.leader.mustRemainEnabled')
      return
    }
    dialogErrors.grantActors = ''
    const current = new Set(grantForm.assignedAgentIds)
    if (enabled) {
      current.add(agentId)
    } else {
      current.delete(agentId)
    }
    grantForm.assignedAgentIds = [...current]
  }

  function setGrantTeamEnabled(teamId: string, enabled: boolean) {
    dialogErrors.grantActors = ''
    const current = new Set(grantForm.assignedTeamIds)
    if (enabled) {
      current.add(teamId)
    } else {
      current.delete(teamId)
    }
    grantForm.assignedTeamIds = [...current]
  }

  function setRuntimeAgentEnabled(agentId: string, enabled: boolean) {
    if (isLeaderAgent(agentId) && !enabled) {
      dialogErrors.runtimeActors = t('projectSettings.leader.mustRemainEnabled')
      return
    }
    dialogErrors.runtimeActors = ''
    const current = new Set(runtimeForm.disabledAgentIds)
    if (enabled) {
      current.delete(agentId)
    } else {
      current.add(agentId)
    }
    runtimeForm.disabledAgentIds = [...current]
  }

  function setRuntimeTeamEnabled(teamId: string, enabled: boolean) {
    dialogErrors.runtimeActors = ''
    const current = new Set(runtimeForm.disabledTeamIds)
    if (enabled) {
      current.delete(teamId)
    } else {
      current.add(teamId)
    }
    runtimeForm.disabledTeamIds = [...current]
  }

  function selectAllGrantModels() {
    grantForm.assignedConfiguredModelIds = unique(workspaceConfiguredModels.value.map(item => item.value))
  }

  function clearGrantModels() {
    grantForm.assignedConfiguredModelIds = []
  }

  function selectAllGrantTools() {
    const next = new Set(grantForm.assignedToolSourceKeys)
    for (const entry of workspaceToolEntries.value.filter(entry => entry.kind === grantToolTab.value)) {
      next.add(entry.sourceKey)
    }
    grantForm.assignedToolSourceKeys = [...next]
  }

  function clearGrantTools() {
    const next = new Set(grantForm.assignedToolSourceKeys)
    for (const entry of workspaceToolEntries.value.filter(entry => entry.kind === grantToolTab.value)) {
      next.delete(entry.sourceKey)
    }
    grantForm.assignedToolSourceKeys = [...next]
  }

  function selectAllGrantActors() {
    if (grantActorTab.value === 'agents') {
      grantForm.assignedAgentIds = unique(workspaceActiveAgents.value.map(agent => agent.id))
      dialogErrors.grantActors = ''
      return
    }
    grantForm.assignedTeamIds = unique(workspaceActiveTeams.value.map(team => team.id))
    dialogErrors.grantActors = ''
  }

  function clearGrantActors() {
    if (grantActorTab.value === 'agents') {
      if (currentLeaderAgentId.value) {
        dialogErrors.grantActors = t('projectSettings.leader.mustRemainEnabled')
        grantActorSearchQuery.value = ''
        return
      }
      grantForm.assignedAgentIds = []
      dialogErrors.grantActors = ''
      return
    }
    grantForm.assignedTeamIds = []
    dialogErrors.grantActors = ''
  }

  function selectAllRuntimeTools() {
    const nextDraft = { ...runtimeForm.toolPermissionDraft }
    for (const entry of grantedToolEntries.value.filter(entry => entry.kind === runtimeToolTab.value)) {
      if (nextDraft[entry.sourceKey] === 'deny') {
        nextDraft[entry.sourceKey] = 'inherit'
      }
    }
    runtimeForm.toolPermissionDraft = nextDraft
  }

  function clearAllRuntimeTools() {
    const nextDraft = { ...runtimeForm.toolPermissionDraft }
    for (const entry of grantedToolEntries.value.filter(entry => entry.kind === runtimeToolTab.value)) {
      nextDraft[entry.sourceKey] = 'deny'
    }
    runtimeForm.toolPermissionDraft = nextDraft
  }

  function selectAllRuntimeActors() {
    if (runtimeActorTab.value === 'agents') {
      runtimeForm.disabledAgentIds = []
      dialogErrors.runtimeActors = ''
      return
    }
    runtimeForm.disabledTeamIds = []
    dialogErrors.runtimeActors = ''
  }

  function clearAllRuntimeActors() {
    if (runtimeActorTab.value === 'agents') {
      if (currentLeaderAgentId.value) {
        dialogErrors.runtimeActors = t('projectSettings.leader.mustRemainEnabled')
        runtimeActorSearchQuery.value = ''
        return
      }
      runtimeForm.disabledAgentIds = unique(grantedAgents.value.map(agent => agent.id))
      dialogErrors.runtimeActors = ''
      return
    }
    runtimeForm.disabledTeamIds = unique(grantedTeams.value.map(team => team.id))
    dialogErrors.runtimeActors = ''
  }

  function validateLeaderCandidate(nextLeaderAgentId: string, grantedLeaderAgents: AgentRecord[], disabledAgentIds: string[]) {
    const leaderAgentId = nextLeaderAgentId.trim()
    if (!leaderAgentId) {
      return !currentLeaderAgentId.value
    }
    return grantedLeaderAgents.some(agent => agent.id === leaderAgentId && !agent.projectId)
      && !disabledAgentIds.includes(leaderAgentId)
  }

  function validateCurrentLeaderForSave(dialogKey: 'grantActors' | 'runtimeActors', nextGrantedAgents: AgentRecord[], nextDisabledAgentIds: string[]) {
    if (!currentLeaderAgentId.value) {
      dialogErrors[dialogKey] = ''
      return true
    }
    if (validateLeaderCandidate(currentLeaderAgentId.value, nextGrantedAgents, nextDisabledAgentIds)) {
      dialogErrors[dialogKey] = ''
      return true
    }
    dialogErrors[dialogKey] = t('projectSettings.leader.mustRemainEnabled')
    return false
  }

  function statusLabelFor(projectStatus: ProjectRecord['status']) {
    return projectStatus === 'archived'
      ? t('projects.status.archived')
      : t('projects.status.active')
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
      leaderAgentId: overrides.leaderAgentId ?? project.value.leaderAgentId,
      ownerUserId: overrides.ownerUserId ?? project.value.ownerUserId,
      memberUserIds: overrides.memberUserIds ?? project.value.memberUserIds,
      permissionOverrides: overrides.permissionOverrides ?? project.value.permissionOverrides,
      linkedWorkspaceAssets: overrides.linkedWorkspaceAssets ?? project.value.linkedWorkspaceAssets,
      assignments: overrides.assignments ?? project.value.assignments,
    }
  }

  function openLeaderDialog() {
    resetLeader()
    dialogOpen.leader = true
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

  async function saveLeader() {
    if (!project.value || saving.leader) {
      return
    }
    if (!leaderDraft.value) {
      dialogErrors.leader = t('projectSettings.leader.selectPlaceholder')
      return
    }
    if (!validateLeaderCandidate(leaderDraft.value, enabledWorkspaceLeaderAgents.value, runtimeState.value.disabledAgentIds)) {
      dialogErrors.leader = t('projectSettings.leader.invalid')
      return
    }

    dialogErrors.leader = ''
    saving.leader = true
    try {
      const updated = await workspaceStore.updateProject(
        project.value.id,
        buildProjectUpdateInput({ leaderAgentId: leaderDraft.value })!,
      )
      if (!updated) {
        dialogErrors.leader = workspaceStore.error || t('projectSettings.leader.saveError')
        return
      }
      dialogOpen.leader = false
    } finally {
      saving.leader = false
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
      const assignments = buildAssignmentsOverride({
        models: {
          configuredModelIds: assignedConfiguredModelIds,
          defaultConfiguredModelId: assignedConfiguredModelIds.length
            ? (grantForm.defaultConfiguredModelId || assignedConfiguredModelIds[0] || '')
            : '',
        },
      })
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
      const enabledWorkspaceSourceKeys = unique(grantForm.assignedToolSourceKeys)
      const excludedSourceKeys = workspaceToolEntries.value
        .map(entry => entry.sourceKey)
        .filter(sourceKey => !enabledWorkspaceSourceKeys.includes(sourceKey))
      const sourceKeys = unique(grantState.value.assignedToolSourceKeys)
      const assignments = buildAssignmentsOverride({
        tools: { sourceKeys, excludedSourceKeys },
      })
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
      const enabledWorkspaceAgentIds = unique(grantForm.assignedAgentIds)
      const enabledWorkspaceTeamIds = unique(grantForm.assignedTeamIds)
      const assignedAgentIds = unique(grantState.value.assignedAgentIds)
      const assignedTeamIds = unique(grantState.value.assignedTeamIds)
      const excludedAgentIds = workspaceActiveAgents.value
        .map(agent => agent.id)
        .filter(agentId => !enabledWorkspaceAgentIds.includes(agentId))
      const excludedTeamIds = workspaceActiveTeams.value
        .map(team => team.id)
        .filter(teamId => !enabledWorkspaceTeamIds.includes(teamId))
      const nextGrantedAgents = resolveProjectGrantedAgents(
        {
          ...(project.value ?? {}),
          assignments: {
            ...(project.value.assignments ?? {}),
            agents: {
              agentIds: assignedAgentIds,
              teamIds: assignedTeamIds,
              excludedAgentIds,
              excludedTeamIds,
            },
          },
        } as ProjectRecord,
        workspaceActiveAgents.value,
        projectOwnedAgents.value,
      )
      const nextGrantedTeams = resolveProjectGrantedTeams(
        {
          ...(project.value ?? {}),
          assignments: {
            ...(project.value.assignments ?? {}),
            agents: {
              agentIds: assignedAgentIds,
              teamIds: assignedTeamIds,
              excludedAgentIds,
              excludedTeamIds,
            },
          },
        } as ProjectRecord,
        workspaceActiveTeams.value,
        projectOwnedTeams.value,
      )
      const nextRuntimeActors = resolveProjectAgentSettings(
        projectSettings.value,
        nextGrantedAgents.map(agent => agent.id),
        nextGrantedTeams.map(team => team.id),
      )
      if (!validateCurrentLeaderForSave('grantActors', nextGrantedAgents.filter(agent => !agent.projectId), nextRuntimeActors.disabledAgentIds)) {
        return
      }

      const assignments = buildAssignmentsOverride({
        agents: {
          agentIds: assignedAgentIds,
          teamIds: assignedTeamIds,
          excludedAgentIds,
          excludedTeamIds,
        },
      })
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
      const disabledSourceKeys = grantedToolEntries.value
        .map(entry => entry.sourceKey)
        .filter(sourceKey => resolveRuntimeToolSelection(sourceKey) === 'deny')
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
      const saved = await workspaceStore.saveProjectToolSettings(project.value.id, { disabledSourceKeys, overrides })
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
      const disabledAgentIds = unique(runtimeForm.disabledAgentIds)
      const disabledTeamIds = unique(runtimeForm.disabledTeamIds)
      if (!validateCurrentLeaderForSave('runtimeActors', grantedWorkspaceAgents.value, disabledAgentIds)) {
        return
      }
      const saved = await workspaceStore.saveProjectAgentSettings(project.value.id, {
        disabledAgentIds,
        disabledTeamIds,
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

  const statusLabel = computed(() => statusLabelFor(project.value?.status ?? 'active'))

  function badgeTone(status: ProjectRecord['status']) {
    return status === 'archived' ? 'warning' : 'success'
  }

  return {
    t,
    workspaceStore,
    project,
    loadingDependencies,
    leaderDraft,
    leaderOptions,
    currentLeader,
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
  }
}
