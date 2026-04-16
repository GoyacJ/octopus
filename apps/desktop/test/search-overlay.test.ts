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

async function flushNavigation() {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await nextTick()
}

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

describe('Workbench search overlay', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture({ preloadConversationMessages: true })
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('opens from the global shortcut and closes with escape', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    await nextTick()

    expect(shell.searchOpen).toBe(true)
    expect(document.body.querySelector('[data-testid="search-overlay-panel"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="search-overlay-input"]')).not.toBeNull()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()

    expect(shell.searchOpen).toBe(false)

    mounted.destroy()
  })

  it('renders conversation results and closes after navigation', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = 'conversation redesign'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const conversationResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="conversation:rt-conv-redesign"]')
    expect(conversationResult).not.toBeNull()
    expect(document.body.textContent).not.toContain('小章 proj-redesign')

    conversationResult?.click()
    await flushNavigation()

    expect(shell.searchOpen).toBe(false)
    expect(router.currentRoute.value.fullPath).toContain('/conversations/conv-redesign')

    mounted.destroy()
  })

  it('routes the navigation smart entry for resources to the project resources surface', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = String(i18n.global.t('sidebar.navigation.resources'))
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const resourcesResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="nav-resources"]')
    expect(resourcesResult).not.toBeNull()

    resourcesResult?.click()
    await flushNavigation()

    expect(router.currentRoute.value.name).toBe('project-resources')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')

    mounted.destroy()
  })

  it('routes the navigation smart entry for deliverables to the project deliverables surface', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = 'deliverable'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const deliverablesResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="nav-deliverables"]')
    expect(deliverablesResult).not.toBeNull()

    deliverablesResult?.click()
    await flushNavigation()

    expect(router.currentRoute.value.name).toBe('project-deliverables')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-local')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')

    mounted.destroy()
  })

  it('returns project deliverables as searchable results', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = 'runtime delivery summary'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const deliverableResult = document.body.querySelector<HTMLButtonElement>('[data-result-id="deliverable:artifact-run-conv-redesign"]')
    expect(deliverableResult).not.toBeNull()

    deliverableResult?.click()
    await flushNavigation()

    expect(router.currentRoute.value.name).toBe('project-deliverables')
    expect(router.currentRoute.value.query.deliverable).toBe('artifact-run-conv-redesign')

    mounted.destroy()
  })
})
