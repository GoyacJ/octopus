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
import { resolveProjectAgentSettings, resolveProjectModelSettings } from './project_settings'

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

function unique(values: string[]) {
  return [...new Set(values.filter(Boolean))]
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
  const saved = projectSettings.tools
  const enabledSourceKeys = saved?.enabledSourceKeys?.length
    ? saved.enabledSourceKeys.filter(sourceKey => assignedSourceKeys.includes(sourceKey))
    : assignedSourceKeys

  return {
    enabledSourceKeys,
    overrides: saved?.overrides ?? {},
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
  const resolvedActors = resolveProjectAgentSettings(
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
  projectSettings: ProjectSettingsConfig
  assignedConfiguredModels: CatalogConfiguredModelOption[]
  assignedToolEntries: CapabilityAssetManifest[]
  workspaceTools: Array<{ kind: string, name: string, permissionMode: WorkspaceToolPermissionMode }>
}): ProjectCapabilitySummary {
  const grantState = buildProjectGrantState(input.project)
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
    tools: toolSourceKeys.length ? { sourceKeys: toolSourceKeys } : undefined,
    agents: agentIds.length || teamIds.length ? { agentIds, teamIds } : undefined,
  }
}

export function buildProjectSetupPresetSeed(
  preset: ProjectSetupPreset,
  context: ProjectPresetContext,
): ProjectSetupPresetSeed {
  const primaryModel = choosePrimaryModel(context.models)
  const engineeringModels = unique(context.models.map(model => model.value))
  const engineeringTools = unique(context.tools.map(entry => entry.sourceKey))
  const engineeringAgents = unique(context.agents.map(agent => agent.id))
  const engineeringTeams = unique(context.teams.map(team => team.id))

  if (preset === 'engineering') {
    return {
      assignments: buildAssignmentsFromSeed({
        configuredModelIds: engineeringModels,
        defaultConfiguredModelId: primaryModel?.value ?? engineeringModels[0] ?? '',
        toolSourceKeys: engineeringTools,
        agentIds: engineeringAgents,
        teamIds: engineeringTeams,
      }),
      modelSettings: engineeringModels.length
        ? {
            allowedConfiguredModelIds: engineeringModels,
            defaultConfiguredModelId: primaryModel?.value ?? engineeringModels[0] ?? '',
          }
        : undefined,
      toolSettings: engineeringTools.length
        ? {
            enabledSourceKeys: engineeringTools,
            overrides: {},
          }
        : undefined,
      agentSettings: engineeringAgents.length || engineeringTeams.length
        ? {
            enabledAgentIds: engineeringAgents,
            enabledTeamIds: engineeringTeams,
          }
        : undefined,
    }
  }

  if (preset === 'documentation') {
    const documentationModels = unique(primaryModel ? [primaryModel.value] : [])
    const documentationTools = unique(
      context.tools
        .filter(entry => entry.kind === 'skill' || entry.sourceKey.includes('help'))
        .map(entry => entry.sourceKey),
    )

    return {
      assignments: buildAssignmentsFromSeed({
        configuredModelIds: documentationModels,
        defaultConfiguredModelId: documentationModels[0] ?? '',
        toolSourceKeys: documentationTools,
        agentIds: [],
        teamIds: [],
      }),
      modelSettings: documentationModels.length
        ? {
            allowedConfiguredModelIds: documentationModels,
            defaultConfiguredModelId: documentationModels[0] ?? '',
          }
        : undefined,
      toolSettings: documentationTools.length
        ? {
            enabledSourceKeys: documentationTools,
            overrides: {},
          }
        : undefined,
      agentSettings: undefined,
    }
  }

  return {}
}
