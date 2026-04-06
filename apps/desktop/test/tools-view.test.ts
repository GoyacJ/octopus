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

async function waitForTextToDisappear(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (container.textContent?.includes(value) ?? false) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text to disappear: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function findButton(container: ParentNode, label: string) {
  return Array.from(container.querySelectorAll<HTMLButtonElement>('button'))
    .find(button => button.textContent?.includes(label))
}

function findTabButton(container: ParentNode, label: string) {
  return Array.from(container.querySelectorAll<HTMLButtonElement>('button'))
    .find(button => button.textContent?.includes(label))
}

describe('Workspace tools view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/tools')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders workspace tools from the real catalog store', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'bash')

    expect(mounted.container.textContent).toContain(String(i18n.global.t('sidebar.navigation.tools')))
    expect(mounted.container.textContent).toContain('bash')

    const skillTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.skill')))
    expect(skillTab).toBeDefined()
    skillTab!.click()
    await waitForText(mounted.container, 'help')

    const mcpTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.mcp')))
    expect(mcpTab).toBeDefined()
    mcpTab!.click()
    await waitForText(mounted.container, 'mcp__ops__tail_logs')

    expect(mounted.container.textContent).toContain('ops')
    expect(mounted.container.textContent).toContain('MCP handshake timed out')

    mounted.destroy()
  })

  it('filters runtime-backed entries and shows detail state without edit actions', async () => {
    const mounted = mountApp()

    const mcpTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.mcp')))
    expect(mcpTab).toBeDefined()
    mcpTab!.click()
    await waitForText(mounted.container, 'ops')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'ops'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('ops')
    expect(mounted.container.textContent).not.toContain('help')
    expect(findButton(mounted.container, String(i18n.global.t('common.save')))).toBeUndefined()
    expect(findButton(mounted.container, String(i18n.global.t('common.delete')))).toBeUndefined()

    mounted.destroy()
  })

  it('shows a runtime settings link for MCP entries', async () => {
    const mounted = mountApp()

    const mcpTab = findTabButton(mounted.container, String(i18n.global.t('tools.tabs.mcp')))
    expect(mcpTab).toBeDefined()
    mcpTab!.click()
    await waitForText(mounted.container, 'mcp__ops__tail_logs')

    const settingsLink = Array.from(mounted.container.querySelectorAll<HTMLAnchorElement>('a'))
      .find(link => link.getAttribute('href') === '/settings')
    expect(settingsLink).toBeDefined()

    mounted.destroy()
  })
})
