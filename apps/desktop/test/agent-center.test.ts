// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useWorkbenchStore } from '@/stores/workbench'

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

describe('workspace and project agents pages', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders the workspace agents library and switches to teams', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.textContent).toContain('Agents Library')
    expect(mounted.container.textContent).toContain('Reusable intelligence assets available for all projects in this workspace.')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Coder Agent')
    expect(mounted.container.textContent).not.toContain('Launch Readiness Team')

    const teamTab = Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('Teams'))
    teamTab?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Team Library')
    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).not.toContain('Launch Readiness Team')

    mounted.destroy()
  })

  it('filters workspace agents by search text', async () => {
    const mounted = mountApp()

    await nextTick()

    const searchInput = mounted.container.querySelector<HTMLInputElement>('input[placeholder="Search workspace library..."]')
    expect(searchInput).not.toBeNull()

    searchInput!.value = 'coder'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.textContent).toContain('Coder Agent')
    expect(mounted.container.textContent).not.toContain('Architect Agent')

    mounted.destroy()
  })

  it('shows pagination on the workspace teams tab when enough teams exist', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    for (let index = 0; index < 22; index += 1) {
      workbench.createTeam('workspace')
    }

    await nextTick()

    const teamTab = Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('Teams'))
    teamTab?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="ui-pagination"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('1 / 2')

    const nextButtons = Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('button'))
      .filter((button) => button.textContent?.includes('Next'))
    nextButtons[nextButtons.length - 1]?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('2 / 2')

    mounted.destroy()
  })

  it('renders the project agents page and separates project teams via query state', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await nextTick()

    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.textContent).toContain('Project Agents')
    expect(mounted.container.textContent).toContain('Manage intelligence specialized for this project.')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Coder Agent')

    const teamTab = Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('Teams'))
    teamTab?.click()
    await nextTick()

    expect(mounted.container.textContent).toContain('Project Teams')
    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain('Redesign Tiger Team')

    mounted.destroy()
  })
})
