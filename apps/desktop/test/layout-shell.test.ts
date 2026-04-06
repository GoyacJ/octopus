// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useShellStore } from '@/stores/shell'
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

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for shell state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Workbench shell layout', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the topbar and sidebar from the real shell and workspace fixtures', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() =>
      (mounted.container.textContent?.includes('Local Workspace') ?? false)
      && (mounted.container.textContent?.includes('Desktop Redesign') ?? false),
    )

    expect(mounted.container.querySelector('[data-testid="workbench-topbar"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Local Workspace')
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.querySelector('[data-testid="global-search-trigger"]')).not.toBeNull()

    mounted.destroy()
  })

  it('does not blank on the root entry route before any explicit navigation', async () => {
    await router.push('/')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() =>
      (mounted.container.textContent?.includes('Local Workspace') ?? false)
      || (mounted.container.textContent?.includes('Octopus') ?? false),
    )

    expect(mounted.container.textContent?.trim().length ?? 0).toBeGreaterThan(0)

    mounted.destroy()
  })

  it('updates theme and locale preferences through the topbar controls', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const shell = useShellStore()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-theme-toggle"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')?.click()
    await nextTick()
    Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.themeModes.light'))))?.click()
    await waitFor(() => shell.preferences.theme === 'light')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-toggle"]')?.click()
    await nextTick()
    Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.localeModes.en-US'))))?.click()
    await waitFor(() => shell.preferences.locale === 'en-US')

    expect(shell.preferences.theme).toBe('light')
    expect(shell.preferences.locale).toBe('en-US')

    mounted.destroy()
  })

  it('switches workspace scope from the sidebar workspace list', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.textContent?.includes('Enterprise Workspace') ?? false)

    const enterpriseButton = Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes('Enterprise Workspace'))
    enterpriseButton?.click()

    await waitFor(() => String(router.currentRoute.value.params.workspaceId) === 'ws-enterprise')
    expect(String(router.currentRoute.value.params.workspaceId)).toBe('ws-enterprise')

    mounted.destroy()
  })
})
