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

  it('renders a productized version center instead of legacy technical rows', async () => {
    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    expect(mounted.container.querySelector('[data-testid="settings-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-layout-row-leftSidebarCollapsed"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-version"]')?.click()
    await waitForSelector(mounted.container, '[data-testid="settings-version-center"]')

    expect(mounted.container.querySelector('[data-testid="settings-version-center"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-version-summary-card"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-version-release-card"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-version-channel-select"]')).not.toBeNull()
    expect(mounted.container.textContent).toMatch(/检查更新|Check for updates/)
    expect(mounted.container.textContent).toMatch(/当前版本|Current version/)
    expect(mounted.container.textContent).not.toContain('tauri2')
    expect(mounted.container.querySelector('[data-testid="settings-version-row-shell"]')).toBeNull()

    mounted.destroy()
  })

  it('keeps only locale and layout controls in general settings and does not expose font family/style controls', async () => {
    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    expect(mounted.container.textContent).toMatch(/显示语言|Locale/)
    expect(mounted.container.textContent).not.toContain('System Font')
    expect(mounted.container.textContent).not.toContain('Font Style')
    expect(mounted.container.textContent).not.toContain('系统字体')
    expect(mounted.container.textContent).not.toContain('字体风格')

    mounted.destroy()
  })

  it('does not expose a runtime tab in settings anymore', async () => {
    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-runtime"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-editor-workspace"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-effective-preview"]')).toBeNull()

    mounted.destroy()
  })

  it('falls back to the general tab when the removed runtime query tab is requested', async () => {
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

    await router.push('/settings?tab=runtime')
    await router.isReady()

    const mounted = mountApp()

    await waitForSelector(mounted.container, '[data-testid="settings-tabs"]')

    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-runtime"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-layout-row-leftSidebarCollapsed"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="settings-runtime-editor-workspace"]')).toBeNull()

    mounted.destroy()
  })
})
