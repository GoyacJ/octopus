import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CreateTaskInterventionRequest,
  CreateTaskRequest,
  LaunchTaskRequest,
  RerunTaskRequest,
  TaskDetail,
  TaskRunSummary,
  TaskSummary,
  UpdateTaskRequest,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

function taskSummaryFromDetail(detail: TaskDetail): TaskSummary {
  return {
    id: detail.id,
    projectId: detail.projectId,
    title: detail.title,
    goal: detail.goal,
    defaultActorRef: detail.defaultActorRef,
    status: detail.status,
    scheduleSpec: detail.scheduleSpec ?? null,
    nextRunAt: detail.nextRunAt ?? null,
    lastRunAt: detail.lastRunAt ?? null,
    latestResultSummary: detail.latestResultSummary ?? null,
    latestFailureCategory: detail.latestFailureCategory ?? null,
    latestTransition: detail.latestTransition ?? null,
    viewStatus: detail.viewStatus,
    attentionReasons: [...detail.attentionReasons],
    attentionUpdatedAt: detail.attentionUpdatedAt ?? null,
    activeTaskRunId: detail.activeTaskRunId ?? null,
    analyticsSummary: { ...detail.analyticsSummary },
    updatedAt: detail.updatedAt,
  }
}

function upsertTaskSummary(records: TaskSummary[], summary: TaskSummary) {
  const index = records.findIndex(task => task.id === summary.id)
  const nextRecords = index >= 0
    ? records.map(task => task.id === summary.id ? summary : task)
    : [summary, ...records]

  return [...nextRecords].sort((left, right) =>
    right.updatedAt - left.updatedAt || left.id.localeCompare(right.id))
}

export interface TaskContextDraftRef {
  kind: string
  refId: string
  title: string
  subtitle?: string
  versionRef?: string | null
  pinMode: string
}

export interface TaskContextDraftBundle {
  refs: TaskContextDraftRef[]
  pinnedInstructions: string
  resolutionMode: string
  lastResolvedAt?: number | null
}

export interface TaskEditorDraft {
  title: string
  goal: string
  brief: string
  defaultActorRef: string
  scheduleSpec?: string | null
  contextBundle: TaskContextDraftBundle
}

export interface TaskListFilterState {
  status?: string | null
  attentionOnly: boolean
  actorRef?: string | null
  scheduleMode?: string | null
  query: string
}

export interface TaskNotificationView {
  taskId: string
  reason: string
  summary: string
  at: number
  read: boolean
}

export const useProjectTaskStore = defineStore('project_task', () => {
  const listByProjectId = ref<Record<string, TaskSummary[]>>({})
  const detailByTaskId = ref<Record<string, TaskDetail>>({})
  const runHistoryByTaskId = ref<Record<string, TaskRunSummary[]>>({})
  const selectedTaskIdByProjectId = ref<Record<string, string | null>>({})
  const filtersByProjectId = ref<Record<string, TaskListFilterState>>({})
  const draftsByTaskId = ref<Record<string, TaskEditorDraft>>({})
  const createDraftByProjectId = ref<Record<string, TaskEditorDraft | null>>({})
  const notificationsByTaskId = ref<Record<string, TaskNotificationView[]>>({})
  const loading = ref({
    list: false,
    detailByTaskId: {} as Record<string, boolean>,
    launchByTaskId: {} as Record<string, boolean>,
    saveByTaskId: {} as Record<string, boolean>,
  })
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  function projectTasksFor(projectId?: string | null) {
    if (!projectId) {
      return []
    }

    return listByProjectId.value[projectId] ?? []
  }

  function getCachedDetail(taskId?: string | null) {
    if (!taskId) {
      return null
    }

    return detailByTaskId.value[taskId] ?? null
  }

  function getCachedRunHistory(taskId?: string | null) {
    if (!taskId) {
      return []
    }

    return runHistoryByTaskId.value[taskId]
      ?? detailByTaskId.value[taskId]?.runHistory
      ?? []
  }

  function setSelectedTask(projectId: string, taskId: string | null) {
    selectedTaskIdByProjectId.value = {
      ...selectedTaskIdByProjectId.value,
      [projectId]: taskId,
    }
  }

  function cacheTaskDetail(detail: TaskDetail, runs?: TaskRunSummary[]) {
    const cachedRuns = runs ?? detail.runHistory ?? []
    const cachedDetail = {
      ...detail,
      runHistory: cachedRuns,
    }

    detailByTaskId.value = {
      ...detailByTaskId.value,
      [detail.id]: cachedDetail,
    }
    runHistoryByTaskId.value = {
      ...runHistoryByTaskId.value,
      [detail.id]: cachedRuns,
    }
    listByProjectId.value = {
      ...listByProjectId.value,
      [detail.projectId]: upsertTaskSummary(
        listByProjectId.value[detail.projectId] ?? [],
        taskSummaryFromDetail(cachedDetail),
      ),
    }
  }

  async function refreshTaskCache(
    projectId: string,
    taskId: string,
    client: NonNullable<ReturnType<typeof resolveWorkspaceClientForConnection>>['client'],
    fallbackRun?: TaskRunSummary | null,
  ) {
    const [detail, runs] = await Promise.all([
      client.tasks.getDetail(projectId, taskId),
      client.tasks.listRuns(projectId, taskId),
    ])

    const resolvedRuns = runs.length
      ? runs
      : fallbackRun
        ? [fallbackRun, ...(detail.runHistory ?? []).filter(existing => existing.id !== fallbackRun.id)]
        : detail.runHistory ?? runHistoryByTaskId.value[taskId] ?? []

    cacheTaskDetail(detail, resolvedRuns)
    setSelectedTask(projectId, taskId)

    return {
      detail: detailByTaskId.value[taskId] ?? detail,
      runs: resolvedRuns,
    }
  }

  async function loadProjectTasks(projectId = workspaceStore.currentProjectId, workspaceConnectionId?: string) {
    if (!projectId) {
      return []
    }
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return []
    }

    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    loading.value = {
      ...loading.value,
      list: true,
    }

    try {
      const tasks = await client.tasks.listProject(projectId)
      if (requestTokens.value[connectionId] !== token) {
        return projectTasksFor(projectId)
      }

      listByProjectId.value = {
        ...listByProjectId.value,
        [projectId]: tasks,
      }
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      const selectedTaskId = selectedTaskIdByProjectId.value[projectId] ?? null
      if (!selectedTaskId || !tasks.some(task => task.id === selectedTaskId)) {
        setSelectedTask(projectId, tasks[0]?.id ?? null)
      }

      return tasks
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project tasks',
        }
      }

      return []
    } finally {
      if (requestTokens.value[connectionId] === token) {
        loading.value = {
          ...loading.value,
          list: false,
        }
      }
    }
  }

  async function getTaskDetail(
    projectId: string,
    taskId: string,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    loading.value = {
      ...loading.value,
      detailByTaskId: {
        ...loading.value.detailByTaskId,
        [taskId]: true,
      },
    }

    try {
      const [detail, runs] = await Promise.all([
        client.tasks.getDetail(projectId, taskId),
        client.tasks.listRuns(projectId, taskId),
      ])

      const resolvedRuns = runs.length ? runs : detail.runHistory ?? []
      cacheTaskDetail(detail, resolvedRuns)
      setSelectedTask(projectId, taskId)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return detailByTaskId.value[taskId] ?? detail
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load task detail',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        detailByTaskId: {
          ...loading.value.detailByTaskId,
          [taskId]: false,
        },
      }
    }
  }

  async function createTask(
    projectId: string,
    input: CreateTaskRequest,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const loadingKey = `create:${projectId}`
    loading.value = {
      ...loading.value,
      saveByTaskId: {
        ...loading.value.saveByTaskId,
        [loadingKey]: true,
      },
    }

    try {
      const detail = await client.tasks.createProject(projectId, input)
      cacheTaskDetail(detail, detail.runHistory ?? [])
      setSelectedTask(projectId, detail.id)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return detailByTaskId.value[detail.id] ?? detail
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to create task',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        saveByTaskId: {
          ...loading.value.saveByTaskId,
          [loadingKey]: false,
        },
      }
    }
  }

  async function updateTask(
    projectId: string,
    taskId: string,
    input: UpdateTaskRequest,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    loading.value = {
      ...loading.value,
      saveByTaskId: {
        ...loading.value.saveByTaskId,
        [taskId]: true,
      },
    }

    try {
      const detail = await client.tasks.updateProject(projectId, taskId, input)
      cacheTaskDetail(detail, detail.runHistory ?? runHistoryByTaskId.value[taskId] ?? [])
      setSelectedTask(projectId, detail.id)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return detailByTaskId.value[taskId] ?? detail
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to update task',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        saveByTaskId: {
          ...loading.value.saveByTaskId,
          [taskId]: false,
        },
      }
    }
  }

  async function launchTask(
    projectId: string,
    taskId: string,
    input: LaunchTaskRequest,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    loading.value = {
      ...loading.value,
      launchByTaskId: {
        ...loading.value.launchByTaskId,
        [taskId]: true,
      },
    }

    try {
      const run = await client.tasks.launch(projectId, taskId, input)
      await refreshTaskCache(projectId, taskId, client, run)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return run
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to launch task',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        launchByTaskId: {
          ...loading.value.launchByTaskId,
          [taskId]: false,
        },
      }
    }
  }

  async function rerunTask(
    projectId: string,
    taskId: string,
    input: RerunTaskRequest,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    loading.value = {
      ...loading.value,
      launchByTaskId: {
        ...loading.value.launchByTaskId,
        [taskId]: true,
      },
    }

    try {
      const run = await client.tasks.rerun(projectId, taskId, input)
      await refreshTaskCache(projectId, taskId, client, run)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return run
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to rerun task',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        launchByTaskId: {
          ...loading.value.launchByTaskId,
          [taskId]: false,
        },
      }
    }
  }

  async function createIntervention(
    projectId: string,
    taskId: string,
    input: CreateTaskInterventionRequest,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    loading.value = {
      ...loading.value,
      saveByTaskId: {
        ...loading.value.saveByTaskId,
        [taskId]: true,
      },
    }

    try {
      const intervention = await client.tasks.createIntervention(projectId, taskId, input)
      await refreshTaskCache(projectId, taskId, client)
      errors.value = {
        ...errors.value,
        [connectionId]: '',
      }

      return intervention
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to record task intervention',
      }
      return null
    } finally {
      loading.value = {
        ...loading.value,
        saveByTaskId: {
          ...loading.value.saveByTaskId,
          [taskId]: false,
        },
      }
    }
  }

  function selectTask(projectId: string, taskId: string | null) {
    setSelectedTask(projectId, taskId)
  }

  return {
    listByProjectId,
    detailByTaskId,
    runHistoryByTaskId,
    selectedTaskIdByProjectId,
    filtersByProjectId,
    draftsByTaskId,
    createDraftByProjectId,
    notificationsByTaskId,
    loading,
    error,
    projectTasksFor,
    getCachedDetail,
    getCachedRunHistory,
    loadProjectTasks,
    getTaskDetail,
    createTask,
    updateTask,
    launchTask,
    rerunTask,
    createIntervention,
    selectTask,
  }
})
