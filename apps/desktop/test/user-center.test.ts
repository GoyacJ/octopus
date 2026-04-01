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

describe('User center RBAC prototype', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/user-center/users')
    await router.isReady()
    document.body.innerHTML = ''
    window.confirm = () => true
  })

  it('renders the left navigation and keeps restricted tabs hidden after switching the session user', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="user-center-nav-profile"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-roles"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-permissions"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-menus"]')).not.toBeNull()

    const switchButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="user-switch-current-user-user-operator"]')
    expect(switchButton).not.toBeNull()

    switchButton?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Lin Zhou')
    expect(mounted.container.querySelector('[data-testid="user-center-nav-profile"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-roles"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-permissions"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-center-nav-menus"]')).toBeNull()

    mounted.destroy()
  })
})
