// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import type { WorkspaceClient } from '@/tauri/workspace-client'
import * as tauriClient from '@/tauri/client'
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

function configureWorkspaceClient(
  transform: (client: WorkspaceClient) => WorkspaceClient,
) {
  const createWorkspaceClientMock = vi.mocked(tauriClient.createWorkspaceClient)
  const baseImplementation = createWorkspaceClientMock.getMockImplementation()
  expect(baseImplementation).toBeTypeOf('function')

  createWorkspaceClientMock.mockImplementation((context) =>
    transform(baseImplementation!(context) as WorkspaceClient) as ReturnType<typeof tauriClient.createWorkspaceClient>,
  )
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
    i18n.global.locale.value = 'en-US'
    installWorkspaceApiFixture({ locale: 'en-US' })
    document.body.innerHTML = ''
  })

  it('renders the workspace overview inside the shared document shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/overview?project=proj-redesign')

    await waitForSelector(mounted.container, '[data-testid="workspace-overview-view"]')
    await waitForText(mounted.container, 'Desktop Redesign')
    await waitForText(mounted.container, 'Conversation Redesign')
    await waitForText(mounted.container, '125,000')

    const overview = mounted.container.querySelector('[data-testid="workspace-overview-view"]')
    expect(overview).not.toBeNull()
    expect(overview?.textContent).toContain('Local Workspace')
    expect(overview?.textContent).toContain('Desktop Redesign')
    expect(overview?.textContent).toContain('Workspace synced')
    expect(overview?.textContent).toContain('Conversation Redesign')
    expect(overview?.textContent).toContain('125,000')

    mounted.destroy()
  })

  it('loads the overview project dashboard only once on first render', async () => {
    let dashboardCalls = 0
    configureWorkspaceClient(client => ({
      ...client,
      projects: {
        ...client.projects,
        async getDashboard(projectId) {
          dashboardCalls += 1
          return await client.projects.getDashboard(projectId)
        },
      },
    }))

    const mounted = await mountRoutedApp('/workspaces/ws-local/overview?project=proj-redesign')

    await waitForSelector(mounted.container, '[data-testid="workspace-overview-view"]')
    await waitForText(mounted.container, 'Conversation Redesign')

    expect(dashboardCalls).toBe(1)

    mounted.destroy()
  })

  it('renders the project dashboard inside the shared document shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/dashboard')

    await waitForSelector(mounted.container, '[data-testid="project-dashboard-view"]')
    await waitForText(mounted.container, 'Desktop Redesign')
    await waitForText(mounted.container, 'Sessions')
    await waitForText(mounted.container, 'Resources')
    await waitForText(mounted.container, 'Usage trend')
    await waitForText(mounted.container, 'Top contributors')
    await waitForText(mounted.container, 'Tool usage')
    await waitForText(mounted.container, 'Approval queue')
    await waitForText(mounted.container, '125,000')
    await waitForText(mounted.container, 'Conversation Redesign')
    await waitForText(mounted.container, 'Runtime-only conversation state is active.')
    await waitForText(mounted.container, 'Workspace synced')
    await waitForText(mounted.container, 'Bootstrap and projections loaded.')

    const dashboard = mounted.container.querySelector('[data-testid="project-dashboard-view"]')
    expect(dashboard).not.toBeNull()
    expect(dashboard?.textContent).toContain('Desktop Redesign')
    expect(dashboard?.textContent).toContain('Conversation Redesign')
    expect(dashboard?.textContent).toContain('Sessions')
    expect(dashboard?.textContent).toContain('Resources')
    expect(dashboard?.textContent).toContain('Usage trend')
    expect(dashboard?.textContent).toContain('Top contributors')
    expect(dashboard?.textContent).toContain('Tool usage')
    expect(dashboard?.textContent).toContain('Approval queue')
    expect(dashboard?.textContent).toContain('125,000')
    expect(dashboard?.textContent).toContain('Workspace synced')
    expect(dashboard?.textContent).toContain('Runtime-only conversation state is active.')
    expect(dashboard?.textContent).toContain('Bootstrap and projections loaded.')

    mounted.destroy()
  })

  it('loads the project dashboard only once on first render', async () => {
    let dashboardCalls = 0
    configureWorkspaceClient(client => ({
      ...client,
      projects: {
        ...client.projects,
        async getDashboard(projectId) {
          dashboardCalls += 1
          return await client.projects.getDashboard(projectId)
        },
      },
    }))

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/dashboard')

    await waitForSelector(mounted.container, '[data-testid="project-dashboard-view"]')
    await waitForText(mounted.container, 'Desktop Redesign')

    expect(dashboardCalls).toBe(1)

    mounted.destroy()
  })
})
