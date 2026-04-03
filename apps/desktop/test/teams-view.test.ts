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

describe('Teams view', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/teams')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders shared team rows and saves edited team details', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="teams-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid^="team-row-"]')).not.toBeNull()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="teams-form-name"]')
    expect(nameInput).not.toBeNull()
    nameInput!.value = 'Unified Team Name'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="teams-form-save"]')?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Unified Team Name')

    mounted.destroy()
  })
})
