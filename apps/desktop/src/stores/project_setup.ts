import type {
  AgentRecord,
  CapabilityAssetManifest,
  ProjectModelSettings,
  ProjectRecord,
  ProjectSettingsConfig,
  ProjectWorkspaceAssignments,
  TeamRecord,
  WorkspaceToolPermissionMode,
} from '@octopus/schema'

import type { CatalogConfiguredModelOption } from './catalog'
import {
  resolveProjectAgentSettings,
  resolveProjectModelSettings,
  resolveProjectToolSettings,
} from './project_settings'

export type ToolPermissionSelection = 'inherit' | WorkspaceToolPermissionMode
export type ProjectSetupPreset = 'general' | 'engineering' | 'documentation' | 'advanced'

export interface ProjectGrantState {
  assignedConfiguredModelIds: string[]
  defaultConfiguredModelId: string
  assignedToolSourceKeys: string[]
  assignedAgentIds: string[]
  assignedTeamIds: string[]
  memberUserIds: string[]
}

export interface ProjectRuntimeRefinementState {
  allowedConfiguredModelIds: string[]
  defaultConfiguredModelId: string
  totalTokens: string
  disabledToolSourceKeys: string[]
  toolPermissionDraft: Record<string, ToolPermissionSelection>
  disabledAgentIds: string[]
  disabledTeamIds: string[]
}

export interface ProjectCapabilitySummary {
  grantedModels: number
  enabledModels: number
  defaultModelLabel: string
  grantedTools: number
  enabledTools: number
  toolOverrideCount: number
  grantedActors: number
  enabledActors: number
  memberCount: number
  editableMemberCount: number
}

export interface ProjectSetupPresetSeed {
  assignments?: ProjectWorkspaceAssignments
  modelSettings?: ProjectModelSettings
}

export interface ProjectPresetContext {
  models: CatalogConfiguredModelOption[]
  tools: CapabilityAssetManifest[]
  agents: AgentRecord[]
  teams: TeamRecord[]
}

function unique(values: string[]) {
  return [...new Set(values.filter(Boolean))]
}

function uniqueBy<T>(items: T[], keyOf: (item: T) => string) {
  const seen = new Set<string>()
  return items.filter((item) => {
    const key = keyOf(item)
    if (!key || seen.has(key)) {
      return false
    }
    seen.add(key)
    return true
  })
}

export function inferWorkspaceToolPermission(
  entry: CapabilityAssetManifest,
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>,
): WorkspaceToolPermissionMode {
  const matchedTool = workspaceTools.find(tool =>
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

export function resolveProjectGrantedToolEntries(
  project: ProjectRecord | null,
  toolEntries: CapabilityAssetManifest[],
) {
  const projectId = project?.id ?? ''
  const excludedSourceKeys = new Set(project?.assignments?.tools?.excludedSourceKeys ?? [])

  return uniqueBy(
    toolEntries.filter((entry) => {
      if (!entry.enabled) {
        return false
      }

      const isProjectOwned = entry.ownerScope === 'project' && entry.ownerId === projectId
      if (isProjectOwned) {
        return true
      }

      return entry.ownerScope !== 'project' && !excludedSourceKeys.has(entry.sourceKey)
    }),
    entry => entry.sourceKey,
  )
}

export function resolveProjectGrantedAgents(
  project: ProjectRecord | null,
  workspaceAgents: AgentRecord[],
  projectOwnedAgents: AgentRecord[],
) {
  const excludedAgentIds = new Set(project?.assignments?.agents?.excludedAgentIds ?? [])
  const inheritedAgents = workspaceAgents.filter(agent =>
    !agent.projectId
    && agent.status === 'active'
    && !excludedAgentIds.has(agent.id),
  )

  return uniqueBy(
    [
      ...projectOwnedAgents.filter(agent => agent.status === 'active'),
      ...inheritedAgents,
    ],
    agent => agent.id,
  )
}

export function resolveProjectGrantedTeams(
  project: ProjectRecord | null,
  workspaceTeams: TeamRecord[],
  projectOwnedTeams: TeamRecord[],
) {
  const excludedTeamIds = new Set(project?.assignments?.agents?.excludedTeamIds ?? [])
  const inheritedTeams = workspaceTeams.filter(team =>
    !team.projectId
    && team.status === 'active'
    && !excludedTeamIds.has(team.id),
  )

  return uniqueBy(
    [
      ...projectOwnedTeams.filter(team => team.status === 'active'),
      ...inheritedTeams,
    ],
    team => team.id,
  )
}

export function resolveProjectPreferredActorValue(input: {
  project: ProjectRecord | null
  projectSettings: ProjectSettingsConfig
  grantedAgents: AgentRecord[]
  grantedTeams: TeamRecord[]
}) {
  const resolvedActors = resolveProjectAgentSettings(
    input.projectSettings,
    input.grantedAgents.map(agent => agent.id),
    input.grantedTeams.map(team => team.id),
  )
  const enabledAgents = input.grantedAgents.filter(agent =>
    !resolvedActors.disabledAgentIds.includes(agent.id),
  )
  const enabledTeams = input.grantedTeams.filter(team =>
    !resolvedActors.disabledTeamIds.includes(team.id),
  )
  const leaderAgentId = input.project?.leaderAgentId?.trim() ?? ''

  if (leaderAgentId) {
    const leader = enabledAgents.find(agent =>
      agent.id === leaderAgentId
      && !agent.projectId
      && agent.status === 'active',
    )
    if (leader) {
      return `agent:${leader.id}`
    }
  }

  const preferredAgent = enabledAgents[0]
  if (preferredAgent) {
    return `agent:${preferredAgent.id}`
  }

  const preferredTeam = enabledTeams[0]
  if (preferredTeam) {
    return `team:${preferredTeam.id}`
  }

  return ''
}

export function buildToolPermissionDraft(
  grantedToolEntries: CapabilityAssetManifest[],
  projectSettings: ProjectSettingsConfig,
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>,
): Record<string, ToolPermissionSelection> {
  const resolvedToolSettings = resolveProjectToolSettings(
    projectSettings,
    grantedToolEntries.map(entry => entry.sourceKey),
  )

  return Object.fromEntries(
    grantedToolEntries.map((entry) => {
      const override = resolvedToolSettings.overrides[entry.sourceKey]
      const disabled = resolvedToolSettings.disabledSourceKeys.includes(entry.sourceKey)
      return [
        entry.sourceKey,
        disabled
          ? 'deny'
          : (override?.permissionMode ?? 'inherit'),
      ]
    }),
  ) as Record<string, ToolPermissionSelection>
}

export function buildProjectGrantState(project: ProjectRecord | null): ProjectGrantState {
  const assignments = project?.assignments

  return {
    assignedConfiguredModelIds: unique(assignments?.models?.configuredModelIds ?? []),
    defaultConfiguredModelId: assignments?.models?.defaultConfiguredModelId ?? '',
    assignedToolSourceKeys: unique(assignments?.tools?.sourceKeys ?? []),
    assignedAgentIds: unique(assignments?.agents?.agentIds ?? []),
    assignedTeamIds: unique(assignments?.agents?.teamIds ?? []),
    memberUserIds: unique([...(project?.memberUserIds ?? []), project?.ownerUserId ?? '']),
  }
}

export function buildProjectRuntimeRefinementState(input: {
  projectSettings: ProjectSettingsConfig
  assignedConfiguredModels: CatalogConfiguredModelOption[]
  assignmentDefaultConfiguredModelId: string
  grantedToolEntries: CapabilityAssetManifest[]
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>
  grantedAgentIds: string[]
  grantedTeamIds: string[]
}): ProjectRuntimeRefinementState {
  const resolvedModels = resolveProjectModelSettings(
    input.projectSettings,
    input.assignedConfiguredModels.map(item => item.value),
    input.assignmentDefaultConfiguredModelId,
  )
  const resolvedActors = resolveProjectAgentSettings(
    input.projectSettings,
    input.grantedAgentIds,
    input.grantedTeamIds,
  )
  const resolvedTools = resolveProjectToolSettings(
    input.projectSettings,
    input.grantedToolEntries.map(entry => entry.sourceKey),
  )

  return {
    allowedConfiguredModelIds: [...resolvedModels.allowedConfiguredModelIds],
    defaultConfiguredModelId: resolvedModels.defaultConfiguredModelId,
    totalTokens: resolvedModels.totalTokens ? String(resolvedModels.totalTokens) : '',
    disabledToolSourceKeys: [...resolvedTools.disabledSourceKeys],
    toolPermissionDraft: buildToolPermissionDraft(
      input.grantedToolEntries,
      input.projectSettings,
      input.workspaceTools,
    ),
    disabledAgentIds: [...resolvedActors.disabledAgentIds],
    disabledTeamIds: [...resolvedActors.disabledTeamIds],
  }
}

export function buildProjectCapabilitySummary(input: {
  project: ProjectRecord | null
  projectSettings: ProjectSettingsConfig
  grantedConfiguredModels: CatalogConfiguredModelOption[]
  grantedToolEntries: CapabilityAssetManifest[]
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>
  grantedAgentIds: string[]
  grantedTeamIds: string[]
}): ProjectCapabilitySummary {
  const grantState = buildProjectGrantState(input.project)
  const runtimeState = buildProjectRuntimeRefinementState({
    projectSettings: input.projectSettings,
    assignedConfiguredModels: input.grantedConfiguredModels,
    assignmentDefaultConfiguredModelId: grantState.defaultConfiguredModelId,
    grantedToolEntries: input.grantedToolEntries,
    workspaceTools: input.workspaceTools,
    grantedAgentIds: input.grantedAgentIds,
    grantedTeamIds: input.grantedTeamIds,
  })
  const defaultModelLabel = input.grantedConfiguredModels.find(
    item => item.value === runtimeState.defaultConfiguredModelId,
  )?.label ?? ''

  return {
    grantedModels: input.grantedConfiguredModels.length,
    enabledModels: runtimeState.allowedConfiguredModelIds.length,
    defaultModelLabel,
    grantedTools: input.grantedToolEntries.length,
    enabledTools: input.grantedToolEntries.length - runtimeState.disabledToolSourceKeys.length,
    toolOverrideCount: Object.values(runtimeState.toolPermissionDraft).filter(value => value !== 'inherit').length,
    grantedActors: input.grantedAgentIds.length + input.grantedTeamIds.length,
    enabledActors: input.grantedAgentIds.length + input.grantedTeamIds.length
      - runtimeState.disabledAgentIds.length
      - runtimeState.disabledTeamIds.length,
    memberCount: grantState.memberUserIds.length,
    editableMemberCount: input.project?.ownerUserId ? 1 : 0,
  }
}

function choosePrimaryModel(models: CatalogConfiguredModelOption[]) {
  return models.find(model => model.value.includes('primary')) ?? models[0]
}

function buildAssignmentsFromSeed(input: {
  configuredModelIds: string[]
  defaultConfiguredModelId: string
  toolSourceKeys: string[]
  agentIds: string[]
  teamIds: string[]
}): ProjectWorkspaceAssignments | undefined {
  const configuredModelIds = unique(input.configuredModelIds)
  const toolSourceKeys = unique(input.toolSourceKeys)
  const agentIds = unique(input.agentIds)
  const teamIds = unique(input.teamIds)

  if (!configuredModelIds.length && !toolSourceKeys.length && !agentIds.length && !teamIds.length) {
    return undefined
  }

  return {
    models: configuredModelIds.length
      ? {
          configuredModelIds,
          defaultConfiguredModelId: configuredModelIds.includes(input.defaultConfiguredModelId)
            ? input.defaultConfiguredModelId
            : configuredModelIds[0] ?? '',
        }
      : undefined,
    tools: toolSourceKeys.length ? { sourceKeys: toolSourceKeys, excludedSourceKeys: [] } : undefined,
    agents: agentIds.length || teamIds.length
      ? { agentIds, teamIds, excludedAgentIds: [], excludedTeamIds: [] }
      : undefined,
  }
}

export function buildProjectSetupPresetSeed(
  preset: ProjectSetupPreset,
  context: ProjectPresetContext,
): ProjectSetupPresetSeed {
  const primaryModel = choosePrimaryModel(context.models)
  const engineeringModels = unique(context.models.map(model => model.value))

  if (preset === 'engineering') {
    return {
      assignments: buildAssignmentsFromSeed({
        configuredModelIds: engineeringModels,
        defaultConfiguredModelId: primaryModel?.value ?? engineeringModels[0] ?? '',
        toolSourceKeys: [],
        agentIds: [],
        teamIds: [],
      }),
      modelSettings: engineeringModels.length
        ? {
            allowedConfiguredModelIds: engineeringModels,
            defaultConfiguredModelId: primaryModel?.value ?? engineeringModels[0] ?? '',
          }
        : undefined,
    }
  }

  if (preset === 'documentation') {
    const documentationModels = unique(primaryModel ? [primaryModel.value] : [])

    return {
      assignments: buildAssignmentsFromSeed({
        configuredModelIds: documentationModels,
        defaultConfiguredModelId: documentationModels[0] ?? '',
        toolSourceKeys: [],
        agentIds: [],
        teamIds: [],
      }),
      modelSettings: documentationModels.length
        ? {
            allowedConfiguredModelIds: documentationModels,
            defaultConfiguredModelId: documentationModels[0] ?? '',
          }
        : undefined,
    }
  }

  return {}
}
