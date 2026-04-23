// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

function mountApp() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitForText(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!(container.textContent?.includes(value) ?? false)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for project tasks state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Project tasks view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/projects/proj-redesign/tasks')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders project tasks and switches detail panels from the selected list row', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Release Brief Refresh')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Regression Sweep')
    expect(mounted.container.querySelector('[data-testid="project-task-detail"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Drafting the updated brief and collecting deliverable links.')

    mounted.container
      .querySelector<HTMLElement>('[data-testid="project-task-row-task-redesign-regression-sweep"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.query.taskId === 'task-redesign-regression-sweep')
    await waitForText(mounted.container, 'FREQ=DAILY;BYHOUR=9;BYMINUTE=0')

    expect(mounted.container.textContent).toContain('Hit a runtime error while validating the workspace overview flow.')

    mounted.destroy()
  })

  it('creates, edits, and launches a task from the project task workbench', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Release Brief Refresh')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-create-button"]')
      ?.click()

    await waitFor(() => document.body.querySelector('[data-testid="project-task-create-dialog"]') !== null)

    const createDialog = document.body.querySelector('[data-testid="project-task-create-dialog"]') as HTMLElement
    const createTitleInput = createDialog.querySelector<HTMLInputElement>('[data-testid="project-task-create-title"]')
    const createGoalInput = createDialog.querySelector<HTMLTextAreaElement>('[data-testid="project-task-create-goal"]')
    const createBriefInput = createDialog.querySelector<HTMLTextAreaElement>('[data-testid="project-task-create-brief"]')
    const createActorInput = createDialog.querySelector<HTMLInputElement>('[data-testid="project-task-create-actor"]')

    createTitleInput!.value = 'Prepare launch checklist'
    createTitleInput!.dispatchEvent(new Event('input', { bubbles: true }))
    createGoalInput!.value = 'Create a launch-ready checklist.'
    createGoalInput!.dispatchEvent(new Event('input', { bubbles: true }))
    createBriefInput!.value = 'Focus on dependencies and sequencing.'
    createBriefInput!.dispatchEvent(new Event('input', { bubbles: true }))
    createActorInput!.value = 'agent:workspace-core'
    createActorInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    createDialog
      .querySelector<HTMLButtonElement>('[data-testid="project-task-create-submit"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.query.taskId === 'task-1')
    await waitForText(mounted.container, 'Prepare launch checklist')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-edit"]')
      ?.click()

    await waitFor(() => document.body.querySelector('[data-testid="project-task-edit-dialog"]') !== null)

    const editDialog = document.body.querySelector('[data-testid="project-task-edit-dialog"]') as HTMLElement
    const editTitleInput = editDialog.querySelector<HTMLInputElement>('[data-testid="project-task-edit-title"]')
    const editBriefInput = editDialog.querySelector<HTMLTextAreaElement>('[data-testid="project-task-edit-brief"]')

    editTitleInput!.value = 'Prepare launch checklist v2'
    editTitleInput!.dispatchEvent(new Event('input', { bubbles: true }))
    editBriefInput!.value = 'Updated brief from the task detail dialog.'
    editBriefInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    editDialog
      .querySelector<HTMLButtonElement>('[data-testid="project-task-edit-submit"]')
      ?.click()

    await waitForText(mounted.container, 'Prepare launch checklist v2')
    await waitForText(mounted.container, 'Updated brief from the task detail dialog.')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-launch"]')
      ?.click()

    await waitForText(mounted.container, 'task-run-1')
    const conversationLink = mounted.container
      .querySelector<HTMLAnchorElement>('[data-testid="project-task-run-conversation-task-run-1"]')
    expect(conversationLink?.getAttribute('href')).toContain('/workspaces/ws-local/projects/proj-redesign/conversations/task-conversation-1')

    mounted.destroy()
  })

  it('clarifies durable task ownership when opened from a conversation', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/tasks?from=conversation&conversationId=conv-redesign')
    await router.isReady()

    const mounted = mountApp()

    await waitForText(mounted.container, 'Release Brief Refresh')
    await waitFor(() => mounted.container.querySelector('[data-testid="project-tasks-conversation-callout"]') !== null)

    expect(mounted.container.querySelector('[data-testid="project-tasks-back-to-conversation"]')).not.toBeNull()

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-tasks-back-to-conversation"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.name === 'project-conversation')
    expect(router.currentRoute.value.params.conversationId).toBe('conv-redesign')

    mounted.destroy()
  })

  it('reruns the selected task from the detail panel', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    await waitForText(mounted.container, 'Release Brief Refresh')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-rerun"]')
      ?.click()

    await waitForText(mounted.container, 'task-run-1')

    expect(taskStore.getCachedRunHistory('task-redesign-release-brief')).toEqual([
      expect.objectContaining({
        id: 'task-run-1',
        triggerType: 'rerun',
      }),
    ])
    const conversationLink = mounted.container
      .querySelector<HTMLAnchorElement>('[data-testid="project-task-run-conversation-task-run-1"]')
    expect(conversationLink?.getAttribute('href')).toContain('/workspaces/ws-local/projects/proj-redesign/conversations/task-conversation-1')

    mounted.destroy()
  })

  it('renders the selected task context bundle details', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      await taskStore.updateTask('proj-redesign', 'task-redesign-release-brief', {
        contextBundle: {
          refs: [
            {
              kind: 'resource',
              refId: 'resource-release-outline',
              title: 'Release Outline',
              subtitle: 'Latest launch draft',
              versionRef: 'v4',
              pinMode: 'snapshot',
            },
            {
              kind: 'knowledge',
              refId: 'knowledge-release-policy',
              title: 'Release Policy',
              subtitle: 'Publishing guardrails',
              versionRef: null,
              pinMode: 'follow_latest',
            },
          ],
          pinnedInstructions: 'Keep the executive summary concise and link the latest deliverables.',
          resolutionMode: 'explicit_plus_project_defaults',
          lastResolvedAt: 1_713_139_200_000,
        },
      })

      await waitForText(mounted.container, 'Release Outline')

      expect(mounted.container.querySelector('[data-testid="project-task-context-panel"]')).not.toBeNull()
      expect(mounted.container.textContent).toContain('Release Policy')
      expect(mounted.container.textContent).toContain('Latest launch draft')
      expect(mounted.container.textContent).toContain('Keep the executive summary concise and link the latest deliverables.')
    } finally {
      mounted.destroy()
    }
  })

  it('applies the execution actor override to launch and rerun actions', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      mounted.container
        .querySelector<HTMLElement>('[data-testid="project-task-row-task-redesign-regression-sweep"]')
        ?.click()

      await waitFor(() => router.currentRoute.value.query.taskId === 'task-redesign-regression-sweep')
      await waitForText(mounted.container, 'FREQ=DAILY;BYHOUR=9;BYMINUTE=0')

      const actorOverrideInput = mounted.container
        .querySelector<HTMLInputElement>('[data-testid="project-task-detail-execution-actor"]')

      actorOverrideInput!.value = 'agent:release-operator'
      actorOverrideInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-launch"]')
        ?.click()

      await waitForText(mounted.container, 'task-run-1')
      expect(taskStore.getCachedRunHistory('task-redesign-regression-sweep')[0]).toMatchObject({
        id: 'task-run-1',
        actorRef: 'agent:release-operator',
        triggerType: 'manual',
      })

      actorOverrideInput!.value = 'agent:qa-lead'
      actorOverrideInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-rerun"]')
        ?.click()

      await waitForText(mounted.container, 'task-run-2')
      expect(taskStore.getCachedRunHistory('task-redesign-regression-sweep')).toEqual([
        expect.objectContaining({
          id: 'task-run-2',
          actorRef: 'agent:qa-lead',
          triggerType: 'rerun',
        }),
        expect.objectContaining({
          id: 'task-run-1',
          actorRef: 'agent:release-operator',
          triggerType: 'manual',
        }),
      ])
    } finally {
      mounted.destroy()
    }
  })

  it('supports brief and takeover interventions from the detail panel', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    await waitForText(mounted.container, 'Release Brief Refresh')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-edit-brief"]')
      ?.click()

    await waitFor(() => document.body.querySelector('[data-testid="project-task-brief-dialog"]') !== null)

    const briefDialog = document.body.querySelector('[data-testid="project-task-brief-dialog"]') as HTMLElement
    const briefInput = briefDialog.querySelector<HTMLTextAreaElement>('[data-testid="project-task-brief-input"]')

    briefInput!.value = 'Focus on the final release notes and linked deliverables.'
    briefInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    briefDialog
      .querySelector<HTMLButtonElement>('[data-testid="project-task-brief-submit"]')
      ?.click()

    await waitForText(mounted.container, 'Focus on the final release notes and linked deliverables.')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-takeover"]')
      ?.click()

    await waitFor(() =>
      taskStore.getCachedDetail('task-redesign-release-brief')?.attentionReasons.includes('takeover_recommended') ?? false)
    expect(mounted.container.textContent).toContain('Task intervention recorded: takeover.')
    expect(mounted.container.querySelector('[data-testid="project-task-intervention-task-intervention-2"]')).not.toBeNull()

    mounted.destroy()
  })

  it('supports comment interventions from the detail panel and renders the note in history', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-comment"]')
        ?.click()

      await waitFor(() => document.body.querySelector('[data-testid="project-task-comment-dialog"]') !== null)

      const commentDialog = document.body.querySelector('[data-testid="project-task-comment-dialog"]') as HTMLElement
      const commentInput = commentDialog.querySelector<HTMLTextAreaElement>('[data-testid="project-task-comment-input"]')

      commentInput!.value = 'Please keep the final release brief under two sections.'
      commentInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      commentDialog
        .querySelector<HTMLButtonElement>('[data-testid="project-task-comment-submit"]')
        ?.click()

      await waitForText(mounted.container, 'Please keep the final release brief under two sections.')

      expect(taskStore.getCachedDetail('task-redesign-release-brief')).toMatchObject({
        interventionHistory: [
          expect.objectContaining({
            type: 'comment',
            payload: {
              note: 'Please keep the final release brief under two sections.',
            },
          }),
        ],
      })
      expect(mounted.container.querySelector('[data-testid="project-task-intervention-task-intervention-1"]')).not.toBeNull()
    } finally {
      mounted.destroy()
    }
  })

  it('supports changing the actor for the active task run from the detail panel', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-rerun"]')
        ?.click()

      await waitForText(mounted.container, 'task-run-1')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-change-actor"]')
        ?.click()

      await waitFor(() => document.body.querySelector('[data-testid="project-task-change-actor-dialog"]') !== null)

      const dialog = document.body.querySelector('[data-testid="project-task-change-actor-dialog"]') as HTMLElement
      const actorInput = dialog.querySelector<HTMLInputElement>('[data-testid="project-task-change-actor-input"]')

      actorInput!.value = 'agent:release-operator'
      actorInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      dialog
        .querySelector<HTMLButtonElement>('[data-testid="project-task-change-actor-submit"]')
        ?.click()

      await waitForText(mounted.container, 'agent:release-operator')

      expect(taskStore.getCachedDetail('task-redesign-release-brief')).toMatchObject({
        defaultActorRef: 'agent:release-operator',
        activeRun: expect.objectContaining({
          id: 'task-run-1',
          actorRef: 'agent:release-operator',
        }),
      })
      expect(mounted.container.querySelector('[data-testid="project-task-intervention-task-intervention-1"]')).not.toBeNull()
    } finally {
      mounted.destroy()
    }
  })

  it('supports approving a waiting-approval task from the detail panel', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()
    const createInterventionSpy = vi.spyOn(taskStore, 'createIntervention')

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      mounted.container
        .querySelector<HTMLElement>('[data-testid="project-task-row-task-redesign-approval-gate"]')
        ?.click()

      await waitFor(() => router.currentRoute.value.query.taskId === 'task-redesign-approval-gate')
      await waitForText(mounted.container, 'Waiting for approval before publishing the release brief update.')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-approve"]')
        ?.click()

      await waitFor(() =>
        taskStore.getCachedDetail('task-redesign-approval-gate')?.activeRun?.status === 'running')
      await waitForText(mounted.container, 'Approval received. Continuing the active run.')

      expect(createInterventionSpy).toHaveBeenCalledWith(
        'proj-redesign',
        'task-redesign-approval-gate',
        expect.objectContaining({
          type: 'approve',
          taskRunId: 'task-run-redesign-approval-gate',
          approvalId: 'approval-task-run-redesign-approval-gate',
        }),
      )
      expect(taskStore.getCachedDetail('task-redesign-approval-gate')).toMatchObject({
        status: 'running',
        attentionReasons: [],
        activeRun: expect.objectContaining({
          id: 'task-run-redesign-approval-gate',
          status: 'running',
        }),
        interventionHistory: [
          expect.objectContaining({
            type: 'approve',
            status: 'applied',
          }),
        ],
      })
      expect(mounted.container.querySelector('[data-testid="project-task-intervention-task-intervention-1"]')).not.toBeNull()
    } finally {
      mounted.destroy()
    }
  })

  it('supports rejecting and resuming a paused task from the detail panel', async () => {
    const mounted = mountApp()
    const { useProjectTaskStore } = await import('@/stores/project_task')
    const taskStore = useProjectTaskStore()

    try {
      await waitForText(mounted.container, 'Release Brief Refresh')

      mounted.container
        .querySelector<HTMLElement>('[data-testid="project-task-row-task-redesign-approval-gate"]')
        ?.click()

      await waitFor(() => router.currentRoute.value.query.taskId === 'task-redesign-approval-gate')
      await waitForText(mounted.container, 'Waiting for approval before publishing the release brief update.')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-reject"]')
        ?.click()

      await waitFor(() =>
        taskStore.getCachedDetail('task-redesign-approval-gate')?.activeRun?.status === 'waiting_input')
      await waitForText(mounted.container, 'Approval rejected. Waiting for updated guidance.')

      mounted.container
        .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-resume"]')
        ?.click()

      await waitFor(() =>
        taskStore.getCachedDetail('task-redesign-approval-gate')?.activeRun?.status === 'running')
      await waitForText(mounted.container, 'Updated guidance received. Continuing the active run.')

      expect(taskStore.getCachedDetail('task-redesign-approval-gate')).toMatchObject({
        status: 'running',
        attentionReasons: [],
        activeRun: expect.objectContaining({
          id: 'task-run-redesign-approval-gate',
          status: 'running',
        }),
        interventionHistory: [
          expect.objectContaining({
            type: 'resume',
            status: 'applied',
          }),
          expect.objectContaining({
            type: 'reject',
            status: 'applied',
          }),
        ],
      })
      expect(mounted.container.querySelector('[data-testid="project-task-intervention-task-intervention-2"]')).not.toBeNull()
    } finally {
      mounted.destroy()
    }
  })

  it('navigates into the task conversation when taking over an active run', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Release Brief Refresh')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-rerun"]')
      ?.click()

    await waitForText(mounted.container, 'task-run-1')

    mounted.container
      .querySelector<HTMLButtonElement>('[data-testid="project-task-detail-takeover"]')
      ?.click()

    await waitFor(() =>
      router.currentRoute.value.fullPath.includes('/workspaces/ws-local/projects/proj-redesign/conversations/task-conversation-1'))

    mounted.destroy()
  })
})
