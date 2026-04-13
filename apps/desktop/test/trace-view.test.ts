// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useRuntimeStore } from '@/stores/runtime'
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
})
