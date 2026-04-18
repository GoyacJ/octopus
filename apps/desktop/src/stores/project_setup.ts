import type {
  AgentRecord,
  CapabilityAssetManifest,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectRecord,
  ProjectSettingsConfig,
  ProjectToolSettings,
  ProjectWorkspaceAssignments,
  TeamRecord,
  WorkspaceToolPermissionMode,
} from '@octopus/schema'

import type { CatalogConfiguredModelOption } from './catalog'
import {
  resolveEnabledProjectAgentIds,
  resolveProjectModelSettings,
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
  enabledToolSourceKeys: string[]
  toolPermissionDraft: Record<string, ToolPermissionSelection>
  enabledAgentIds: string[]
  enabledTeamIds: string[]
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
  toolSettings?: ProjectToolSettings
  agentSettings?: ProjectAgentSettings
}

export interface ProjectPresetContext {
  models: CatalogConfiguredModelOption[]
  tools: CapabilityAssetManifest[]
  agents: AgentRecord[]
  teams: TeamRecord[]
}

export interface PreferredProjectActorOption {
  value: string
  kind: 'agent' | 'team'
}

interface LegacyProjectToolAssignments {
  excludedSourceKeys?: string[]
  sourceKeys?: string[]
}

interface LegacyProjectAgentAssignments {
  excludedAgentIds?: string[]
  excludedTeamIds?: string[]
  agentIds?: string[]
  teamIds?: string[]
}

function unique(values: string[]) {
  return [...new Set(values.filter(Boolean))]
}

function readStringArray(value: unknown) {
  return Array.isArray(value)
    ? unique(value.filter((item): item is string => typeof item === 'string'))
    : []
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

export function resolveProjectToolSettings(
  projectSettings: ProjectSettingsConfig,
  assignedToolEntries: CapabilityAssetManifest[],
) {
  const assignedSourceKeys = assignedToolEntries.map(entry => entry.sourceKey)
  const saved = projectSettings.tools as (ProjectToolSettings & {
    __hasDisabledSourceKeys?: boolean
    __hasLegacyEnabledSourceKeys?: boolean
    __legacyEnabledSourceKeys?: string[]
    enabledSourceKeys?: string[]
  }) | undefined
  const hasDisabledSourceKeys = saved?.__hasDisabledSourceKeys ?? Array.isArray(saved?.disabledSourceKeys)
  const hasLegacyEnabledSourceKeys = !hasDisabledSourceKeys
    && (saved?.__hasLegacyEnabledSourceKeys ?? Array.isArray(saved?.enabledSourceKeys))
  const legacyEnabledSourceKeys = saved?.__legacyEnabledSourceKeys ?? readStringArray(saved?.enabledSourceKeys)
  const enabledSourceKeys = hasDisabledSourceKeys
    ? assignedSourceKeys.filter(sourceKey => !(saved?.disabledSourceKeys ?? []).includes(sourceKey))
    : hasLegacyEnabledSourceKeys
      ? legacyEnabledSourceKeys.filter(sourceKey => assignedSourceKeys.includes(sourceKey))
      : assignedSourceKeys

  return {
    enabledSourceKeys,
    overrides: saved?.overrides ?? {},
  }
}

export function resolveProjectGrantedToolSourceKeys(
  assignments: ProjectWorkspaceAssignments | undefined,
  workspaceToolSourceKeys: string[],
) {
  const sourceKeys = unique(workspaceToolSourceKeys)
  const tools = assignments?.tools as LegacyProjectToolAssignments | undefined
  if (tools && Array.isArray(tools.excludedSourceKeys)) {
    const excludedSourceKeys = readStringArray(tools.excludedSourceKeys)
    return sourceKeys.filter(sourceKey => !excludedSourceKeys.includes(sourceKey))
  }
  if (tools && Array.isArray(tools.sourceKeys)) {
    const legacySourceKeys = readStringArray(tools.sourceKeys)
    return sourceKeys.filter(sourceKey => legacySourceKeys.includes(sourceKey))
  }
  return sourceKeys
}

export function resolveProjectGrantedActorIds(
  assignments: ProjectWorkspaceAssignments | undefined,
  workspaceAgentIds: string[],
  workspaceTeamIds: string[],
) {
  const agentIds = unique(workspaceAgentIds)
  const teamIds = unique(workspaceTeamIds)
  const agents = assignments?.agents as LegacyProjectAgentAssignments | undefined
  if (agents && (Array.isArray(agents.excludedAgentIds) || Array.isArray(agents.excludedTeamIds))) {
    const excludedAgentIds = readStringArray(agents.excludedAgentIds)
    const excludedTeamIds = readStringArray(agents.excludedTeamIds)
    return {
      assignedAgentIds: agentIds.filter(agentId => !excludedAgentIds.includes(agentId)),
      assignedTeamIds: teamIds.filter(teamId => !excludedTeamIds.includes(teamId)),
    }
  }
  if (agents && (Array.isArray(agents.agentIds) || Array.isArray(agents.teamIds))) {
    const legacyAgentIds = readStringArray(agents.agentIds)
    const legacyTeamIds = readStringArray(agents.teamIds)
    return {
      assignedAgentIds: agentIds.filter(agentId => legacyAgentIds.includes(agentId)),
      assignedTeamIds: teamIds.filter(teamId => legacyTeamIds.includes(teamId)),
    }
  }
  return {
    assignedAgentIds: agentIds,
    assignedTeamIds: teamIds,
  }
}

export function buildToolPermissionDraft(
  assignedToolEntries: CapabilityAssetManifest[],
  projectSettings: ProjectSettingsConfig,
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>,
): Record<string, ToolPermissionSelection> {
  const resolvedToolSettings = resolveProjectToolSettings(projectSettings, assignedToolEntries)

  return Object.fromEntries(
    assignedToolEntries.map((entry) => {
      const override = resolvedToolSettings.overrides[entry.sourceKey]
      const disabled = !resolvedToolSettings.enabledSourceKeys.includes(entry.sourceKey)
      return [
        entry.sourceKey,
        disabled
          ? 'deny'
          : (override?.permissionMode ?? 'inherit'),
      ]
    }),
  ) as Record<string, ToolPermissionSelection>
}

export function buildProjectGrantState(
  project: ProjectRecord | null,
  input: {
    workspaceToolSourceKeys?: string[]
    workspaceAgentIds?: string[]
    workspaceTeamIds?: string[]
  } = {},
): ProjectGrantState {
  const assignments = project?.assignments
  const resolvedActors = (
    input.workspaceAgentIds || input.workspaceTeamIds
  )
    ? resolveProjectGrantedActorIds(assignments, input.workspaceAgentIds ?? [], input.workspaceTeamIds ?? [])
    : {
        assignedAgentIds: readStringArray((assignments?.agents as { agentIds?: string[] } | undefined)?.agentIds),
        assignedTeamIds: readStringArray((assignments?.agents as { teamIds?: string[] } | undefined)?.teamIds),
      }

  return {
    assignedConfiguredModelIds: unique(assignments?.models?.configuredModelIds ?? []),
    defaultConfiguredModelId: assignments?.models?.defaultConfiguredModelId ?? '',
    assignedToolSourceKeys: input.workspaceToolSourceKeys
      ? resolveProjectGrantedToolSourceKeys(assignments, input.workspaceToolSourceKeys)
      : readStringArray((assignments?.tools as { sourceKeys?: string[] } | undefined)?.sourceKeys),
    assignedAgentIds: resolvedActors.assignedAgentIds,
    assignedTeamIds: resolvedActors.assignedTeamIds,
    memberUserIds: unique([...(project?.memberUserIds ?? []), project?.ownerUserId ?? '']),
  }
}

export function buildProjectRuntimeRefinementState(input: {
  projectSettings: ProjectSettingsConfig
  assignedConfiguredModels: CatalogConfiguredModelOption[]
  assignmentDefaultConfiguredModelId: string
  assignedToolEntries: CapabilityAssetManifest[]
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>
  assignedAgentIds: string[]
  assignedTeamIds: string[]
}): ProjectRuntimeRefinementState {
  const resolvedModels = resolveProjectModelSettings(
    input.projectSettings,
    input.assignedConfiguredModels.map(item => item.value),
    input.assignmentDefaultConfiguredModelId,
  )
  const resolvedActors = resolveEnabledProjectAgentIds(
    input.projectSettings,
    input.assignedAgentIds,
    input.assignedTeamIds,
  )
  const resolvedTools = resolveProjectToolSettings(input.projectSettings, input.assignedToolEntries)

  return {
    allowedConfiguredModelIds: [...resolvedModels.allowedConfiguredModelIds],
    defaultConfiguredModelId: resolvedModels.defaultConfiguredModelId,
    totalTokens: resolvedModels.totalTokens ? String(resolvedModels.totalTokens) : '',
    enabledToolSourceKeys: [...resolvedTools.enabledSourceKeys],
    toolPermissionDraft: buildToolPermissionDraft(
      input.assignedToolEntries,
      input.projectSettings,
      input.workspaceTools,
    ),
    enabledAgentIds: [...resolvedActors.enabledAgentIds],
    enabledTeamIds: [...resolvedActors.enabledTeamIds],
  }
}

export function buildProjectCapabilitySummary(input: {
  project: ProjectRecord | null
  grantState?: ProjectGrantState
  projectSettings: ProjectSettingsConfig
  assignedConfiguredModels: CatalogConfiguredModelOption[]
  assignedToolEntries: CapabilityAssetManifest[]
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>
}): ProjectCapabilitySummary {
  const grantState = input.grantState ?? buildProjectGrantState(input.project)
  const runtimeState = buildProjectRuntimeRefinementState({
    projectSettings: input.projectSettings,
    assignedConfiguredModels: input.assignedConfiguredModels,
    assignmentDefaultConfiguredModelId: grantState.defaultConfiguredModelId,
    assignedToolEntries: input.assignedToolEntries,
    workspaceTools: input.workspaceTools,
    assignedAgentIds: grantState.assignedAgentIds,
    assignedTeamIds: grantState.assignedTeamIds,
  })
  const defaultModelLabel = input.assignedConfiguredModels.find(
    item => item.value === runtimeState.defaultConfiguredModelId,
  )?.label ?? ''

  return {
    grantedModels: grantState.assignedConfiguredModelIds.length,
    enabledModels: runtimeState.allowedConfiguredModelIds.length,
    defaultModelLabel,
    grantedTools: grantState.assignedToolSourceKeys.length,
    enabledTools: runtimeState.enabledToolSourceKeys.length,
    toolOverrideCount: Object.values(runtimeState.toolPermissionDraft).filter(value => value !== 'inherit').length,
    grantedActors: grantState.assignedAgentIds.length + grantState.assignedTeamIds.length,
    enabledActors: runtimeState.enabledAgentIds.length + runtimeState.enabledTeamIds.length,
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
  allToolSourceKeys: string[]
  agentIds: string[]
  allAgentIds: string[]
  teamIds: string[]
  allTeamIds: string[]
}): ProjectWorkspaceAssignments | undefined {
  const configuredModelIds = unique(input.configuredModelIds)
  const toolSourceKeys = unique(input.toolSourceKeys)
  const agentIds = unique(input.agentIds)
  const teamIds = unique(input.teamIds)
  const excludedSourceKeys = unique(input.allToolSourceKeys).filter(sourceKey => !toolSourceKeys.includes(sourceKey))
  const excludedAgentIds = unique(input.allAgentIds).filter(agentId => !agentIds.includes(agentId))
  const excludedTeamIds = unique(input.allTeamIds).filter(teamId => !teamIds.includes(teamId))

  if (!configuredModelIds.length && !input.allToolSourceKeys.length && !input.allAgentIds.length && !input.allTeamIds.length) {
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
    tools: input.allToolSourceKeys.length ? { excludedSourceKeys } : undefined,
    agents: input.allAgentIds.length || input.allTeamIds.length
      ? { excludedAgentIds, excludedTeamIds }
      : undefined,
  }
}

export function buildInheritedProjectAssignments(
  modelAssignments?: ProjectWorkspaceAssignments['models'],
): ProjectWorkspaceAssignments {
  return {
    ...(modelAssignments ? { models: modelAssignments } : {}),
    tools: {
      excludedSourceKeys: [],
    },
    agents: {
      excludedAgentIds: [],
      excludedTeamIds: [],
    },
  }
}

export function resolvePreferredProjectActorRef(
  options: PreferredProjectActorOption[],
  leaderAgentId?: string | null,
) {
  const normalizedLeaderAgentId = typeof leaderAgentId === 'string' ? leaderAgentId.trim() : ''
  if (normalizedLeaderAgentId) {
    const leaderOption = options.find(option =>
      option.kind === 'agent' && option.value === `agent:${normalizedLeaderAgentId}`,
    )
    if (leaderOption) {
      return leaderOption.value
    }
  }

  return options[0]?.value ?? ''
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
        allToolSourceKeys: [],
        agentIds: [],
        allAgentIds: [],
        teamIds: [],
        allTeamIds: [],
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
        allToolSourceKeys: [],
        agentIds: [],
        allAgentIds: [],
        teamIds: [],
        allTeamIds: [],
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
