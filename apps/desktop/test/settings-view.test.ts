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

describe('Settings view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/settings')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('uses shared tabs and record/list rows for general and version sections', async () => {
    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    expect(mounted.container.querySelector('[data-testid="settings-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-layout-row-leftSidebarCollapsed"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-version"]')?.click()
    await waitForSelector(mounted.container, '[data-testid="settings-version-row-shell"]')

    expect(mounted.container.querySelector('[data-testid="settings-version-row-shell"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('tauri2')

    mounted.destroy()
  })

  it('renders only the workspace runtime editor and effective preview on the runtime tab', async () => {
    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-runtime"]')?.click()
    await waitForSelector(mounted.container, '[data-testid="settings-runtime-editor-workspace"]')

    expect(mounted.container.querySelector('[data-testid="settings-runtime-editor-workspace"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-editor-project"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-editor-user"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-effective-preview"]')).not.toBeNull()

    mounted.destroy()
  })

  it('shows workspace displayPath metadata instead of absolute source paths', async () => {
    installWorkspaceApiFixture({
      localRuntimeConfigTransform(config) {
        return {
          ...config,
          sources: [
            {
              scope: 'workspace',
              displayPath: 'config/runtime/workspace.json',
              sourceKey: 'workspace',
              exists: false,
              loaded: false,
            },
          ],
        }
      },
    })

    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-runtime"]')?.click()
    await waitForSelector(mounted.container, '[data-testid="settings-runtime-editor-workspace"]')

    const workspaceCard = mounted.container.querySelector('[data-testid="settings-runtime-editor-workspace"]')
    expect(workspaceCard?.textContent).toContain('config/runtime/workspace.json')
    expect(workspaceCard?.textContent).not.toContain('/tmp/octopus-workspace')

    mounted.destroy()
  })
})
