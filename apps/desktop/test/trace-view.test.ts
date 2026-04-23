// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useRuntimeStore } from '@/stores/runtime'
import { installWorkspaceApiFixture } from './support/workspace-fixture'
import { createSessionDetail, createTraceItem } from './support/workspace-fixture-runtime'

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
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function flushUi() {
  await nextTick()
  await new Promise((resolve) => window.setTimeout(resolve, 0))
  await nextTick()
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for trace state')
    }

    await new Promise((resolve) => window.setTimeout(resolve, 20))
  }
}

describe('TraceView runtime integration', () => {
  beforeEach(async () => {
    window.localStorage.clear()
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/projects/proj-redesign/trace')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders the active runtime run and trace timeline instead of the old workbench mock run', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-redesign',
      projectId: 'proj-redesign',
      title: 'Trace Runtime Session',
    })
    await runtime.submitTurn({
      content: 'Summarize the runtime trace state.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.activeRun?.status === 'completed' && runtime.activeTrace.length > 0)
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="trace-runtime-status"]')?.textContent).toMatch(/completed|已完成/)
    expect(mounted.container.textContent).toContain('Architect Agent · Agent')
    expect(mounted.container.querySelectorAll('[data-testid="trace-runtime-item"]').length).toBeGreaterThan(0)

    runtime.dispose()
    mounted.destroy()
  })

  it('renders team resolved labels in the trace view', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-team-trace',
      projectId: 'proj-redesign',
      title: 'Team Trace Runtime Session',
      selectedActorRef: 'team:team-studio',
    })
    await runtime.submitTurn({
      content: 'Summarize the runtime trace state.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.activeRun?.status === 'completed' && runtime.activeTrace.length > 0)
    await flushUi()

    expect(mounted.container.textContent).toContain('Studio Direction Team · Team')

    runtime.dispose()
    mounted.destroy()
  })

  it('renders each trace item with its own tone and lightweight meta labels', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-mixed-trace',
      projectId: 'proj-redesign',
      title: 'Mixed Trace Runtime Session',
    })

    const detail = createSessionDetail(
      'conv-mixed-trace',
      'proj-redesign',
      'Mixed Trace Runtime Session',
    )
    detail.summary.status = 'completed'
    detail.run.status = 'completed'
    detail.trace = [
      {
        ...createTraceItem({ detail, events: [], nextSequence: 1 }, 'Started run.', 'info'),
        id: 'trace-mixed-1',
        kind: 'step',
        title: 'Run started',
      },
      {
        ...createTraceItem({ detail, events: [], nextSequence: 2 }, 'Waiting for workspace command approval.', 'warning'),
        id: 'trace-mixed-2',
        kind: 'tool',
        title: 'Workspace command',
        relatedToolName: 'workspace-api',
      },
      {
        ...createTraceItem({ detail, events: [], nextSequence: 3 }, 'Published the deliverable.', 'success'),
        id: 'trace-mixed-3',
        kind: 'artifact',
        title: 'Deliverable published',
      },
    ]

    runtime.setActiveSession(detail)
    await flushUi()
    await waitFor(() => mounted.container.querySelectorAll('[data-testid="trace-runtime-item"]').length === 3)

    const items = Array.from(
      mounted.container.querySelectorAll<HTMLElement>('[data-testid="trace-runtime-item"] [data-ui-tone]'),
    )

    expect(items.map(item => item.getAttribute('data-ui-tone'))).toEqual(['info', 'warning', 'success'])
    expect(mounted.container.textContent).toContain('Step')
    expect(mounted.container.textContent).toContain('Tool')
    expect(mounted.container.textContent).toContain('Artifact')
    expect(mounted.container.textContent).toContain('workspace-api')

    runtime.dispose()
    mounted.destroy()
  })

  it('copies trace detail and the current trace link from the context menu', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-trace-copy',
      projectId: 'proj-redesign',
      title: 'Trace Copy Runtime Session',
    })

    const detail = createSessionDetail(
      'conv-trace-copy',
      'proj-redesign',
      'Trace Copy Runtime Session',
    )
    detail.summary.status = 'completed'
    detail.run.status = 'completed'
    detail.trace = [
      {
        ...createTraceItem({ detail, events: [], nextSequence: 1 }, 'Collected the final trace detail.', 'info'),
        id: 'trace-copy-1',
        kind: 'tool',
        title: 'Workspace sync',
        relatedToolName: 'workspace-api',
      },
    ]

    runtime.setActiveSession(detail)
    await flushUi()
    await waitFor(() => mounted.container.querySelector('[data-testid="trace-runtime-item"][data-trace-id="trace-copy-1"]') !== null)

    const traceItem = mounted.container.querySelector<HTMLElement>('[data-testid="trace-runtime-item"][data-trace-id="trace-copy-1"]')
    expect(traceItem).not.toBeNull()

    traceItem?.dispatchEvent(new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: 72,
      clientY: 108,
    }))
    await waitFor(() => document.body.querySelector('[data-testid="ui-context-item-copy-detail"]') !== null)
    document.body.querySelector<HTMLElement>('[data-testid="ui-context-item-copy-detail"]')?.click()

    await waitFor(() => writeText.mock.calls.length === 1)
    expect(writeText).toHaveBeenNthCalledWith(1, 'Collected the final trace detail.')

    traceItem?.dispatchEvent(new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: 84,
      clientY: 116,
    }))
    await waitFor(() => document.body.querySelector('[data-testid="ui-context-item-copy-link"]') !== null)
    document.body.querySelector<HTMLElement>('[data-testid="ui-context-item-copy-link"]')?.click()

    await waitFor(() => writeText.mock.calls.length === 2)
    expect(String(writeText.mock.calls[1]?.[0])).toContain(router.currentRoute.value.fullPath)

    runtime.dispose()
    mounted.destroy()
  })

  it('renders runtime approval actions and updates the trace view after approval', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-approval',
      projectId: 'proj-redesign',
      title: 'Approval Runtime Session',
    })
    await runtime.submitTurn({
      content: 'Run bash pwd in the workspace terminal.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.pendingApproval !== null)
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="trace-runtime-approval"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Architect Agent · Agent')
    expect(mounted.container.querySelector('[data-testid="trace-runtime-approve"]')?.textContent).toMatch(/批准|Approve/)
    expect(mounted.container.querySelector('[data-testid="trace-runtime-reject"]')?.textContent).toMatch(/驳回|Reject/)

    const approveButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="trace-runtime-approve"]')
    approveButton?.click()

    await waitFor(() => runtime.pendingApproval === null && runtime.activeRun?.status === 'completed')
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="trace-runtime-approval"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="trace-runtime-status"]')?.textContent).toMatch(/completed|已完成/)
    expect(mounted.container.querySelectorAll('[data-testid="trace-runtime-item"]').length).toBeGreaterThan(0)

    runtime.dispose()
    mounted.destroy()
  })

  it('renders auth mediation from typed runtime state', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-auth-trace',
      projectId: 'proj-redesign',
      title: 'Auth Trace Runtime Session',
    })

    const detail = createSessionDetail(
      'conv-auth-trace',
      'proj-redesign',
      'Auth Trace Runtime Session',
    )
    detail.summary.status = 'waiting_input'
    detail.pendingMediation = {
      mediationKind: 'auth',
      state: 'pending',
      summary: 'Provider sign-in required',
      detail: 'Resolve the provider auth challenge before execution can continue.',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      authChallengeId: 'auth-workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    }
    detail.summary.pendingMediation = detail.pendingMediation
    detail.run.status = 'waiting_input'
    detail.run.currentStep = 'awaiting_auth'
    detail.run.nextAction = 'auth'
    detail.run.pendingMediation = detail.pendingMediation
    detail.run.authTarget = {
      id: 'auth-workspace-api',
      sessionId: detail.summary.id,
      conversationId: detail.summary.conversationId,
      runId: detail.run.id,
      summary: 'Provider sign-in required',
      detail: 'Resolve the provider auth challenge before execution can continue.',
      status: 'pending',
      createdAt: detail.run.updatedAt,
      approvalLayer: 'provider-auth',
      escalationReason: 'provider authentication required',
      targetKind: 'provider-auth',
      targetRef: 'provider:workspace-api',
      providerKey: 'workspace-api',
      requiresAuth: true,
      requiresApproval: false,
    }

    runtime.setActiveSession(detail)
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="trace-runtime-approval"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Provider sign-in required')
    expect(mounted.container.textContent).toContain('workspace-api')

    runtime.dispose()
    mounted.destroy()
  })

  it('renders run state, recovery, and timeline as inspector panels instead of stacked subtle cards', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await runtime.ensureSession({
      conversationId: 'conv-trace-layout',
      projectId: 'proj-redesign',
      title: 'Trace Layout Runtime Session',
    })
    await runtime.submitTurn({
      content: 'Summarize the runtime trace state.',
      permissionMode: 'auto',
    })

    await waitFor(() => runtime.activeRun?.status === 'completed' && runtime.activeTrace.length > 0)
    await flushUi()

    const runState = mounted.container.querySelector<HTMLElement>('[data-testid="trace-run-state"]')
    const recovery = mounted.container.querySelector<HTMLElement>('[data-testid="trace-recovery"]')
    const timeline = mounted.container.querySelector<HTMLElement>('[data-testid="trace-timeline"]')

    expect(runState).not.toBeNull()
    expect(recovery).not.toBeNull()
    expect(timeline).not.toBeNull()
    expect(runState?.querySelector('[data-testid="ui-inspector-panel-header"]')).not.toBeNull()
    expect(runState?.querySelector('[data-testid="ui-inspector-panel-body"]')).not.toBeNull()
    expect(recovery?.querySelector('[data-testid="ui-inspector-panel-header"]')).not.toBeNull()
    expect(recovery?.querySelector('[data-testid="ui-inspector-panel-body"]')).not.toBeNull()
    expect(timeline?.querySelector('[data-testid="ui-inspector-panel-header"]')).not.toBeNull()
    expect(timeline?.querySelector('[data-testid="ui-inspector-panel-body"]')).not.toBeNull()

    runtime.dispose()
    mounted.destroy()
  })
})
