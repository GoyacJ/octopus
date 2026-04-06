// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, h, nextTick } from 'vue'

import i18n from '@/plugins/i18n'
import { router } from '@/router'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { useShellStore } from '@/stores/shell'

async function flushNavigation() {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await nextTick()
}

function mountLayout() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp({
    render: () => h(WorkbenchLayout, null, {
      default: () => h('div', { 'data-testid': 'workbench-slot' }, 'slot'),
    }),
  })

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

describe('Workbench search overlay', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('opens from the global shortcut and closes with escape', async () => {
    const mounted = mountLayout()
    const shell = useShellStore()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    await nextTick()

    expect(shell.searchOpen).toBe(true)
    expect(document.body.querySelector('[data-testid="search-overlay-dialog"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="search-overlay-panel"]')).not.toBeNull()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()

    expect(shell.searchOpen).toBe(false)

    mounted.destroy()
  })

  it('renders local navigation and conversation results and closes after navigation', async () => {
    const mounted = mountLayout()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = 'conversation'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const conversationResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="conversation-conv-redesign"]')
    expect(conversationResult).not.toBeNull()

    conversationResult?.click()
    await flushNavigation()

    expect(shell.searchOpen).toBe(false)
    expect(router.currentRoute.value.fullPath).toContain('/conversations/')

    mounted.destroy()
  })

  it('routes the navigation smart entry for agents to the workspace-level agent center', async () => {
    const mounted = mountLayout()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = '智能体'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const agentsResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="nav-agents"]')
    expect(agentsResult).not.toBeNull()

    agentsResult?.click()
    await flushNavigation()

    expect(router.currentRoute.value.name).toBe('workspace-agents')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')

    mounted.destroy()
  })
})
