import type {
  JsonValue,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectSettingsConfig,
  ProjectToolSettings,
} from '@octopus/schema'
import type { WorkspaceToolPermissionMode } from '@octopus/schema'

type ProjectToolSettingsCompat = ProjectToolSettings & {
  __hasDisabledSourceKeys?: boolean
}

type ProjectAgentSettingsCompat = ProjectAgentSettings & {
  __hasDisabledAgentIds?: boolean
  __hasDisabledTeamIds?: boolean
}

type ProjectModelSettingsCompat = ProjectModelSettings & {
  __hasDisabledConfiguredModelIds?: boolean
}

function uniqueStrings(values: string[]) {
  return [...new Set(values.filter(Boolean))]
}

function parseStringArray(value: unknown) {
  return Array.isArray(value)
    ? uniqueStrings(value.filter((item): item is string => typeof item === 'string'))
    : []
}

export function isObjectRecord(value: unknown): value is Record<string, JsonValue> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function findProjectRuntimeSourceDocument(config: { sources: Array<{ scope: string, document?: unknown }> } | null) {
  const source = config?.sources.find(item => item.scope === 'project')
  return isObjectRecord(source?.document) ? source.document : {}
}

export function parseProjectModelSettings(value: unknown): ProjectModelSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const hasDisabledConfiguredModelIds = Object.prototype.hasOwnProperty.call(value, 'disabledConfiguredModelIds')
  const disabledConfiguredModelIds = parseStringArray(value.disabledConfiguredModelIds)
  const allowedConfiguredModelIds = Array.isArray(value.allowedConfiguredModelIds)
    ? value.allowedConfiguredModelIds.filter((item): item is string => typeof item === 'string')
    : Array.isArray(value.selectedConfiguredModelIds)
      ? value.selectedConfiguredModelIds.filter((item): item is string => typeof item === 'string')
      : []
  const defaultConfiguredModelId = typeof value.defaultConfiguredModelId === 'string'
    ? value.defaultConfiguredModelId
    : ''
  const totalTokens = typeof value.totalTokens === 'number' && Number.isFinite(value.totalTokens) && value.totalTokens > 0
    ? Math.trunc(value.totalTokens)
    : undefined

  if (
    !allowedConfiguredModelIds.length
    && !defaultConfiguredModelId
    && totalTokens === undefined
    && !hasDisabledConfiguredModelIds
  ) {
    return undefined
  }

  const settings: ProjectModelSettingsCompat = {
    disabledConfiguredModelIds,
    allowedConfiguredModelIds,
    defaultConfiguredModelId,
    totalTokens,
  }
  if (hasDisabledConfiguredModelIds) {
    settings.__hasDisabledConfiguredModelIds = true
  }

  return settings
}

export function parseProjectToolSettings(value: unknown): ProjectToolSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const hasDisabledSourceKeys = Object.prototype.hasOwnProperty.call(value, 'disabledSourceKeys')
  const disabledSourceKeys = parseStringArray(value.disabledSourceKeys)
  const overrides = isObjectRecord(value.overrides)
    ? Object.fromEntries(
        Object.entries(value.overrides)
          .flatMap(([sourceKey, entry]) => {
            if (!isObjectRecord(entry) || typeof entry.permissionMode !== 'string') {
              return []
            }
            return [[sourceKey, { permissionMode: entry.permissionMode as WorkspaceToolPermissionMode }]]
          }),
      )
    : {}

  if (
    !disabledSourceKeys.length
    && !Object.keys(overrides).length
    && !hasDisabledSourceKeys
  ) {
    return undefined
  }

  const settings: ProjectToolSettingsCompat = {
    disabledSourceKeys,
    overrides,
  }
  if (hasDisabledSourceKeys) {
    settings.__hasDisabledSourceKeys = true
  }
  return settings
}

export function parseProjectAgentSettings(value: unknown): ProjectAgentSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const hasDisabledAgentIds = Object.prototype.hasOwnProperty.call(value, 'disabledAgentIds')
  const hasDisabledTeamIds = Object.prototype.hasOwnProperty.call(value, 'disabledTeamIds')
  const disabledAgentIds = parseStringArray(value.disabledAgentIds)
  const disabledTeamIds = parseStringArray(value.disabledTeamIds)

  if (
    !disabledAgentIds.length
    && !disabledTeamIds.length
    && !hasDisabledAgentIds
    && !hasDisabledTeamIds
  ) {
    return undefined
  }

  const settings: ProjectAgentSettingsCompat = {
    disabledAgentIds,
    disabledTeamIds,
  }
  if (hasDisabledAgentIds) {
    settings.__hasDisabledAgentIds = true
  }
  if (hasDisabledTeamIds) {
    settings.__hasDisabledTeamIds = true
  }
  return settings
}

export function parseProjectSettingsDocument(document: Record<string, JsonValue>): ProjectSettingsConfig {
  const projectSettings = isObjectRecord(document.projectSettings) ? document.projectSettings : {}
  return {
    models: parseProjectModelSettings(projectSettings.models),
    tools: parseProjectToolSettings(projectSettings.tools),
    agents: parseProjectAgentSettings(projectSettings.agents),
  }
}

export function resolveProjectModelSettings(
  projectSettings: ProjectSettingsConfig,
  assignedConfiguredModelIds: string[],
  assignmentDefaultConfiguredModelId = '',
): ProjectModelSettings {
  const configuredIds = [...new Set(assignedConfiguredModelIds.filter(Boolean))]
  const saved = projectSettings.models
  const allowedConfiguredModelIds = saved?.allowedConfiguredModelIds?.length
    ? saved.allowedConfiguredModelIds.filter(item => configuredIds.includes(item))
    : configuredIds
  const fallbackDefaultConfiguredModelId = configuredIds.includes(assignmentDefaultConfiguredModelId)
    ? assignmentDefaultConfiguredModelId
    : configuredIds[0] ?? ''
  const defaultConfiguredModelId = allowedConfiguredModelIds.includes(saved?.defaultConfiguredModelId ?? '')
    ? saved?.defaultConfiguredModelId ?? ''
    : allowedConfiguredModelIds.includes(fallbackDefaultConfiguredModelId)
      ? fallbackDefaultConfiguredModelId
      : allowedConfiguredModelIds[0] ?? ''

  return {
    allowedConfiguredModelIds,
    defaultConfiguredModelId,
    disabledConfiguredModelIds: saved?.disabledConfiguredModelIds,
    totalTokens: saved?.totalTokens,
  }
}

export function resolveProjectAgentSettings(
  projectSettings: ProjectSettingsConfig,
  grantedAgentIds: string[],
  grantedTeamIds: string[],
): ProjectAgentSettings {
  const normalizedGrantedAgentIds = uniqueStrings(grantedAgentIds)
  const normalizedGrantedTeamIds = uniqueStrings(grantedTeamIds)
  const saved = projectSettings.agents as ProjectAgentSettingsCompat | undefined

  const disabledAgentIds = saved?.__hasDisabledAgentIds
    ? (saved.disabledAgentIds ?? []).filter(agentId => normalizedGrantedAgentIds.includes(agentId))
    : []
  const disabledTeamIds = saved?.__hasDisabledTeamIds
    ? (saved.disabledTeamIds ?? []).filter(teamId => normalizedGrantedTeamIds.includes(teamId))
    : []

  return {
    disabledAgentIds,
    disabledTeamIds,
  }
}

export function resolveProjectToolSettings(
  projectSettings: ProjectSettingsConfig,
  grantedSourceKeys: string[],
): ProjectToolSettings {
  const normalizedGrantedSourceKeys = uniqueStrings(grantedSourceKeys)
  const saved = projectSettings.tools as ProjectToolSettingsCompat | undefined
  const disabledSourceKeys = saved?.__hasDisabledSourceKeys
    ? (saved.disabledSourceKeys ?? []).filter(sourceKey => normalizedGrantedSourceKeys.includes(sourceKey))
    : []

  return {
    disabledSourceKeys,
    overrides: Object.fromEntries(
      Object.entries(saved?.overrides ?? {}).filter(([sourceKey]) => normalizedGrantedSourceKeys.includes(sourceKey)),
    ),
  }
}

export function resolveProjectGrantedModelIds(
  projectSettings: ProjectSettingsConfig,
  workspaceConfiguredModelIds: string[],
) {
  const normalizedWorkspaceConfiguredModelIds = uniqueStrings(workspaceConfiguredModelIds)
  const disabledConfiguredModelIds = new Set(projectSettings.models?.disabledConfiguredModelIds ?? [])

  return normalizedWorkspaceConfiguredModelIds.filter(modelId => !disabledConfiguredModelIds.has(modelId))
}

export function resolveProjectGrantedToolSourceKeys(
  projectSettings: ProjectSettingsConfig,
  workspaceSourceKeys: string[],
) {
  const normalizedWorkspaceSourceKeys = uniqueStrings(workspaceSourceKeys)
  const disabledSourceKeys = new Set(projectSettings.tools?.disabledSourceKeys ?? [])
  return normalizedWorkspaceSourceKeys.filter(sourceKey => !disabledSourceKeys.has(sourceKey))
}

export function resolveProjectGrantedAgentIds(
  projectSettings: ProjectSettingsConfig,
  workspaceAgentIds: string[],
) {
  const normalizedWorkspaceAgentIds = uniqueStrings(workspaceAgentIds)
  const disabledAgentIds = new Set(projectSettings.agents?.disabledAgentIds ?? [])
  return normalizedWorkspaceAgentIds.filter(agentId => !disabledAgentIds.has(agentId))
}

export function resolveProjectGrantedTeamIds(
  projectSettings: ProjectSettingsConfig,
  workspaceTeamIds: string[],
) {
  const normalizedWorkspaceTeamIds = uniqueStrings(workspaceTeamIds)
  const disabledTeamIds = new Set(projectSettings.agents?.disabledTeamIds ?? [])
  return normalizedWorkspaceTeamIds.filter(teamId => !disabledTeamIds.has(teamId))
}
