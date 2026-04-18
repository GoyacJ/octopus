import type { ComputedRef, Ref } from 'vue'

import type {
  JsonValue,
  ProjectAgentSettings,
  ProjectModelSettings,
  ProjectSettingsConfig,
  ProjectToolSettings,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
} from '@octopus/schema'

import {
  createRuntimeConfigDraftsFromConfig,
  parseRuntimeConfigDraft,
} from './runtime-config'
import { resolveWorkspaceClientForConnection } from './workspace-scope'
import {
  cloneJson,
  findProjectRuntimeSourceDocument,
  parseProjectSettingsDocument,
} from './project_settings'

interface WorkspaceRuntimeContext {
  activeConnectionId: ComputedRef<string>
  currentProjectId: Ref<string>
  projectRuntimeConfigs: Ref<Record<string, RuntimeEffectiveConfig>>
  projectRuntimeDrafts: Ref<Record<string, string>>
  projectRuntimeValidations: Ref<Record<string, RuntimeConfigValidationResult | null>>
  projectRuntimeLoading: Ref<Record<string, boolean>>
  projectRuntimeSaving: Ref<Record<string, boolean>>
  projectRuntimeValidating: Ref<Record<string, boolean>>
  setConnectionError: (connectionId: string, message: string) => void
}

export function createWorkspaceRuntimeActions(context: WorkspaceRuntimeContext) {
  const inflightLoads = new Map<string, Promise<RuntimeEffectiveConfig | null>>()

  function resolveProjectRuntimeKey(projectId = context.currentProjectId.value, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? context.activeConnectionId.value
    if (!connectionId || !projectId) {
      return ''
    }
    return `${connectionId}:${projectId}`
  }

  function setProjectRuntimeDraft(projectId: string, value: string, workspaceConnectionId?: string) {
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    if (!key) {
      return
    }
    context.projectRuntimeDrafts.value = {
      ...context.projectRuntimeDrafts.value,
      [key]: value,
    }
  }

  function applyProjectRuntimeConfigState(key: string, config: RuntimeEffectiveConfig) {
    context.projectRuntimeConfigs.value = {
      ...context.projectRuntimeConfigs.value,
      [key]: config,
    }
    context.projectRuntimeDrafts.value = {
      ...context.projectRuntimeDrafts.value,
      [key]: createRuntimeConfigDraftsFromConfig(config).project,
    }
    context.projectRuntimeValidations.value = {
      ...context.projectRuntimeValidations.value,
      [key]: config.validation,
    }
  }

  function getProjectSettings(projectId = context.currentProjectId.value, workspaceConnectionId?: string): ProjectSettingsConfig {
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    return parseProjectSettingsDocument(findProjectRuntimeSourceDocument(key ? context.projectRuntimeConfigs.value[key] ?? null : null))
  }

  async function validateProjectRuntimePatch(
    projectId: string,
    patch: Record<string, JsonValue>,
    workspaceConnectionId?: string,
  ): Promise<RuntimeConfigValidationResult> {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return {
        valid: false,
        errors: ['Active workspace connection is unavailable'],
        warnings: [],
      }
    }

    const { client, connectionId } = resolvedClient
    const key = resolveProjectRuntimeKey(projectId, connectionId)
    context.projectRuntimeValidating.value = {
      ...context.projectRuntimeValidating.value,
      [key]: true,
    }

    try {
      const result = await client.runtime.validateProjectConfig(projectId, {
        scope: 'project',
        patch,
      })
      context.projectRuntimeValidations.value = {
        ...context.projectRuntimeValidations.value,
        [key]: result,
      }
      return result
    } catch (cause) {
      const result = {
        valid: false,
        errors: [cause instanceof Error ? cause.message : 'Failed to validate project runtime config'],
        warnings: [],
      }
      context.projectRuntimeValidations.value = {
        ...context.projectRuntimeValidations.value,
        [key]: result,
      }
      context.setConnectionError(connectionId, result.errors[0] ?? '')
      return result
    } finally {
      context.projectRuntimeValidating.value = {
        ...context.projectRuntimeValidating.value,
        [key]: false,
      }
    }
  }

  async function saveProjectRuntimePatch(
    projectId: string,
    patch: Record<string, JsonValue>,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const key = resolveProjectRuntimeKey(projectId, connectionId)
    const validation = await validateProjectRuntimePatch(projectId, patch, connectionId)
    if (!validation.valid) {
      return null
    }

    context.projectRuntimeSaving.value = {
      ...context.projectRuntimeSaving.value,
      [key]: true,
    }

    try {
      const config = await client.runtime.saveProjectConfig(projectId, {
        scope: 'project',
        patch,
      })
      applyProjectRuntimeConfigState(key, config)
      context.setConnectionError(connectionId, '')
      return config
    } catch (cause) {
      context.setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to save project runtime config')
      return null
    } finally {
      context.projectRuntimeSaving.value = {
        ...context.projectRuntimeSaving.value,
        [key]: false,
      }
    }
  }

  async function loadProjectRuntimeConfig(projectId = context.currentProjectId.value, force = false, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    const key = resolveProjectRuntimeKey(projectId, connectionId)
    if (!key) {
      return null
    }
    if (context.projectRuntimeConfigs.value[key] && !force) {
      return context.projectRuntimeConfigs.value[key]
    }

    context.projectRuntimeLoading.value = {
      ...context.projectRuntimeLoading.value,
      [key]: true,
    }

    try {
      const config = await client.runtime.getProjectConfig(projectId)
      applyProjectRuntimeConfigState(key, config)
      context.projectRuntimeValidations.value = {
        ...context.projectRuntimeValidations.value,
        [key]: null,
      }
      return config
    } catch (cause) {
      context.setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load project runtime config')
      return null
    } finally {
      context.projectRuntimeLoading.value = {
        ...context.projectRuntimeLoading.value,
        [key]: false,
      }
    }
  }

  async function ensureProjectRuntimeConfig(
    projectId = context.currentProjectId.value,
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    const connectionId = resolvedClient?.connectionId ?? context.activeConnectionId.value
    const key = resolveProjectRuntimeKey(projectId, connectionId)
    if (!key) {
      return null
    }

    if (!options.force && Object.prototype.hasOwnProperty.call(context.projectRuntimeConfigs.value, key)) {
      return context.projectRuntimeConfigs.value[key] ?? null
    }

    const inflight = inflightLoads.get(key)
    if (inflight && !options.force) {
      return await inflight
    }

    const task = loadProjectRuntimeConfig(projectId, Boolean(options.force), connectionId)
    inflightLoads.set(key, task)
    try {
      return await task
    } finally {
      if (inflightLoads.get(key) === task) {
        inflightLoads.delete(key)
      }
    }
  }

  async function validateProjectRuntimeConfig(projectId = context.currentProjectId.value, workspaceConnectionId?: string): Promise<RuntimeConfigValidationResult> {
    if (!projectId) {
      return {
        valid: false,
        errors: ['Project runtime config requires a project id'],
        warnings: [],
      }
    }

    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    const patch = parseRuntimeConfigDraft('project', context.projectRuntimeDrafts.value[key] ?? '{}')
    return await validateProjectRuntimePatch(projectId, patch.patch, workspaceConnectionId)
  }

  async function saveProjectRuntimeConfig(projectId = context.currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    const patch = parseRuntimeConfigDraft('project', context.projectRuntimeDrafts.value[key] ?? '{}')
    return await saveProjectRuntimePatch(projectId, patch.patch, workspaceConnectionId)
  }

  async function saveProjectModelSettings(
    projectId: string,
    settings: ProjectModelSettings,
    workspaceConnectionId?: string,
  ) {
    return await saveProjectRuntimePatch(projectId, {
      projectSettings: {
        models: {
          allowedConfiguredModelIds: [...settings.allowedConfiguredModelIds],
          defaultConfiguredModelId: settings.defaultConfiguredModelId,
          totalTokens: settings.totalTokens ?? null,
        },
      },
    }, workspaceConnectionId)
  }

  async function saveProjectToolSettings(
    projectId: string,
    settings: ProjectToolSettings,
    workspaceConnectionId?: string,
  ) {
    const existing = getProjectSettings(projectId, workspaceConnectionId).tools
    const existingOverrides = existing?.overrides ?? {}
    const nextOverrides: Record<string, JsonValue> = {}

    for (const [sourceKey, entry] of Object.entries(settings.overrides)) {
      nextOverrides[sourceKey] = {
        permissionMode: entry.permissionMode,
      }
    }

    for (const sourceKey of Object.keys(existingOverrides)) {
      if (!(sourceKey in nextOverrides)) {
        nextOverrides[sourceKey] = null
      }
    }

    return await saveProjectRuntimePatch(projectId, {
      projectSettings: {
        tools: {
          enabledSourceKeys: cloneJson(settings.enabledSourceKeys),
          overrides: nextOverrides,
        },
      },
    }, workspaceConnectionId)
  }

  async function saveProjectAgentSettings(
    projectId: string,
    settings: ProjectAgentSettings,
    workspaceConnectionId?: string,
  ) {
    return await saveProjectRuntimePatch(projectId, {
      projectSettings: {
        agents: {
          enabledAgentIds: cloneJson(settings.enabledAgentIds),
          enabledTeamIds: cloneJson(settings.enabledTeamIds),
        },
      },
    }, workspaceConnectionId)
  }

  return {
    resolveProjectRuntimeKey,
    setProjectRuntimeDraft,
    applyProjectRuntimeConfigState,
    getProjectSettings,
    loadProjectRuntimeConfig,
    ensureProjectRuntimeConfig,
    validateProjectRuntimeConfig,
    saveProjectRuntimeConfig,
    saveProjectModelSettings,
    saveProjectToolSettings,
    saveProjectAgentSettings,
  }
}
