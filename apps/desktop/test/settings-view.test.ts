// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'

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

describe('Workspace settings view', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/settings')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('uses shared tabs and record/list rows for general and version sections', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="settings-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-layout-row-leftSidebarCollapsed"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-version"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="settings-version-row-shell"]')).not.toBeNull()

    mounted.destroy()
  })
})
