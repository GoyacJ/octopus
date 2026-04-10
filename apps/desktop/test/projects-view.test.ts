// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import ProjectsView from '@/views/workspace/ProjectsView.vue'
import { useShellStore } from '@/stores/shell'
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
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function mountProjectsView(props?: Record<string, unknown>) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(ProjectsView, props)
  app.use(pinia)
  app.use(i18n)
  app.mount(container)

  const shellStore = useShellStore()
  await shellStore.bootstrap('ws-local', 'proj-redesign')
  await nextTick()

  return {
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for projects management state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Workspace project management view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/console/projects')
    await router.isReady()
  })

  it('renders the project management view from workspace project data', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-projects-embedded"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workspace-console-view"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-projects-embedded"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Workspace Governance')
    expect(mounted.container.querySelector('[data-testid="projects-name-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-description-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-default-model-select"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.builtin')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.skill')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.mcp')))

    mounted.destroy()
  })

  it('supports an embedded mode without rendering the standalone page shell', async () => {
    const mounted = await mountProjectsView({ embedded: true })

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-projects-embedded"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workspace-projects-embedded"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-projects-view"]')).toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')

    mounted.destroy()
  })

  it('creates a new active project and persists workspace assignments', async () => {
    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-create-button"]') !== null)

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="projects-description-input"]')
    const defaultModelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="projects-default-model-select"]')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()
    expect(defaultModelSelect).not.toBeNull()

    nameInput!.value = 'Agent Studio'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Project management workspace surface.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))

    mounted.container.querySelector<HTMLElement>('[aria-label="Claude Primary"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="bash"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="help"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="ops"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="Architect Agent"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="Studio Direction Team"]')?.click()
    await nextTick()

    defaultModelSelect!.value = 'anthropic-primary'
    defaultModelSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => mounted.container.textContent?.includes('Agent Studio') ?? false)
    const created = workspaceStore.projects.find(project => project.name === 'Agent Studio')
    expect(created).toBeTruthy()
    expect(created?.assignments?.models?.configuredModelIds).toEqual(['anthropic-primary'])
    expect(created?.assignments?.models?.defaultConfiguredModelId).toBe('anthropic-primary')
    expect(created?.assignments?.tools?.sourceKeys).toEqual([
      'builtin:bash',
      'skill:data/skills/help/SKILL.md',
      'mcp:ops',
    ])
    expect(created?.assignments?.agents?.agentIds).toEqual(['agent-architect'])
    expect(created?.assignments?.agents?.teamIds).toEqual(['team-studio'])

    mounted.destroy()
  })

  it('selects the edited project as the active project scope for downstream project surfaces', async () => {
    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-governance"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-governance"]')?.click()
    await waitFor(() => workspaceStore.currentProjectId === 'proj-governance')

    expect(workspaceStore.currentProjectId).toBe('proj-governance')

    mounted.destroy()
  })

  it('archives the current project, hides it from the sidebar tree, and switches to the next active project', async () => {
    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-redesign"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await waitFor(() => workspaceStore.currentProjectId === 'proj-redesign')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() =>
      workspaceStore.currentProjectId === 'proj-governance'
      && mounted.container.textContent?.includes('Workspace Governance')
      && mounted.container.textContent?.includes('archived'),
    )

    expect(workspaceStore.currentProjectId).toBe('proj-governance')
    expect(mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]')).toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('archived')

    mounted.destroy()
  })

  it('prevents archiving the last active project and surfaces an error', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-governance"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()
    await waitFor(() => mounted.container.textContent?.includes('archived') ?? false)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-governance"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-error"]') !== null)
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projects.errors.lastActiveProject')))

    mounted.destroy()
  })
})
