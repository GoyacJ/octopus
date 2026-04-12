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

describe('Workspace resources view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/resources')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('groups workspace, personal, and project resources in separate sections', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Shared Specs')

    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.workspaceSections.workspace')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.workspaceSections.personal')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('resources.workspaceSections.projectGroups')))
    expect(mounted.container.textContent).toContain('Shared Specs')
    expect(mounted.container.textContent).toContain('Personal Scratchpad')
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Desktop Redesign Shared Assets')

    mounted.destroy()
  })
})
