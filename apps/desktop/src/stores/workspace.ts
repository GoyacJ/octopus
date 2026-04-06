import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ProjectDashboardSnapshot,
  ProjectRecord,
  WorkspaceOverviewSnapshot,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

type WorkspaceScoped<T> = Record<string, T>

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentWorkspaceId = ref('')
  const currentProjectId = ref('')
  const currentConversationId = ref('')

  const summaries = ref<WorkspaceScoped<WorkspaceOverviewSnapshot['workspace']>>({})
  const overviews = ref<WorkspaceScoped<WorkspaceOverviewSnapshot>>({})
  const projectsByConnection = ref<WorkspaceScoped<ProjectRecord[]>>({})
  const dashboards = ref<Record<string, ProjectDashboardSnapshot>>({})
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
  const loading = computed(() => loadingByConnection.value[activeConnectionId.value] ?? false)
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

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
    loading,
    error,
    syncRouteScope,
    bootstrap,
    loadProjectDashboard,
    clearWorkspaceScope,
  }
})
