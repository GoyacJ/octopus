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

  it('tracks an active result and moves it with the keyboard', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await flushNavigation()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    const getResults = () => [...document.body.querySelectorAll<HTMLButtonElement>('[data-result-id]')]

    expect(getResults().length).toBeGreaterThan(1)
    expect(getResults()[0]?.dataset.active).toBe('true')
    expect(getResults()[1]?.dataset.active).toBe('false')

    input!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }))
    await flushNavigation()

    expect(getResults()[0]?.dataset.active).toBe('false')
    expect(getResults()[1]?.dataset.active).toBe('true')
    expect(document.body.querySelector('[data-testid="search-overlay-shortcuts"]')).not.toBeNull()

    input!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }))
    await flushNavigation()

    expect(getResults()[0]?.dataset.active).toBe('true')

    mounted.destroy()
  })

  it('keeps the search field frame and active result integrated with the overlay surface', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await flushNavigation()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    const activeResult = document.body.querySelector<HTMLButtonElement>('[data-active="true"]')

    expect(input).not.toBeNull()
    expect(input?.parentElement?.className).not.toContain('shadow-xs')
    expect(activeResult).not.toBeNull()
    expect(activeResult?.className).toContain('border-border-strong')
    expect(activeResult?.className).toContain('bg-accent')
    expect(activeResult?.className).not.toContain('shadow-xs')

    mounted.destroy()
  })

  it('keeps inactive search affordances neutral inside the overlay shell', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await flushNavigation()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    const activeResult = document.body.querySelector<HTMLButtonElement>('[data-active="true"]')
    const inactiveResult = document.body.querySelector<HTMLButtonElement>('[data-active="false"]')
    const shortcutSpans = [...document.body.querySelectorAll<HTMLElement>('[data-testid="search-overlay-shortcuts"] span')]

    const inputShell = input?.parentElement
    const inactiveResultIcon = inactiveResult?.firstElementChild as HTMLElement | null
    const activeResultAction = activeResult?.lastElementChild as HTMLElement | null
    const enterShortcutKey = shortcutSpans.find(element => element.textContent?.trim() === 'Enter')

    expect(inputShell).not.toBeNull()
    expect(inputShell?.className).toContain('bg-subtle')
    expect(inputShell?.className).not.toContain('bg-background')

    expect(inactiveResultIcon).not.toBeNull()
    expect(inactiveResultIcon?.className).toContain('bg-subtle')
    expect(inactiveResultIcon?.className).toContain('text-text-secondary')
    expect(inactiveResultIcon?.className).not.toContain('bg-primary/10')
    expect(inactiveResultIcon?.className).not.toContain('text-primary')

    expect(activeResultAction).not.toBeNull()
    expect(activeResultAction?.className).toContain('bg-surface')
    expect(activeResultAction?.className).not.toContain('bg-background')

    expect(enterShortcutKey).not.toBeNull()
    expect(enterShortcutKey?.className).toContain('bg-surface')
    expect(enterShortcutKey?.className).not.toContain('bg-background')

    mounted.destroy()
  })

  it('renders helper and empty states as integrated command palette sections', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await flushNavigation()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    const shortcuts = document.body.querySelector<HTMLElement>('[data-testid="search-overlay-shortcuts"]')

    expect(shortcuts).not.toBeNull()
    expect(shortcuts?.className).toContain('border-t')
    expect(shortcuts?.className).toContain('bg-subtle')

    expect(input).not.toBeNull()
    input!.value = '__no_results_expected__'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const emptyState = document.body.querySelector<HTMLElement>('[data-testid="search-overlay-empty"]')

    expect(emptyState).not.toBeNull()
    expect(emptyState?.className).toContain('border-border')
    expect(emptyState?.className).toContain('bg-subtle')
    expect(emptyState?.className).not.toContain('shadow-xs')

    mounted.destroy()
  })

  it('opens the active result when pressing enter', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    shell.openSearch()
    await nextTick()

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="search-overlay-input"]')
    expect(input).not.toBeNull()

    input!.value = 'conversation redesign'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await flushNavigation()

    const activeResult = document.body.querySelector<HTMLElement>('[data-result-id="conversation:rt-conv-redesign"]')
    expect(activeResult?.dataset.active).toBe('true')

    input!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushNavigation()

    expect(shell.searchOpen).toBe(false)
    expect(router.currentRoute.value.fullPath).toContain('/conversations/conv-redesign')

    mounted.destroy()
  })
})
