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
} from './workspace-scope'
import { parseProjectSettingsDocument, findProjectRuntimeSourceDocument } from './project_settings'
import { createWorkspaceActions } from './workspace_actions'
import { createWorkspaceRuntimeActions } from './workspace_runtime'

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
  const bootstrapLoadedAtByConnection = ref<Record<string, number>>({})
  const dashboardLoadedAtByKey = ref<Record<string, number>>({})

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
  const workspaceActions = createWorkspaceActions({
    activeConnectionId,
    currentWorkspaceId,
    currentProjectId,
    currentConversationId,
    summaries,
    overviews,
    projectsByConnection,
    dashboards,
    projectRuntimeConfigs,
    projectRuntimeDrafts,
    projectRuntimeValidations,
    projectRuntimeLoading,
    projectRuntimeSaving,
    projectRuntimeValidating,
    loadingByConnection,
    errors,
    requestTokens,
    bootstrapLoadedAtByConnection,
    dashboardLoadedAtByKey,
  })
  const runtimeActions = createWorkspaceRuntimeActions({
    activeConnectionId,
    currentProjectId,
    projectRuntimeConfigs,
    projectRuntimeDrafts,
    projectRuntimeValidations,
    projectRuntimeLoading,
    projectRuntimeSaving,
    projectRuntimeValidating,
    setConnectionError: workspaceActions.setConnectionError,
  })

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
    syncRouteScope: workspaceActions.syncRouteScope,
    bootstrap: workspaceActions.bootstrap,
    ensureWorkspaceBootstrap: workspaceActions.ensureWorkspaceBootstrap,
    createProject: workspaceActions.createProject,
    updateProject: workspaceActions.updateProject,
    archiveProject: workspaceActions.archiveProject,
    restoreProject: workspaceActions.restoreProject,
    loadProjectDashboard: workspaceActions.loadProjectDashboard,
    getProjectDashboard: workspaceActions.getProjectDashboard,
    setProjectRuntimeDraft: runtimeActions.setProjectRuntimeDraft,
    getProjectSettings: runtimeActions.getProjectSettings,
    loadProjectRuntimeConfig: runtimeActions.loadProjectRuntimeConfig,
    ensureProjectRuntimeConfig: runtimeActions.ensureProjectRuntimeConfig,
    validateProjectRuntimeConfig: runtimeActions.validateProjectRuntimeConfig,
    saveProjectRuntimeConfig: runtimeActions.saveProjectRuntimeConfig,
    saveProjectModelSettings: runtimeActions.saveProjectModelSettings,
    saveProjectToolSettings: runtimeActions.saveProjectToolSettings,
    saveProjectAgentSettings: runtimeActions.saveProjectAgentSettings,
    clearWorkspaceScope: workspaceActions.clearWorkspaceScope,
  }
})
