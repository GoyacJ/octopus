// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useWorkspaceStore } from '@/stores/workspace'
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

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for project settings state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Project settings view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the project settings form from the project route', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-basics"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-models"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-tools"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-agents"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-name-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-description-input"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')

    mounted.destroy()
  })

  it('loads assigned models, tools, agents, and project members in the project settings tabs', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-models"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('Claude Primary') ?? false)
    expect(mounted.container.textContent).toContain('Claude Primary')
    expect(mounted.container.textContent).toContain('Claude Alt')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-tools"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('builtin') ?? false)
    expect(mounted.container.textContent).toContain('bash')
    expect(mounted.container.textContent).toContain('ops')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-agents"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('Architect Agent') ?? false)
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Studio Direction Team')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-users"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('Lin Zhou') ?? false)
    expect(mounted.container.textContent).toContain('Lin Zhou')
    expect(mounted.container.textContent).toContain('Octopus Owner')

    mounted.destroy()
  })

  it('shows project refinement limited to assigned workspace scope', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-models"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('Claude Primary') ?? false)
    expect(workspaceStore.getProjectSettings('proj-redesign').models?.allowedConfiguredModelIds).toEqual(['anthropic-primary'])
    expect(workspaceStore.getProjectSettings('proj-redesign').models?.defaultConfiguredModelId).toBe('anthropic-primary')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-tools"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('bash') ?? false)
    const toolSelects = [...mounted.container.querySelectorAll('select')]
    expect(toolSelects).toHaveLength(2)
    expect(workspaceStore.getProjectSettings('proj-redesign').tools?.enabledSourceKeys).toEqual(['builtin:bash'])

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-agents"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('Architect Agent') ?? false)
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledAgentIds).toEqual(['agent-architect'])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledTeamIds).toEqual(['team-studio'])

    mounted.destroy()
  })

  it('shows empty project refinement states when no workspace assignments exist', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-governance/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-models"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes(String(i18n.global.t('projectSettings.models.emptyTitle'))) ?? false)
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.models.emptyTitle')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.models.emptyDescription')))
    expect(mounted.container.textContent).not.toContain('Claude Primary')
    expect(mounted.container.textContent).not.toContain('Claude Alt')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-tools"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes(String(i18n.global.t('projectSettings.tools.emptyTitle'))) ?? false)
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.emptyTitle')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.emptyDescription')))
    expect(mounted.container.textContent).not.toContain('bash')
    expect(mounted.container.textContent).not.toContain('ops')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-agents"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes(String(i18n.global.t('projectSettings.agents.emptyTitle'))) ?? false)
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.agents.emptyTitle')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.agents.emptyDescription')))
    expect(mounted.container.textContent).not.toContain('Architect Agent')
    expect(mounted.container.textContent).not.toContain('Studio Direction Team')

    mounted.destroy()
  })

  it('updates project basics and keeps sidebar and topbar in sync', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-save-button"]')

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="project-settings-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="project-settings-description-input"]')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()

    nameInput!.value = 'Redesign HQ'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Refined launch scope for the desktop refresh.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-save-button"]')?.click()

    await waitFor(() =>
      (mounted.container.textContent?.includes('Redesign HQ') ?? false)
      && (mounted.container.textContent?.includes('Refined launch scope for the desktop refresh.') ?? false),
    )

    expect(mounted.container.textContent).toContain('Redesign HQ')
    expect(mounted.container.textContent).toContain('Refined launch scope for the desktop refresh.')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('sidebar.navigation.projectSettings')))

    mounted.destroy()
  })
})
