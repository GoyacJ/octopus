// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
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

async function flushUi() {
  await nextTick()
  await new Promise(resolve => window.setTimeout(resolve, 0))
  await nextTick()
}

describe('App auth gate', () => {
  beforeEach(async () => {
    window.localStorage.clear()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
  })

  it('shows the registration gate for an uninitialized local workspace', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: false,
      localSetupRequired: true,
      preloadWorkspaceSessions: false,
    })

    const mounted = mountApp()
    await flushUi()

    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.register.title')))
    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.fields.username')))

    mounted.destroy()
  })

  it('shows the login gate after a persisted session expires', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: true,
      localSessionValid: false,
    })

    const mounted = mountApp()
    await flushUi()

    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.login.title')))
    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.fields.password')))

    mounted.destroy()
  })
})
