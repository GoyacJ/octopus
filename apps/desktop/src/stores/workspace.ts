import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ProjectDashboardSnapshot,
  ProjectRecord,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  WorkspaceOverviewSnapshot,
} from '@octopus/schema'

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
      if (!currentProjectId.value) {
        currentProjectId.value = workspace.defaultProjectId ?? projectList[0]?.id ?? ''
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace scope',
        }
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
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project dashboard',
        }
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
        [key]: null,
      }
      return config
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project runtime config',
      }
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
      const patch = parseRuntimeConfigDraft('project', projectRuntimeDrafts.value[key] ?? '{}')
      const result = await client.runtime.validateProjectConfig(projectId, patch)
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
      } satisfies RuntimeConfigValidationResult
      projectRuntimeValidations.value = {
        ...projectRuntimeValidations.value,
        [key]: result,
      }
      errors.value = {
        ...errors.value,
        [connectionId]: result.errors[0] ?? '',
      }
      return result
    } finally {
      projectRuntimeValidating.value = {
        ...projectRuntimeValidating.value,
        [key]: false,
      }
    }
  }

  async function saveProjectRuntimeConfig(projectId = currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    const key = resolveProjectRuntimeKey(projectId, connectionId)
    const validation = await validateProjectRuntimeConfig(projectId, connectionId)
    if (!validation.valid) {
      return null
    }

    projectRuntimeSaving.value = {
      ...projectRuntimeSaving.value,
      [key]: true,
    }

    try {
      const patch = parseRuntimeConfigDraft('project', projectRuntimeDrafts.value[key] ?? '{}')
      const config = await client.runtime.saveProjectConfig(projectId, patch)
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
      return config
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to save project runtime config',
      }
      return null
    } finally {
      projectRuntimeSaving.value = {
        ...projectRuntimeSaving.value,
        [key]: false,
      }
    }
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
    activeProjectRuntimeDraft,
    activeProjectRuntimeValidation,
    activeProjectRuntimeLoading,
    activeProjectRuntimeSaving,
    activeProjectRuntimeValidating,
    loading,
    error,
    syncRouteScope,
    bootstrap,
    loadProjectDashboard,
    setProjectRuntimeDraft,
    loadProjectRuntimeConfig,
    validateProjectRuntimeConfig,
    saveProjectRuntimeConfig,
    clearWorkspaceScope,
  }
})
