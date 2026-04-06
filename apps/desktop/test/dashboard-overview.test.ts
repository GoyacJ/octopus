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
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for overview/dashboard state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Dashboard and overview surfaces', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
    window.localStorage.clear()
  })

  it('renders the workspace overview from the real workspace projection API fixture', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => mounted.container.textContent?.includes('Local Workspace') ?? false)

    expect(mounted.container.textContent).toContain('Local Workspace')
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Conversation Redesign')
    expect(mounted.container.textContent).toContain('Workspace synced')

    mounted.destroy()
  })

  it('renders the project dashboard from the real project dashboard projection API fixture', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => mounted.container.textContent?.includes('Desktop Redesign') ?? false)

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Conversation Redesign')
    expect(mounted.container.textContent).toContain('Workspace synced')

    mounted.destroy()
  })
})
