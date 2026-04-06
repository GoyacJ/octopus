// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { prepareRouterStartup } from '@/startup/router'
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
  return {
    async start() {
      await prepareRouterStartup(router)
      app.use(router)
      app.mount(container)
    },
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function flushUi(timeoutMs = 2000) {
  const startedAt = Date.now()
  while (Date.now() - startedAt < timeoutMs) {
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
    if (router.currentRoute.value.name) {
      return
    }
  }

  throw new Error('Timed out waiting for initial router install navigation')
}

describe('desktop startup router install', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    window.location.hash = ''
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('mounts from the root entry without pre-pushing a route', async () => {
    const mounted = mountApp()
    await mounted.start()

    await flushUi()

    expect(router.currentRoute.value.name).toBeTruthy()
    expect(String(router.currentRoute.value.params.workspaceId ?? '')).not.toBe('')
    expect(mounted.container.textContent?.trim().length ?? 0).toBeGreaterThan(0)

    mounted.destroy()
  })
})
