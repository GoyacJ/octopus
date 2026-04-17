// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, defineComponent, h, nextTick, ref } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import {
  installRuntimeAppErrorHandling,
  reportRuntimeAppError,
  resetRuntimeAppErrorState,
} from '@/runtime/app-error-boundary'
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
  const stopRuntimeErrorHandling = installRuntimeAppErrorHandling(app, router)

  return {
    container,
    destroy() {
      stopRuntimeErrorHandling()
      app.unmount()
      container.remove()
      resetRuntimeAppErrorState()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for runtime error boundary state')
    }

    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function dispatchUnhandledRejection(reason: unknown) {
  const event = new Event('unhandledrejection') as PromiseRejectionEvent
  Object.defineProperty(event, 'reason', {
    configurable: true,
    value: reason,
  })
  window.dispatchEvent(event)
}

describe('App runtime error boundary', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    document.body.innerHTML = ''
    resetRuntimeAppErrorState()
    installWorkspaceApiFixture({
      preloadConversationMessages: true,
    })
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()
  })

  it('shows a page-level fallback for post-mount component errors and retries the current page', async () => {
    const removeRoute = router.addRoute({
      path: '/workspaces/:workspaceId/projects/:projectId/runtime-component-crash',
      name: 'runtime-component-crash',
      component: defineComponent({
        setup() {
          const shouldCrash = ref(false)

          return () => {
            if (shouldCrash.value) {
              throw new Error('component crashed after mount')
            }

            return h('button', {
              'data-testid': 'runtime-component-crash-trigger',
              onClick: () => {
                shouldCrash.value = true
              },
            }, 'Crash')
          }
        },
      }),
    })

    const mounted = mountApp()

    await router.push({
      name: 'runtime-component-crash',
      params: {
        workspaceId: 'ws-local',
        projectId: 'proj-redesign',
      },
    })
    await waitFor(() => mounted.container.querySelector('[data-testid="runtime-component-crash-trigger"]') !== null)

    const trigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="runtime-component-crash-trigger"]')
    trigger?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workbench-topbar"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain(String(i18n.global.t('app.runtimeError.title')))
    expect(mounted.container.textContent).toContain('component crashed after mount')
    expect(mounted.container.textContent).not.toContain('Fatal startup error')

    const retryButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="app-runtime-error-retry"]')
    retryButton?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="runtime-component-crash-trigger"]') !== null)
    expect(mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]')).toBeNull()

    mounted.destroy()
    removeRoute()
  })

  it('shows a page-level fallback for router navigation errors and returns to the project conversations list', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-tabs"]') !== null)

    reportRuntimeAppError(new Error('router crashed after mount'), {
      source: 'router',
    })

    await waitFor(() => mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]') !== null)

    expect(mounted.container.textContent).toContain('router crashed after mount')
    expect(mounted.container.textContent).not.toContain('Fatal startup error')

    const projectButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="app-runtime-error-project"]')
    projectButton?.click()

    await waitFor(() => ['project-conversations', 'project-conversation'].includes(String(router.currentRoute.value.name)))
    expect(mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]')).toBeNull()

    mounted.destroy()
  })

  it('shows a page-level fallback for unhandled rejections and returns to the workspace overview', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-tabs"]') !== null)

    dispatchUnhandledRejection(new Error('post-mount rejection'))

    await waitFor(() => mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workbench-topbar"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('post-mount rejection')
    expect(mounted.container.textContent).not.toContain('Fatal startup error')

    const overviewButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="app-runtime-error-overview"]')
    overviewButton?.click()

    await waitFor(() => String(router.currentRoute.value.name) === 'workspace-overview')
    expect(mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]')).toBeNull()

    mounted.destroy()
  })

  it('separates recovery actions from diagnostics inside integrated error sections', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-tabs"]') !== null)

    dispatchUnhandledRejection(new Error('segmented recovery state'))

    await waitFor(() => mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]') !== null)

    const recoverySection = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-recovery"]')
    const detailsSection = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-details-section"]')
    const detailsPanel = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-details"]')
    const copyButton = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-copy"]')

    expect(recoverySection).not.toBeNull()
    expect(recoverySection?.className).toContain('border-border')
    expect(recoverySection?.className).toContain('bg-subtle')

    expect(detailsSection).not.toBeNull()
    expect(detailsSection?.className).toContain('border-border')
    expect(detailsSection?.className).toContain('bg-subtle')

    expect(copyButton).not.toBeNull()
    expect(copyButton?.className).toContain('border-border-subtle')
    expect(copyButton?.className).toContain('bg-surface')
    expect(recoverySection?.contains(copyButton ?? null)).toBe(false)

    expect(detailsPanel).not.toBeNull()
    expect(detailsPanel?.className).toContain('bg-surface')
    expect(detailsPanel?.className).not.toContain('bg-surface-muted')

    mounted.destroy()
  })

  it('renders the runtime error boundary with a calm intro band instead of a loose content block', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-tabs"]') !== null)

    dispatchUnhandledRejection(new Error('runtime intro band'))

    await waitFor(() => mounted.container.querySelector('[data-testid="app-runtime-error-boundary"]') !== null)

    const boundary = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-boundary"]')
    const intro = mounted.container.querySelector<HTMLElement>('[data-testid="app-runtime-error-intro"]')

    expect(boundary).not.toBeNull()
    expect(boundary?.className).toContain('overflow-hidden')

    expect(intro).not.toBeNull()
    expect(intro?.className).toContain('border-b')
    expect(intro?.className).toContain('bg-subtle')

    mounted.destroy()
  })
})
