// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useShellStore } from '@/stores/shell'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

describe('project task store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
  })

  async function prepareTaskStore() {
    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign', [])
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const store = useProjectTaskStore()
    return { shell, store }
  }

  it('exposes the phase 0 task state buckets for list, detail, drafts, and notifications', async () => {
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const store = useProjectTaskStore()

    expect(store.listByProjectId).toEqual({})
    expect(store.detailByTaskId).toEqual({})
    expect(store.runHistoryByTaskId).toEqual({})
    expect(store.selectedTaskIdByProjectId).toEqual({})
    expect(store.filtersByProjectId).toEqual({})
    expect(store.draftsByTaskId).toEqual({})
    expect(store.createDraftByProjectId).toEqual({})
    expect(store.notificationsByTaskId).toEqual({})
    expect(store.loading).toEqual({
      list: false,
      detailByTaskId: {},
      launchByTaskId: {},
      saveByTaskId: {},
    })
  })

  it('loads project tasks and caches the selected task detail for the active workspace connection', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')

    expect(store.projectTasksFor('proj-redesign')).toEqual(expect.arrayContaining([
      expect.objectContaining({
        id: 'task-redesign-release-brief',
        title: 'Release Brief Refresh',
      }),
      expect.objectContaining({
        id: 'task-redesign-regression-sweep',
        title: 'Regression Sweep',
      }),
    ]))
    expect(store.selectedTaskIdByProjectId['proj-redesign']).toBe('task-redesign-release-brief')

    const detail = await store.getTaskDetail('proj-redesign', 'task-redesign-regression-sweep')

    expect(detail).toMatchObject({
      id: 'task-redesign-regression-sweep',
      goal: 'Run the desktop regression checklist and summarize failures.',
      status: 'attention',
    })
    expect(store.getCachedDetail('task-redesign-regression-sweep')).toMatchObject({
      id: 'task-redesign-regression-sweep',
      projectId: 'proj-redesign',
    })
    expect(store.runHistoryByTaskId['task-redesign-regression-sweep']).toEqual([])
  })

  it('creates, updates, and launches a task while keeping list and detail caches in sync', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')

    const created = await store.createTask('proj-redesign', {
      title: 'Prepare launch checklist',
      goal: 'Create a launch-ready checklist.',
      brief: 'Focus on dependencies and sequencing.',
      defaultActorRef: 'agent:workspace-core',
      scheduleSpec: null,
      contextBundle: {
        refs: [],
        pinnedInstructions: '',
        resolutionMode: 'explicit_only',
        lastResolvedAt: null,
      },
    })

    expect(created).toMatchObject({
      id: 'task-1',
      title: 'Prepare launch checklist',
      brief: 'Focus on dependencies and sequencing.',
      status: 'ready',
    })
    expect(store.selectedTaskIdByProjectId['proj-redesign']).toBe('task-1')
    expect(store.projectTasksFor('proj-redesign')[0]).toMatchObject({
      id: 'task-1',
      title: 'Prepare launch checklist',
      status: 'ready',
    })
    expect(store.getCachedDetail('task-1')).toMatchObject({
      id: 'task-1',
      brief: 'Focus on dependencies and sequencing.',
    })

    const updated = await store.updateTask('proj-redesign', 'task-1', {
      title: 'Prepare launch checklist v2',
      brief: 'Updated brief.',
    })

    expect(updated).toMatchObject({
      id: 'task-1',
      title: 'Prepare launch checklist v2',
      brief: 'Updated brief.',
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-1')).toMatchObject({
      id: 'task-1',
      title: 'Prepare launch checklist v2',
    })
    expect(store.getCachedDetail('task-1')).toMatchObject({
      id: 'task-1',
      title: 'Prepare launch checklist v2',
      brief: 'Updated brief.',
    })

    const run = await store.launchTask('proj-redesign', 'task-1', {
      actorRef: 'agent:workspace-core',
    })

    expect(run).toMatchObject({
      id: 'task-run-1',
      taskId: 'task-1',
      actorRef: 'agent:workspace-core',
      status: 'running',
      triggerType: 'manual',
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-1')).toMatchObject({
      id: 'task-1',
      status: 'running',
      activeTaskRunId: 'task-run-1',
    })
    expect(store.getCachedDetail('task-1')).toMatchObject({
      id: 'task-1',
      status: 'running',
      activeTaskRunId: 'task-run-1',
    })
    expect(store.getCachedRunHistory('task-1')).toEqual(expect.arrayContaining([
      expect.objectContaining({
        id: 'task-run-1',
        status: 'running',
      }),
    ]))
  })

  it('reruns a task and refreshes the cached detail and run history', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')
    await store.getTaskDetail('proj-redesign', 'task-redesign-release-brief')

    const rerun = await store.rerunTask('proj-redesign', 'task-redesign-release-brief', {
      actorRef: 'agent:workspace-core',
      sourceTaskRunId: 'task-run-redesign-release-brief',
    })

    expect(rerun).toMatchObject({
      id: 'task-run-1',
      taskId: 'task-redesign-release-brief',
      actorRef: 'agent:workspace-core',
      status: 'running',
      triggerType: 'rerun',
      conversationId: 'task-conversation-1',
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      status: 'running',
      activeTaskRunId: 'task-run-1',
      viewStatus: 'healthy',
    })
    expect(store.getCachedDetail('task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      status: 'running',
      activeTaskRunId: 'task-run-1',
      latestTransition: expect.objectContaining({
        kind: 'launched',
      }),
    })
    expect(store.getCachedRunHistory('task-redesign-release-brief')).toEqual([
      expect.objectContaining({
        id: 'task-run-1',
        triggerType: 'rerun',
        status: 'running',
      }),
    ])
  })

  it('records a brief intervention and updates the cached detail state', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')
    await store.getTaskDetail('proj-redesign', 'task-redesign-release-brief')

    const intervention = await store.createIntervention('proj-redesign', 'task-redesign-release-brief', {
      type: 'edit_brief',
      taskRunId: 'task-run-redesign-release-brief',
      payload: {
        brief: 'Focus on the final release notes and linked deliverables.',
      },
    })

    expect(intervention).toMatchObject({
      id: 'task-intervention-1',
      taskId: 'task-redesign-release-brief',
      taskRunId: 'task-run-redesign-release-brief',
      type: 'edit_brief',
      status: 'accepted',
    })
    expect(store.getCachedDetail('task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      brief: 'Focus on the final release notes and linked deliverables.',
      latestTransition: expect.objectContaining({
        kind: 'intervened',
      }),
      interventionHistory: [
        expect.objectContaining({
          id: 'task-intervention-1',
          type: 'edit_brief',
        }),
      ],
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      latestTransition: expect.objectContaining({
        kind: 'intervened',
      }),
    })
  })

  it('records takeover intervention state and surfaces attention in list and detail caches', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-governance')
    await store.getTaskDetail('proj-governance', 'task-governance-menu-audit')

    const intervention = await store.createIntervention('proj-governance', 'task-governance-menu-audit', {
      type: 'takeover',
      payload: {},
    })

    expect(intervention).toMatchObject({
      id: 'task-intervention-1',
      taskId: 'task-governance-menu-audit',
      type: 'takeover',
      status: 'accepted',
    })
    expect(store.getCachedDetail('task-governance-menu-audit')).toMatchObject({
      id: 'task-governance-menu-audit',
      viewStatus: 'attention',
      attentionReasons: ['takeover_recommended'],
      interventionHistory: [
        expect.objectContaining({
          id: 'task-intervention-1',
          type: 'takeover',
        }),
      ],
    })
    expect(store.projectTasksFor('proj-governance').find(task => task.id === 'task-governance-menu-audit')).toMatchObject({
      id: 'task-governance-menu-audit',
      viewStatus: 'attention',
      attentionReasons: ['takeover_recommended'],
    })
  })

  it('records an actor-change intervention and updates the task default plus active run actor', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')
    await store.getTaskDetail('proj-redesign', 'task-redesign-release-brief')
    await store.rerunTask('proj-redesign', 'task-redesign-release-brief', {
      actorRef: 'agent:workspace-core',
      sourceTaskRunId: 'task-run-redesign-release-brief',
    })

    const intervention = await store.createIntervention('proj-redesign', 'task-redesign-release-brief', {
      type: 'change_actor' as never,
      taskRunId: 'task-run-1',
      payload: {
        actorRef: 'agent:release-operator',
      },
    })

    expect(intervention).toMatchObject({
      id: 'task-intervention-1',
      taskId: 'task-redesign-release-brief',
      taskRunId: 'task-run-1',
      type: 'change_actor',
      status: 'accepted',
      payload: {
        actorRef: 'agent:release-operator',
      },
    })
    expect(store.getCachedDetail('task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      defaultActorRef: 'agent:release-operator',
      activeRun: expect.objectContaining({
        id: 'task-run-1',
        actorRef: 'agent:release-operator',
      }),
      runHistory: [
        expect.objectContaining({
          id: 'task-run-1',
          actorRef: 'agent:release-operator',
        }),
      ],
      interventionHistory: [
        expect.objectContaining({
          id: 'task-intervention-1',
          type: 'change_actor',
        }),
      ],
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-redesign-release-brief')).toMatchObject({
      id: 'task-redesign-release-brief',
      defaultActorRef: 'agent:release-operator',
    })
  })

  it('approves a waiting-approval task and resumes the active run', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')
    const detail = await store.getTaskDetail('proj-redesign', 'task-redesign-approval-gate')

    expect(detail).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'attention',
      attentionReasons: ['needs_approval'],
      activeRun: expect.objectContaining({
        id: 'task-run-redesign-approval-gate',
        status: 'waiting_approval',
      }),
    })

    const intervention = await store.createIntervention('proj-redesign', 'task-redesign-approval-gate', {
      type: 'approve',
      taskRunId: 'task-run-redesign-approval-gate',
      payload: {},
    })

    expect(intervention).toMatchObject({
      id: 'task-intervention-1',
      taskId: 'task-redesign-approval-gate',
      taskRunId: 'task-run-redesign-approval-gate',
      type: 'approve',
      status: 'applied',
    })
    expect(store.getCachedDetail('task-redesign-approval-gate')).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'running',
      viewStatus: 'healthy',
      attentionReasons: [],
      latestResultSummary: 'Approval received. Continuing the active run.',
      activeRun: expect.objectContaining({
        id: 'task-run-redesign-approval-gate',
        status: 'running',
        viewStatus: 'healthy',
        attentionReasons: [],
      }),
      runHistory: [
        expect.objectContaining({
          id: 'task-run-redesign-approval-gate',
          status: 'running',
          viewStatus: 'healthy',
          attentionReasons: [],
        }),
      ],
      interventionHistory: [
        expect.objectContaining({
          id: 'task-intervention-1',
          type: 'approve',
          status: 'applied',
        }),
      ],
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-redesign-approval-gate')).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'running',
      viewStatus: 'healthy',
      attentionReasons: [],
    })
  })

  it('rejects a waiting-approval task and allows the active run to resume later', async () => {
    const { store } = await prepareTaskStore()

    await store.loadProjectTasks('proj-redesign')
    await store.getTaskDetail('proj-redesign', 'task-redesign-approval-gate')

    const rejected = await store.createIntervention('proj-redesign', 'task-redesign-approval-gate', {
      type: 'reject',
      taskRunId: 'task-run-redesign-approval-gate',
      payload: {},
    })

    expect(rejected).toMatchObject({
      id: 'task-intervention-1',
      taskId: 'task-redesign-approval-gate',
      type: 'reject',
      status: 'applied',
    })
    expect(store.getCachedDetail('task-redesign-approval-gate')).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'attention',
      viewStatus: 'attention',
      attentionReasons: ['waiting_input'],
      latestResultSummary: 'Approval rejected. Waiting for updated guidance.',
      activeRun: expect.objectContaining({
        id: 'task-run-redesign-approval-gate',
        status: 'waiting_input',
        viewStatus: 'attention',
        attentionReasons: ['waiting_input'],
      }),
    })
    expect(store.projectTasksFor('proj-redesign').find(task => task.id === 'task-redesign-approval-gate')).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'attention',
      attentionReasons: ['waiting_input'],
    })

    const resumed = await store.createIntervention('proj-redesign', 'task-redesign-approval-gate', {
      type: 'resume',
      taskRunId: 'task-run-redesign-approval-gate',
      payload: {},
    })

    expect(resumed).toMatchObject({
      id: 'task-intervention-2',
      taskId: 'task-redesign-approval-gate',
      type: 'resume',
      status: 'applied',
    })
    expect(store.getCachedDetail('task-redesign-approval-gate')).toMatchObject({
      id: 'task-redesign-approval-gate',
      status: 'running',
      viewStatus: 'healthy',
      attentionReasons: [],
      latestResultSummary: 'Updated guidance received. Continuing the active run.',
      activeRun: expect.objectContaining({
        id: 'task-run-redesign-approval-gate',
        status: 'running',
        viewStatus: 'healthy',
        attentionReasons: [],
      }),
      interventionHistory: [
        expect.objectContaining({
          id: 'task-intervention-2',
          type: 'resume',
          status: 'applied',
        }),
        expect.objectContaining({
          id: 'task-intervention-1',
          type: 'reject',
          status: 'applied',
        }),
      ],
    })
  })
})
