import type {
  JsonValue,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectSettingsConfig,
  ProjectToolSettings,
} from '@octopus/schema'
import type { WorkspaceToolPermissionMode } from '@octopus/schema'

type ProjectToolSettingsCompat = ProjectToolSettings & {
  __legacyEnabledSourceKeys?: string[]
}

type ProjectAgentSettingsCompat = ProjectAgentSettings & {
  __legacyEnabledAgentIds?: string[]
  __legacyEnabledTeamIds?: string[]
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

  const disabledSourceKeys = parseStringArray(value.disabledSourceKeys)
  const legacyEnabledSourceKeys = parseStringArray(value.enabledSourceKeys)
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

  if (!disabledSourceKeys.length && !legacyEnabledSourceKeys.length && !Object.keys(overrides).length) {
    return undefined
  }

  const settings: ProjectToolSettingsCompat = {
    disabledSourceKeys,
    overrides,
  }
  if (!disabledSourceKeys.length && legacyEnabledSourceKeys.length) {
    settings.__legacyEnabledSourceKeys = legacyEnabledSourceKeys
  }
  return settings
}

export function parseProjectAgentSettings(value: unknown): ProjectAgentSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const disabledAgentIds = parseStringArray(value.disabledAgentIds)
  const disabledTeamIds = parseStringArray(value.disabledTeamIds)
  const legacyEnabledAgentIds = parseStringArray(value.enabledAgentIds)
  const legacyEnabledTeamIds = parseStringArray(value.enabledTeamIds)

  if (
    !disabledAgentIds.length
    && !disabledTeamIds.length
    && !legacyEnabledAgentIds.length
    && !legacyEnabledTeamIds.length
  ) {
    return undefined
  }

  const settings: ProjectAgentSettingsCompat = {
    disabledAgentIds,
    disabledTeamIds,
  }
  if (!disabledAgentIds.length && legacyEnabledAgentIds.length) {
    settings.__legacyEnabledAgentIds = legacyEnabledAgentIds
  }
  if (!disabledTeamIds.length && legacyEnabledTeamIds.length) {
    settings.__legacyEnabledTeamIds = legacyEnabledTeamIds
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

  const disabledAgentIds = saved?.__legacyEnabledAgentIds?.length
    ? normalizedGrantedAgentIds.filter(agentId => !saved.__legacyEnabledAgentIds?.includes(agentId))
    : (saved?.disabledAgentIds ?? []).filter(agentId => normalizedGrantedAgentIds.includes(agentId))
  const disabledTeamIds = saved?.__legacyEnabledTeamIds?.length
    ? normalizedGrantedTeamIds.filter(teamId => !saved.__legacyEnabledTeamIds?.includes(teamId))
    : (saved?.disabledTeamIds ?? []).filter(teamId => normalizedGrantedTeamIds.includes(teamId))

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
  const disabledSourceKeys = saved?.__legacyEnabledSourceKeys?.length
    ? normalizedGrantedSourceKeys.filter(sourceKey => !saved.__legacyEnabledSourceKeys?.includes(sourceKey))
    : (saved?.disabledSourceKeys ?? []).filter(sourceKey => normalizedGrantedSourceKeys.includes(sourceKey))

  return {
    disabledSourceKeys,
    overrides: Object.fromEntries(
      Object.entries(saved?.overrides ?? {}).filter(([sourceKey]) => normalizedGrantedSourceKeys.includes(sourceKey)),
    ),
  }
}
