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

function mountApp(pinia = createPinia()) {
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

async function mountRoutedApp(path: string) {
  const pinia = createPinia()
  setActivePinia(pinia)
  await router.push(path)
  await router.isReady()
  return mountApp(pinia)
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

async function waitForSelector(container: HTMLElement, selector: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!container.querySelector(selector)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for selector: ${selector}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('User center RBAC prototype', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the owner-accessible user center tabs and user records from the real RBAC API', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await waitForSelector(mounted.container, '[data-testid="user-center-tabs"]')
    await waitForText(mounted.container, 'Lin Zhou')

    expect(mounted.container.querySelector('[data-testid="user-center-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-profile"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-roles"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-permissions"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-menus"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Lobster Owner')
    expect(mounted.container.textContent).toContain('Lin Zhou')

    mounted.destroy()
  })

  it('renders permissions, roles, menus, and profile pages through the new user center surfaces', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await router.push('/workspaces/ws-local/user-center/permissions')
    await waitForText(mounted.container, 'Manage users')
    expect(mounted.container.textContent).toContain('Manage users')
    expect(mounted.container.textContent).toContain('Manage roles')

    await router.push('/workspaces/ws-local/user-center/roles')
    await waitForText(mounted.container, 'Owner')
    expect(mounted.container.textContent).toContain('Owner')
    expect(mounted.container.textContent).toContain('Operator')

    await router.push('/workspaces/ws-local/user-center/menus')
    await waitForText(mounted.container, 'Profile')
    expect(mounted.container.textContent).toContain('Profile')
    expect(mounted.container.textContent).toContain('Users')

    await router.push('/workspaces/ws-local/user-center/profile')
    await waitForText(mounted.container, 'Lobster Owner')
    expect(mounted.container.textContent).toContain('Lobster Owner')
    expect(mounted.container.textContent).toContain('owner')

    mounted.destroy()
  })
})
