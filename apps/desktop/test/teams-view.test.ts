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

describe('Teams view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/teams')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders workspace teams and saves edited team details', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Studio Direction Team')

    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('teams.metrics.total')))

    const nameInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(nameInput).not.toBeNull()
    nameInput!.value = 'Unified Team Name'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('button'))
      .find(button => button.textContent?.includes(String(i18n.global.t('common.save'))))
      ?.click()
    await waitForText(mounted.container, 'Unified Team Name')

    mounted.destroy()
  })
})
