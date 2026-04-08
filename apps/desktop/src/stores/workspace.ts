import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CreateProjectRequest,
  JsonValue,
  ProjectAgentSettings,
  ProjectDashboardSnapshot,
  ProjectModelSettings,
  ProjectRecord,
  ProjectSettingsConfig,
  ProjectToolSettings,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  UpdateProjectRequest,
  WorkspaceOverviewSnapshot,
} from '@octopus/schema'
import type { WorkspaceToolPermissionMode } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import {
  createRuntimeConfigDraftsFromConfig,
  parseRuntimeConfigDraft,
} from './runtime-config'

type WorkspaceScoped<T> = Record<string, T>

function isObjectRecord(value: unknown): value is Record<string, JsonValue> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function findProjectRuntimeSourceDocument(config: RuntimeEffectiveConfig | null) {
  const source = config?.sources.find(item => item.scope === 'project')
  return isObjectRecord(source?.document) ? source.document : {}
}

function parseProjectModelSettings(value: unknown): ProjectModelSettings | undefined {
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

  if (!allowedConfiguredModelIds.length && !defaultConfiguredModelId) {
    return undefined
  }

  return {
    allowedConfiguredModelIds,
    defaultConfiguredModelId,
  }
}

function parseProjectToolSettings(value: unknown): ProjectToolSettings | undefined {
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

function parseProjectAgentSettings(value: unknown): ProjectAgentSettings | undefined {
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

function parseProjectSettingsDocument(document: Record<string, JsonValue>): ProjectSettingsConfig {
  const projectSettings = isObjectRecord(document.projectSettings) ? document.projectSettings : {}
  return {
    models: parseProjectModelSettings(projectSettings.models),
    tools: parseProjectToolSettings(projectSettings.tools),
    agents: parseProjectAgentSettings(projectSettings.agents),
  }
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentWorkspaceId = ref('')
  const currentProjectId = ref('')
  const currentConversationId = ref('')

  const summaries = ref<WorkspaceScoped<WorkspaceOverviewSnapshot['workspace']>>({})
  const overviews = ref<WorkspaceScoped<WorkspaceOverviewSnapshot>>({})
  const projectsByConnection = ref<WorkspaceScoped<ProjectRecord[]>>({})
  const dashboards = ref<Record<string, ProjectDashboardSnapshot>>({})
  const projectRuntimeConfigs = ref<Record<string, RuntimeEffectiveConfig>>({})
  const projectRuntimeDrafts = ref<Record<string, string>>({})
  const projectRuntimeValidations = ref<Record<string, RuntimeConfigValidationResult | null>>({})
  const projectRuntimeLoading = ref<Record<string, boolean>>({})
  const projectRuntimeSaving = ref<Record<string, boolean>>({})
  const projectRuntimeValidating = ref<Record<string, boolean>>({})
  const loadingByConnection = ref<Record<string, boolean>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const activeWorkspace = computed(() => summaries.value[activeConnectionId.value] ?? overviews.value[activeConnectionId.value]?.workspace ?? null)
  const projects = computed(() => projectsByConnection.value[activeConnectionId.value] ?? [])
  const activeProject = computed(() => projects.value.find(project => project.id === currentProjectId.value) ?? null)
  const activeOverview = computed(() => overviews.value[activeConnectionId.value] ?? null)
  const activeDashboard = computed(() => {
    if (!activeConnectionId.value || !currentProjectId.value) {
      return null
    }
    return dashboards.value[`${activeConnectionId.value}:${currentProjectId.value}`] ?? null
  })
  const activeProjectRuntimeKey = computed(() =>
    activeConnectionId.value && currentProjectId.value
      ? `${activeConnectionId.value}:${currentProjectId.value}`
      : '',
  )
  const activeProjectRuntimeConfig = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeConfigs.value[activeProjectRuntimeKey.value] ?? null
      : null,
  )
  const activeProjectSettings = computed(() =>
    parseProjectSettingsDocument(findProjectRuntimeSourceDocument(activeProjectRuntimeConfig.value)),
  )
  const activeProjectRuntimeDraft = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeDrafts.value[activeProjectRuntimeKey.value] ?? '{}'
      : '{}',
  )
  const activeProjectRuntimeValidation = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeValidations.value[activeProjectRuntimeKey.value] ?? null
      : null,
  )
  const activeProjectRuntimeLoading = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeLoading.value[activeProjectRuntimeKey.value] ?? false
      : false,
  )
  const activeProjectRuntimeSaving = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeSaving.value[activeProjectRuntimeKey.value] ?? false
      : false,
  )
  const activeProjectRuntimeValidating = computed(() =>
    activeProjectRuntimeKey.value
      ? projectRuntimeValidating.value[activeProjectRuntimeKey.value] ?? false
      : false,
  )
  const loading = computed(() => loadingByConnection.value[activeConnectionId.value] ?? false)
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  function setConnectionError(connectionId: string, message: string) {
    errors.value = {
      ...errors.value,
      [connectionId]: message,
    }
  }

  function activeProjectsForConnection(connectionId: string) {
    return (projectsByConnection.value[connectionId] ?? []).filter(project => project.status === 'active')
  }

  function setProjectsForConnection(connectionId: string, projects: ProjectRecord[]) {
    projectsByConnection.value = {
      ...projectsByConnection.value,
      [connectionId]: projects,
    }
  }

  function setDefaultProjectIdForConnection(connectionId: string, projectId: string) {
    const summary = summaries.value[connectionId]
    if (summary) {
      summaries.value = {
        ...summaries.value,
        [connectionId]: {
          ...summary,
          defaultProjectId: projectId,
        },
      }
    }

    const overview = overviews.value[connectionId]
    if (overview) {
      overviews.value = {
        ...overviews.value,
        [connectionId]: {
          ...overview,
          workspace: {
            ...overview.workspace,
            defaultProjectId: projectId,
          },
        },
      }
    }
  }

  function syncCurrentProjectSelection(connectionId: string, preferredProjectId?: string) {
    const projects = projectsByConnection.value[connectionId] ?? []
    const activeProjects = projects.filter(project => project.status === 'active')
    const preferred = preferredProjectId
      ? activeProjects.find(project => project.id === preferredProjectId) ?? projects.find(project => project.id === preferredProjectId)
      : undefined
    const current = projects.find(project => project.id === currentProjectId.value && project.status === 'active')
    const fallback = activeProjects[0] ?? projects[0]
    const nextProject = current ?? preferred ?? fallback

    if (connectionId === activeConnectionId.value || !currentProjectId.value) {
      currentProjectId.value = nextProject?.id ?? ''
    }

    if (!preferred && activeProjects[0] && connectionId === activeConnectionId.value) {
      setDefaultProjectIdForConnection(connectionId, activeProjects[0].id)
    }
  }

  function resolveProjectRuntimeKey(projectId = currentProjectId.value, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    if (!connectionId || !projectId) {
      return ''
    }
    return `${connectionId}:${projectId}`
  }

  function syncRouteScope(workspaceId?: string, projectId?: string, conversationId?: string) {
    if (workspaceId) {
      currentWorkspaceId.value = workspaceId
    }
    if (projectId !== undefined) {
      currentProjectId.value = projectId
    }
    if (conversationId !== undefined) {
      currentConversationId.value = conversationId
    }
  }

  async function bootstrap(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    loadingByConnection.value = {
      ...loadingByConnection.value,
      [connectionId]: true,
    }
    errors.value = {
      ...errors.value,
      [connectionId]: '',
    }

    try {
      const [workspace, projectList, overview] = await Promise.all([
        client.workspace.get(),
        client.projects.list(),
        client.workspace.getOverview(),
      ])

      if (requestTokens.value[connectionId] !== token) {
        return
      }

      summaries.value = {
        ...summaries.value,
        [connectionId]: workspace,
      }
      projectsByConnection.value = {
        ...projectsByConnection.value,
        [connectionId]: projectList,
      }
      overviews.value = {
        ...overviews.value,
        [connectionId]: overview,
      }

      if (!currentWorkspaceId.value) {
        currentWorkspaceId.value = workspace.id
      }
      syncCurrentProjectSelection(connectionId, workspace.defaultProjectId)
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load workspace scope')
      }
    } finally {
      if (requestTokens.value[connectionId] === token) {
        loadingByConnection.value = {
          ...loadingByConnection.value,
          [connectionId]: false,
        }
      }
    }
  }

  async function loadProjectDashboard(projectId = currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    loadingByConnection.value = {
      ...loadingByConnection.value,
      [connectionId]: true,
    }

    try {
      const dashboard = await client.projects.getDashboard(projectId)
      if (requestTokens.value[connectionId] !== token) {
        return null
      }

      dashboards.value = {
        ...dashboards.value,
        [`${connectionId}:${projectId}`]: dashboard,
      }
      return dashboard
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load project dashboard')
      }
      return null
    } finally {
      if (requestTokens.value[connectionId] === token) {
        loadingByConnection.value = {
          ...loadingByConnection.value,
          [connectionId]: false,
        }
      }
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextSummaries = { ...summaries.value }
    const nextOverviews = { ...overviews.value }
    const nextProjects = { ...projectsByConnection.value }
    const nextLoading = { ...loadingByConnection.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    delete nextSummaries[workspaceConnectionId]
    delete nextOverviews[workspaceConnectionId]
    delete nextProjects[workspaceConnectionId]
    delete nextLoading[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    summaries.value = nextSummaries
    overviews.value = nextOverviews
    projectsByConnection.value = nextProjects
    loadingByConnection.value = nextLoading
    errors.value = nextErrors
    requestTokens.value = nextTokens
    Object.keys(dashboards.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete dashboards.value[key]
      })
    Object.keys(projectRuntimeConfigs.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete projectRuntimeConfigs.value[key]
        delete projectRuntimeDrafts.value[key]
        delete projectRuntimeValidations.value[key]
        delete projectRuntimeLoading.value[key]
        delete projectRuntimeSaving.value[key]
        delete projectRuntimeValidating.value[key]
      })
  }

  function setProjectRuntimeDraft(projectId: string, value: string, workspaceConnectionId?: string) {
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    if (!key) {
      return
    }
    projectRuntimeDrafts.value = {
      ...projectRuntimeDrafts.value,
      [key]: value,
    }
  }

  function applyProjectRuntimeConfigState(key: string, config: RuntimeEffectiveConfig) {
    projectRuntimeConfigs.value = {
      ...projectRuntimeConfigs.value,
      [key]: config,
    }
    projectRuntimeDrafts.value = {
      ...projectRuntimeDrafts.value,
      [key]: createRuntimeConfigDraftsFromConfig(config).project,
    }
    projectRuntimeValidations.value = {
      ...projectRuntimeValidations.value,
      [key]: config.validation,
    }
  }

  function getProjectSettings(projectId = currentProjectId.value, workspaceConnectionId?: string): ProjectSettingsConfig {
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    return parseProjectSettingsDocument(findProjectRuntimeSourceDocument(key ? projectRuntimeConfigs.value[key] ?? null : null))
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
    projectRuntimeValidating.value = {
      ...projectRuntimeValidating.value,
      [key]: true,
    }

    try {
      const result = await client.runtime.validateProjectConfig(projectId, {
        scope: 'project',
        patch,
      })
      projectRuntimeValidations.value = {
        ...projectRuntimeValidations.value,
        [key]: result,
      }
      return result
    } catch (cause) {
      const result = {
        valid: false,
        errors: [cause instanceof Error ? cause.message : 'Failed to validate project runtime config'],
        warnings: [],
      }
      projectRuntimeValidations.value = {
        ...projectRuntimeValidations.value,
        [key]: result,
      }
      setConnectionError(connectionId, result.errors[0] ?? '')
      return result
    } finally {
      projectRuntimeValidating.value = {
        ...projectRuntimeValidating.value,
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

    projectRuntimeSaving.value = {
      ...projectRuntimeSaving.value,
      [key]: true,
    }

    try {
      const config = await client.runtime.saveProjectConfig(projectId, {
        scope: 'project',
        patch,
      })
      applyProjectRuntimeConfigState(key, config)
      setConnectionError(connectionId, '')
      return config
    } catch (cause) {
      setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to save project runtime config')
      return null
    } finally {
      projectRuntimeSaving.value = {
        ...projectRuntimeSaving.value,
        [key]: false,
      }
    }
  }

  async function createProject(input: CreateProjectRequest, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient

    try {
      const project = await client.projects.create(input)
      setProjectsForConnection(connectionId, [...(projectsByConnection.value[connectionId] ?? []), project])
      if (connectionId === activeConnectionId.value && !currentProjectId.value && project.status === 'active') {
        currentProjectId.value = project.id
      }
      setConnectionError(connectionId, '')
      return project
    } catch (cause) {
      setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to create project')
      return null
    }
  }

  async function updateProject(projectId: string, input: UpdateProjectRequest, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient

    try {
      const project = await client.projects.update(projectId, input)
      setProjectsForConnection(
        connectionId,
        (projectsByConnection.value[connectionId] ?? []).map(item => item.id === projectId ? project : item),
      )
      setConnectionError(connectionId, '')
      return project
    } catch (cause) {
      setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to update project')
      return null
    }
  }

  async function archiveProject(projectId = currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { connectionId } = resolvedClient
    const projects = projectsByConnection.value[connectionId] ?? []
    const target = projects.find(project => project.id === projectId)
    if (!target) {
      return null
    }

    const nextActiveProject = activeProjectsForConnection(connectionId).find(project => project.id !== projectId)
    if (target.status === 'active' && !nextActiveProject) {
      setConnectionError(connectionId, 'Cannot archive the last active project')
      return null
    }

    const updated = await updateProject(projectId, {
      name: target.name,
      description: target.description,
      status: 'archived',
      assignments: target.assignments,
    }, connectionId)
    if (!updated) {
      return null
    }

    if (currentProjectId.value === projectId) {
      currentProjectId.value = nextActiveProject?.id ?? ''
    }
    if (nextActiveProject) {
      setDefaultProjectIdForConnection(connectionId, nextActiveProject.id)
    }
    return updated
  }

  async function restoreProject(projectId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { connectionId } = resolvedClient
    const projects = projectsByConnection.value[connectionId] ?? []
    const target = projects.find(project => project.id === projectId)
    if (!target) {
      return null
    }

    const updated = await updateProject(projectId, {
      name: target.name,
      description: target.description,
      status: 'active',
      assignments: target.assignments,
    }, connectionId)
    if (!updated) {
      return null
    }

    if (!currentProjectId.value) {
      currentProjectId.value = updated.id
    }
    return updated
  }

  async function loadProjectRuntimeConfig(projectId = currentProjectId.value, force = false, workspaceConnectionId?: string) {
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
    if (projectRuntimeConfigs.value[key] && !force) {
      return projectRuntimeConfigs.value[key]
    }

    projectRuntimeLoading.value = {
      ...projectRuntimeLoading.value,
      [key]: true,
    }

    try {
      const config = await client.runtime.getProjectConfig(projectId)
      applyProjectRuntimeConfigState(key, config)
      projectRuntimeValidations.value = {
        ...projectRuntimeValidations.value,
        [key]: null,
      }
      return config
    } catch (cause) {
      setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load project runtime config')
      return null
    } finally {
      projectRuntimeLoading.value = {
        ...projectRuntimeLoading.value,
        [key]: false,
      }
    }
  }

  async function validateProjectRuntimeConfig(projectId = currentProjectId.value, workspaceConnectionId?: string): Promise<RuntimeConfigValidationResult> {
    if (!projectId) {
      return {
        valid: false,
        errors: ['Project runtime config requires a project id'],
        warnings: [],
      }
    }

    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    const patch = parseRuntimeConfigDraft('project', projectRuntimeDrafts.value[key] ?? '{}')
    return await validateProjectRuntimePatch(projectId, patch.patch, workspaceConnectionId)
  }

  async function saveProjectRuntimeConfig(projectId = currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }
    const key = resolveProjectRuntimeKey(projectId, workspaceConnectionId)
    const patch = parseRuntimeConfigDraft('project', projectRuntimeDrafts.value[key] ?? '{}')
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
    currentWorkspaceId,
    currentProjectId,
    currentConversationId,
    activeConnectionId,
    activeWorkspace,
    projects,
    activeProject,
    activeOverview,
    activeDashboard,
    activeProjectRuntimeConfig,
    activeProjectSettings,
    activeProjectRuntimeDraft,
    activeProjectRuntimeValidation,
    activeProjectRuntimeLoading,
    activeProjectRuntimeSaving,
    activeProjectRuntimeValidating,
    loading,
    error,
    syncRouteScope,
    bootstrap,
    createProject,
    updateProject,
    archiveProject,
    restoreProject,
    loadProjectDashboard,
    setProjectRuntimeDraft,
    getProjectSettings,
    loadProjectRuntimeConfig,
    validateProjectRuntimeConfig,
    saveProjectRuntimeConfig,
    saveProjectModelSettings,
    saveProjectToolSettings,
    saveProjectAgentSettings,
    clearWorkspaceScope,
  }
})
