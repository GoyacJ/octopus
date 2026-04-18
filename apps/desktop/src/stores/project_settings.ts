import type {
  JsonValue,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectSettingsConfig,
  ProjectToolSettings,
} from '@octopus/schema'
import type { WorkspaceToolPermissionMode } from '@octopus/schema'

interface ParsedProjectToolSettings extends ProjectToolSettings {
  __hasDisabledSourceKeys?: boolean
  __hasLegacyEnabledSourceKeys?: boolean
  __legacyEnabledSourceKeys?: string[]
}

interface ParsedProjectAgentSettings extends ProjectAgentSettings {
  __hasDisabledAgentIds?: boolean
  __hasDisabledTeamIds?: boolean
  __hasLegacyEnabledAgentIds?: boolean
  __hasLegacyEnabledTeamIds?: boolean
  __legacyEnabledAgentIds?: string[]
  __legacyEnabledTeamIds?: string[]
}

export function isObjectRecord(value: unknown): value is Record<string, JsonValue> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function hasOwnProperty(value: Record<string, JsonValue>, key: string) {
  return Object.prototype.hasOwnProperty.call(value, key)
}

function readStringArray(value: unknown) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : []
}

export function findProjectRuntimeSourceDocument(config: { sources: Array<{ scope: string, document?: unknown }> } | null) {
  const source = config?.sources.find(item => item.scope === 'project')
  return isObjectRecord(source?.document) ? source.document : {}
}

export function parseProjectModelSettings(value: unknown): ProjectModelSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

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

  if (!allowedConfiguredModelIds.length && !defaultConfiguredModelId && totalTokens === undefined) {
    return undefined
  }

  return {
    allowedConfiguredModelIds,
    defaultConfiguredModelId,
    totalTokens,
  }
}

export function parseProjectToolSettings(value: unknown): ProjectToolSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const disabledSourceKeys = Array.isArray(value.disabledSourceKeys)
    ? value.disabledSourceKeys.filter((item): item is string => typeof item === 'string')
    : []
  const legacyEnabledSourceKeys = Array.isArray(value.enabledSourceKeys)
    ? value.enabledSourceKeys.filter((item): item is string => typeof item === 'string')
    : []
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

  const hasDisabledSourceKeys = hasOwnProperty(value, 'disabledSourceKeys')
  const hasLegacyEnabledSourceKeys = hasOwnProperty(value, 'enabledSourceKeys')
  if (!hasDisabledSourceKeys && !hasLegacyEnabledSourceKeys && !Object.keys(overrides).length) {
    return undefined
  }

  const parsed: ParsedProjectToolSettings = {
    disabledSourceKeys,
    overrides,
    __hasDisabledSourceKeys: hasDisabledSourceKeys,
    __hasLegacyEnabledSourceKeys: hasLegacyEnabledSourceKeys,
    __legacyEnabledSourceKeys: legacyEnabledSourceKeys,
  }

  return parsed
}

export function parseProjectAgentSettings(value: unknown): ProjectAgentSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const disabledAgentIds = Array.isArray(value.disabledAgentIds)
    ? value.disabledAgentIds.filter((item): item is string => typeof item === 'string')
    : []
  const disabledTeamIds = Array.isArray(value.disabledTeamIds)
    ? value.disabledTeamIds.filter((item): item is string => typeof item === 'string')
    : []
  const legacyEnabledAgentIds = Array.isArray(value.enabledAgentIds)
    ? value.enabledAgentIds.filter((item): item is string => typeof item === 'string')
    : []
  const legacyEnabledTeamIds = Array.isArray(value.enabledTeamIds)
    ? value.enabledTeamIds.filter((item): item is string => typeof item === 'string')
    : []

  const hasDisabledAgentIds = hasOwnProperty(value, 'disabledAgentIds')
  const hasDisabledTeamIds = hasOwnProperty(value, 'disabledTeamIds')
  const hasLegacyEnabledAgentIds = hasOwnProperty(value, 'enabledAgentIds')
  const hasLegacyEnabledTeamIds = hasOwnProperty(value, 'enabledTeamIds')
  if (
    !hasDisabledAgentIds
    && !hasDisabledTeamIds
    && !hasLegacyEnabledAgentIds
    && !hasLegacyEnabledTeamIds
  ) {
    return undefined
  }

  const parsed: ParsedProjectAgentSettings = {
    disabledAgentIds,
    disabledTeamIds,
    __hasDisabledAgentIds: hasDisabledAgentIds,
    __hasDisabledTeamIds: hasDisabledTeamIds,
    __hasLegacyEnabledAgentIds: hasLegacyEnabledAgentIds,
    __hasLegacyEnabledTeamIds: hasLegacyEnabledTeamIds,
    __legacyEnabledAgentIds: legacyEnabledAgentIds,
    __legacyEnabledTeamIds: legacyEnabledTeamIds,
  }

  return parsed
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
    totalTokens: saved?.totalTokens,
  }
}

export function resolveProjectAgentSettings(
  projectSettings: ProjectSettingsConfig,
  assignedAgentIds: string[],
  assignedTeamIds: string[],
): ProjectAgentSettings {
  const normalizedAssignedAgentIds = [...new Set(assignedAgentIds.filter(Boolean))]
  const normalizedAssignedTeamIds = [...new Set(assignedTeamIds.filter(Boolean))]
  const saved = projectSettings.agents as (ParsedProjectAgentSettings & {
    enabledAgentIds?: string[]
    enabledTeamIds?: string[]
  }) | undefined
  const hasDisabledAgentIds = saved?.__hasDisabledAgentIds ?? Array.isArray(saved?.disabledAgentIds)
  const hasDisabledTeamIds = saved?.__hasDisabledTeamIds ?? Array.isArray(saved?.disabledTeamIds)

  return {
    disabledAgentIds: (
      hasDisabledAgentIds
        ? readStringArray(saved?.disabledAgentIds).filter(agentId => normalizedAssignedAgentIds.includes(agentId))
        : []
    ),
    disabledTeamIds: (
      hasDisabledTeamIds
        ? readStringArray(saved?.disabledTeamIds).filter(teamId => normalizedAssignedTeamIds.includes(teamId))
        : []
    ),
  }
}

export function resolveEnabledProjectAgentIds(
  projectSettings: ProjectSettingsConfig,
  assignedAgentIds: string[],
  assignedTeamIds: string[],
) {
  const normalizedAssignedAgentIds = [...new Set(assignedAgentIds.filter(Boolean))]
  const normalizedAssignedTeamIds = [...new Set(assignedTeamIds.filter(Boolean))]
  const saved = projectSettings.agents as (ParsedProjectAgentSettings & {
    enabledAgentIds?: string[]
    enabledTeamIds?: string[]
  }) | undefined
  const hasDisabledAgentIds = saved?.__hasDisabledAgentIds ?? Array.isArray(saved?.disabledAgentIds)
  const hasDisabledTeamIds = saved?.__hasDisabledTeamIds ?? Array.isArray(saved?.disabledTeamIds)
  const hasLegacyEnabledAgentIds = !hasDisabledAgentIds
    && (saved?.__hasLegacyEnabledAgentIds ?? Array.isArray(saved?.enabledAgentIds))
  const hasLegacyEnabledTeamIds = !hasDisabledTeamIds
    && (saved?.__hasLegacyEnabledTeamIds ?? Array.isArray(saved?.enabledTeamIds))
  const legacyEnabledAgentIds = saved?.__legacyEnabledAgentIds ?? readStringArray(saved?.enabledAgentIds)
  const legacyEnabledTeamIds = saved?.__legacyEnabledTeamIds ?? readStringArray(saved?.enabledTeamIds)
  const disabledAgentIds = readStringArray(saved?.disabledAgentIds)
  const disabledTeamIds = readStringArray(saved?.disabledTeamIds)

  return {
    enabledAgentIds: hasDisabledAgentIds
      ? normalizedAssignedAgentIds.filter(agentId => !disabledAgentIds.includes(agentId))
      : hasLegacyEnabledAgentIds
        ? legacyEnabledAgentIds.filter(agentId => normalizedAssignedAgentIds.includes(agentId))
        : normalizedAssignedAgentIds,
    enabledTeamIds: hasDisabledTeamIds
      ? normalizedAssignedTeamIds.filter(teamId => !disabledTeamIds.includes(teamId))
      : hasLegacyEnabledTeamIds
        ? legacyEnabledTeamIds.filter(teamId => normalizedAssignedTeamIds.includes(teamId))
        : normalizedAssignedTeamIds,
  }
}
