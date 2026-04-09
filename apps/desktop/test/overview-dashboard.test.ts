// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

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

describe('Overview and dashboard views', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the workspace overview inside the shared document shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/overview?project=proj-redesign')

    await waitForSelector(mounted.container, '[data-testid="workspace-overview-view"]')
    await waitForText(mounted.container, 'Desktop Redesign')
    await waitForText(mounted.container, 'Conversation Redesign')

    const overview = mounted.container.querySelector('[data-testid="workspace-overview-view"]')
    expect(overview).not.toBeNull()
    expect(overview?.textContent).toContain('Local Workspace')
    expect(overview?.textContent).toContain('Desktop Redesign')
    expect(overview?.textContent).toContain('Workspace synced')
    expect(overview?.textContent).toContain('Conversation Redesign')

    mounted.destroy()
  })

  it('renders the project dashboard inside the shared document shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/dashboard')

    await waitForSelector(mounted.container, '[data-testid="project-dashboard-view"]')
    await waitForText(mounted.container, 'Desktop Redesign')
    await waitForText(mounted.container, 'Conversation Redesign')

    const dashboard = mounted.container.querySelector('[data-testid="project-dashboard-view"]')
    expect(dashboard).not.toBeNull()
    expect(dashboard?.textContent).toContain('Desktop Redesign')
    expect(dashboard?.textContent).toContain('Conversation Redesign')
    expect(dashboard?.textContent).toContain('Workspace synced')

    mounted.destroy()
  })
})
