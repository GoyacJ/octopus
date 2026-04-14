import type {
  JsonValue,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectSettingsConfig,
  ProjectToolSettings,
} from '@octopus/schema'
import type { WorkspaceToolPermissionMode } from '@octopus/schema'

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

  const enabledSourceKeys = Array.isArray(value.enabledSourceKeys)
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

  if (!enabledSourceKeys.length && !Object.keys(overrides).length) {
    return undefined
  }

  return { enabledSourceKeys, overrides }
}

export function parseProjectAgentSettings(value: unknown): ProjectAgentSettings | undefined {
  if (!isObjectRecord(value)) {
    return undefined
  }

  const enabledAgentIds = Array.isArray(value.enabledAgentIds)
    ? value.enabledAgentIds.filter((item): item is string => typeof item === 'string')
    : []
  const enabledTeamIds = Array.isArray(value.enabledTeamIds)
    ? value.enabledTeamIds.filter((item): item is string => typeof item === 'string')
    : []

  if (!enabledAgentIds.length && !enabledTeamIds.length) {
    return undefined
  }

  return {
    enabledAgentIds,
    enabledTeamIds,
  }
}

export function parseProjectSettingsDocument(document: Record<string, JsonValue>): ProjectSettingsConfig {
  const projectSettings = isObjectRecord(document.projectSettings) ? document.projectSettings : {}
  return {
    models: parseProjectModelSettings(projectSettings.models),
    tools: parseProjectToolSettings(projectSettings.tools),
    agents: parseProjectAgentSettings(projectSettings.agents),
  }
}
