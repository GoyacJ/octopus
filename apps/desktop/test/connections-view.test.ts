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

describe('Connections view', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/connections')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders product and host connections through shared record cards', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="connections-product-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid^="connection-record-"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="connections-host-list"]')).not.toBeNull()

    mounted.destroy()
  })
})
