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

    await waitForText(mounted.container, 'Terminal')

    expect(mounted.container.textContent).toContain(String(i18n.global.t('sidebar.navigation.tools')))
    expect(mounted.container.textContent).toContain('Read')
    expect(mounted.container.textContent).toContain('Terminal')

    mounted.destroy()
  })

  it('updates the selected tool through the workspace API store', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Terminal')

    const toolCards = mounted.container.querySelectorAll<HTMLElement>('article[role="button"]')
    expect(toolCards.length).toBeGreaterThan(1)
    toolCards[1]?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('input')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('textarea')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()

    nameInput!.value = 'Terminal Updated'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Execute shell commands safely.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    findButton(mounted.container, String(i18n.global.t('common.save')))?.click()
    await waitForText(mounted.container, 'Terminal Updated')

    expect(mounted.container.textContent).toContain('Terminal Updated')

    mounted.destroy()
  })

  it('creates and deletes a new tool record without any mock fallback', async () => {
    const mounted = mountApp()

    await waitForText(mounted.container, 'Terminal')

    findButton(mounted.container, String(i18n.global.t('common.reset')))?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('input')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('textarea')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()

    nameInput!.value = 'Ops MCP'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Remote operations connector.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    findButton(mounted.container, String(i18n.global.t('common.save')))?.click()
    await waitForText(mounted.container, 'Ops MCP')

    findButton(mounted.container, String(i18n.global.t('common.delete')))?.click()
    await waitForTextToDisappear(mounted.container, 'Ops MCP')

    mounted.destroy()
  })
})
