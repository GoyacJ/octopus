import type { ComputedRef, Ref } from 'vue'

import type {
  CreateProjectRequest,
  ProjectDashboardSnapshot,
  ProjectRecord,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  UpdateProjectRequest,
  WorkspaceOverviewSnapshot,
} from '@octopus/schema'

import {
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

interface WorkspaceActionContext {
  activeConnectionId: ComputedRef<string>
  currentWorkspaceId: Ref<string>
  currentProjectId: Ref<string>
  currentConversationId: Ref<string>
  summaries: Ref<Record<string, WorkspaceOverviewSnapshot['workspace']>>
  overviews: Ref<Record<string, WorkspaceOverviewSnapshot>>
  projectsByConnection: Ref<Record<string, ProjectRecord[]>>
  dashboards: Ref<Record<string, ProjectDashboardSnapshot>>
  projectRuntimeConfigs: Ref<Record<string, RuntimeEffectiveConfig>>
  projectRuntimeDrafts: Ref<Record<string, string>>
  projectRuntimeValidations: Ref<Record<string, RuntimeConfigValidationResult | null>>
  projectRuntimeLoading: Ref<Record<string, boolean>>
  projectRuntimeSaving: Ref<Record<string, boolean>>
  projectRuntimeValidating: Ref<Record<string, boolean>>
  loadingByConnection: Ref<Record<string, boolean>>
  errors: Ref<Record<string, string>>
  requestTokens: Ref<Record<string, number>>
  bootstrapLoadedAtByConnection: Ref<Record<string, number>>
  dashboardLoadedAtByKey: Ref<Record<string, number>>
}

export function createWorkspaceActions(context: WorkspaceActionContext) {
  const bootstrapInflightByConnection: Record<string, Promise<void> | undefined> = {}
  const dashboardInflightByKey: Record<string, Promise<ProjectDashboardSnapshot | null> | undefined> = {}

  function logDevTiming(label: string, startedAt: number, detail?: string) {
    if (!import.meta.env.DEV) {
      return
    }

    const suffix = detail ? ` ${detail}` : ''
    console.debug(`[workspace] ${label}${suffix} ${Math.round(performance.now() - startedAt)}ms`)
  }

  function setConnectionError(connectionId: string, message: string) {
    context.errors.value = {
      ...context.errors.value,
      [connectionId]: message,
    }
  }

  function activeProjectsForConnection(connectionId: string) {
    return (context.projectsByConnection.value[connectionId] ?? []).filter(project => project.status === 'active')
  }

  function setProjectsForConnection(connectionId: string, projects: ProjectRecord[]) {
    context.projectsByConnection.value = {
      ...context.projectsByConnection.value,
      [connectionId]: projects,
    }
  }

  function setDefaultProjectIdForConnection(connectionId: string, projectId: string) {
    const summary = context.summaries.value[connectionId]
    if (summary) {
      context.summaries.value = {
        ...context.summaries.value,
        [connectionId]: {
          ...summary,
          defaultProjectId: projectId,
        },
      }
    }

    const overview = context.overviews.value[connectionId]
    if (overview) {
      context.overviews.value = {
        ...context.overviews.value,
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
    const projects = context.projectsByConnection.value[connectionId] ?? []
    const activeProjects = projects.filter(project => project.status === 'active')
    const preferred = preferredProjectId
      ? activeProjects.find(project => project.id === preferredProjectId) ?? projects.find(project => project.id === preferredProjectId)
      : undefined
    const current = projects.find(project => project.id === context.currentProjectId.value && project.status === 'active')
    const fallback = activeProjects[0] ?? projects[0]
    const nextProject = current ?? preferred ?? fallback

    if (connectionId === context.activeConnectionId.value || !context.currentProjectId.value) {
      context.currentProjectId.value = nextProject?.id ?? ''
    }

    if (!preferred && activeProjects[0] && connectionId === context.activeConnectionId.value) {
      setDefaultProjectIdForConnection(connectionId, activeProjects[0].id)
    }
  }

  function syncRouteScope(workspaceId?: string, projectId?: string, conversationId?: string) {
    if (workspaceId) {
      context.currentWorkspaceId.value = workspaceId
    }
    if (projectId !== undefined) {
      context.currentProjectId.value = projectId
    }
    if (conversationId !== undefined) {
      context.currentConversationId.value = conversationId
    }
  }

  function getProjectDashboard(projectId = context.currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    const connectionId = resolvedClient?.connectionId ?? context.activeConnectionId.value
    if (!connectionId) {
      return null
    }

    return context.dashboards.value[`${connectionId}:${projectId}`] ?? null
  }

  async function bootstrap(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const startedAt = performance.now()
    const token = createWorkspaceRequestToken(context.requestTokens.value[connectionId] ?? 0)
    context.requestTokens.value[connectionId] = token
    context.loadingByConnection.value = {
      ...context.loadingByConnection.value,
      [connectionId]: true,
    }
    context.errors.value = {
      ...context.errors.value,
      [connectionId]: '',
    }

    try {
      const [workspace, projectList, overview] = await Promise.all([
        client.workspace.get(),
        client.projects.list(),
        client.workspace.getOverview(),
      ])

      if (context.requestTokens.value[connectionId] !== token) {
        return
      }

      context.summaries.value = {
        ...context.summaries.value,
        [connectionId]: workspace,
      }
      context.projectsByConnection.value = {
        ...context.projectsByConnection.value,
        [connectionId]: projectList,
      }
      context.overviews.value = {
        ...context.overviews.value,
        [connectionId]: overview,
      }
      context.bootstrapLoadedAtByConnection.value = {
        ...context.bootstrapLoadedAtByConnection.value,
        [connectionId]: Date.now(),
      }

      if (!context.currentWorkspaceId.value) {
        context.currentWorkspaceId.value = workspace.id
      }
      syncCurrentProjectSelection(connectionId, workspace.defaultProjectId)
    } catch (cause) {
      if (context.requestTokens.value[connectionId] === token) {
        setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load workspace scope')
      }
    } finally {
      if (context.requestTokens.value[connectionId] === token) {
        context.loadingByConnection.value = {
          ...context.loadingByConnection.value,
          [connectionId]: false,
        }
      }
      logDevTiming('bootstrap', startedAt, connectionId)
    }
  }

  function hasBootstrapCache(connectionId: string) {
    return Boolean(
      context.summaries.value[connectionId]
      && context.overviews.value[connectionId]
      && context.projectsByConnection.value[connectionId],
    )
  }

  async function ensureWorkspaceBootstrap(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { connectionId } = resolvedClient
    if (!options.force && hasBootstrapCache(connectionId)) {
      return
    }

    const inflight = bootstrapInflightByConnection[connectionId]
    if (inflight && !options.force) {
      await inflight
      return
    }

    const task = bootstrap(connectionId)
    bootstrapInflightByConnection[connectionId] = task
    try {
      await task
    } finally {
      if (bootstrapInflightByConnection[connectionId] === task) {
        delete bootstrapInflightByConnection[connectionId]
      }
    }
  }

  async function loadProjectDashboard(
    projectId = context.currentProjectId.value,
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient
    const dashboardKey = `${connectionId}:${projectId}`
    if (!options.force && context.dashboards.value[dashboardKey]) {
      return context.dashboards.value[dashboardKey] ?? null
    }

    const inflight = dashboardInflightByKey[dashboardKey]
    if (inflight && !options.force) {
      return await inflight
    }

    const startedAt = performance.now()
    const token = createWorkspaceRequestToken(context.requestTokens.value[connectionId] ?? 0)
    context.requestTokens.value[connectionId] = token
    context.loadingByConnection.value = {
      ...context.loadingByConnection.value,
      [connectionId]: true,
    }

    const task = (async () => {
      const dashboard = await client.projects.getDashboard(projectId)
      if (context.requestTokens.value[connectionId] !== token) {
        return null
      }

      context.dashboards.value = {
        ...context.dashboards.value,
        [dashboardKey]: dashboard,
      }
      context.dashboardLoadedAtByKey.value = {
        ...context.dashboardLoadedAtByKey.value,
        [dashboardKey]: Date.now(),
      }
      return dashboard
    })()
    dashboardInflightByKey[dashboardKey] = task

    try {
      return await task
    } catch (cause) {
      if (context.requestTokens.value[connectionId] === token) {
        setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to load project dashboard')
      }
      return null
    } finally {
      if (dashboardInflightByKey[dashboardKey] === task) {
        delete dashboardInflightByKey[dashboardKey]
      }
      if (context.requestTokens.value[connectionId] === token) {
        context.loadingByConnection.value = {
          ...context.loadingByConnection.value,
          [connectionId]: false,
        }
      }
      logDevTiming('loadProjectDashboard', startedAt, dashboardKey)
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextSummaries = { ...context.summaries.value }
    const nextOverviews = { ...context.overviews.value }
    const nextProjects = { ...context.projectsByConnection.value }
    const nextLoading = { ...context.loadingByConnection.value }
    const nextErrors = { ...context.errors.value }
    const nextTokens = { ...context.requestTokens.value }
    const nextBootstrapLoadedAt = { ...context.bootstrapLoadedAtByConnection.value }
    delete nextSummaries[workspaceConnectionId]
    delete nextOverviews[workspaceConnectionId]
    delete nextProjects[workspaceConnectionId]
    delete nextLoading[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    delete nextBootstrapLoadedAt[workspaceConnectionId]
    context.summaries.value = nextSummaries
    context.overviews.value = nextOverviews
    context.projectsByConnection.value = nextProjects
    context.loadingByConnection.value = nextLoading
    context.errors.value = nextErrors
    context.requestTokens.value = nextTokens
    context.bootstrapLoadedAtByConnection.value = nextBootstrapLoadedAt
    delete bootstrapInflightByConnection[workspaceConnectionId]
    Object.keys(context.dashboards.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete context.dashboards.value[key]
        delete context.dashboardLoadedAtByKey.value[key]
        delete dashboardInflightByKey[key]
      })
    Object.keys(context.projectRuntimeConfigs.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete context.projectRuntimeConfigs.value[key]
        delete context.projectRuntimeDrafts.value[key]
        delete context.projectRuntimeValidations.value[key]
        delete context.projectRuntimeLoading.value[key]
        delete context.projectRuntimeSaving.value[key]
        delete context.projectRuntimeValidating.value[key]
      })
  }

  async function createProject(input: CreateProjectRequest, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { client, connectionId } = resolvedClient

    try {
      const project = await client.projects.create(input)
      setProjectsForConnection(connectionId, [...(context.projectsByConnection.value[connectionId] ?? []), project])
      if (connectionId === context.activeConnectionId.value && !context.currentProjectId.value && project.status === 'active') {
        context.currentProjectId.value = project.id
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
        (context.projectsByConnection.value[connectionId] ?? []).map(item => item.id === projectId ? project : item),
      )
      setConnectionError(connectionId, '')
      return project
    } catch (cause) {
      setConnectionError(connectionId, cause instanceof Error ? cause.message : 'Failed to update project')
      return null
    }
  }

  async function archiveProject(projectId = context.currentProjectId.value, workspaceConnectionId?: string) {
    if (!projectId) {
      return null
    }

    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const { connectionId } = resolvedClient
    const projects = context.projectsByConnection.value[connectionId] ?? []
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
      resourceDirectory: target.resourceDirectory,
      status: 'archived',
      assignments: target.assignments,
    }, connectionId)
    if (!updated) {
      return null
    }

    if (context.currentProjectId.value === projectId) {
      context.currentProjectId.value = nextActiveProject?.id ?? ''
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
    const projects = context.projectsByConnection.value[connectionId] ?? []
    const target = projects.find(project => project.id === projectId)
    if (!target) {
      return null
    }

    const updated = await updateProject(projectId, {
      name: target.name,
      description: target.description,
      resourceDirectory: target.resourceDirectory,
      status: 'active',
      assignments: target.assignments,
    }, connectionId)
    if (!updated) {
      return null
    }

    if (!context.currentProjectId.value) {
      context.currentProjectId.value = updated.id
    }
    return updated
  }

  return {
    setConnectionError,
    activeProjectsForConnection,
    setProjectsForConnection,
    setDefaultProjectIdForConnection,
    syncCurrentProjectSelection,
    syncRouteScope,
    bootstrap,
    ensureWorkspaceBootstrap,
    loadProjectDashboard,
    getProjectDashboard,
    clearWorkspaceScope,
    createProject,
    updateProject,
    archiveProject,
    restoreProject,
  }
}
