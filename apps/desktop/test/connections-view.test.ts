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

describe('Connections view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/connections')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders product and host connections through shared record cards', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Local Workspace')

    expect(mounted.container.querySelector('[data-testid="connections-product-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid^="connection-record-"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="connections-host-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="host-backend-connection"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Enterprise Workspace')
    expect(mounted.container.textContent).toContain('http://127.0.0.1:43127')

    mounted.destroy()
  })
})
