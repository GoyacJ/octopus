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

describe('Workspace tools view', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    await router.push('/workspaces/ws-local/tools')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('uses agent-style top tabs and removes the extra tools definition heading', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="tools-title"]')?.textContent).toContain('工作区工具')
    expect(mounted.container.querySelector('[data-testid="tools-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-builtin"]')?.textContent).toContain('内置工具')
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-skill"]')?.textContent).toContain('Skill')
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-mcp"]')?.textContent).toContain('MCP')
    expect(mounted.container.textContent).not.toContain('工具定义')

    mounted.destroy()
  })

  it('keeps description read-only and supports creating and deleting skill entries', async () => {
    const mounted = mountApp()

    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-skill"]')?.click()
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="tools-create-button"]')?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-form-name"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="tools-form-description"]')
    const contentInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="tools-form-content"]')

    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()
    expect(descriptionInput?.disabled).toBe(true)
    expect(contentInput).not.toBeNull()

    nameInput!.value = 'Incident Playbook'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    contentInput!.value = 'Collect impact, timeline, and next mitigation steps.'
    contentInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="tools-form-save"]')?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Incident Playbook')
    expect(mounted.container.querySelector('[data-testid="tools-record-list"]')).not.toBeNull()

    const deleteButton = Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('[data-testid^="tool-delete-"]'))
      .find((button) => button.getAttribute('data-testid')?.includes('skill-ws-local'))
    expect(deleteButton).not.toBeUndefined()

    deleteButton?.click()
    await nextTick()

    expect(mounted.container.textContent).not.toContain('Incident Playbook')

    mounted.destroy()
  })

  it('supports search, pagination, mcp creation, and builtin disable toggles', async () => {
    const mounted = mountApp()

    await nextTick()

    const searchInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-search-input"]')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'terminal'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Terminal')
    expect(mounted.container.textContent).not.toContain('Write')

    searchInput!.value = ''
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-mcp"]')?.click()
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="tools-create-button"]')?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-form-name"]')
    const endpointInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-form-endpoint"]')
    const serverInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-form-server-name"]')
    const toolNamesInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="tools-form-tool-names"]')

    nameInput!.value = 'Ops MCP'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    serverInput!.value = 'ops-mcp'
    serverInput!.dispatchEvent(new Event('input', { bubbles: true }))
    endpointInput!.value = 'https://example.test/mcp/ops'
    endpointInput!.dispatchEvent(new Event('input', { bubbles: true }))
    toolNamesInput!.value = 'list_ops, get_ops'
    toolNamesInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="tools-form-save"]')?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Ops MCP')
    expect(mounted.container.querySelector('[data-testid="tools-pagination-summary"]')?.textContent).toContain('第 1 / 1 页')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-builtin"]')?.click()
    await nextTick()

    const toggleButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="tool-toggle-builtin-read"]')
    expect(toggleButton).not.toBeNull()
    toggleButton?.click()
    await nextTick()

    const readCard = mounted.container.querySelector('[data-testid="tool-item-builtin-read"]')
    expect(readCard?.textContent).toContain('Read')
    expect(readCard?.textContent).toContain('禁用')

    mounted.destroy()
  })
})
